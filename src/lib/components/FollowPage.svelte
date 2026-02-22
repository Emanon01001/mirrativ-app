<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import LiveCard from "$lib/components/LiveCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let { onOpenLive } = $props<{ onOpenLive: (live: any) => void }>();

  let lives = $state<any[]>([]);
  let loading = $state(false);
  let error = $state("");

  const getLiveId = (item: any) =>
    item?.live_id ?? item?.id ?? item?.live?.live_id ?? item?.live?.id ?? "";

  const extractLives = (res: any) => {
    const candidates =
      res?.list ??
      res?.lives ??
      res?.live_list ??
      res?.data?.list ??
      res?.data?.lives ??
      res?.data?.live_list ??
      res?.data ??
      [];

    if (!Array.isArray(candidates)) return [];

    const normalized = candidates
      .map((item) => item?.live ?? item)
      .filter(Boolean);

    const seen = new Set<string>();
    const unique: any[] = [];
    for (const item of normalized) {
      const id = String(getLiveId(item) ?? "");
      if (id && seen.has(id)) continue;
      if (id) seen.add(id);
      unique.push(item);
    }
    return unique;
  };

  const loadFollow = async () => {
    loading = true;
    error = "";
    try {
      const res: any = await invoke("get_catalog_follow");
      lives = extractLives(res);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  };

  onMount(loadFollow);
</script>

<section class="follow">
  <header class="section-head">
    <div>
      <p class="kicker">Following</p>
      <h2>フォロー中の配信</h2>
    </div>
    <button class="ghost" onclick={loadFollow} disabled={loading}>
      {loading ? "更新中..." : "更新"}
    </button>
  </header>

  {#if error}
    <ErrorMessage
      message={error}
      onRetry={loadFollow}
      onDismiss={() => (error = "")}
    />
  {/if}

  <div class="live-grid">
    {#if loading}
      {#each Array(4) as _}
        <Skeleton variant="card" />
      {/each}
    {:else if lives.length === 0}
      <div class="empty">フォロー中の配信がありません</div>
    {:else}
      {#each lives as live}
        <LiveCard live={live} onSelect={onOpenLive} />
      {/each}
    {/if}
  </div>
</section>

<style>
  .follow {
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

  .live-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
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
</style>
