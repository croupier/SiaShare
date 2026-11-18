<script lang="ts">
  import { onMount } from 'svelte';
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

  // --- Параметры раздачи ---
  let sessionName = 'Мои файлы';
  let password = '';
  let validDays = 7;

  // --- Прогресс активной сессии ---
  interface SessionInfo {
    id: string; name: string; status: string; created_at: number;
    last_error: string | null; seed_hex: string | null;
    total: number; uploaded: number; failed: number;
  }

  let activeSessionId: string | null = null;
  let totalChunks = 0;
  let uploadedChunks = 0;
  let failedChunks = 0;
  let uploadError = '';
  let lastChunkError = '';
  // Доли незавершенных чанков (file_id:chunk_index -> 0..1) для живого прогресса
  let inflight = new Map<string, number>();
  let completedKeyBytes: number[] | null = null;
  let revoked = false;
  let cleaned = false; // объекты сессии удалены из Sia (cleanup_session)

  // --- Preflight: несовместимые с Windows имена (игнор или отмена) ---
  interface PathWarning { rel_path: string; kind: string; message: string }
  let pathWarnings: PathWarning[] = [];
  let pendingPaths: string[] | null = null;

  $: inflightSum = [...inflight.values()].reduce((a, b) => a + b, 0);
  $: percent = totalChunks > 0
    ? Math.min(100, Math.round(((uploadedChunks + inflightSum) / totalChunks) * 100))
    : 0;

  function hexToBytes(hex: string): number[] {
    return Array.from(hex.match(/.{2}/g) ?? [], (b) => parseInt(b, 16));
  }

  // После перезапуска воркеры продолжают загрузку сами, но UI должен снова
  // показывать сессию. Берем старейшую незавершенную (воркеры разбирают в том
  // же порядке), иначе — последнюю сессию (завершенную/проваленную).
  async function restoreSession() {
    const sessions = await invoke<SessionInfo[]>('list_sessions');
    const unfinished = sessions.filter((s) => s.status === 'uploading' || s.status === 'finalizing');
    const s = unfinished.length ? unfinished[unfinished.length - 1] : sessions[0];
    if (!s) return;
    activeSessionId = s.id;
    sessionName = s.name;
    totalChunks = s.total; uploadedChunks = s.uploaded; failedChunks = s.failed;
    inflight = new Map();
    lastChunkError = '';
    uploadError = s.status === 'failed' ? (s.last_error ?? 'Раздача не удалась') : '';
    revoked = s.status === 'revoked';
    cleaned = s.status === 'cleaned';
    completedKeyBytes = (s.status === 'completed' || s.status === 'revoked') && s.seed_hex
      ? hexToBytes(s.seed_hex) : null;
  }

  onMount(async () => {
    await listen<string>('auth-url', (e) => { authUrl = e.payload; });
    await listen<string>('auth-phrase', (e) => { backupPhrase = e.payload; });

    // Точные счетчики чанков из БД; доли сбросятся и перезаполнятся
    // воркерами в течение ~250 мс (троттлинг прогресса на Rust-стороне)
    await listen<{ session_id: string; uploaded: number; total: number; failed: number }>(
      'session-progress', (e) => {
        const p = e.payload;
        if (p.session_id !== activeSessionId) return;
        uploadedChunks = p.uploaded; totalChunks = p.total; failedChunks = p.failed;
        inflight = new Map();
      });
    await listen<{ session_id: string; file_id: string; chunk_index: number; fraction: number }>(
      'chunk-progress', (e) => {
        const p = e.payload;
        if (p.session_id !== activeSessionId) return;
        inflight.set(`${p.file_id}:${p.chunk_index}`, p.fraction);
        inflight = inflight; // присваивание триггерит реактивность Svelte
      });
    await listen<{ session_id: string; error: string; failed: boolean }>('chunk-error', (e) => {
      // Неокончательная ошибка чанка = предупреждение (будет retry с backoff)
      if (e.payload.session_id === activeSessionId && !e.payload.failed)
        lastChunkError = e.payload.error;
    });
    await listen<[string, number[]]>('session-completed', (e) => {
      const [sid, key] = e.payload;
      if (sid === activeSessionId) {
        completedKeyBytes = key; uploadError = ''; lastChunkError = '';
      }
    });
    await listen<[string, string]>('session-failed', (e) => {
      const [sid, msg] = e.payload;
      if (sid === activeSessionId) uploadError = msg;
    });

    // Автовход по ключу из системного keychain
    await autoLogin();
  });

  async function autoLogin() {
    authError = '';
    authChecked = false;
    try {
      isConnected = await invoke<boolean>('check_saved_auth');
      if (isConnected) await restoreSession();
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
      .then(async () => { isConnected = true; await restoreSession(); })
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
    // Preflight: предупреждаем о несовместимых с Windows именах до загрузки
    try {
      const warnings = await invoke<PathWarning[]>('validate_paths', { paths });
      if (warnings.length) { pendingPaths = paths; pathWarnings = warnings; return; }
    } catch (e) { alert(`Ошибка проверки путей: ${e}`); return; }
    await startUpload(paths);
  }

  async function startUpload(paths: string[]) {
    // Сброс состояния прошлой раздачи
    completedKeyBytes = null; revoked = false; cleaned = false; uploadError = ''; lastChunkError = '';
    uploadedChunks = 0; totalChunks = 0; failedChunks = 0; inflight = new Map();
    try {
      activeSessionId = await invoke<string>('start_upload_session', { sessionName, paths, validDays });
      // Сразу подтягиваем счетчики: воркеры могли начать раньше первого события
      const [t, u, f] = await invoke<[number, number, number]>(
        'get_session_progress', { sessionId: activeSessionId });
      totalChunks = t; uploadedChunks = u; failedChunks = f;
    } catch (e) { activeSessionId = null; alert(`Ошибка запуска загрузки: ${e}`); }
  }

  function closeWarnings() { pathWarnings = []; pendingPaths = null; }

  async function ignoreWarnings() {
    const paths = pendingPaths;
    closeWarnings();
    if (paths) await startUpload(paths);
  }

  async function exportShareZip() {
    if (!completedKeyBytes) return;
    const savePath = await save({ defaultPath: 'Sia.zip', filters: [{ name: 'ZIP', extensions: ['zip'] }] });
    if (!savePath) return;
    try {
      await invoke('generate_share_zip', {
        outputZipPath: savePath,
        sharingKeyBytes: completedKeyBytes,
        password: password.trim() || null,
      });
      alert('Готово! Отправьте архив Sia.zip получателю.');
    } catch (e) { alert(`Ошибка сборки пакета: ${e}`); }
  }

  async function revokeAccess() {
    if (!activeSessionId) return;
    // Честное предупреждение: отзыв не обрывает уже выданные токены мгновенно
    if (!confirm('Отозвать доступ? Уже начатые скачивания смогут продолжаться до ~5 минут.')) return;
    try { await invoke('revoke_session', { sessionId: activeSessionId }); revoked = true; }
    catch (e) { alert(`Ошибка отзыва: ${e}`); }
  }

  async function cleanupSession() {
    if (!activeSessionId) return;
    if (!confirm('Удалить загруженные объекты этой раздачи из Sia? Квота освободится, раздача станет недоступной навсегда.')) return;
    try { await invoke('cleanup_session', { sessionId: activeSessionId }); cleaned = true; }
    catch (e) { alert(`Ошибка очистки: ${e}`); }
  }
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
      <h2 class="mb-4 text-lg font-semibold">Параметры раздачи</h2>
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
          </label>
        </div>
      </div>
    </div>

    <div class="mb-6 flex gap-3">
      <button class="btn-primary" on:click={() => selectAndUpload(false)}>Выбрать файлы</button>
      <button class="btn-secondary" on:click={() => selectAndUpload(true)}>Выбрать папку целиком</button>
    </div>

    {#if activeSessionId && !completedKeyBytes && !uploadError}
      <div class="card mb-6">
        <div class="relative mb-3 h-6 overflow-hidden rounded-full bg-slate-800">
          <div class="h-full rounded-full bg-gradient-to-r from-emerald-500 to-emerald-400 transition-[width] duration-200" style="width:{percent}%"></div>
          <span class="absolute inset-0 grid place-items-center text-xs font-semibold text-white">{percent}%</span>
        </div>
        <p class="text-sm text-slate-300">Закреплено чанков: <strong class="text-white">{uploadedChunks}</strong> из {totalChunks}
          {#if failedChunks > 0}<span class="text-amber-400">(ошибок: {failedChunks})</span>{/if}
        </p>
        {#if lastChunkError}<p class="muted mt-2">Последняя ошибка (будет повтор): {lastChunkError}</p>{/if}
        <p class="muted mt-3 text-xs">Вы можете закрыть программу — при следующем запуске загрузка продолжится с незавершенных чанков.</p>
      </div>
    {/if}

    {#if uploadError}
      <div class="card mb-6 border-red-500/40 bg-red-500/10">
        <h3 class="mb-2 text-lg font-semibold text-red-300">Раздача не удалась</h3>
        <p class="text-sm text-red-300/90">{uploadError}</p>
        {#if cleaned}
          <p class="muted mt-3">Загруженные объекты удалены из Sia, квота освобождена.</p>
        {:else}
          <button class="btn-danger mt-4" on:click={cleanupSession}>Очистить квоту (удалить объекты)</button>
        {/if}
      </div>
    {/if}

    {#if cleaned && !uploadError && !completedKeyBytes}
      <div class="card mb-6">
        <p class="text-sm text-slate-300">Раздача удалена: объекты убраны из Sia, квота освобождена.</p>
      </div>
    {/if}

    {#if completedKeyBytes}
      <div class="card border-emerald-500/40 bg-emerald-500/10">
        <h3 class="mb-4 text-lg font-semibold text-emerald-300">Загрузка завершена!</h3>
        {#if revoked}
          <p class="text-sm text-amber-300">Доступ отозван — получатель больше не сможет скачать файлы.</p>
          {#if cleaned}
            <p class="muted mt-3">Объекты удалены из Sia, квота освобождена.</p>
          {:else}
            <button class="btn-danger mt-4" on:click={cleanupSession}>Очистить квоту (удалить объекты)</button>
          {/if}
        {:else}
          <div class="flex flex-wrap gap-3">
            <button class="btn-primary" on:click={exportShareZip}>Сохранить Sia.zip для отправки</button>
            <button class="btn-danger" on:click={revokeAccess}>Отозвать доступ</button>
          </div>
        {/if}
      </div>
    {/if}
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
