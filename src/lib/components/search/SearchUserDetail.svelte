<script lang="ts">
  import { onDestroy } from "svelte";
  import LiveCard from "$lib/components/LiveCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let {
    activeUser,
    activeUserName,
    activeUserId,
    activeUserAvatar,
    activeUserDesc,
    activeUserFollowers,
    activeUserFollowing,
    activeUserLiveCount,
    activeUserDetailRows,
    activeUserLiveId,
    selectedUserLiveHistory,
    userHistoryLoadingMore,
    userHistoryHasMore,
    userHistoryTotal,
    userHistoryCurrentPage,
    userHistoryNextPage,
    userHistoryPreviousPage,
    userDetailLoading,
    userDetailError,
    userHistoryError,
    onRetry,
    onClearDetailError,
    onClearHistoryError,
    onClose,
    onLoadMoreHistory,
    onOpenLive
  } = $props<{
    activeUser: any;
    activeUserName: string;
    activeUserId: string;
    activeUserAvatar: string;
    activeUserDesc: string;
    activeUserFollowers: number;
    activeUserFollowing: number;
    activeUserLiveCount: number;
    activeUserDetailRows: Array<{ label: string; value: string }>;
    activeUserLiveId: string;
    selectedUserLiveHistory: any[];
    userHistoryLoadingMore: boolean;
    userHistoryHasMore: boolean;
    userHistoryTotal: number | null;
    userHistoryCurrentPage: number | null;
    userHistoryNextPage: number | null;
    userHistoryPreviousPage: number | null;
    userDetailLoading: boolean;
    userDetailError: string;
    userHistoryError: string;
    onRetry: () => void;
    onClearDetailError: () => void;
    onClearHistoryError: () => void;
    onClose: () => void;
    onLoadMoreHistory: () => void;
    onOpenLive: (live: any) => void;
  }>();

  let historySentinel: HTMLDivElement | null = $state(null);
  let historyObserver: IntersectionObserver | null = null;

  const initHistoryObserver = () => {
    if (historyObserver) historyObserver.disconnect();
    historyObserver = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onLoadMoreHistory();
        }
      },
      { root: null, rootMargin: "200px", threshold: 0 }
    );
    if (historySentinel) historyObserver.observe(historySentinel);
  };

  $effect(() => {
    if (historySentinel) {
      initHistoryObserver();
    } else if (historyObserver) {
      historyObserver.disconnect();
    }
  });

  onDestroy(() => {
    if (historyObserver) historyObserver.disconnect();
  });
</script>

<div class="user-detail">
  <div class="detail-head">
    <h3>ユーザー詳細</h3>
    <button class="close-btn" onclick={onClose}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"/>
        <line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
      閉じる
    </button>
  </div>
  {#if userDetailLoading}
    <div class="detail-body">
      <Skeleton variant="avatar" />
      <div class="detail-info">
        <Skeleton variant="text" height="22px" width="60%" />
        <Skeleton variant="text" height="16px" width="40%" />
        <Skeleton variant="text" height="16px" width="80%" />
      </div>
    </div>
  {:else if userDetailError}
    <ErrorMessage message={userDetailError} onRetry={onRetry} onDismiss={onClearDetailError} />
  {:else if activeUser}
    <div class="detail-body">
      <div class="detail-avatar">
        {#if activeUserAvatar}
          <img src={activeUserAvatar} alt={activeUserName} />
        {:else}
          <span>U</span>
        {/if}
      </div>
      <div class="detail-info">
        <div class="detail-title">
          <h4>{activeUserName}</h4>
          {#if activeUserId}
            <span class="detail-id">@{activeUserId}</span>
          {/if}
        </div>
        {#if activeUserDesc}
          <p class="detail-desc">{activeUserDesc}</p>
        {/if}
        <div class="detail-stats">
          <div class="stat-pill">
            <span class="stat-value">{activeUserFollowers.toLocaleString()}</span>
            <span class="stat-label">フォロワー</span>
          </div>
          <div class="stat-pill">
            <span class="stat-value">{activeUserFollowing.toLocaleString()}</span>
            <span class="stat-label">フォロー</span>
          </div>
          <div class="stat-pill">
            <span class="stat-value">{activeUserLiveCount.toLocaleString()}</span>
            <span class="stat-label">配信数</span>
          </div>
        </div>
        {#if activeUserDetailRows.length > 0}
          <div class="detail-grid">
            {#each activeUserDetailRows as row, i}
              <div class="detail-row" class:even={i % 2 === 1}>
                <span class="detail-key">{row.label}</span>
                <span class="detail-value">{row.value}</span>
              </div>
            {/each}
          </div>
        {/if}
        {#if activeUser?.custom_thanks_message}
          <p class="detail-thanks">"{activeUser.custom_thanks_message}"</p>
        {/if}
        {#if activeUserLiveId}
          <button
            class="watch-btn"
            onclick={() => onOpenLive({ live_id: activeUserLiveId, owner: activeUser })}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <polygon points="5 3 19 12 5 21 5 3"/>
            </svg>
            配信を視聴
          </button>
        {/if}
      </div>
    </div>

    <div class="detail-history">
      <div class="detail-history-head">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/>
          <polyline points="12 6 12 12 16 14"/>
        </svg>
        <h4>配信履歴</h4>
      </div>
      {#if userHistoryTotal !== null || userHistoryCurrentPage !== null || userHistoryNextPage !== null || userHistoryPreviousPage !== null}
        <div class="detail-history-meta">
          {#if userHistoryTotal !== null}
            <span class="meta-pill">合計 {userHistoryTotal.toLocaleString()}件</span>
          {/if}
          {#if userHistoryCurrentPage !== null}
            <span class="meta-pill">ページ {userHistoryCurrentPage}</span>
          {/if}
          {#if userHistoryPreviousPage !== null}
            <span class="meta-pill">前 {userHistoryPreviousPage}</span>
          {/if}
          {#if userHistoryNextPage !== null}
            <span class="meta-pill">次 {userHistoryNextPage}</span>
          {/if}
        </div>
      {/if}
      {#if userHistoryError}
        <ErrorMessage message={userHistoryError} onRetry={onRetry} onDismiss={onClearHistoryError} />
      {:else if selectedUserLiveHistory.length === 0}
        <p class="empty">履歴はありません</p>
      {:else}
        <div class="detail-history-grid">
          {#each selectedUserLiveHistory as item}
            <LiveCard live={item.live ?? item} onSelect={onOpenLive} />
          {/each}
        </div>
        {#if userHistoryLoadingMore}
          <p class="history-loading">履歴を読み込み中...</p>
        {/if}
        {#if userHistoryHasMore}
          <div bind:this={historySentinel} class="history-sentinel" />
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .user-detail {
    display: grid;
    gap: 12px;
    padding: 14px;
    border-radius: 18px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
    animation: slideUp 0.25s ease-out;
  }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .detail-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .detail-head h3 {
    margin: 0;
    font-size: 1rem;
  }

  .close-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    border: 1px solid rgba(16, 27, 30, 0.15);
    background: var(--card-surface);
    color: var(--ink-700);
    border-radius: 999px;
    padding: 6px 14px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.2s ease, background 0.2s ease;
  }

  .close-btn:hover {
    border-color: rgba(16, 27, 30, 0.3);
    background: rgba(16, 27, 30, 0.04);
  }

  .detail-body {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 12px;
    align-items: start;
  }

  .detail-avatar {
    width: 64px;
    height: 64px;
    border-radius: 18px;
    overflow: hidden;
    background: rgba(16, 27, 30, 0.08);
    display: grid;
    place-items: center;
    font-weight: 700;
    color: var(--ink-600);
  }

  .detail-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .detail-info {
    display: grid;
    gap: 8px;
    min-width: 0;
  }

  .detail-title {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }

  .detail-title h4 {
    margin: 0;
    font-size: 1.05rem;
    font-family: var(--font-display);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .detail-id {
    font-size: 0.75rem;
    color: var(--ink-600);
    white-space: nowrap;
  }

  .detail-desc {
    margin: 0;
    color: var(--ink-600);
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .detail-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .stat-pill {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 14px;
    border-radius: 12px;
    background: rgba(16, 27, 30, 0.04);
    min-width: 70px;
  }

  .stat-value {
    font-size: 1.05rem;
    font-weight: 700;
    font-family: var(--font-display);
    color: var(--ink-900);
  }

  .stat-label {
    font-size: 0.7rem;
    color: var(--ink-500);
    margin-top: 2px;
  }

  .detail-grid {
    display: grid;
    gap: 4px;
    font-size: 0.78rem;
    color: var(--ink-700);
  }

  .detail-row {
    display: flex;
    gap: 10px;
    padding: 4px 8px;
    border-radius: 8px;
  }

  .detail-row.even {
    background: rgba(16, 27, 30, 0.03);
  }

  .detail-key {
    flex: 0 0 120px;
    color: var(--ink-600);
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
  }

  .detail-value {
    flex: 1;
    min-width: 0;
    word-break: break-all;
  }

  .detail-thanks {
    margin: 0;
    font-size: 0.82rem;
    color: var(--ink-600);
    background: rgba(16, 27, 30, 0.04);
    padding: 8px 10px;
    border-radius: 12px;
  }

  .watch-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    justify-self: flex-start;
    border: none;
    border-radius: 999px;
    padding: 8px 16px;
    background: var(--accent-500);
    color: #fff;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.2s ease;
  }

  .watch-btn:hover {
    background: var(--accent-700);
  }

  .detail-history {
    display: grid;
    gap: 10px;
  }

  .detail-history-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .meta-pill {
    font-size: 0.75rem;
    color: var(--ink-600);
    background: rgba(16, 27, 30, 0.06);
    padding: 4px 10px;
    border-radius: 999px;
    font-weight: 600;
  }

  .detail-history-head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink-700);
  }

  .detail-history-head svg {
    flex-shrink: 0;
    color: var(--accent-500);
  }

  .detail-history-head h4 {
    margin: 0;
    font-size: 0.95rem;
  }

  .detail-history-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px;
  }

  .history-loading {
    margin: 4px 0 0;
    font-size: 0.8rem;
    color: var(--ink-500);
  }

  .history-sentinel {
    height: 1px;
  }

  .empty {
    padding: 10px;
    border-radius: 10px;
    background: rgba(16, 27, 30, 0.06);
    color: var(--ink-600);
  }
</style>
