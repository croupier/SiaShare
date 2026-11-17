<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open, save } from '@tauri-apps/plugin-dialog';

  // --- Авторизация ---
  let isConnected = false;
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

  $: inflightSum = [...inflight.values()].reduce((a, b) => a + b, 0);
  $: percent = totalChunks > 0
    ? Math.min(100, Math.round(((uploadedChunks + inflightSum) / totalChunks) * 100))
    : 0;

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
    isConnected = await invoke<boolean>('check_saved_auth');
  });

  function connect(newAccount: boolean) {
    authError = ''; authUrl = ''; authWaiting = true;
    // invoke висит до одобрения в браузере — не await'им, UI живет на событиях
    invoke('connect_sia', { seedPhrase: newAccount ? null : seedPhrase.trim() || null })
      .then(() => { isConnected = true; })
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
    // Сброс состояния прошлой раздачи
    completedKeyBytes = null; revoked = false; uploadError = ''; lastChunkError = '';
    uploadedChunks = 0; totalChunks = 0; failedChunks = 0; inflight = new Map();
    try {
      activeSessionId = await invoke<string>('start_upload_session', { sessionName, paths, validDays });
      // Сразу подтягиваем счетчики: воркеры могли начать раньше первого события
      const [t, u, f] = await invoke<[number, number, number]>(
        'get_session_progress', { sessionId: activeSessionId });
      totalChunks = t; uploadedChunks = u; failedChunks = f;
    } catch (e) { activeSessionId = null; alert(`Ошибка запуска загрузки: ${e}`); }
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
</script>

<main class="container">
  {#if !isConnected}
    <div class="card">
      <h2>Подключение к Sia Storage</h2>
      {#if authWaiting}
        <p>Подтвердите подключение в открывшемся браузере…</p>
        {#if authUrl}
          <p class="muted">Если браузер не открылся, скопируйте ссылку:</p>
          <code class="phrase">{authUrl}</code>
          <button on:click={() => copyText(authUrl)}>Скопировать ссылку</button>
        {/if}
      {:else}
        <button on:click={() => connect(true)}>Создать новый аккаунт</button>
        <p class="muted">или войдите существующей 12-словной фразой:</p>
        <textarea bind:value={seedPhrase} rows="3" placeholder="word1 word2 … word12" />
        <button on:click={() => connect(false)} disabled={!seedPhrase.trim()}>Войти по фразе</button>
      {/if}
      {#if authError}<p class="warn">{authError}</p>{/if}
    </div>
  {:else}
    {#if backupPhrase}
      <div class="card warn-box">
        <h3>Запишите фразу восстановления!</h3>
        <p class="muted">Она показывается один раз и НЕ сохраняется на диске. Без нее не восстановить аккаунт.</p>
        <code class="phrase">{backupPhrase}</code>
        <button on:click={() => copyText(backupPhrase)}>Скопировать</button>
        <button on:click={() => (backupPhrase = '')}>Я сохранил фразу</button>
      </div>
    {/if}

    <h2>Отправка файлов в Sia Storage</h2>
    <div class="form-group">
      <label>Название раздачи:<input type="text" bind:value={sessionName} /></label>
      <label>Срок жизни ссылки (дней):<input type="number" min="1" bind:value={validDays} /></label>
      <label>Пароль на скачивание (необязательно):<input type="password" bind:value={password} /></label>
    </div>
    <div class="actions">
      <button on:click={() => selectAndUpload(false)}>Выбрать файлы</button>
      <button on:click={() => selectAndUpload(true)}>Выбрать папку целиком</button>
    </div>

    {#if activeSessionId && !completedKeyBytes && !uploadError}
      <div class="status-box">
        <div class="progress-bar">
          <div class="fill" style="width:{percent}%"></div>
          <span>{percent}%</span>
        </div>
        <p>Закреплено чанков: <strong>{uploadedChunks}</strong> из {totalChunks}
          {#if failedChunks > 0}<span class="warn">(ошибок: {failedChunks})</span>{/if}
        </p>
        {#if lastChunkError}<p class="muted">Последняя ошибка (будет повтор): {lastChunkError}</p>{/if}
        <small>Вы можете закрыть программу — при следующем запуске загрузка продолжится с незавершенных чанков.</small>
      </div>
    {/if}

    {#if uploadError}
      <div class="status-box warn-box">
        <h3>Раздача не удалась</h3>
        <p class="warn">{uploadError}</p>
      </div>
    {/if}

    {#if completedKeyBytes}
      <div class="status-box success">
        <h3>Загрузка завершена!</h3>
        {#if revoked}
          <p class="warn">Доступ отозван — получатель больше не сможет скачать файлы.</p>
        {:else}
          <button on:click={exportShareZip}>Сохранить Sia.zip для отправки</button>
          <button class="danger" on:click={revokeAccess}>Отозвать доступ</button>
        {/if}
      </div>
    {/if}
  {/if}
</main>

<style>
  .container { font-family: system-ui, sans-serif; padding: 24px; max-width: 720px; margin: 0 auto; }
  .form-group { display: flex; flex-direction: column; gap: 12px; margin-bottom: 16px; }
  .form-group label { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  input, textarea { padding: 8px; border: 1px solid #ccc; border-radius: 6px; }
  textarea { width: 100%; margin-bottom: 12px; box-sizing: border-box; }
  .actions { display: flex; gap: 12px; margin-bottom: 20px; }
  button { padding: 10px 16px; background: #2563eb; color: white; border: none; border-radius: 6px; cursor: pointer; margin-right: 8px; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }
  button.danger { background: #dc2626; }
  .card { border: 1px solid #ddd; padding: 24px; border-radius: 8px; margin-bottom: 16px; }
  .status-box { padding: 16px; border-radius: 8px; background: #f3f4f6; border: 1px solid #ddd; margin-bottom: 16px; }
  .status-box.success { background: #ecfdf5; border-color: #10b981; }
  .warn-box { background: #fef2f2; border-color: #dc2626; }
  .warn { color: #dc2626; }
  .muted { color: #666; font-size: 0.9em; }
  .phrase { display: block; background: #f9fafb; border: 1px solid #ddd; padding: 10px; border-radius: 6px; margin: 10px 0; word-break: break-all; user-select: all; }
  .progress-bar { background: #eee; border-radius: 6px; height: 24px; position: relative; overflow: hidden; margin-bottom: 10px; }
  .progress-bar .fill { background: #10b981; height: 100%; transition: width 0.2s; }
  .progress-bar span { position: absolute; inset: 0; text-align: center; line-height: 24px; font-size: 0.85em; }
</style>
