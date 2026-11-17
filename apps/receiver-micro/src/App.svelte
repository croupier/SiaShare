<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { save } from '@tauri-apps/plugin-dialog';

  // Поля snake_case — ровно как сериализует Rust (SharedFileEntry)
  interface SharedFileEntry {
    rel_path: string; total_size: number; total_chunks: number;
    chunk_size: number; chunk_object_ids: string[];
  }
  type InitResponse =
    | { status: 'Ready'; data: SharedFileEntry[] }
    | { status: 'PasswordRequired' }
    | { status: 'Error'; data: string };

  let status: 'loading' | 'password' | 'ready' | 'error' = 'loading';
  let errorMessage = '';
  let password = '';
  let pwdError = '';
  let manualPayload = '';    // ручной ввод ключа, если имя файла потеряно
  let files: SharedFileEntry[] = [];
  let selectedPaths = new Set<string>();
  let isDownloading = false;
  let downloadMode: 'one' | 'zip' | null = null;
  let downloaded = 0;
  let total = 0;
  let notice = '';

  $: percent = total > 0 ? Math.round((downloaded / total) * 100) : 0;

  function applyInit(res: InitResponse) {
    if (res.status === 'Ready') {
      files = res.data;
      selectedPaths = new Set(files.map((f) => f.rel_path));
      status = 'ready';
    } else if (res.status === 'PasswordRequired') {
      status = 'password';
    } else {
      errorMessage = res.data || 'Неизвестная ошибка';
      status = 'error';
    }
  }

  onMount(async () => {
    // Прогресс троттлится на Rust-стороне (~200 мс) — здесь просто отображаем
    await listen<{ downloaded: number; total: number }>('download-progress', (e) => {
      downloaded = e.payload.downloaded; total = e.payload.total;
    });
    try { applyInit(await invoke<InitResponse>('init_receiver')); }
    catch (e) { errorMessage = String(e); status = 'error'; }
  });

  async function handleUnlock() {
    pwdError = '';
    try {
      files = await invoke<SharedFileEntry[]>('unlock_with_password', { password });
      selectedPaths = new Set(files.map((f) => f.rel_path));
      status = 'ready';
    } catch (e) { pwdError = String(e); }
  }

  // Запасной путь: мессенджер переименовал файл и ключ из имени не прочитался
  async function handleManualKey() {
    try { applyInit(await invoke<InitResponse>('submit_key_payload', { payload: manualPayload })); }
    catch (e) { errorMessage = String(e); }
  }

  function toggleSelection(path: string) {
    if (selectedPaths.has(path)) selectedPaths.delete(path);
    else selectedPaths.add(path);
    selectedPaths = new Set(selectedPaths);
  }
  function toggleAll() {
    selectedPaths = selectedPaths.size === files.length
      ? new Set() : new Set(files.map((f) => f.rel_path));
  }

  async function runDownload(mode: 'one' | 'zip', fn: () => Promise<void>) {
    isDownloading = true; downloadMode = mode; downloaded = 0; total = 0; notice = '';
    try {
      await fn();
      notice = mode === 'zip' ? 'Архив успешно сохранён.' : 'Файл успешно скачан.';
    } catch (e) {
      const msg = String(e);
      if (msg.includes('отменена')) {
        // Одиночный файл: .part сохранен для докачки; ZIP: недописанный удален
        notice = mode === 'one'
          ? 'Скачивание отменено. Недокачанный .part сохранён — повторное скачивание продолжит с места обрыва.'
          : 'Скачивание отменено, неполный ZIP удалён.';
      } else {
        alert(`Ошибка скачивания: ${msg}`);
      }
    } finally { isDownloading = false; downloadMode = null; }
  }

  async function downloadOne(entry: SharedFileEntry) {
    const fileName = entry.rel_path.split('/').pop() || 'file.bin';
    const saveTo = await save({ defaultPath: fileName });
    if (!saveTo) return;
    await runDownload('one', () => invoke('download_one', { relPath: entry.rel_path, saveTo }));
  }

  async function downloadSelectedZip() {
    if (selectedPaths.size === 0) return;
    const saveZipTo = await save({ defaultPath: 'SiaFiles.zip', filters: [{ name: 'ZIP', extensions: ['zip'] }] });
    if (!saveZipTo) return;
    await runDownload('zip', () =>
      invoke('download_selected_zip', { relPaths: [...selectedPaths], saveZipTo }));
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Б';
    const k = 1024;
    const sizes = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  }
</script>

<main class="container">
  {#if status === 'loading'}
    <p>Подключение к сети Sia Storage…</p>
  {:else if status === 'error'}
    <div class="card error">
      <h3>Ошибка доступа</h3>
      <p>{errorMessage}</p>
      <p class="muted">Если файл был переименован, вставьте ключ (r_… / p1_…) или содержимое key.sia:</p>
      <textarea bind:value={manualPayload} rows="3" placeholder="r_… или p1_…" />
      <button on:click={handleManualKey} disabled={!manualPayload.trim()}>Открыть</button>
    </div>
  {:else if status === 'password'}
    <div class="card">
      <h3>Доступ защищен паролем</h3>
      <input type="password" bind:value={password} placeholder="Введите пароль"
             on:keydown={(e) => e.key === 'Enter' && handleUnlock()} />
      <button on:click={handleUnlock}>Открыть файлы</button>
      {#if pwdError}<p class="warn">{pwdError}</p>{/if}
    </div>
  {:else if status === 'ready'}
    <header>
      <h2>Доступные файлы ({files.length})</h2>
      <label class="select-all">
        <input type="checkbox" checked={selectedPaths.size === files.length && files.length > 0}
               on:change={toggleAll} /> все
      </label>
      <button disabled={isDownloading || selectedPaths.size === 0} on:click={downloadSelectedZip}>
        Скачать выбранное в ZIP ({selectedPaths.size})
      </button>
    </header>

    {#if isDownloading}
      <div class="progress-bar">
        <div class="fill" style="width:{percent}%"></div>
        <span>{percent}% · {formatBytes(downloaded)} / {formatBytes(total)}</span>
      </div>
      <button class="danger" on:click={() => invoke('cancel_download')}>Отменить</button>
    {/if}
    {#if notice}<p class="muted">{notice}</p>{/if}

    <ul class="file-list">
      {#each files as file (file.rel_path)}
        <li>
          <label>
            <input type="checkbox" checked={selectedPaths.has(file.rel_path)}
                   on:change={() => toggleSelection(file.rel_path)} />
            <span class="path">{file.rel_path}</span>
            <span class="size">{formatBytes(file.total_size)}</span>
          </label>
          <button disabled={isDownloading} on:click={() => downloadOne(file)}>Скачать</button>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .container { font-family: system-ui, sans-serif; padding: 20px; max-width: 760px; margin: 0 auto; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 12px; }
  .select-all { display: flex; align-items: center; gap: 6px; margin-left: auto; }
  .file-list { list-style: none; padding: 0; border: 1px solid #ddd; border-radius: 8px; }
  .file-list li { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border-bottom: 1px solid #eee; }
  .file-list li:last-child { border-bottom: none; }
  .file-list label { display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0; }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .size { color: #666; font-size: 0.9em; margin-left: auto; margin-right: 12px; white-space: nowrap; }
  .progress-bar { background: #eee; border-radius: 6px; height: 24px; position: relative; overflow: hidden; margin: 10px 0; }
  .progress-bar .fill { background: #10b981; height: 100%; transition: width 0.2s; }
  .progress-bar span { position: absolute; inset: 0; text-align: center; line-height: 24px; font-size: 0.85em; }
  .card { border: 1px solid #ddd; padding: 24px; border-radius: 8px; }
  .card.error { border-color: #dc2626; background: #fef2f2; }
  input[type='password'] { padding: 8px 12px; margin-right: 8px; border: 1px solid #ccc; border-radius: 6px; }
  textarea { width: 100%; box-sizing: border-box; padding: 8px; border: 1px solid #ccc; border-radius: 6px; margin-bottom: 10px; }
  button { padding: 8px 14px; background: #2563eb; color: white; border: none; border-radius: 6px; cursor: pointer; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
  button.danger { background: #dc2626; margin-bottom: 10px; }
  .warn { color: #dc2626; }
  .muted { color: #666; font-size: 0.9em; }
</style>
