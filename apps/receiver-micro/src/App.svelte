<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
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

  // --- Скорость: скользящее окно ~3 с по приросту байтов ---
  let speedSamples: { t: number; bytes: number }[] = [];
  let speedBps = 0; // байт/с
  let speedTimer: ReturnType<typeof setInterval> | undefined;

  function pushSpeedSample(bytes: number) {
    const now = performance.now();
    const last = speedSamples[speedSamples.length - 1];
    // Откат счетчика — сдвигаем окно, сохраняя набранные дельты
    if (last && bytes < last.bytes) {
      const drop = last.bytes - bytes;
      speedSamples = speedSamples.map((s) => ({ t: s.t, bytes: s.bytes - drop }));
    }
    speedSamples.push({ t: now, bytes });
    while (speedSamples.length > 2 && now - speedSamples[0].t > 3000) speedSamples.shift();
    refreshSpeed(now);
  }

  function refreshSpeed(now: number) {
    if (speedSamples.length < 2) { speedBps = 0; return; }
    const first = speedSamples[0];
    const last = speedSamples[speedSamples.length - 1];
    const dt = (last.t - first.t) / 1000;
    // Нет свежих событий дольше 2 с — скачивание встало
    if (dt < 0.5 || now - last.t > 2000) { speedBps = 0; return; }
    speedBps = Math.max(0, (last.bytes - first.bytes) / dt);
  }

  $: percent = total > 0 ? Math.round((downloaded / total) * 100) : 0;
  $: pushSpeedSample(downloaded);

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

    // Пересчет скорости даже без событий: пауза должна гасить счетчик
    speedTimer = setInterval(() => refreshSpeed(performance.now()), 1000);
  });

  onDestroy(() => clearInterval(speedTimer));

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
    speedSamples = []; speedBps = 0;
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
    if (bytes <= 0) return '0 Б';
    const k = 1024;
    const sizes = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'];
    const i = Math.min(sizes.length - 1, Math.floor(Math.log(bytes) / Math.log(k)));
    const v = bytes / Math.pow(k, i);
    return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${sizes[i]}`;
  }

  function formatSpeed(bytesPerSec: number): string {
    const bits = bytesPerSec * 8;
    if (bits < 1000) return `${bits.toFixed(0)} бит/с`;
    const k = 1000;
    const units = ['Кбит/с', 'Мбит/с', 'Гбит/с'];
    const i = Math.min(units.length - 1, Math.floor(Math.log(bits) / Math.log(k)) - 1);
    const v = bits / Math.pow(k, i + 1);
    return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`;
  }
</script>

<main class="mx-auto max-w-3xl px-6 py-10">
  <header class="mb-8 flex items-center gap-3">
    <div class="grid h-10 w-10 place-items-center rounded-xl bg-gradient-to-br from-indigo-500 to-emerald-500 text-lg font-bold text-white">S</div>
    <div>
      <h1 class="text-xl font-bold leading-tight">SiaShare</h1>
      <p class="muted">Скачивание файлов из Sia Storage</p>
    </div>
  </header>

  {#if status === 'loading'}
    <div class="card">
      <p class="animate-pulse text-sm text-slate-300">Подключение к сети Sia Storage…</p>
    </div>
  {:else if status === 'error'}
    <div class="card border-red-500/40 bg-red-500/10">
      <h3 class="mb-2 text-lg font-semibold text-red-300">Ошибка доступа</h3>
      <p class="mb-3 text-sm text-red-300/90">{errorMessage}</p>
      <p class="muted mb-2">Если файл был переименован, вставьте ключ (r_… / p1_…) или содержимое key.sia:</p>
      <textarea class="input mb-3" bind:value={manualPayload} rows="3" placeholder="r_… или p1_…" />
      <button class="btn-primary" on:click={handleManualKey} disabled={!manualPayload.trim()}>Открыть</button>
    </div>
  {:else if status === 'password'}
    <div class="card">
      <h3 class="mb-4 text-lg font-semibold">Доступ защищен паролем</h3>
      <div class="flex gap-3">
        <input class="input" type="password" bind:value={password} placeholder="Введите пароль"
               on:keydown={(e) => e.key === 'Enter' && handleUnlock()} />
        <button class="btn-primary shrink-0" on:click={handleUnlock}>Открыть файлы</button>
      </div>
      {#if pwdError}<p class="mt-3 text-sm text-red-400">{pwdError}</p>{/if}
    </div>
  {:else if status === 'ready'}
    <div class="mb-4 flex flex-wrap items-center gap-3">
      <h2 class="text-lg font-semibold">Доступные файлы</h2>
      <span class="rounded-full bg-slate-800 px-2.5 py-0.5 text-xs font-medium text-slate-300">{files.length}</span>
      <label class="ml-auto flex cursor-pointer items-center gap-2 text-sm text-slate-300">
        <input class="h-4 w-4 accent-indigo-500" type="checkbox" checked={selectedPaths.size === files.length && files.length > 0}
               on:change={toggleAll} /> все
      </label>
      <button class="btn-primary" disabled={isDownloading || selectedPaths.size === 0} on:click={downloadSelectedZip}>
        Скачать выбранное в ZIP ({selectedPaths.size})
      </button>
    </div>

    {#if isDownloading}
      <div class="card mb-4">
        <div class="relative mb-2 h-6 overflow-hidden rounded-full bg-slate-800">
          <div class="h-full rounded-full bg-gradient-to-r from-emerald-500 to-emerald-400 transition-[width] duration-200" style="width:{percent}%"></div>
          <span class="absolute inset-0 grid place-items-center text-xs font-semibold text-white">{percent}%</span>
        </div>
        <div class="mb-3 flex items-baseline justify-between gap-3 text-xs">
          <span class="text-slate-400">{formatBytes(downloaded)} из {formatBytes(total)}</span>
          <span class="font-semibold tabular-nums text-emerald-300">{formatSpeed(speedBps)}</span>
        </div>
        <button class="btn-danger" on:click={() => invoke('cancel_download')}>Отменить</button>
      </div>
    {/if}
    {#if notice}<p class="muted mb-4">{notice}</p>{/if}

    <ul class="divide-y divide-slate-800 overflow-hidden rounded-2xl border border-slate-800 bg-slate-900/70 shadow-xl shadow-black/30">
      {#each files as file (file.rel_path)}
        <li class="flex items-center gap-3 px-4 py-3 transition-colors hover:bg-slate-800/40">
          <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-3">
            <input class="h-4 w-4 shrink-0 accent-indigo-500" type="checkbox" checked={selectedPaths.has(file.rel_path)}
                   on:change={() => toggleSelection(file.rel_path)} />
            <span class="truncate text-sm text-slate-200" title={file.rel_path}>{file.rel_path}</span>
            <span class="ml-auto shrink-0 text-xs text-slate-400">{formatBytes(file.total_size)}</span>
          </label>
          <button class="btn-secondary shrink-0 px-3 py-1.5 text-xs" disabled={isDownloading} on:click={() => downloadOne(file)}>Скачать</button>
        </li>
      {/each}
    </ul>
  {/if}
</main>
