<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import LoginPage from "$lib/components/LoginPage.svelte";
  import HomePage from "$lib/components/HomePage.svelte";
  import SearchPage from "$lib/components/SearchPage.svelte";
  import FollowPage from "$lib/components/FollowPage.svelte";
  import ProfilePage from "$lib/components/ProfilePage.svelte";
  import WatchPage from "$lib/components/WatchPage.svelte";
  import Debugpage from "$lib/components/debugpage.svelte";
  import Streampage from "$lib/components/Streampage.svelte";

  type Page = "home" | "search" | "follow" | "watch" | "stream" |"profile" | "settings" | "debugpage";
  type SessionTuple = [string, string];
  type FinalizeAuthOptions = {
    assignFields?: boolean;
    persist?: boolean;
    shouldRemember?: boolean;
  };
  const NOTICE_POLL_INTERVAL_MS = 60_000;

  let page = $state<Page>("home");
  let selectedLiveId = $state("");
  let selectedLive = $state<any>(null);
  let debugCatalogTabs = $state<any[]>([]);

  let mrId = $state("");
  let unique = $state("");
  let remember = $state(true);
  let authed = $state(false);
  let loginLoading = $state(false);
  let loginError = $state("");

  let currentUser = $state<any>(null);
  let noticeCounts = $state<Record<string, number> | null>(null);
  let noticeError = $state("");

  let noticeTimer: ReturnType<typeof setInterval> | null = null;
  let twitterLoginLoading = $state(false);
  let authUnlisten: (() => void) | null = null;
  let hasSavedSession = $state(false);

  type AuthResult = {
    success: boolean;
    mr_id: string;
    unique: string;
    error: string | null;
  };

  const pageTitle = $derived(
    (() => {
      switch (page) {
        case "home":
          return "カタログ";
        case "search":
          return "検索";
        case "follow":
          return "フォロー";
        case "watch":
          return "視聴";
        case "stream":
          return "配信";
        case "profile":
          return "プロフィール";
        case "settings":
          return "設定";
        case "debugpage":
          return "テストページ";
        default:
          return "Mirrativ";
      }
    })()
  );

  const totalNotices = $derived(
    Object.values(noticeCounts ?? {}).reduce((sum, value) => {
      if (typeof value === "number") return sum + value;
      return sum;
    }, 0)
  );

  const openLive = (live: any) => {
    const liveId =
      live?.live_id ?? live?.id ?? live?.live?.live_id ?? live?.live?.id ?? "";
    if (!liveId) return;
    selectedLiveId = String(liveId);
    selectedLive = live;
    page = "watch";
  };

  const handleTabsLoaded = (tabs: any[]) => {
    debugCatalogTabs = tabs;
  };

  const handleNavigate = (next: string) => {
    const nextPage = next as Page;
    if (nextPage === "watch") {
      selectedLiveId = "";
      selectedLive = null;
    }
    page = nextPage;
  };

  const normalizeSession = (inputMrId: string, inputUnique: string): SessionTuple => [
    inputMrId.trim(),
    inputUnique.trim()
  ];

  const setSessionFields = ([savedMrId, savedUnique]: SessionTuple) => {
    mrId = savedMrId;
    unique = savedUnique;
  };

  const loadSavedSession = async () => invoke<SessionTuple | null>("load_session");

  const clearNoticeTimer = () => {
    if (!noticeTimer) return;
    clearInterval(noticeTimer);
    noticeTimer = null;
  };

  const startNoticeTimer = () => {
    clearNoticeTimer();
    noticeTimer = setInterval(() => {
      void refreshNotices();
    }, NOTICE_POLL_INTERVAL_MS);
  };

  const persistSession = async (session: SessionTuple, shouldRemember: boolean) => {
    const [savedMrId, savedUnique] = session;
    if (shouldRemember) {
      await invoke("save_session", { mrId: savedMrId, unique: savedUnique });
      hasSavedSession = true;
      return;
    }
    await invoke("delete_session");
    hasSavedSession = false;
  };

  const refreshSavedSessionFlag = async () => {
    hasSavedSession = (await loadSavedSession()) !== null;
  };

  const ensureGuestSession = async () => {
    try {
      await invoke("bootstrap_guest");
    } catch (e) {
      console.warn("ゲストセッション初期化失敗", e);
    }
  };

  const refreshUser = async () => {
    if (!authed) return;
    try {
      currentUser = await invoke("get_my_profile");
    } catch (e) {
      console.warn("プロフィール取得失敗", e);
    }
  };

  const refreshNotices = async () => {
    if (!authed) return;
    try {
      noticeCounts = await invoke("get_notice_counts");
      noticeError = "";
    } catch (e) {
      noticeError = e instanceof Error ? e.message : String(e);
    }
  };

  const finalizeAuth = async (
    session: SessionTuple,
    {
      assignFields = true,
      persist = false,
      shouldRemember = remember
    }: FinalizeAuthOptions = {}
  ) => {
    const [savedMrId, savedUnique] = session;
    await invoke("login", { mrId: savedMrId, unique: savedUnique });
    if (assignFields) {
      setSessionFields(session);
    }
    authed = true;
    loginError = "";

    if (persist) {
      await persistSession(session, shouldRemember);
    }

    await refreshUser();
    await refreshNotices();
    startNoticeTimer();
  };

  const handleLogin = async (event: Event) => {
    event.preventDefault();
    loginError = "";
    loginLoading = true;
    try {
      const session = normalizeSession(mrId, unique);
      if (!session[0] || !session[1]) {
        loginError = "mr_id と unique を入力してください";
        authed = false;
        return;
      }
      await finalizeAuth(session, { persist: true, shouldRemember: remember });
    } catch (e) {
      loginError = e instanceof Error ? e.message : String(e);
      authed = false;
      clearNoticeTimer();
    } finally {
      loginLoading = false;
    }
  };

  const handleReset = () => {
    mrId = "";
    unique = "";
    loginError = "";
  };

  const handleLogout = async () => {
    await invoke("reset_session");
    // ゲストセッションを再取得（配信閲覧用）
    await invoke("bootstrap_guest");
    authed = false;
    noticeCounts = null;
    noticeError = "";
    loginError = "";
    clearNoticeTimer();
    // mrId, unique, currentUser は保存セッション表示用に残す
    await refreshSavedSessionFlag();
  };

  const handleRelogin = async () => {
    const saved = await loadSavedSession();
    if (!saved) return;
    loginLoading = true;
    loginError = "";
    try {
      await finalizeAuth(saved);
    } catch (e) {
      loginError = e instanceof Error ? e.message : String(e);
      authed = false;
      clearNoticeTimer();
    } finally {
      loginLoading = false;
    }
  };

  const handleDeleteSession = async () => {
    await invoke("delete_session");
    hasSavedSession = false;
    loginError = "";
    if (!authed) {
      mrId = "";
      unique = "";
      currentUser = null;
    }
  };

  const handleTwitterLogin = async () => {
    twitterLoginLoading = true;
    loginError = "";
    try {
      await invoke("open_twitter_login");
    } catch (e) {
      loginError = e instanceof Error ? e.message : String(e);
      twitterLoginLoading = false;
    }
  };

  onMount(async () => {
    // 保存済みセッションは保持するが、起動時は常にゲストで開始
    const saved = await loadSavedSession();
    hasSavedSession = saved !== null;
    if (saved) {
      setSessionFields(saved);
    }
    await ensureGuestSession();

    // Twitter認証の成功イベントをリスン
    authUnlisten = await listen<AuthResult>("auth://login-success", async (event) => {
      const result = event.payload;
      if (result.success) {
        try {
          await finalizeAuth(normalizeSession(result.mr_id, result.unique), {
            persist: true,
            shouldRemember: remember
          });
        } catch (e) {
          loginError = e instanceof Error ? e.message : String(e);
          authed = false;
          clearNoticeTimer();
        } finally {
          twitterLoginLoading = false;
        }
      } else {
        loginError = result.error ?? "Twitter認証に失敗しました";
        twitterLoginLoading = false;
      }
    });

    // 認証ウィンドウが閉じられた場合
    const cancelUnlisten = await listen("auth://login-cancelled", () => {
      twitterLoginLoading = false;
    });

    // cleanup 用に authUnlisten を拡張
    const originalUnlisten = authUnlisten;
    authUnlisten = () => {
      originalUnlisten();
      cancelUnlisten();
    };
  });

  onDestroy(() => {
    clearNoticeTimer();
    if (authUnlisten) authUnlisten();
  });
</script>

<a href="#main-content" class="skip-link">
  メインコンテンツへスキップ
</a>

<div class="app-shell">
  <Sidebar
    {page}
    onNavigate={handleNavigate}
    {noticeCounts}
    user={currentUser}
    {authed}
  />

  <main id="main-content" class="main">
    <header class="topbar">
      <div>
        <p class="eyebrow">Mirrativ Desktop</p>
        <h1>{pageTitle}</h1>
      </div>
      <div class="top-meta">
        <div class="pill">
          <span>通知</span>
          <strong>{authed ? totalNotices : "-"}</strong>
        </div>
        <div class="profile-pill">
          <span>{authed ? currentUser?.name ?? "ユーザー" : "ゲスト"}</span>
        </div>
      </div>
    </header>

    {#if noticeError}
      <p class="notice-error">{noticeError}</p>
    {/if}

    <section class="content">
      {#if page === "home"}
        <HomePage
          onOpenLive={openLive}
          onTabsLoaded={handleTabsLoaded}
        />
      {:else if page === "search"}
        <SearchPage onOpenLive={openLive} />
      {:else if page === "follow"}
        <FollowPage onOpenLive={openLive} />
      {:else if page === "watch"}
          <WatchPage initialLiveId={selectedLiveId} initialLive={selectedLive} {authed} />
      {:else if page === "stream"}
          <Streampage {authed} />
      {:else if page === "profile"}
        <ProfilePage onOpenLive={openLive} />
      {:else if page === "settings"}
        <div class="settings">
          {#if authed}
            <div class="session-info">
              <div class="session-header">
                {#if currentUser?.profile_image_url}
                  <img class="session-avatar" src={currentUser.profile_image_url} alt="" />
                {/if}
                <div>
                  <h2 class="session-name">{currentUser?.name ?? "ユーザー"}</h2>
                  <p class="session-sub">ログイン中</p>
                </div>
              </div>
              <div class="session-details">
                <div class="session-row">
                  <span class="session-label">MR_ID</span>
                  <span class="session-value">{mrId}</span>
                </div>
              </div>
            </div>
            <div class="settings-actions">
              <button class="danger" onclick={handleLogout}>ログアウト</button>
              <button class="ghost" onclick={handleDeleteSession}>保存データ削除</button>
            </div>
          {:else if hasSavedSession}
            <div class="session-info">
              <div class="session-header">
                {#if currentUser?.profile_image_url}
                  <img class="session-avatar" src={currentUser.profile_image_url} alt="" />
                {/if}
                <div>
                  <h2 class="session-name">{currentUser?.name ?? "ユーザー"}</h2>
                  <p class="session-sub">ログアウト中</p>
                </div>
              </div>
              <div class="session-details">
                <div class="session-row">
                  <span class="session-label">MR_ID</span>
                  <span class="session-value">{mrId}</span>
                </div>
                <div class="session-row">
                  <span class="session-label">UNIQUE (F)</span>
                  <span class="session-value">{unique}</span>
                </div>
              </div>
              <div class="settings-actions">
                <button onclick={handleRelogin} disabled={loginLoading}>
                  {loginLoading ? "ログイン中..." : "再ログイン"}
                </button>
                <button class="ghost" onclick={handleDeleteSession}>保存データ削除</button>
              </div>
              {#if loginError}
                <p class="error">{loginError}</p>
              {/if}
            </div>
          {:else}
            <LoginPage
              bind:mrId
              bind:unique
              bind:remember
              loading={loginLoading}
              error={loginError}
              onLogin={handleLogin}
              onReset={handleReset}
              onTwitterLogin={handleTwitterLogin}
              {twitterLoginLoading}
            />
          {/if}
        </div>
      {:else if page == "debugpage"}
        <Debugpage
          mrId={mrId}
          unique={unique}
          user={currentUser}
          authed={authed}
          catalogTabs={debugCatalogTabs} />
      {/if}
    </section>
  </main>
</div>

<style>
  :global(:root) {
    font-family: "Noto Sans JP", "Space Grotesk", sans-serif;
    color: #101b1e;
    background-color: #f7f3ea;
    --font-display: "Space Grotesk", "Noto Sans JP", sans-serif;
    --ink-900: #0b1b1e;
    --ink-700: #1f3739;
    --ink-600: #3a5152;
    --ink-500: #607173;
    --ink-50: #f7f3ea;
    --accent-500: #f25f4c;
    --accent-700: #d4483a;
    --card-surface: #fff;
    --panel-surface: rgba(255, 255, 255, 0.85);
    --sidebar-surface: #0d2a2b;
    --shadow-soft: 0 12px 28px rgba(15, 42, 39, 0.08);
    --shadow-bold: 0 18px 40px rgba(15, 42, 39, 0.18);
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    min-height: 100vh;
    background:
      radial-gradient(1200px 600px at 12% -10%, rgba(242, 95, 76, 0.2), transparent 60%),
      radial-gradient(900px 600px at 90% 10%, rgba(61, 176, 162, 0.2), transparent 55%),
      linear-gradient(160deg, #f7f3ea 0%, #e7f4f1 100%);
    color: var(--ink-900);
  }

  :global(a) {
    color: inherit;
    text-decoration: none;
  }

  /* フォーカスインジケーター */
  :global(button:focus-visible),
  :global(input:focus-visible),
  :global(textarea:focus-visible),
  :global(select:focus-visible),
  :global(a:focus-visible) {
    outline: 2px solid var(--accent-500);
    outline-offset: 2px;
    border-radius: 4px;
  }

  :global(button),
  :global(input),
  :global(textarea),
  :global(select) {
    font-family: inherit;
  }

  .skip-link {
    position: absolute;
    top: -100px;
    left: 8px;
    z-index: 9999;
    background: var(--accent-500);
    color: #fff;
    padding: 12px 16px;
    font-weight: 600;
    border-radius: 8px;
    box-shadow: var(--shadow-bold);
    transition: top 0.2s;
  }

  .skip-link:focus {
    top: 8px;
    outline: 2px solid var(--ink-900);
    outline-offset: 2px;
  }

  .app-shell {
    display: grid;
    grid-template-columns: minmax(220px, 280px) minmax(0, 1fr);
    gap: 24px;
    padding: 24px;
    min-height: 100vh;
  }

  .main {
    display: grid;
    grid-template-rows: auto 1fr;
    align-content: start;
    gap: 10px;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 18px;
    border-radius: 20px;
    background: var(--panel-surface);
    box-shadow: var(--shadow-soft);
  }

  .eyebrow {
    text-transform: uppercase;
    letter-spacing: 0.3em;
    font-size: 0.7rem;
    color: var(--accent-500);
    margin: 0 0 6px 0;
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(1.4rem, 2.6vw, 2.2rem);
  }

  .top-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(242, 95, 76, 0.12);
    color: var(--accent-700);
    padding: 6px 12px;
    border-radius: 999px;
    font-weight: 700;
  }

  .profile-pill {
    background: rgba(15, 42, 39, 0.08);
    padding: 6px 12px;
    border-radius: 999px;
    font-weight: 600;
  }

  .content {
    display: grid;
    gap: 10px;
    align-content: start;
  }

  .settings {
    padding: 24px;
    border-radius: 20px;
    background: var(--panel-surface);
    box-shadow: var(--shadow-soft);
    display: grid;
    gap: 12px;
  }

  .settings-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .settings button {
    border: none;
    border-radius: 999px;
    padding: 10px 18px;
    font-weight: 700;
    background: var(--accent-500);
    color: #fff;
    cursor: pointer;
  }

  .session-saved-msg {
    margin: 0;
    color: var(--ink-600);
    font-size: 0.9rem;
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }

  .settings .ghost {
    background: transparent;
    border: 1px solid rgba(16, 27, 30, 0.2);
    color: var(--ink-700);
  }

  .settings .danger {
    background: var(--accent-500);
    color: #fff;
  }

  .session-info {
    display: grid;
    gap: 16px;
    padding: 20px;
    border-radius: 20px;
    background: var(--card-surface);
  }

  .session-header {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .session-avatar {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    object-fit: cover;
  }

  .session-name {
    margin: 0;
    font-size: 1.1rem;
  }

  .session-sub {
    margin: 0;
    font-size: 0.8rem;
    color: var(--ink-500);
  }

  .session-details {
    display: grid;
    gap: 8px;
  }

  .session-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(15, 42, 39, 0.04);
    font-size: 0.85rem;
  }

  .session-label {
    color: var(--ink-500);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .session-value {
    color: var(--ink-700);
    font-weight: 500;
  }

  .notice-error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }

  @media (max-width: 980px) {
    .app-shell {
      grid-template-columns: 1fr;
      padding: 16px;
    }

    .topbar {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
