use rusqlite::{params, Connection, Result};
use sia_core::sia_client::DEFAULT_CHUNK_SIZE;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct PendingChunkTask {
    pub file_id: String,
    pub session_id: String,
    pub local_path: String,
    pub rel_path: String,
    pub total_file_size: u64,
    pub total_chunks: u32,
    pub chunk_size: u64,
    pub chunk_index: u32,
    pub offset_bytes: u64,
    pub length_bytes: u64,
    /// Номер текущей попытки (уже увеличен при claim) — для экспоненциального backoff
    pub attempts: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: i64,
    pub last_error: Option<String>,
    pub seed_hex: Option<String>,
    pub total: i64,
    pub uploaded: i64,
    pub failed: i64,
}

#[derive(Debug, Clone)]
pub struct FinalizeTask {
    pub session_id: String,
    pub name: String,
    pub expires_at: Option<i64>,
    /// Seed уже созданного SharingKey (hex) — переиспользуется при повторе финализации
    pub seed_hex: Option<String>,
}

impl Database {
    pub fn init(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // WAL-режим позволяет фоновым воркерам параллельно читать и писать очередь
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS upload_sessions(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                -- uploading -> finalizing -> completed | failed | revoked
                status TEXT NOT NULL,
                sharing_key_seed TEXT,
                last_error TEXT,
                expires_at INTEGER,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS upload_files(
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES upload_sessions(id) ON DELETE CASCADE,
                local_path TEXT NOT NULL,
                rel_path TEXT NOT NULL,
                total_size INTEGER NOT NULL,
                chunk_size INTEGER NOT NULL,
                total_chunks INTEGER NOT NULL
            );
            -- Страховка от склейки одноименных файлов в один (дедуп делает collect)
            CREATE UNIQUE INDEX IF NOT EXISTS idx_files_rel ON upload_files(session_id, rel_path);
            CREATE TABLE IF NOT EXISTS upload_chunks(
                file_id TEXT NOT NULL REFERENCES upload_files(id) ON DELETE CASCADE,
                chunk_index INTEGER NOT NULL,
                offset_bytes INTEGER NOT NULL,
                length_bytes INTEGER NOT NULL,
                -- pending | uploading | uploaded | failed
                status TEXT NOT NULL,
                -- Hex Hash256 закрепленного объекта. НЕ serde SDK-типов:
                -- полный Object догружается по id при финализации.
                sia_object_id TEXT,
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                PRIMARY KEY(file_id, chunk_index)
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_status ON upload_chunks(status);
            ",
        )?;
        // Восстанавливаем чанки, зависшие при закрытии программы. Безопасно только
        // при single-instance: второй экземпляр сбросил бы чанки работающего первого.
        conn.execute(
            "UPDATE upload_chunks SET status='pending' WHERE status='uploading'",
            [],
        )?;
        // Сессии в 'finalizing' НЕ трогаем: финализатор продолжит их с сохраненным seed.
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Создает сессию и нарезает файлы на чанки по 256 МиБ.
    pub fn create_session(
        &self,
        session_name: &str,
        files: &[(String, String, u64)], // (local_path, rel_path, total_size)
        expires_at: i64,
    ) -> Result<String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let session_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        tx.execute(
            "INSERT INTO upload_sessions(id,name,status,expires_at,created_at) VALUES(?1,?2,'uploading',?3,?4)",
            params![session_id, session_name, expires_at, now],
        )?;
        for (local_path, rel_path, total_size) in files {
            let file_id = Uuid::new_v4().to_string();
            let total_chunks = if *total_size == 0 {
                1
            } else {
                (*total_size + DEFAULT_CHUNK_SIZE - 1) / DEFAULT_CHUNK_SIZE
            };
            tx.execute(
                "INSERT INTO upload_files(id,session_id,local_path,rel_path,total_size,chunk_size,total_chunks) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![file_id, session_id, local_path, rel_path, *total_size as i64, DEFAULT_CHUNK_SIZE as i64, total_chunks],
            )?;
            for idx in 0..total_chunks {
                let offset = idx as u64 * DEFAULT_CHUNK_SIZE;
                let length = if *total_size == 0 {
                    0
                } else {
                    std::cmp::min(DEFAULT_CHUNK_SIZE, total_size - offset)
                };
                tx.execute(
                    "INSERT INTO upload_chunks(file_id,chunk_index,offset_bytes,length_bytes,status) VALUES(?1,?2,?3,?4,'pending')",
                    params![file_id, idx, offset as i64, length as i64],
                )?;
            }
        }
        tx.commit()?;
        Ok(session_id)
    }

    /// Атомарно забирает пачку ожидающих чанков (старейшая сессия первой) и
    /// переводит их в 'uploading', сразу увеличивая счетчик попыток (переживает
    /// даже kill -9 посреди загрузки). Мелкие чанки набираются до
    /// max_logical_bytes, чтобы упаковать их в один слаб; чанк больше лимита
    /// уходит одиночной пачкой.
    pub fn claim_next_pending_batch(
        &self,
        max_logical_bytes: u64,
    ) -> Result<Vec<PendingChunkTask>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let candidates: Vec<PendingChunkTask> = {
            let mut stmt = tx.prepare(
                "SELECT c.file_id, f.session_id, f.local_path, f.rel_path, f.total_size,
                        f.total_chunks, f.chunk_size, c.chunk_index, c.offset_bytes, c.length_bytes, c.attempts
                 FROM upload_chunks c
                 JOIN upload_files f ON c.file_id=f.id
                 JOIN upload_sessions s ON f.session_id=s.id
                 WHERE c.status='pending' AND s.status='uploading'
                 ORDER BY s.created_at ASC, f.id ASC, c.chunk_index ASC LIMIT 1024",
            )?;
            let mut rows = stmt.query([])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                out.push(PendingChunkTask {
                    file_id: row.get(0)?,
                    session_id: row.get(1)?,
                    local_path: row.get(2)?,
                    rel_path: row.get(3)?,
                    total_file_size: row.get::<_, i64>(4)? as u64,
                    total_chunks: row.get(5)?,
                    chunk_size: row.get::<_, i64>(6)? as u64,
                    chunk_index: row.get(7)?,
                    offset_bytes: row.get::<_, i64>(8)? as u64,
                    length_bytes: row.get::<_, i64>(9)? as u64,
                    attempts: row.get::<_, u32>(10)? + 1,
                });
            }
            out
        };
        let mut batch = Vec::new();
        let mut total = 0u64;
        for t in candidates {
            if !batch.is_empty() && total + t.length_bytes > max_logical_bytes {
                break;
            }
            total += t.length_bytes;
            batch.push(t);
            if total >= max_logical_bytes {
                break;
            }
        }
        for t in &batch {
            tx.execute(
                "UPDATE upload_chunks SET status='uploading', attempts=attempts+1 WHERE file_id=?1 AND chunk_index=?2",
                params![t.file_id, t.chunk_index],
            )?;
        }
        tx.commit()?;
        Ok(batch)
    }

    /// Фиксирует успешную загрузку. Возвращает ПРЕЖНИЙ id объекта, если он был
    /// (повторная загрузка после сбоя) — вызывающий удалит сироту, чтобы не текла квота.
    pub fn mark_chunk_uploaded(
        &self,
        file_id: &str,
        chunk_index: u32,
        object_id_hex: &str,
    ) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let old: Option<String> = conn.query_row(
            "SELECT sia_object_id FROM upload_chunks WHERE file_id=?1 AND chunk_index=?2",
            params![file_id, chunk_index],
            |r| r.get(0),
        )?;
        conn.execute(
            "UPDATE upload_chunks SET status='uploaded', sia_object_id=?1, last_error=NULL WHERE file_id=?2 AND chunk_index=?3",
            params![object_id_hex, file_id, chunk_index],
        )?;
        Ok(old.filter(|o| o != object_id_hex))
    }

    /// Ошибка чанка: до max_attempts — обратно в pending (retry), иначе — failed.
    /// Возвращает true, если чанк окончательно провалился.
    pub fn record_chunk_error(
        &self,
        file_id: &str,
        chunk_index: u32,
        err: &str,
        max_attempts: u32,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let attempts: u32 = conn.query_row(
            "SELECT attempts FROM upload_chunks WHERE file_id=?1 AND chunk_index=?2",
            params![file_id, chunk_index],
            |r| r.get(0),
        )?;
        let failed = attempts >= max_attempts;
        conn.execute(
            "UPDATE upload_chunks SET status=?1, last_error=?2 WHERE file_id=?3 AND chunk_index=?4",
            params![
                if failed { "failed" } else { "pending" },
                err,
                file_id,
                chunk_index
            ],
        )?;
        if failed {
            // Проваленный чанк делает невозможной всю сессию — помечаем сразу
            conn.execute(
                "UPDATE upload_sessions SET status='failed', last_error=?1
                 WHERE id=(SELECT session_id FROM upload_files WHERE id=?2)",
                params![err, file_id],
            )?;
        }
        Ok(failed)
    }

    /// (всего, загружено, провалено) — для прогресса сессии в UI.
    pub fn session_chunk_counts(&self, session_id: &str) -> Result<(i64, i64, i64)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*), SUM(c.status='uploaded'), SUM(c.status='failed')
             FROM upload_chunks c JOIN upload_files f ON c.file_id=f.id WHERE f.session_id=?1",
            params![session_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                ))
            },
        )
    }

    /// Все сессии с прогрессом — для восстановления UI после перезапуска.
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.status, s.created_at, s.last_error, s.sharing_key_seed,
                    COUNT(c.chunk_index), COALESCE(SUM(c.status='uploaded'),0), COALESCE(SUM(c.status='failed'),0)
             FROM upload_sessions s
             LEFT JOIN upload_files f ON f.session_id=s.id
             LEFT JOIN upload_chunks c ON c.file_id=f.id
             GROUP BY s.id
             ORDER BY s.created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionInfo {
                id: r.get(0)?,
                name: r.get(1)?,
                status: r.get(2)?,
                created_at: r.get(3)?,
                last_error: r.get(4)?,
                seed_hex: r.get(5)?,
                total: r.get(6)?,
                uploaded: r.get(7)?,
                failed: r.get(8)?,
            })
        })?;
        rows.collect()
    }

    /// Сессии, еще формально в 'uploading' — кандидаты на проверку готовности.
    pub fn uploading_session_ids(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM upload_sessions WHERE status='uploading'")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    /// Атомарный переход uploading -> finalizing, только если ВСЕ чанки uploaded.
    /// Побеждает ровно один вызов (UPDATE ... WHERE status='uploading').
    pub fn try_begin_finalize(&self, session_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE upload_sessions SET status='finalizing'
             WHERE id=?1 AND status='uploading'
               AND NOT EXISTS (
                 SELECT 1 FROM upload_chunks c JOIN upload_files f ON c.file_id=f.id
                 WHERE f.session_id=?1 AND c.status != 'uploaded')",
            params![session_id],
        )?;
        Ok(n == 1)
    }

    /// Следующая сессия для финализатора (единственный потребитель — гонок нет).
    pub fn next_finalizing_session(&self) -> Result<Option<FinalizeTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id,name,expires_at,sharing_key_seed FROM upload_sessions
             WHERE status='finalizing' ORDER BY created_at ASC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        Ok(rows
            .next()?
            .map(|r| -> Result<FinalizeTask> {
                Ok(FinalizeTask {
                    session_id: r.get(0)?,
                    name: r.get(1)?,
                    expires_at: r.get(2)?,
                    seed_hex: r.get(3)?,
                })
            })
            .transpose()?)
    }

    /// Сохраняем seed СРАЗУ после создания ключа: повтор финализации не должен
    /// плодить новые ключи (старый остался бы валидным у неизвестно кого).
    pub fn save_session_seed(&self, session_id: &str, seed_hex: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE upload_sessions SET sharing_key_seed=?1 WHERE id=?2",
            params![seed_hex, session_id],
        )?;
        Ok(())
    }

    pub fn mark_session_completed(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE upload_sessions SET status='completed', last_error=NULL WHERE id=?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub fn mark_session_failed(&self, session_id: &str, err: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE upload_sessions SET status='failed', last_error=?1 WHERE id=?2",
            params![err, session_id],
        )?;
        Ok(())
    }

    pub fn mark_session_revoked(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE upload_sessions SET status='revoked' WHERE id=?1",
            params![session_id],
        )?;
        Ok(())
    }

    /// Все id объектов сессии в порядке склейки.
    pub fn session_object_ids(&self, session_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.sia_object_id FROM upload_chunks c JOIN upload_files f ON c.file_id=f.id
             WHERE f.session_id=?1 AND c.sia_object_id IS NOT NULL
             ORDER BY f.rel_path ASC, c.chunk_index ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    /// Seed раздачи — нужен для досрочного отзыва доступа.
    pub fn session_seed(&self, session_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT sharing_key_seed FROM upload_sessions WHERE id=?1",
            params![session_id],
            |r| r.get(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_db_chunking_and_recovery() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::init(&db_path).unwrap();
        // Файл 600 МиБ -> 3 чанка (256 + 256 + 88)
        let files = vec![("C:\\big.iso".into(), "big.iso".into(), 600 * 1024 * 1024u64)];
        let sid = db.create_session("Test", &files, 999999999).unwrap();

        let c0 = db.claim_next_pending_batch(DEFAULT_CHUNK_SIZE).unwrap();
        assert_eq!(c0.len(), 1); // чанк 256 МиБ больше лимита паковки — одиночная пачка
        let c0 = &c0[0];
        assert_eq!(c0.chunk_index, 0);
        assert_eq!(c0.length_bytes, DEFAULT_CHUNK_SIZE);
        assert_eq!(c0.attempts, 1);

        // Имитируем перезапуск программы при зависшем чанке c0
        let db2 = Database::init(&db_path).unwrap();
        let c0_again = db2.claim_next_pending_batch(DEFAULT_CHUNK_SIZE).unwrap();
        assert_eq!(c0_again[0].chunk_index, 0);
        assert_eq!(c0_again[0].attempts, 2);

        db2.mark_chunk_uploaded(&c0_again[0].file_id, 0, &"aa".repeat(32))
            .unwrap();
        assert!(!db2.try_begin_finalize(&sid).unwrap()); // еще 2 чанка
    }

    #[test]
    fn test_batch_packs_small_chunks() {
        let dir = tempdir().unwrap();
        let db = Database::init(&dir.path().join("t.db")).unwrap();
        let files: Vec<(String, String, u64)> = (0..5)
            .map(|i| (format!("p{i}"), format!("f{i}"), 1000))
            .collect();
        db.create_session("S", &files, 0).unwrap();
        let batch = db.claim_next_pending_batch(40 * 1024 * 1024).unwrap();
        assert_eq!(batch.len(), 5); // все мелкие чанки — в одной пачке (один слаб)
        assert!(db
            .claim_next_pending_batch(40 * 1024 * 1024)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_finalize_flow_and_seed_reuse() {
        let dir = tempdir().unwrap();
        let db = Database::init(&dir.path().join("t.db")).unwrap();
        let sid = db
            .create_session("S", &[("p".into(), "f.bin".into(), 10)], 123)
            .unwrap();
        let t = db
            .claim_next_pending_batch(DEFAULT_CHUNK_SIZE)
            .unwrap()
            .remove(0);
        db.mark_chunk_uploaded(&t.file_id, 0, &"bb".repeat(32))
            .unwrap();
        assert!(db.try_begin_finalize(&sid).unwrap());
        assert!(!db.try_begin_finalize(&sid).unwrap()); // повторно — уже finalizing
        let f = db.next_finalizing_session().unwrap().unwrap();
        assert_eq!(f.session_id, sid);
        assert_eq!(f.expires_at, Some(123));
        db.save_session_seed(&sid, &"cc".repeat(32)).unwrap();
        assert_eq!(
            db.next_finalizing_session().unwrap().unwrap().seed_hex,
            Some("cc".repeat(32))
        );
        db.mark_session_completed(&sid).unwrap();
        assert!(db.next_finalizing_session().unwrap().is_none());
        assert_eq!(db.session_object_ids(&sid).unwrap(), vec!["bb".repeat(32)]);
    }

    #[test]
    fn test_list_sessions() {
        let dir = tempdir().unwrap();
        let db = Database::init(&dir.path().join("t.db")).unwrap();
        let sid = db
            .create_session("S", &[("p".into(), "f".into(), 10)], 0)
            .unwrap();
        let t = db
            .claim_next_pending_batch(DEFAULT_CHUNK_SIZE)
            .unwrap()
            .remove(0);
        db.mark_chunk_uploaded(&t.file_id, 0, &"bb".repeat(32))
            .unwrap();
        let list = db.list_sessions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, sid);
        assert_eq!(list[0].status, "uploading");
        assert_eq!((list[0].total, list[0].uploaded, list[0].failed), (1, 1, 0));
        assert!(list[0].seed_hex.is_none());
    }

    #[test]
    fn test_failed_after_max_attempts() {
        let dir = tempdir().unwrap();
        let db = Database::init(&dir.path().join("t.db")).unwrap();
        let sid = db
            .create_session("S", &[("p".into(), "f".into(), 10)], 0)
            .unwrap();
        for i in 1..=3 {
            let t = db
                .claim_next_pending_batch(DEFAULT_CHUNK_SIZE)
                .unwrap()
                .remove(0);
            assert_eq!(t.attempts, i);
            let failed = db.record_chunk_error(&t.file_id, 0, "net", 3).unwrap();
            assert_eq!(failed, i == 3);
        }
        assert!(db
            .claim_next_pending_batch(DEFAULT_CHUNK_SIZE)
            .unwrap()
            .is_empty());
        assert!(!db.try_begin_finalize(&sid).unwrap());
    }
}
