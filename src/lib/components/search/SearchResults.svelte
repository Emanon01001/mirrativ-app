<script lang="ts">
  import LiveCard from "$lib/components/LiveCard.svelte";
  import UserCard from "$lib/components/UserCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";

  let { mode, results, loading, searched, totalEntries, onOpenLive, onSelectUser } = $props<{
    mode: "live" | "user";
    results: any[];
    loading: boolean;
    searched: boolean;
    totalEntries?: number | null;
    onOpenLive: (live: any) => void;
    onSelectUser: (user: any) => void;
  }>();
</script>

{#if searched && !loading && results.length > 0}
  <div class="result-header">
    <span class="result-count">
      {results.length}件{#if totalEntries != null} / {totalEntries.toLocaleString()}件{/if}
    </span>
    <span class="result-mode">{mode === "live" ? "配信" : "ユーザー"}</span>
  </div>
{/if}

<div class="result-grid">
  {#if loading}
    {#each Array(6) as _}
      <Skeleton variant="card" />
    {/each}
  {:else if results.length === 0}
    <div class="empty-state">
      <svg class="empty-icon" width="48" height="48" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"/>
        <line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <p class="empty-title">
        {searched ? "検索結果がありません" : "キーワードを入力して検索してください"}
      </p>
      <p class="empty-hint">
        {searched
          ? "別のキーワードで検索してみてください"
          : mode === "live" ? "例: 雑談, 歌, ゲーム" : "例: 名前, ID"}
      </p>
    </div>
  {:else}
    {#if mode === "live"}
      {#each results as live}
        <LiveCard live={live} onSelect={onOpenLive} />
      {/each}
    {:else}
      {#each results as user}
        <UserCard user={user} onOpenLive={onOpenLive} onSelect={onSelectUser} />
      {/each}
    {/if}
  {/if}
</div>

<style>
  .result-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 0;
    font-size: 0.82rem;
    color: var(--ink-600);
  }

  .result-count {
    font-weight: 700;
    color: var(--ink-700);
  }

  .result-mode {
    background: rgba(16, 27, 30, 0.06);
    padding: 2px 10px;
    border-radius: 999px;
    font-weight: 600;
    font-size: 0.75rem;
  }

  .result-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }

  .empty-state {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    border-radius: 16px;
    background: rgba(16, 27, 30, 0.04);
    text-align: center;
  }

  .empty-icon {
    color: var(--ink-500);
    opacity: 0.5;
    margin-bottom: 12px;
  }

  .empty-title {
    margin: 0 0 4px 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--ink-700);
  }

  .empty-hint {
    margin: 0;
    font-size: 0.85rem;
    color: var(--ink-500);
  }
</style>
