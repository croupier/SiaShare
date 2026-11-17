# SiaShare — вводный контекст для кодинг-агента

Сверено по `CONTEXT_DUMP.md` от 2026-11-17 (реальные файлы, манифесты, сигнатуры
Tauri-команд и публичный API `sia_storage` из кэша cargo) и по истории проекта.
**Источник правды — файлы в репозитории.** Код в чатах местами устарел.

Корень: `D:\_sites\3406\SiaShare`. Все команды — отсюда, где лежит корневой `Cargo.toml`.

## 1. Продукт

Бессерверный файлообменник поверх децентрализованной сети **Sia Storage**. Своего
бэкенда, домена, хостинга и сертификатов нет: клиенты ходят напрямую к индексатору
и хостам Sia.

1. **Отправитель** (`uploader-desktop`) грузит файлы/папки чанками по 256 МиБ,
   каждый чанк сразу пиннится. Очередь — SQLite (WAL), переживает закрытие программы.
2. По завершении создаётся **SharingKey** (read-only, со сроком жизни) и собирается
   `Sia.zip` с портативным `receiver-micro` внутри; ключ зашит в **имя exe**.
3. **Получатель** распаковывает архив, запускает `S#r_….exe`. Клиент читает ключ из
   собственного имени файла, подключается как `SharedSdk`, качает поштучно или ZIP.
   Регистрация, аккаунт и монеты получателю не нужны — платит владелец ключа.

## 2. Цели и принятые решения

| Цель | Как решено | Где |
|---|---|---|
| Обмен без своего сервера | Официальный Rust SDK `sia_storage` 0.13 + хостед-индексатор `https://sia.storage` | `sia_client.rs` |
| Портал не видит содержимое | SDK шифрует данные и метаданные на клиенте до отправки | `upload_chunk` |
| Получателю не нужен аккаунт | `SharingKey` вместо `AppKey`; `SharedSdk::connect(url, seed)` | `connect_shared` |
| Ключ не утекает в сеть | Ключ — в имени файла, не в URL и не в теле запроса. URL-схемы и deep-link нет намеренно | `crypto.rs` |
| Опциональный пароль | Argon2id(19456, t=2, p=1) → AES-256-GCM; блоб `[salt16][nonce12][ct+tag16]`; префикс `p1_` | `crypto.rs` |
| Загрузка 10+ ГБ переживает перезапуск | Чанки 256 МиБ + немедленный `pin_object` + очередь SQLite + сброс `uploading→pending` при старте | `db.rs`, `worker.rs` |
| Большие файлы без раздувания RAM | `AsyncReadExt::take`, `max_buffered_slabs=2`, `max_buffered_chunks=64`, буфер 256 КиБ | `sia_client.rs`, `downloader.rs` |
| Не выдать битый файл как «целый» | Непрерывность `chunk_idx` 0..n−1, сходимость размеров, `downloaded == total_size` перед rename | `group_summaries_into_tree`, `download_single_file` |
| Докачка | Отправитель — по чанкам из БД; получатель — до байта через `DownloadOptions.offset` и `.part` | `downloader.rs` |
| Отзыв доступа | `revoke_sharing_key`; выданные токены доживают ~5 минут | `revoke_share` |
| AppKey не в plaintext | Системный keychain (`keyring` v3), запись `siashare-desktop` / `sia-app-key` | `commands.rs` |
| Папки и ZIP >4 ГиБ | Синхронный `zip` с `large_file(true)` на `spawn_blocking`, мост async→sync через `Handle::block_on` | `download_as_zip` |
| Малый переносимый receiver | Tauri v2 + Rust-ядро + тонкий Svelte; release: `opt-level="z"`, `lto`, `panic="abort"`, `strip` | корневой `Cargo.toml` |
| Прогресс без флуда и «заморозок» | Колбэки `ShardProgress` + троттлинг 250 мс (upload) / 200 мс (download) | `worker.rs`, `downloader.rs` |
| Повторный вход не оставляет старый SDK | Единый источник правды — `watch::Sender<Option<Arc<Sdk>>>`; отдельного `Mutex` нет | `UploaderState` |

## 3. Архитектура и карта файлов

```
Svelte 4 + TS (UI)  ──invoke──▶  Rust (Tauri v2)  ──HTTPS/RHP4──▶  индексатор + хосты Sia
        ▲                              │
        └────────── emit ──────────────┘
```

UI не знает про Sia и крипту. Вся сеть, файлы и криптография — в Rust.
Размеры ниже — реальные строки кода, чтобы понимать, где живёт логика.

```
Cargo.toml                       workspace: 3 крейта, общие версии зависимостей
crates/sia-core/src/             ядро без зависимости от Tauri (821 строка)
  sia_client.rs   305            SDK-обёртка: вход, квота, загрузка, шаринг, дерево
  downloader.rs   246            .part-докачка, ZIP64, отмена, троттлинг
  crypto.rs       202            имя S#…, Argon2id+AES-GCM, parse_payload, key.sia
  models.rs        61            ChunkMetadata, SharedFileEntry, sanitize_rel_path
  lib.rs            7            только pub mod
apps/uploader-desktop/src-tauri/src/   (766 строк)
  db.rs           346            схема SQLite, claim/finalize, счётчики, тесты
  commands.rs     210            6 Tauri-команд + обход папок + дедуп путей
  worker.rs       168            пул воркеров + финализатор
  main.rs          42            плагины, setup, manage
apps/uploader-desktop/src/App.svelte   208
apps/receiver-micro/src-tauri/src/     (144 строки)
  commands.rs     112            6 Tauri-команд, гейт скачивания
  main.rs          32            без single-instance — намеренно
apps/receiver-micro/src/App.svelte     183
.github/workflows/ci-cd.yml     109    CI + релиз по тегу v* на трёх ОС
build.ps1 / build.sh             17    receiver → resources → установщики
```

Публичный API `sia-core` (строки — точки входа при навигации):

- `sia_client`: `DEFAULT_INDEXER_URL`:13, `DEFAULT_CHUNK_SIZE`:14, `APP_META`:20,
  `SiaClientError`:30, `parse_object_id`:56, `connect_with_app_key`:63,
  `begin_connection`:74, `finish_connection`:86, `ensure_capacity`:114,
  `upload_chunk`:133, `share_session_objects`:170, `revoke_share`:194,
  `connect_shared`:201, `fetch_shared_tree`:210, `group_summaries_into_tree`:228
- `downloader`: `calculate_resume_chunk`:20, `download_single_file`:62, `download_as_zip`:159
- `crypto`: `CryptoError`:12, `ExtractedKey`:27, `build_receiver_filename`:40,
  `extract_key_from_path`:66, `parse_payload`:86, `decrypt_with_password`:123
  (`encrypt_with_password` и `derive_key` — приватные)
- `models`: `ChunkMetadata`:6, `SharedFileEntry`:21, `sanitize_rel_path`:37

Стек фронта: **Svelte 4.2 + Vite 5 + TypeScript**, без SvelteKit, менеджер — **npm**.
Порты dev: отправитель **1420** (HMR 1421), получатель **1430** (HMR 1431).

## 4. Контракт IPC (сверен по коду; менять только с согласия владельца)

Аргументы `invoke` — camelCase (Tauri конвертирует), поля событий — **snake_case**.

**Отправитель**
| Команда | Аргументы | Возврат |
|---|---|---|
| `check_saved_auth` | — | `bool` |
| `connect_sia` | `seedPhrase: string \| null` | `()` |
| `start_upload_session` | `sessionName, paths: string[], validDays` | `sessionId` |
| `generate_share_zip` | `outputZipPath, sharingKeyBytes: number[], password` | `()` |
| `revoke_session` | `sessionId` | `()` |
| `get_session_progress` | `sessionId` | `[total, uploaded, failed]` |

События: `auth-url` (string), `auth-phrase` (string), `chunk-progress`
`{session_id,file_id,chunk_index,fraction}`, `session-progress` `{session_id,uploaded,total,failed}`,
`chunk-uploaded` (session_id), `chunk-error` `{session_id,chunk_index,error,failed}`,
`session-completed` `[sid, number[]]`, `session-failed` `[sid, string]`.

**Получатель**
| Команда | Аргументы | Возврат |
|---|---|---|
| `init_receiver` | — | `{status:'Ready'\|'PasswordRequired'\|'Error', data}` |
| `submit_key_payload` | `payload` | то же |
| `unlock_with_password` | `password` | `SharedFileEntry[]` |
| `download_one` | `relPath, saveTo` | `()` |
| `download_selected_zip` | `relPaths: string[], saveZipTo` | `()` |
| `cancel_download` | — | `()` |

Событие: `download-progress` `{downloaded, total}`.

**Плагины (фактически подключены).** Отправитель: `single-instance` (первым!),
`dialog`, `opener`; permissions: `core:default`, `dialog:default`,
`opener:allow-open-url` со scope `https://sia.storage/*`.
Получатель: только `dialog`; permissions: `core:default`, `dialog:default`.
**`plugin-fs` не подключён нигде** — см. раздел 10.

## 5. Форматы и состояния

**Имя файла получателя:** `S#r_<base64url(32 байта)>.exe` или `S#p1_<base64url(блоб)>.exe`.
Лимит имени — 165 символов (запас под `MAX_PATH`). Парсер терпит хвост ` (1)` от браузера.
Fallback — `key.sia` рядом с бинарником (подъём до 4 уровней, для macOS `.app`).

**ChunkMetadata** (JSON в `Object.metadata`, шифруется SDK):
`rel_path, chunk_idx, total_chunks, total_file_size, chunk_size`.
`rel_path` — только прямые слеши, без `..`, `.`, пустых сегментов, `:` и `\`.

**SQLite** (`%APPDATA%\com.siashare.uploader\queue.db`, WAL):
- `upload_sessions(id,name,status,sharing_key_seed,last_error,expires_at,created_at)`
  статусы: `uploading → finalizing → completed | failed | revoked`
- `upload_files(id,session_id,local_path,rel_path,total_size,chunk_size,total_chunks)`
  + UNIQUE(session_id, rel_path)
- `upload_chunks(file_id,chunk_index,offset_bytes,length_bytes,status,sia_object_id,attempts,last_error)`
  статусы: `pending | uploading | uploaded | failed`

В БД — **hex-id объекта (`Hash256`)**, не сериализация SDK-типов (`Object` не `Serialize`).
При старте `uploading`-чанки сбрасываются в `pending`; сессии в `finalizing` не трогаются.

**Константы:** чанк 256 МиБ, 3 воркера, `MAX_CHUNK_ATTEMPTS=8`, `MAX_FINALIZE_ATTEMPTS=5`,
страница пагинации 100, backoff чанков 5·2^n (потолок 300 с), финализации 10·2^n (потолок 300 с).

## 6. Инварианты — нельзя ломать

1. **`APP_META.id` (`sia_client.rs:20`) неизменен навсегда.** Из него деривируются
   ключи шифрования: смена = потеря доступа ко всем ранее загруженным данным.
2. Получателю никогда не отдаётся `AppKey` — только `SharingKey`.
3. Сид-фраза показывается один раз и автоматически на диск не пишется.
   AppKey — только в keychain. Ни то, ни другое не логируется.
4. Файл не читается целиком в память.
5. `.part` не переименовывается в итоговый файл без проверки размера.
6. Ошибки целостности не проглатываются (`filter_map(...ok())` запрещён в этих путях).
7. Формат блоба версионирован (`p1_`); смена параметров KDF → новый префикс.
8. Старый `S#…exe` должен открываться новой версией ядра.
9. `single-instance` у отправителя — первый плагин; получателю его не ставить.

## 7. `sia_storage` 0.13.6 — реальные сигнатуры

Проверено по исходникам крейта в `%USERPROFILE%\.cargo\registry\src\*\sia_storage-0.13.6\`.
**Перед написанием SDK-кода сверяйся там или на docs.rs — не угадывай.** Исходное ТЗ
проекта было написано по выдуманным методам (смесь JS- и Rust-API) и не компилировалось.

- `Builder::new<U: IntoUrl>(url, AppMetadata) -> Result<Self>`; `connected(&AppKey) -> Result<Option<Sdk>>`
- Typestate: `request_connection()` → показать `response_url()` → `wait_for_approval()` → `register(phrase)`
- `ApprovedState::reconnecting()`, `matches_existing_app_key(phrase)` — иначе новая фраза молча создаст другой аккаунт
- `AppKey::import/export`, `SharingKey::import/export` — `[u8; 32]`, **без Result**
- `Object::new(Some(Vec<u8>))` / `Object::default()`; поля `metadata`, `created_at`, `updated_at` публичные; `id()`, `size()`, `encoded_size()`, `truncate()`, `seal()`
- `sdk.upload(obj, reader, UploadOptions)` — 3 аргумента; `pin_object(&obj)` обязателен
- `sdk.download(&obj, DownloadOptions)` — **синхронный**, возвращает `Download` (`AsyncRead`); drop = отмена. У `Download` есть `write_to_path` и `read_chunk`, но мы читаем вручную ради `.part`-докачки
- `sdk.object(&Hash256)`; `Hash256: FromStr/Display` (hex), `From<[u8;32]>`
- `object_summaries(offset, limit)` — только у `SharedSdk`; у `Sdk` — `object_event_summaries(cursor, limit)`
- `create_sharing_key(SharingKeyOptions{description, expires_at})`; `share_object` идемпотентен
- `Account.remaining_storage` — логические байты; на сети ~×3 (10+20). Бесплатно 50 ГБ
- `generate_recovery_phrase() -> String`, `validate_recovery_phrase(&str)`, `encoded_size(...)`
- Есть `upload_packed` / `PackedUpload::{add, add_path, finalize}` — для мелких файлов, у нас не используется
- Есть `preauthorized_keys` + `Builder::connect_pre_authorized` — вход без браузера, не используется
- **Есть `mock.rs`: `MockNetwork::{new, add_hosts, set_slow_hosts, set_pin_slabs_failures, sdk, shared_sdk}`** и `rhp4/mock.rs` (`set_read_failures`, задержки). В docs.rs модуль `mock` не показан — вероятно, за feature-флагом. **Решение владельца: интеграционные тесты на `MockNetwork`/локальной сети пока не делаем** (см. раздел 10, задача 5).

Версии, зафиксированные реальной сборкой (не поднимать без необходимости):
`sia_storage 0.13.6`, `zip 2.6.1`, `tokio 1.54.1`, `tauri 2.12.6`,
`tauri-runtime-wry 2.12.4`, `tao 0.37.2`.

## 8. Сборка и запуск (Windows 11, PowerShell)

```powershell
# Порядок важен: generate_context! вшивает frontendDist на этапе КОМПИЛЯЦИИ
cd apps\uploader-desktop; npm install; npm run build; cd ..\..
cd apps\receiver-micro;   npm install; npm run build; cd ..\..
cargo test --workspace

# Шаблон получателя нужен отправителю в resources/ ДО его запуска
cd apps\receiver-micro; npm run tauri build -- --no-bundle; cd ..\..
Copy-Item target\release\receiver-micro.exe apps\uploader-desktop\src-tauri\resources\ -Force
cd apps\uploader-desktop; npm run tauri dev
```

Быстрая проверка без линковки: `cargo check -p sia-core --all-targets`, затем `--workspace`.
Релиз целиком: `.\build.ps1`. Требования: Rust MSVC + VS Build Tools (C++), Node 20+, WebView2.

Имена бинарников: `cargo build` даёт имя крейта (`uploader-desktop.exe`),
`tauri build` — `mainBinaryName` (`sia-share-uploader`); у получателя оба имени совпадают.

## 9. Грабли, на которые уже наступали

| Симптом | Причина и решение |
|---|---|
| `link.exe not found` | Нет VS Build Tools. Минимум: `VC.Tools.x86.x64` + `Windows11SDK.22621` (~3–4 ГБ) |
| Installer `2147942512` | `0x80070070` = ERROR_DISK_FULL, кончилось место на `C:` |
| Installer `87` | ERROR_INVALID_PARAMETER: `--cachePath/--sharedPath` работают только при самой первой установке VS на машине |
| `ZipWriter<File>: write_all` не найден | В скоупе только `AsyncWriteExt`. Добавлен `use std::io::Write as _;` |
| Паника `there is no reactor running` | `tokio::spawn` в синхронном `setup()`. Заменён на **`tauri::async_runtime::spawn`**. `spawn_blocking` внутри async-команд не трогать |
| Красное в `main.rs`, ошибка `generate_context!` | Не собран фронтенд (`../dist`). Это же ломает rust-analyzer |
| `connectionfailure` на localhost:1420 | **VPN.** Ломает и dev-сервер, и RHP4-соединения SDK |
| После переноса папки — ошибки со старым путём | Снести `target/`, `node_modules/`, `dist/`, `src-tauri/gen/schemas/`. Lock-файлы не удалять. В `queue.db` абсолютные пути к исходникам |
| `Blocking waiting for file lock` | rust-analyzer и ручной cargo дерутся за `target/`. В Zed: `cargo.targetDir: true` |
| В release паника убивает процесс молча | `panic="abort"` + `strip=true`; `catch_unwind` не работает, только `Result` |
| `.ps1` с кириллицей падает с `MissingEndParenthesis` | UTF-8 без BOM + CP1251: тире `—` превращается в «умную кавычку» и рвёт строку. Пересохранить с BOM или запускать `pwsh` |

Zed: перезапускать целиком после установки тулчейна; `file_scan_exclusions` для
`target`/`node_modules`/`dist`/`gen/schemas`; `procMacro.enable: true`.

## 10. Состояние проекта

**Проверено в бою:**
- `sia-core` компилируется, тесты проходят.
- Вход через браузер на sia.storage работает.
- **Сквозной прогон пройден полностью:** загрузка → `Sia.zip` → запуск `S#…exe` → скачивание.
- В коде нет ни одной метки `TODO`/`FIXME` — весь техдолг живёт только в этом файле.

**Открытые задачи:**
1. **Резервное копирование сид-фразы не реализовано.** В UI кнопка «Скопировать»
   через `navigator.clipboard` (в WebView2 ненадёжна) и «Я сохранил фразу», которая
   просто очищает состояние. Рабочий обходной путь — ручной: у `.phrase` стоит
   `user-select: all`, один клик выделяет фразу целиком. Нативное сохранение в файл
   требует четырёх изменений: `@tauri-apps/plugin-fs` в package.json,
   `tauri-plugin-fs` в Cargo.toml, `.plugin(tauri_plugin_fs::init())` в main.rs,
   permission **и scope** в capabilities (одного `fs:allow-write-text-file` мало).
2. URL индексатора не передаётся в payload — получатель зашит на `sia.storage`.
3. Плоский список файлов в получателе вместо дерева папок.
4. Нет blake3-хэша файла целиком (целостность только по размерам).
5. Нет интеграционных тестов. Локальной сети/`indexd` и `MockNetwork` пока не будет —
   решение отложено сознательно; интеграционные прогоны делаются вручную на бою
   (см. раздел 8), юнит-тесты чистого кода (`crypto`, `db`, `models`) это не отменяет.
6. `upload_packed` не используется: мелкие файлы паддятся до слаба, квота тратится зря.
7. `fmt`/`clippy` в CI — advisory; ужесточить после `cargo fmt --all`.
8. `opt-level="z"` применяется и к отправителю; можно поднять до 3 через
   `[profile.release.package.uploader-desktop]` (per-package работает только для `opt-level`).

## 11. Гигиена репозитория

- `package-lock.json` — **задача закрыта (п.1: «готово»)**. В актуальной ветке
  `qwen` на GitHub оба лок-файла закоммичены
  (`apps/receiver-micro/package-lock.json`, `apps/uploader-desktop/package-lock.json`,
  lockfileVersion 3), строки `package-lock.json` в `.gitignore` больше нет.
  Проверено через `git ls-tree FETCH_HEAD` после fetch из удалённого репозитория.
  Осталось желательное: в CI заменить `npm install` на `npm ci` для строгой
  воспроизводимости сборок.
- **`.srcBuilders/`** — полная вторая копия дерева проекта (те же файлы, те же размеры).
  В `.gitignore` уже есть, но из индексации редактора не исключена: поиск и grep дают
  двойные результаты, агент может отредактировать не ту копию. Добавить в
  `file_scan_exclusions` Zed, а лучше вынести за пределы проекта.
- **`Cargo copy.toml`** — вариант корневого манифеста без комментариев, не отслеживается
  git и не участвует в сборке. Удалить, чтобы не путать.
- `collect-context.ps1` — сборщик дампа контекста (`-SiaSdkApi` добавляет реальные
  сигнатуры SDK). Требует UTF-8 **с BOM**. Фильтр по расширениям не показывает `.py`,
  `.sh`, `.gitignore`, иконки и lock-файлы — их отсутствие в дампе не значит, что их нет.
- Git-история короткая (два коммита), откатываться почти некуда: **коммит перед правками**.

## 12. Как агент должен работать

1. Общение и **комментарии в коде — на русском**; doc-комментарии к публичным
   элементам плюс пояснения «почему» в неочевидных местах.
2. **Выдавать целые файлы** или самодостаточные скрипты-генераторы/патчи на Python
   (идемпотентные, запуск из корня, печатают список созданных файлов, последняя
   строка — `    main()`). Не предлагать «вставь три строки куда-то туда».
3. Скрипты не должны обрываться: лучше несколько коротких, чем один длинный.
4. Редактор **Zed**, ОС **Windows 11**, проект на `D:\_sites\3406\SiaShare`.
   Не предлагать расширения VS Code и `.vscode/*`. PowerShell-скрипты — ASCII либо
   с явной инструкцией сохранить в UTF-8 with BOM.
5. Перед правкой — прочитать реальный файл, а не копию из `.srcBuilders/`.
6. SDK-код писать только после сверки с исходниками/docs.rs `sia_storage` 0.13.x.
7. Контракты IPC (4), форматы (5) и инварианты (6) — не менять молча.
8. Проверка перед сдачей: `cargo check -p sia-core --all-targets` → `cargo check --workspace`
   → `cargo test --workspace`; фронтенды собраны.
9. Не хватает контекста — задать один точный вопрос вместо предположения.
