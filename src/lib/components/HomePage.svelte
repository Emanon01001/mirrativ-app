<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import LiveCard from "$lib/components/LiveCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let { onOpenLive } = $props<{ onOpenLive: (live: any) => void }>();

  let tabs = $state<any[]>([]);
  let selectedTab = $state<any>(null);
  let banners = $state<any[]>([]);
  let lives = $state<any[]>([]);
  let currentCursor = $state<string | null>(null);
  let nextCursor = $state<string | null>(null);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state("");
  let bannerLoading = $state(false);
  let bootstrapped = $state(false);

  const getTabId = (tab: any) =>
    tab?.id ?? tab?.tab_id ?? tab?.tab?.id ?? tab?.tab?.tab_id ?? null;
  const getTabLabel = (tab: any) =>
    tab?.tab_name ?? tab?.name ?? tab?.title ?? tab?.tab?.name ?? "タブ";
  const getTabAppId = (tab: any) =>
    tab?.app_id ?? tab?.app?.app_id ?? tab?.app?.id ?? null;

  const extractList = (res: any, keys: string[]) => {
    for (const key of keys) {
      const value = res?.[key];
      if (Array.isArray(value)) return value;
    }
    return [];
  };

  const extractCursors = (res: any) => ({
    current: res?.current_cursor ?? res?.currentCursor ?? null,
    next: res?.next_cursor ?? res?.nextCursor ?? null
  });

  const getLiveId = (live: any) =>
    live?.live_id ?? live?.id ?? live?.live?.live_id ?? live?.live?.id ?? null;

  const mergeLives = (base: any[], incoming: any[]) => {
    const seen = new Set(base.map(getLiveId).filter(Boolean));
    const merged = [...base];
    for (const item of incoming) {
      const id = getLiveId(item);
      if (!id || seen.has(id)) continue;
      merged.push(item);
      seen.add(id);
    }
    return merged;
  };

  const applyLivesResponse = (res: any, replace: boolean) => {
    const list = extractList(res, ["lives", "live_list", "data", "catalog_lives"]);
    lives = replace ? list : mergeLives(lives, list);
    const { current, next } = extractCursors(res);
    if (current !== null) currentCursor = current;
    if (next !== undefined) nextCursor = next ?? null;
  };

  const loadTabs = async () => {
    loading = true;
    error = "";
    try {
      if (!bootstrapped) {
        await invoke("bootstrap_guest");
        bootstrapped = true;
      }
      const res: any = await invoke("get_catalog_tabs");
      const list = extractList(res, ["tabs", "tab_list", "data", "catalog_tabs"]);
      tabs = list;
      if (!selectedTab && list.length > 0) {
        selectedTab = list[0];
      }
      await loadTabData();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  };

  const getLivesFromTab = (tab: any) => {
    const list =
      tab?.lives ??
      tab?.live_list ??
      tab?.data?.lives ??
      tab?.data?.live_list ??
      [];
    return Array.isArray(list) ? list : [];
  };

  const loadTabData = async () => {
    if (!selectedTab) return;
    const tabId = getTabId(selectedTab);
    if (!tabId) return;

    const appId = getTabAppId(selectedTab);

    loading = true;
    bannerLoading = true;
    error = "";
    currentCursor = null;
    nextCursor = null;
    try {
      const args: Record<string, any> = { tabId: String(tabId) };
      if (appId) args.appId = String(appId);

      const tabLives = getLivesFromTab(selectedTab);
      if (tabLives.length > 0) {
        lives = tabLives;
      }

      const livesRes = await invoke("get_catalog_lives", args);
      applyLivesResponse(livesRes, true);

      const bannersRes = await invoke("get_catalog_banners", args);
      banners = extractList(bannersRes, ["banners", "banner_list", "data", "catalog_banners"]);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
      bannerLoading = false;
    }
  };

  const handleTabSelect = async (tab: any) => {
    selectedTab = tab;
    await loadTabData();
  };

  const loadMore = async () => {
    if (!selectedTab || !nextCursor) return;
    const tabId = getTabId(selectedTab);
    if (!tabId) return;

    const appId = getTabAppId(selectedTab);
    loadingMore = true;
    error = "";
    try {
      const args: Record<string, any> = { tabId: String(tabId), cursor: nextCursor };
      if (appId) args.appId = String(appId);
      const res = await invoke("get_catalog_lives", args);
      applyLivesResponse(res, false);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loadingMore = false;
    }
  };

  onMount(() => {
    loadTabs();
  });
</script>

<section class="home">
  <header class="section-head">
    <div>
      <p class="kicker">Catalog</p>
      <h2>いま注目の配信</h2>
    </div>
    <button class="ghost" onclick={loadTabs} disabled={loading}>
      {loading ? "更新中..." : "更新"}
    </button>
  </header>

  {#if error}
    <ErrorMessage
      message={error}
      onRetry={loadTabs}
      onDismiss={() => (error = "")}
    />
  {/if}

  <div class="tab-row">
    {#each tabs as tab}
      <button
        type="button"
        class:active={selectedTab === tab}
        onclick={() => handleTabSelect(tab)}
      >
        {getTabLabel(tab)}
      </button>
    {/each}
  </div>

  <div class="banner-row">
    {#if bannerLoading}
      <p>バナー読み込み中...</p>
    {:else if banners.length === 0}
      <p class="muted">バナーはありません</p>
    {:else}
      {#each banners as banner}
        <div class="banner">
          {#if banner?.image_url || banner?.banner_image_url}
            <img src={banner.image_url ?? banner.banner_image_url} alt={banner?.title ?? "banner"} />
          {:else}
            <div class="banner-placeholder">BANNER</div>
          {/if}
          <div class="banner-meta">
            <p>{banner?.title ?? "キャンペーン"}</p>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="live-grid">
    {#if loading}
      {#each Array(6) as _}
        <Skeleton variant="card" />
      {/each}
    {:else if lives.length === 0}
      <div class="empty">配信が見つかりません</div>
    {:else}
      {#each lives as live}
        <LiveCard live={live} onSelect={onOpenLive} />
      {/each}
    {/if}
  </div>

  <div class="load-more">
    {#if nextCursor}
      <button class="ghost" onclick={loadMore} disabled={loadingMore}>
        {loadingMore ? "読み込み中..." : "さらに読み込む"}
      </button>
      <span class="cursor">next_cursor: {nextCursor}</span>
    {:else if !loading && lives.length > 0}
      <span class="muted">これ以上の配信はありません</span>
    {/if}
  </div>
</section>

<style>
  .home {
    display: grid;
    gap: 20px;
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

  .tab-row {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .tab-row button {
    border: none;
    border-radius: 999px;
    padding: 6px 14px;
    background: rgba(16, 27, 30, 0.08);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .tab-row button.active {
    background: var(--accent-500);
    color: #fff;
  }

  .banner-row {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(220px, 1fr);
    gap: 16px;
    overflow-x: auto;
    padding-bottom: 6px;
  }

  .banner {
    border-radius: 18px;
    background: var(--card-surface);
    overflow: hidden;
    position: relative;
    box-shadow: var(--shadow-soft);
  }

  .banner img {
    width: 100%;
    height: 120px;
    object-fit: cover;
    display: block;
  }

  .banner-placeholder {
    height: 120px;
    display: grid;
    place-items: center;
    background: linear-gradient(120deg, #132725, #24524c);
    color: rgba(255, 255, 255, 0.7);
    letter-spacing: 0.3em;
    font-size: 0.7rem;
  }

  .banner-meta {
    padding: 10px 12px;
    font-weight: 600;
  }

  .live-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
  }

  .load-more {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .cursor {
    font-size: 0.75rem;
    color: var(--ink-500);
    word-break: break-all;
  }

  .skeleton,
  .empty {
    padding: 20px;
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
</style>
