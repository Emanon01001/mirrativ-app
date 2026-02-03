<script lang="ts">
  let {
    mrId = $bindable(),
    unique = $bindable(),
    remember = $bindable(),
    loading,
    error,
    onLogin,
    onReset,
    onTwitterLogin,
    twitterLoginLoading = false,
  } = $props<{
    mrId: string;
    unique: string;
    remember: boolean;
    loading: boolean;
    error: string;
    onLogin: (event: Event) => void;
    onReset: () => void;
    onTwitterLogin: () => void;
    twitterLoginLoading: boolean;
  }>();
</script>

<section class="login">
  <div class="login-hero">
    <p class="eyebrow">Mirrativ PC Client</p>
    <h1>配信に“近い”操作感を、デスクトップに。</h1>
    <p class="lead">
      Androidアプリのセッション情報を利用して、安全に閲覧・フォローの体験を再現します。
    </p>
  </div>

  <button
    type="button"
    class="twitter-btn"
    onclick={onTwitterLogin}
    disabled={twitterLoginLoading || loading}
  >
    {twitterLoginLoading ? "認証ウィンドウを開いています..." : "Twitterでログイン"}
  </button>

  <div class="divider">
    <span>または手動でセッション情報を入力</span>
  </div>

  <form class="login-card" onsubmit={onLogin}>
    <div class="field">
      <label for="mr_id">mr_id</label>
      <input id="mr_id" placeholder="例: 123456789" bind:value={mrId} />
    </div>
    <div class="field">
      <label for="unique">unique (f)</label>
      <input id="unique" placeholder="例: 9c2f..." bind:value={unique} />
    </div>

    <label class="remember">
      <input type="checkbox" bind:checked={remember} />
      セッションをこの端末に保存
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="actions">
      <button type="submit" disabled={loading}>
        {loading ? "接続中..." : "セッションを開始"}
      </button>
      <button type="button" class="ghost" onclick={onReset}>クリア</button>
    </div>
  </form>

  <div class="login-hint">
    <h2>取得方法のヒント</h2>
    <ol>
      <li>AndroidのMirrativアプリでログイン</li>
      <li>HTTPキャプチャでCookieの<code>mr_id</code>と<code>f</code>を確認</li>
      <li>この画面に貼り付けて開始</li>
    </ol>
  </div>
</section>

<style>
  .login {
    display: grid;
    gap: 24px;
    padding: 32px;
    border-radius: 28px;
    background: var(--panel-surface);
    box-shadow: var(--shadow-soft);
  }

  .login-hero {
    display: grid;
    gap: 12px;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.3em;
    font-size: 0.7rem;
    color: var(--accent-500);
  }

  h1 {
    font-family: var(--font-display);
    font-size: clamp(1.6rem, 2.4vw, 2.4rem);
    margin: 0;
  }

  .lead {
    margin: 0;
    color: var(--ink-600);
    max-width: 48ch;
  }

  .twitter-btn {
    border: none;
    border-radius: 999px;
    padding: 12px 24px;
    font-weight: 700;
    font-size: 1rem;
    background: #1da1f2;
    color: #fff;
    cursor: pointer;
  }

  .twitter-btn:hover:not(:disabled) {
    background: #0d8bd9;
  }

  .twitter-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--ink-600);
    font-size: 0.8rem;
  }

  .divider::before,
  .divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: rgba(16, 27, 30, 0.15);
  }

  .login-card {
    display: grid;
    gap: 16px;
    padding: 20px;
    border-radius: 20px;
    background: var(--card-surface);
  }

  .field {
    display: grid;
    gap: 6px;
  }

  label {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.18em;
    color: var(--ink-600);
  }

  input {
    border: 1px solid rgba(16, 27, 30, 0.15);
    border-radius: 12px;
    padding: 10px 12px;
    background: #fff;
    font-size: 0.95rem;
  }

  .remember {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--ink-600);
  }

  .actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 10px 18px;
    font-weight: 700;
    background: var(--accent-500);
    color: #fff;
    cursor: pointer;
  }

  button.ghost {
    background: transparent;
    color: var(--ink-700);
    border: 1px solid rgba(16, 27, 30, 0.2);
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }

  .login-hint {
    padding: 18px;
    border-radius: 18px;
    background: rgba(242, 95, 76, 0.08);
  }

  .login-hint h2 {
    margin-top: 0;
    font-size: 1rem;
  }

  .login-hint ol {
    margin: 0;
    padding-left: 18px;
    color: var(--ink-700);
  }

  code {
    background: rgba(15, 42, 39, 0.08);
    padding: 2px 6px;
    border-radius: 6px;
  }
</style>
