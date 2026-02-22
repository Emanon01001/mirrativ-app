<script lang="ts">
  let {
    rankingType,
    giftRankingLoaded,
    giftRankingView,
    giftRankingAll,
    giftRankingLoading,
    onChangeRankingType,
    onLoadMore
  } = $props<{
    rankingType: string;
    giftRankingLoaded: boolean;
    giftRankingView: Array<{
      rank: string | number;
      userName: string;
      userId: string;
      points: string;
      giftName: string;
      giftImage: string;
      userImage: string;
    }>;
    giftRankingAll: boolean;
    giftRankingLoading: boolean;
    onChangeRankingType: (value: string) => void;
    onLoadMore: () => void;
  }>();
</script>

<h3>ギフトランキング</h3>
<div class="ranking-toggle">
  <button class:active={rankingType === "live"} onclick={() => onChangeRankingType("live")}>
    live
  </button>
  <button class:active={rankingType === "daily"} onclick={() => onChangeRankingType("daily")}>
    daily
  </button>
  <button class:active={rankingType === "monthly"} onclick={() => onChangeRankingType("monthly")}>
    monthly
  </button>
</div>
{#if giftRankingLoaded}
  {#if giftRankingView.length === 0}
    <p class="muted">ランキングなし</p>
  {:else}
    <div class="ranking-list">
      {#each giftRankingView as item}
        <div class="ranking-item">
          <span class="rank">{item.rank ?? "-"}</span>
          <div class="ranking-user">
            {#if item.userImage}
              <img class="rank-avatar" src={item.userImage} alt={item.userName} loading="lazy" />
            {:else}
              <div class="rank-avatar placeholder">?</div>
            {/if}
            <div class="ranking-meta">
              <span class="name">{item.userName}</span>
              {#if item.userId}
                <span class="sub">@{item.userId}</span>
              {/if}
            </div>
          </div>
          {#if item.giftImage || item.giftName}
            <div class="ranking-gift">
              {#if item.giftImage}
                <img class="gift-thumb" src={item.giftImage} alt={item.giftName || "gift"} loading="lazy" />
              {/if}
              {#if item.giftName}
                <span class="gift-name">{item.giftName}</span>
              {/if}
            </div>
          {/if}
          <span class="points">{item.points}</span>
        </div>
      {/each}
    </div>
  {/if}
  {#if !giftRankingAll}
    <button class="ghost load-more" onclick={onLoadMore} disabled={giftRankingLoading}>
      {giftRankingLoading ? "読み込み中..." : "さらに読み込む"}
    </button>
  {/if}
{:else}
  <p class="muted">参加後に取得できます</p>
{/if}

<style>
  h3 {
    margin: 0;
    font-size: 0.98rem;
    letter-spacing: 0.02em;
  }

  .ranking-toggle {
    display: flex;
    gap: 6px;
    padding: 4px;
    border-radius: 999px;
    background: rgba(16, 27, 30, 0.06);
    border: 1px solid rgba(16, 27, 30, 0.1);
  }

  .ranking-toggle button {
    flex: 1 1 0;
    background: transparent;
    color: var(--ink-700);
    border: 1px solid transparent;
    font-size: 0.76rem;
    padding: 6px 10px;
  }

  .ranking-toggle button.active {
    background: var(--accent-500);
    border-color: var(--accent-500);
    color: #fff;
    box-shadow: 0 8px 14px rgba(15, 42, 39, 0.15);
  }

  .ranking-list {
    display: grid;
    gap: 9px;
    max-height: 330px;
    overflow: auto;
    padding-right: 2px;
  }

  .ranking-item {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) auto;
    column-gap: 10px;
    row-gap: 4px;
    grid-auto-rows: auto;
    align-items: center;
    font-size: 0.85rem;
    border-radius: 12px;
    padding: 8px 10px;
    background: rgba(16, 27, 30, 0.04);
    border: 1px solid rgba(16, 27, 30, 0.08);
  }

  .rank {
    font-weight: 700;
    color: var(--accent-500);
    grid-column: 1;
    grid-row: 1 / span 2;
    justify-self: center;
  }

  .ranking-user {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    grid-column: 2;
    grid-row: 1;
  }

  .rank-avatar {
    width: 32px;
    height: 32px;
    border-radius: 999px;
    object-fit: cover;
    background: rgba(16, 27, 30, 0.08);
  }

  .ranking-meta {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .ranking-meta .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ranking-meta .sub {
    font-size: 0.72rem;
    color: var(--ink-600);
  }

  .ranking-gift {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    grid-column: 2;
    grid-row: 2;
  }

  .gift-thumb {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    object-fit: cover;
    background: rgba(16, 27, 30, 0.08);
  }

  .gift-name {
    font-size: 0.75rem;
    color: var(--ink-600);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  .points {
    font-weight: 700;
    color: var(--ink-800);
    grid-column: 3;
    grid-row: 1 / span 2;
    justify-self: end;
  }

  .load-more {
    justify-self: center;
  }

  .placeholder {
    display: grid;
    place-items: center;
    color: var(--ink-600);
    font-size: 0.75rem;
  }

  button {
    border: 1px solid transparent;
    border-radius: 999px;
    padding: 8px 16px;
    font-weight: 700;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
    transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
  }

  button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 14px rgba(15, 42, 39, 0.14);
  }

  .ghost {
    background: rgba(255, 255, 255, 0.82);
    border-color: rgba(16, 27, 30, 0.22);
    color: var(--ink-700);
  }

  .muted {
    color: var(--ink-500);
  }
</style>
