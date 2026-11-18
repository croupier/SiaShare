<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open, save } from '@tauri-apps/plugin-dialog';

  // --- Авторизация ---
  let isConnected = false;
  let authChecked = false;   // до результата проверки не показываем форму входа
  let authWaiting = false;   // ждем одобрения на странице индексатора
  let authUrl = '';          // fallback-ссылка (браузер открывает Rust-сторона)
  let authError = '';
  let seedPhrase = '';
  let backupPhrase = '';     // новая 12-словная фраза — показываем один раз

  // --- Параметры новой раздачи ---
  let sessionName = 'Мои файлы';
  let password = '';
  let validDays = 7;

  // --- Список всех раздач ---
  interface SessionInfo {
    id: string; name: string; status: string; created_at: number;
    last_error: string | null; seed_hex: string | null;
    total: number; uploaded: number; failed: number;
    total_bytes: number; uploaded_bytes: number;
  }

  // SessionInfo из БД + живое состояние UI (доли чанков, скорость, ключ)
  interface SessionView extends SessionInfo {
    keyBytes: number[] | null;
    // Доли незавершенных чанков (file_id:chunk_index -> доля и размер)
    inflight: Map<string, { fraction: number; length: number }>;
    speedSamples: { t: number; bytes: number }[];
    speedBps: number;
    lastChunkError: string;
  }

  let sessions: SessionView[] = [];

  const STATUS_META: Record<string, { label: string; cls: string }> = {
    uploading:  { label: 'Загружается', cls: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300' },
    finalizing: { label: 'Финализация', cls: 'border-indigo-500/30 bg-indigo-500/10 text-indigo-300' },
    completed:  { label: 'Готова', cls: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300' },
    failed:     { label: 'Ошибка', cls: 'border-red-500/30 bg-red-500/10 text-red-300' },
    revoked:    { label: 'Отозвана', cls: 'border-amber-500/30 bg-amber-500/10 text-amber-300' },
    cleaned:    { label: 'Удалена', cls: 'border-slate-600/40 bg-slate-700/20 text-slate-400' },
  };

  function statusMeta(s: SessionView) {
    // Все чанки закреплены, но ключ еще создается — воркер в фазе финализации
    if (s.status === 'uploading' && s.total > 0 && s.uploaded >= s.total)
      return STATUS_META.finalizing;
    return STATUS_META[s.status] ?? { label: s.status, cls: 'border-slate-600/40 bg-slate-700/20 text-slate-400' };
  }

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleString('ru-RU', {
      day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit',
    });
  }

  function hexToBytes(hex: string): number[] {
    return Array.from(hex.match(/.{2}/g) ?? [], (b) => parseInt(b, 16));
  }

  function inflightSum(s: SessionView): number {
    let sum = 0;
    for (const v of s.inflight.values()) sum += v.fraction;
    return sum;
  }

  function percentOf(s: SessionView): number {
    return s.total > 0
      ? Math.min(100, Math.round(((s.uploaded + inflightSum(s)) / s.total) * 100))
      : 0;
  }

  // Байты закрепленных чанков + оценка незавершенных по их долям
  function uploadedBytesEstOf(s: SessionView): number {
    let bytes = 0;
    for (const v of s.inflight.values()) bytes += v.fraction * v.length;
    return Math.min(s.total_bytes, s.uploaded_bytes + bytes);
  }

  // --- Скорость: скользящее окно ~3 с по приросту байтов, отдельно на сессию ---
  function pushSpeedSample(s: SessionView, bytes: number) {
    const now = performance.now();
    const last = s.speedSamples[s.speedSamples.length - 1];
    // Оценка байтов может просесть (сброс долей при session-progress, retry):
    // сдвигаем окно на величину просадки, чтобы не терять набранную скорость
    if (last && bytes < last.bytes) {
      const drop = last.bytes - bytes;
      s.speedSamples = s.speedSamples.map((p) => ({ t: p.t, bytes: p.bytes - drop }));
    }
    s.speedSamples.push({ t: now, bytes });
    while (s.speedSamples.length > 2 && now - s.speedSamples[0].t > 3000) s.speedSamples.shift();
    refreshSpeed(s, now);
  }

  function refreshSpeed(s: SessionView, now: number) {
    if (s.speedSamples.length < 2) { s.speedBps = 0; return; }
    const first = s.speedSamples[0];
    const last = s.speedSamples[s.speedSamples.length - 1];
    const dt = (last.t - first.t) / 1000;
    // Нет свежих событий дольше 2 с — передача встала (пауза/ретрай)
    if (dt < 0.5 || now - last.t > 2000) { s.speedBps = 0; return; }
    s.speedBps = Math.max(0, (last.bytes - first.bytes) / dt);
  }

  let speedTimer: ReturnType<typeof setInterval> | undefined;

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

  // --- Preflight: несовместимые с Windows имена (игнор или отмена) ---
  interface PathWarning { rel_path: string; kind: string; message: string }
  let pathWarnings: PathWarning[] = [];
  let pendingPaths: string[] | null = null;

  // --- Подтверждение загрузки после выбора файлов ---
  let confirmSelection: { paths: string[]; count: number; bytes: number } | null = null;

  function pluralFiles(n: number): string {
    const m10 = n % 10, m100 = n % 100;
    if (m10 === 1 && m100 !== 11) return 'файл';
    if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return 'файла';
    return 'файлов';
  }

  // После перезапуска воркеры продолжают загрузку сами, но UI должен снова
  // показать ВСЕ сессии. Живое состояние (доли чанков, скорость) сохраняем
  // для уже известных id — список обновляется и после старта новой раздачи.
  async function refreshSessions() {
    const list = await invoke<SessionInfo[]>('list_sessions');
    const prev = new Map(sessions.map((s) => [s.id, s]));
    sessions = list.map((info) => {
      const old = prev.get(info.id);
      const keyBytes = (info.status === 'completed' || info.status === 'revoked') && info.seed_hex
        ? hexToBytes(info.seed_hex) : null;
      return {
        ...info,
        keyBytes: old?.keyBytes ?? keyBytes,
        inflight: old?.inflight ?? new Map(),
        speedSamples: old?.speedSamples ?? [],
        speedBps: old?.speedBps ?? 0,
        lastChunkError: old?.lastChunkError ?? '',
      };
    });
  }

  function findSession(id: string): SessionView | undefined {
    return sessions.find((s) => s.id === id);
  }

  onMount(async () => {
    await listen<string>('auth-url', (e) => { authUrl = e.payload; });
    await listen<string>('auth-phrase', (e) => { backupPhrase = e.payload; });

    // Точные счетчики чанков из БД; доли сбросятся и перезаполнятся
    // воркерами в течение ~250 мс (троттлинг прогресса на Rust-стороне)
    await listen<{ session_id: string; uploaded: number; total: number; failed: number;
                   total_bytes: number; uploaded_bytes: number }>(
      'session-progress', (e) => {
        const p = e.payload;
        const s = findSession(p.session_id);
        if (!s) return;
        s.uploaded = p.uploaded; s.total = p.total; s.failed = p.failed;
        s.total_bytes = p.total_bytes; s.uploaded_bytes = p.uploaded_bytes;
        s.inflight = new Map();
        pushSpeedSample(s, uploadedBytesEstOf(s));
        sessions = sessions;
      });
    await listen<{ session_id: string; file_id: string; chunk_index: number;
                   fraction: number; length_bytes: number }>(
      'chunk-progress', (e) => {
        const p = e.payload;
        const s = findSession(p.session_id);
        if (!s) return;
        s.inflight.set(`${p.file_id}:${p.chunk_index}`, { fraction: p.fraction, length: p.length_bytes });
        pushSpeedSample(s, uploadedBytesEstOf(s));
        sessions = sessions; // присваивание триггерит реактивность Svelte
      });
    await listen<{ session_id: string; error: string; failed: boolean }>('chunk-error', (e) => {
      // Неокончательная ошибка чанка = предупреждение (будет retry с backoff)
      const s = findSession(e.payload.session_id);
      if (s && !e.payload.failed) {
        s.lastChunkError = e.payload.error;
        sessions = sessions;
      }
    });
    await listen<[string, number[]]>('session-completed', (e) => {
      const [sid, key] = e.payload;
      const s = findSession(sid);
      if (!s) return;
      s.status = 'completed'; s.keyBytes = key; s.last_error = null;
      s.lastChunkError = ''; s.inflight = new Map(); s.speedBps = 0;
      sessions = sessions;
    });
    await listen<[string, string]>('session-failed', (e) => {
      const [sid, msg] = e.payload;
      const s = findSession(sid);
      if (!s) return;
      s.status = 'failed'; s.last_error = msg; s.speedBps = 0;
      sessions = sessions;
    });

    // Автовход по ключу из системного keychain
    await autoLogin();

    // Пересчет скорости даже без событий: пауза между пачками должна гасить счетчик
    speedTimer = setInterval(() => {
      const now = performance.now();
      for (const s of sessions)
        if (s.status === 'uploading') refreshSpeed(s, now);
      sessions = sessions;
    }, 1000);
  });

  onDestroy(() => clearInterval(speedTimer));

  async function autoLogin() {
    authError = '';
    authChecked = false;
    try {
      isConnected = await invoke<boolean>('check_saved_auth');
      if (isConnected) await refreshSessions();
    } catch (e) {
      // Сбой keychain/сети — не «ключа нет»: показываем ошибку и даём повтор
      isConnected = false;
      authError = `Автовход не удался: ${e}`;
    } finally {
      authChecked = true;
    }
  }

  function connect(newAccount: boolean) {
    authError = ''; authUrl = ''; authWaiting = true;
    // invoke висит до одобрения в браузере — не await'им, UI живет на событиях
    invoke('connect_sia', { seedPhrase: newAccount ? null : seedPhrase.trim() || null })
      .then(async () => { isConnected = true; await refreshSessions(); })
      .catch((e) => { authError = String(e); })
      .finally(() => { authWaiting = false; });
  }

  async function copyText(text: string) {
    try { await navigator.clipboard.writeText(text); }
    catch { alert(text); } // clipboard может быть недоступен в webview
  }

  async function selectAndUpload(isDirectory: boolean) {
    const selected = await open({ directory: isDirectory, multiple: !isDirectory });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    // Считаем файлы и размер (папки — рекурсивно) и спрашиваем подтверждение
    try {
      const [count, bytes] = await invoke<[number, number]>('describe_selection', { paths });
      if (count === 0) { alert('В выбранной папке нет файлов.'); return; }
      confirmSelection = { paths, count, bytes };
    } catch (e) { alert(`Ошибка чтения выбранных путей: ${e}`); }
  }

  async function confirmUpload() {
    const selection = confirmSelection;
    confirmSelection = null;
    if (!selection) return;
    // Preflight: предупреждаем о несовместимых с Windows именах до загрузки
    try {
      const warnings = await invoke<PathWarning[]>('validate_paths', { paths: selection.paths });
      if (warnings.length) { pendingPaths = selection.paths; pathWarnings = warnings; return; }
    } catch (e) { alert(`Ошибка проверки путей: ${e}`); return; }
    await startUpload(selection.paths);
  }

  async function startUpload(paths: string[]) {
    try {
      await invoke<string>('start_upload_session', { sessionName, paths, validDays });
      // Новая сессия уже в БД со счетчиками — просто обновляем весь список
      await refreshSessions();
    } catch (e) { alert(`Ошибка запуска загрузки: ${e}`); }
  }

  function closeWarnings() { pathWarnings = []; pendingPaths = null; }

  async function ignoreWarnings() {
    const paths = pendingPaths;
    closeWarnings();
    if (paths) await startUpload(paths);
  }

  async function exportShareZip(s: SessionView) {
    if (!s.keyBytes) return;
    const savePath = await save({ defaultPath: 'Sia.zip', filters: [{ name: 'ZIP', extensions: ['zip'] }] });
    if (!savePath) return;
    try {
      await invoke('generate_share_zip', {
        outputZipPath: savePath,
        sharingKeyBytes: s.keyBytes,
        password: password.trim() || null,
      });
      alert('Готово! Отправьте архив Sia.zip получателю.');
    } catch (e) { alert(`Ошибка сборки пакета: ${e}`); }
  }

  async function revokeAccess(s: SessionView) {
    // Честное предупреждение: отзыв не обрывает уже выданные токены мгновенно
    if (!confirm(`Отозвать доступ к раздаче «${s.name}»? Уже начатые скачивания смогут продолжаться до ~5 минут.`)) return;
    try {
      await invoke('revoke_session', { sessionId: s.id });
      s.status = 'revoked';
      sessions = sessions;
    } catch (e) { alert(`Ошибка отзыва: ${e}`); }
  }

  async function cleanupSession(s: SessionView) {
    if (!confirm(`Удалить загруженные объекты раздачи «${s.name}» из Sia? Квота освободится, раздача станет недоступной навсегда.`)) return;
    try {
      await invoke('cleanup_session', { sessionId: s.id });
      s.status = 'cleaned';
      sessions = sessions;
    } catch (e) { alert(`Ошибка очистки: ${e}`); }
  }

  $: hasActiveUploads = sessions.some((s) => s.status === 'uploading' || s.status === 'finalizing');
</script>

<main class="mx-auto max-w-2xl px-6 py-10">
  <header class="mb-8 flex items-center gap-3">
    <div class="grid h-10 w-10 place-items-center rounded-xl bg-gradient-to-br from-indigo-500 to-emerald-500 text-lg font-bold text-white">S</div>
    <div>
      <h1 class="text-xl font-bold leading-tight">SiaShare</h1>
      <p class="muted">Отправка файлов в Sia Storage</p>
    </div>
    {#if isConnected}
      <span class="ml-auto inline-flex items-center gap-2 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-xs font-medium text-emerald-300">
        <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Подключено
      </span>
    {/if}
  </header>

  {#if !authChecked}
    <div class="card">
      <p class="animate-pulse text-sm text-slate-300">Проверка подключения к Sia Storage…</p>
    </div>
  {:else if !isConnected}
    <div class="card">
      <h2 class="mb-4 text-lg font-semibold">Подключение к Sia Storage</h2>
      {#if authWaiting}
        <p class="animate-pulse text-sm text-slate-300">Подтвердите подключение в открывшемся браузере…</p>
        {#if authUrl}
          <p class="muted mt-3">Если браузер не открылся, скопируйте ссылку:</p>
          <code class="phrase my-3">{authUrl}</code>
          <button class="btn-secondary" on:click={() => copyText(authUrl)}>Скопировать ссылку</button>
        {/if}
      {:else}
        <div class="flex flex-col gap-4">
          <button class="btn-primary" on:click={() => connect(true)}>Создать новый аккаунт</button>
          <div>
            <p class="muted mb-2">или войдите существующей 12-словной фразой:</p>
            <textarea class="input mb-3" bind:value={seedPhrase} rows="3" placeholder="word1 word2 … word12" />
            <button class="btn-secondary w-full" on:click={() => connect(false)} disabled={!seedPhrase.trim()}>Войти по фразе</button>
          </div>
        </div>
      {/if}
      {#if authError}<p class="mt-4 text-sm text-red-400">{authError}</p>{/if}
      {#if authError && !authWaiting}
        <button class="btn-secondary mt-3" on:click={autoLogin}>Повторить автовход</button>
      {/if}
    </div>
  {:else}
    {#if backupPhrase}
      <div class="card mb-6 border-amber-500/40 bg-amber-500/10">
        <h3 class="mb-2 text-lg font-semibold text-amber-300">Запишите фразу восстановления!</h3>
        <p class="muted mb-3">Она показывается один раз и НЕ сохраняется на диске. Без нее не восстановить аккаунт.</p>
        <code class="phrase mb-4">{backupPhrase}</code>
        <div class="flex gap-3">
          <button class="btn-primary" on:click={() => copyText(backupPhrase)}>Скопировать</button>
          <button class="btn-secondary" on:click={() => (backupPhrase = '')}>Я сохранил фразу</button>
        </div>
      </div>
    {/if}

    <div class="card mb-6">
      <h2 class="mb-4 text-lg font-semibold">Новая раздача</h2>
      <div class="grid gap-4">
        <label class="block">
          <span class="mb-1.5 block text-sm font-medium text-slate-300">Название раздачи</span>
          <input class="input" type="text" bind:value={sessionName} />
        </label>
        <div class="grid grid-cols-2 gap-4">
          <label class="block">
            <span class="mb-1.5 block text-sm font-medium text-slate-300">Срок жизни ссылки (дней)</span>
            <input class="input" type="number" min="0" bind:value={validDays} />
            <span class="muted mt-1 block text-xs">0 — бессрочная раздача; отсчёт — с момента завершения загрузки</span>
          </label>
          <label class="block">
            <span class="mb-1.5 block text-sm font-medium text-slate-300">Пароль на скачивание</span>
            <input class="input" type="password" bind:value={password} placeholder="необязательно" />
            <span class="muted mt-1 block text-xs">Применяется при сохранении Sia.zip</span>
          </label>
        </div>
      </div>
      <div class="mt-4 flex gap-3">
        <button class="btn-primary" on:click={() => selectAndUpload(false)}>Выбрать файлы</button>
        <button class="btn-secondary" on:click={() => selectAndUpload(true)}>Выбрать папку целиком</button>
      </div>
    </div>

    <div class="mb-4 flex items-center justify-between gap-3">
      <h2 class="text-lg font-semibold">Мои раздачи {#if sessions.length}<span class="muted font-normal">({sessions.length})</span>{/if}</h2>
      <button class="btn-secondary px-3 py-1.5 text-xs" on:click={refreshSessions}>Обновить</button>
    </div>

    {#if sessions.length === 0}
      <div class="card mb-6">
        <p class="muted">Пока нет раздач — выберите файлы или папку выше, чтобы начать первую загрузку.</p>
      </div>
    {/if}

    {#each sessions as s (s.id)}
      {@const meta = statusMeta(s)}
      <div class="card mb-4">
        <div class="mb-1 flex flex-wrap items-center gap-x-3 gap-y-1">
          <h3 class="min-w-0 break-all text-base font-semibold text-white">{s.name}</h3>
          <span class="inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium {meta.cls}">
            {#if s.status === 'uploading' || s.status === 'finalizing'}
              <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-current"></span>
            {/if}
            {meta.label}
          </span>
          <span class="muted ml-auto shrink-0 text-xs">{formatDate(s.created_at)}</span>
        </div>

        {#if s.status === 'uploading' || s.status === 'finalizing'}
          <div class="mt-3">
            <div class="relative mb-2 h-6 overflow-hidden rounded-full bg-slate-800">
              <div class="h-full rounded-full bg-gradient-to-r from-emerald-500 to-emerald-400 transition-[width] duration-200" style="width:{percentOf(s)}%"></div>
              <span class="absolute inset-0 grid place-items-center text-xs font-semibold text-white">{percentOf(s)}%</span>
            </div>
            <div class="mb-3 flex items-baseline justify-between gap-3 text-xs">
              <span class="text-slate-400">{formatBytes(uploadedBytesEstOf(s))} из {formatBytes(s.total_bytes)}</span>
              <span class="font-semibold tabular-nums text-emerald-300">{formatSpeed(s.speedBps)}</span>
            </div>
            <p class="text-sm text-slate-300">Закреплено чанков: <strong class="text-white">{s.uploaded}</strong> из {s.total}
              {#if s.failed > 0}<span class="text-amber-400">(ошибок: {s.failed})</span>{/if}
            </p>
            {#if s.lastChunkError}<p class="muted mt-2">Последняя ошибка (будет повтор): {s.lastChunkError}</p>{/if}
          </div>
        {:else if s.status === 'completed'}
          <p class="muted mt-1 mb-4">{formatBytes(s.total_bytes)} · чанков: {s.total}</p>
          <div class="flex flex-wrap gap-3">
            <button class="btn-primary" on:click={() => exportShareZip(s)}>Сохранить Sia.zip для отправки</button>
            <button class="btn-danger" on:click={() => revokeAccess(s)}>Отозвать доступ</button>
          </div>
        {:else if s.status === 'failed'}
          <p class="mt-2 text-sm text-red-300/90">{s.last_error ?? 'Раздача не удалась'}</p>
          <button class="btn-danger mt-4" on:click={() => cleanupSession(s)}>Очистить квоту (удалить объекты)</button>
        {:else if s.status === 'revoked'}
          <p class="mt-2 text-sm text-amber-300">Доступ отозван — получатель больше не сможет скачать файлы.</p>
          <button class="btn-danger mt-4" on:click={() => cleanupSession(s)}>Очистить квоту (удалить объекты)</button>
        {:else if s.status === 'cleaned'}
          <p class="muted mt-2">Объекты удалены из Sia, квота освобождена.</p>
        {/if}
      </div>
    {/each}

    {#if hasActiveUploads}
      <p class="muted mt-2 text-xs">Вы можете закрыть программу — при следующем запуске загрузка продолжится с незавершенных чанков.</p>
    {/if}
  {/if}

  {#if confirmSelection}
    <div class="fixed inset-0 z-50 grid place-items-center bg-black/60 p-6">
      <div class="card w-full max-w-md">
        <h3 class="mb-2 text-lg font-semibold">Загрузить?</h3>
        <p class="mb-5 text-sm text-slate-300">
          {confirmSelection.count} {pluralFiles(confirmSelection.count)} размером
          <strong class="text-white">{formatBytes(confirmSelection.bytes)}</strong>
        </p>
        <div class="flex gap-3">
          <button class="btn-primary" on:click={confirmUpload}>Да</button>
          <button class="btn-secondary" on:click={() => (confirmSelection = null)}>Нет</button>
        </div>
      </div>
    </div>
  {/if}

  {#if pathWarnings.length}
    <div class="fixed inset-0 z-50 grid place-items-center bg-black/60 p-6">
      <div class="card max-h-[80vh] w-full max-w-lg overflow-auto">
        <h3 class="mb-2 text-lg font-semibold text-amber-300">Несовместимые с Windows имена ({pathWarnings.length})</h3>
        <p class="muted mb-4">Получатель на Windows может не сохранить эти файлы или молча перезаписать одноимённые. Исправьте имена сами и повторите — или загрузите как есть на свой риск.</p>
        <ul class="mb-4 flex flex-col gap-2">
          {#each pathWarnings as w}
            <li class="rounded-lg border border-amber-500/30 bg-amber-500/5 p-2 text-sm">
              <span class="block break-all font-medium text-slate-200">{w.rel_path}</span>
              <span class="muted">{w.message}</span>
            </li>
          {/each}
        </ul>
        <div class="flex gap-3">
          <button class="btn-primary" on:click={closeWarnings}>Отмена</button>
          <button class="btn-secondary" on:click={ignoreWarnings}>Игнорировать и загрузить</button>
        </div>
      </div>
    </div>
  {/if}
</main>