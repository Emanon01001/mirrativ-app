<script lang="ts">
  import { onDestroy } from "svelte";
  import UserCard from "$lib/components/UserCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let {
    users,
    loading,
    loadingMore,
    error,
    hasMore,
    onRefresh,
    onSelect,
    onOpenLive,
    onLoadMore,
    onClearError
  } = $props<{
    users: any[];
    loading: boolean;
    loadingMore: boolean;
    error: string;
    hasMore: boolean;
    onRefresh: () => void;
    onSelect: (user: any) => void;
    onOpenLive: (live: any) => void;
    onLoadMore: () => void;
    onClearError: () => void;
  }>();

  let sentinel: HTMLDivElement | null = $state(null);
  let observer: IntersectionObserver | null = null;

  const initObserver = () => {
    if (observer) observer.disconnect();
    observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          if (loading || loadingMore || !hasMore) return;
          onLoadMore();
        }
      },
      { root: null, rootMargin: "200px", threshold: 0 }
    );
    if (sentinel) observer.observe(sentinel);
  };

  $effect(() => {
    if (sentinel) {
      initObserver();
    } else if (observer) {
      observer.disconnect();
    }
  });

  onDestroy(() => {
    if (observer) observer.disconnect();
  });
</script>

<div class="recommend">
  <div class="recommend-head">
    <div class="recommend-title">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="none">
        <path d="M12 2l2.4 7.4H22l-6.2 4.5 2.4 7.4L12 16.8l-6.2 4.5 2.4-7.4L2 9.4h7.6z"/>
      </svg>
      <h3>おすすめユーザー</h3>
    </div>
    <button class="refresh-btn" onclick={onRefresh} disabled={loading}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
           class:spinning={loading}>
        <polyline points="23 4 23 10 17 10"/>
        <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
      </svg>
      {loading ? "更新中..." : "更新"}
    </button>
  </div>
  {#if error}
    <ErrorMessage message={error} onRetry={onRefresh} onDismiss={onClearError} />
  {/if}
  <div class="recommend-grid">
    {#if loading && users.length === 0}
      {#each Array(4) as _}
        <Skeleton variant="card" />
      {/each}
    {:else if users.length === 0}
      <p class="empty">おすすめユーザーが見つかりません</p>
    {:else}
      {#each users as user}
        <UserCard user={user} onOpenLive={onOpenLive} onSelect={onSelect} />
      {/each}
    {/if}
  </div>
  {#if loadingMore}
    <div class="loading-more">
      <svg class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
      </svg>
      <span>読み込み中...</span>
    </div>
  {/if}
  <div bind:this={sentinel} style="height:1px" />
</div>

<style>
  .recommend {
    display: grid;
    gap: 10px;
    padding: 12px;
    border-radius: 16px;
    background: rgba(16, 27, 30, 0.04);
  }

  .recommend-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .recommend-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink-700);
  }

  .recommend-title svg {
    color: var(--accent-500);
    flex-shrink: 0;
  }

  .recommend-head h3 {
    margin: 0;
    font-size: 1rem;
  }

  .recommend-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }

  .empty {
    padding: 10px;
    border-radius: 10px;
    background: rgba(16, 27, 30, 0.06);
    color: var(--ink-600);
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid rgba(16, 27, 30, 0.15);
    background: var(--card-surface);
    color: var(--ink-700);
    border-radius: 999px;
    padding: 6px 14px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.2s ease, background 0.2s ease;
  }

  .refresh-btn:hover:not(:disabled) {
    border-color: var(--accent-500);
    background: rgba(242, 95, 76, 0.06);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  .loading-more {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px;
    color: var(--ink-500);
    font-size: 0.85rem;
  }

  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
