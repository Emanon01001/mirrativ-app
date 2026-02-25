<script lang="ts">

  let { page, onNavigate, noticeCounts, user, authed } = $props<{
    page: string;
    onNavigate: (page: string) => void;
    noticeCounts: Record<string, number> | null;
    user: any;
    authed: boolean;
  }>();


  const items = [
    { id: "home", label: "ホーム", icon: "◆" },
    { id: "search", label: "検索", icon: "🔍" },
    { id: "follow", label: "フォロー", icon: "◎" },
    { id: "watch", label: "視聴", icon: "▶" },
    { id: "stream", label: "配信", icon: "📡" },
    { id: "profile", label: "プロフィール", icon: "▣" },
    { id: "settings", label: "設定", icon: "⚙" },
    { id: "debugpage", label: "テストページ", icon: "🧪" }
  ];

  const totalNotice = $derived(
    (Object.values(noticeCounts ?? {}) as number[]).reduce(
      (sum, value) => sum + (Number.isFinite(value) ? value : 0),
      0
    )
  );
</script>

<aside class="sidebar">
  <div class="brand">
    <div class="mark">MIRRATIV</div>
    <div class="sub">Desktop Studio</div>
  </div>

  <div class="user-card">
    <div class="user-avatar">
      {#if user?.profile_image_url}
        <img src={user.profile_image_url} alt="avatar" />
      {:else}
        <span>ME</span>
      {/if}
    </div>
    <div class="user-meta">
      <p class="user-name">{user?.name ?? "ゲスト"}</p>
      <p class="user-id">{authed ? `@${user?.user_id ?? "-"}` : "ゲスト"}</p>
    </div>
  </div>

  <nav class="nav">
    {#each items as item}
      <button
        type="button"
        class:active={page === item.id}
        onclick={() => onNavigate(item.id)}
      >
        <span class="icon">{item.icon}</span>
        <span>{item.label}</span>
        {#if item.id === "follow" && totalNotice > 0}
          <span class="badge">{totalNotice}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="notice-card">
    <div class="notice-head">
      <span>通知</span>
      {#if totalNotice > 0}
        <span class="notice-pill">{totalNotice}</span>
      {/if}
    </div>
    <div class="notice-grid">
      {#if noticeCounts}
        {#each Object.entries(noticeCounts) as [key, value]}
          <div class="notice-item">
            <span class="notice-key">{key}</span>
            <span class="notice-value">{value}</span>
          </div>
        {/each}
      {:else}
        <p class="notice-empty">通知データなし</p>
      {/if}
    </div>
  </div>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 22px;
    border-radius: 24px;
    background: var(--sidebar-surface);
    color: var(--ink-50);
    box-shadow: var(--shadow-bold);
    position: sticky;
    top: 24px;
    align-self: start;
    min-height: calc(100vh - 48px);
  }

  .brand {
    display: grid;
    gap: 4px;
  }

  .mark {
    font-family: var(--font-display);
    font-size: 1.1rem;
    letter-spacing: 0.2em;
  }

  .sub {
    font-size: 0.7rem;
    opacity: 0.7;
    text-transform: uppercase;
    letter-spacing: 0.2em;
  }

  .user-card {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 12px;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.08);
  }

  .user-avatar {
    width: 44px;
    height: 44px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.15);
    display: grid;
    place-items: center;
    overflow: hidden;
    font-weight: 700;
  }

  .user-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .user-meta {
    display: grid;
    gap: 2px;
  }

  .user-name {
    margin: 0;
    font-weight: 600;
  }

  .user-id {
    margin: 0;
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .nav {
    display: grid;
    gap: 8px;
  }

  .nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    border: none;
    border-radius: 14px;
    padding: 10px 12px;
    background: transparent;
    color: inherit;
    cursor: pointer;
    text-align: left;
    font-weight: 600;
    transition: background 0.2s ease, transform 0.2s ease;
  }

  .nav button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .nav button:hover:not(:disabled),
  .nav button.active {
    background: rgba(255, 255, 255, 0.12);
    transform: translateX(2px);
  }

  .icon {
    width: 20px;
    display: inline-flex;
    justify-content: center;
  }

  .badge {
    margin-left: auto;
    background: var(--accent-500);
    color: #fff;
    font-size: 0.7rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 999px;
  }

  .notice-card {
    margin-top: auto;
    padding: 14px;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.08);
    display: grid;
    gap: 10px;
  }

  .notice-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-weight: 600;
  }

  .notice-pill {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 0.7rem;
  }

  .notice-grid {
    display: grid;
    gap: 6px;
  }

  .notice-item {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    opacity: 0.85;
  }

  .notice-key {
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .notice-empty {
    font-size: 0.75rem;
    opacity: 0.6;
    margin: 0;
  }

  @media (max-width: 980px) {
    .sidebar {
      position: static;
      min-height: auto;
    }

    .notice-card {
      display: none;
    }

    .nav {
      grid-auto-flow: column;
      grid-auto-columns: max-content;
      overflow-x: auto;
      padding-bottom: 4px;
    }
  }

  @media (max-width: 720px) {
    .sidebar {
      padding: 14px;
      gap: 14px;
    }

    .user-card {
      display: none;
    }
  }
</style>
