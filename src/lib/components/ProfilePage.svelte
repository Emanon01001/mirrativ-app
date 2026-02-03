<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import LiveCard from "$lib/components/LiveCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let { onOpenLive } = $props<{ onOpenLive: (live: any) => void }>();

  let profile = $state<any>(null);
  let currency = $state<any>(null);
  let liveHistory = $state<any[]>([]);
  let viewHistory = $state<any[]>([]);
  let loading = $state(false);
  let error = $state("");

  const extractLives = (res: any) => {
    const list = res?.lives ?? res?.live_list ?? res?.history ?? res?.data ?? [];
    return Array.isArray(list) ? list : [];
  };

  const loadProfile = async () => {
    loading = true;
    error = "";
    try {
      profile = await invoke("get_my_profile");
      currency = await invoke("get_user_currency");

      const userId = profile?.user_id ?? profile?.id ?? profile?.user?.user_id;
      if (userId) {
        const liveRes: any = await invoke("get_live_history", { userId: String(userId) });
        liveHistory = extractLives(liveRes);

        const viewRes: any = await invoke("get_view_history", { userId: String(userId) });
        viewHistory = extractLives(viewRes);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  };

  onMount(loadProfile);
</script>

<section class="profile">
  <header class="section-head">
    <div>
      <p class="kicker">Profile</p>
      <h2>アカウント概要</h2>
    </div>
    <button class="ghost" onclick={loadProfile} disabled={loading}>
      {loading ? "更新中..." : "更新"}
    </button>
  </header>

  {#if error}
    <ErrorMessage
      message={error}
      onRetry={loadProfile}
      onDismiss={() => (error = "")}
    />
  {/if}

  {#if loading}
    <div class="profile-card">
      <Skeleton variant="avatar" />
      <div class="info" style="flex: 1;">
        <Skeleton variant="text" height="28px" width="60%" />
        <Skeleton variant="text" height="20px" width="90%" />
        <Skeleton variant="text" height="20px" width="40%" />
      </div>
    </div>
  {:else if profile}
    <div class="profile-card">
      <div class="avatar">
        {#if profile.profile_image_url}
          <img src={profile.profile_image_url} alt="avatar" />
        {:else}
          <span>ME</span>
        {/if}
      </div>
      <div class="info">
        <h3>{profile.name ?? "名前なし"}</h3>
        <p class="desc">{profile.description ?? "自己紹介はまだありません"}</p>
        <div class="stats">
          <span>フォロワー {profile.follower_num ?? 0}</span>
          <span>フォロー {profile.following_num ?? 0}</span>
          <span>配信数 {profile.live_count ?? 0}</span>
        </div>
      </div>
    </div>
  {/if}

  <div class="wallet">
    <h3>ウォレット</h3>
    {#if currency}
      <div class="wallet-grid">
        {#each Object.entries(currency) as [key, value]}
          <div class="wallet-item">
            <span class="wallet-key">{key}</span>
            <span class="wallet-value">{value}</span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="muted">通貨データなし</p>
    {/if}
  </div>

  <div class="history">
    <h3>配信履歴</h3>
    <div class="live-grid">
      {#if loading}
        {#each Array(3) as _}
          <Skeleton variant="card" />
        {/each}
      {:else if liveHistory.length === 0}
        <div class="empty">履歴はありません</div>
      {:else}
        {#each liveHistory as item}
          <LiveCard live={item.live ?? item} onSelect={onOpenLive} />
        {/each}
      {/if}
    </div>
  </div>

  <div class="history">
    <h3>視聴履歴</h3>
    <div class="live-grid">
      {#if loading}
        {#each Array(3) as _}
          <Skeleton variant="card" />
        {/each}
      {:else if viewHistory.length === 0}
        <div class="empty">履歴はありません</div>
      {:else}
        {#each viewHistory as item}
          <LiveCard live={item.live ?? item} onSelect={onOpenLive} />
        {/each}
      {/if}
    </div>
  </div>
</section>

<style>
  .profile {
    display: grid;
    gap: 22px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .kicker {
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.7rem;
    color: var(--accent-500);
    margin: 0 0 6px 0;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(1.2rem, 2vw, 1.8rem);
  }

  .profile-card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 16px;
    padding: 18px;
    border-radius: 20px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
  }

  .avatar {
    width: 72px;
    height: 72px;
    border-radius: 20px;
    background: rgba(16, 27, 30, 0.1);
    display: grid;
    place-items: center;
    overflow: hidden;
    font-weight: 700;
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .info h3 {
    margin: 0 0 6px 0;
    font-size: 1.2rem;
    font-family: var(--font-display);
  }

  .desc {
    margin: 0 0 10px 0;
    color: var(--ink-600);
  }

  .stats {
    display: flex;
    gap: 12px;
    font-size: 0.85rem;
    color: var(--ink-600);
  }

  .wallet {
    padding: 18px;
    border-radius: 18px;
    background: rgba(16, 27, 30, 0.04);
  }

  .wallet h3 {
    margin: 0 0 12px 0;
    font-size: 1rem;
  }

  .wallet-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 12px;
  }

  .wallet-item {
    padding: 10px 12px;
    border-radius: 14px;
    background: #fff;
    box-shadow: var(--shadow-soft);
    display: grid;
    gap: 6px;
  }

  .wallet-key {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--ink-500);
  }

  .wallet-value {
    font-weight: 700;
    font-size: 1rem;
  }

  .history h3 {
    margin: 0 0 12px 0;
    font-size: 1rem;
  }

  .live-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
  }

  .ghost {
    border: 1px solid rgba(16, 27, 30, 0.2);
    background: transparent;
    color: var(--ink-700);
    border-radius: 999px;
    padding: 8px 16px;
    font-weight: 600;
    cursor: pointer;
  }

  .ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty {
    padding: 16px;
    border-radius: 16px;
    background: rgba(16, 27, 30, 0.06);
    color: var(--ink-600);
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }

  .muted {
    color: var(--ink-500);
  }

  @media (max-width: 720px) {
    .profile-card {
      grid-template-columns: 1fr;
    }
  }
</style>
