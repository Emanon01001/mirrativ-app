<script lang="ts">
  let { liveId, loading, onJoin, onLiveIdChange } = $props<{
    liveId: string;
    loading: boolean;
    onJoin: (event?: Event) => void;
    onLiveIdChange: (value: string) => void;
  }>();

  const handleInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    onLiveIdChange(target?.value ?? "");
  };
</script>

<div class="join">
  {#if liveId}
    <div class="join-status">
      <span>配信ID:</span>
      <strong>{liveId}</strong>
      {#if loading}
        <span class="connecting">接続中...</span>
      {/if}
    </div>
    <button type="button" class="ghost" onclick={onJoin} disabled={loading}>
      {loading ? "接続中..." : "再接続"}
    </button>
  {:else}
    <div class="join-placeholder">ホームの配信サムネをクリックして視聴を開始してください</div>
    <details class="join-manual">
      <summary>手動で live_id を入力</summary>
      <form class="join-form" onsubmit={onJoin}>
        <input placeholder="live_id を入力" value={liveId} oninput={handleInput} />
        <button type="submit" disabled={loading}>
          {loading ? "接続中..." : "参加"}
        </button>
      </form>
    </details>
  {/if}
</div>

<style>
  .join {
    display: grid;
    gap: 10px;
    padding: 12px 16px;
    border-radius: 16px;
    background: var(--panel-surface);
    box-shadow: var(--shadow-soft);
  }

  .join-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.95rem;
  }

  .join-status strong {
    font-family: var(--font-display);
  }

  .connecting {
    color: var(--accent-500);
    font-weight: 600;
    font-size: 0.85rem;
  }

  .join-placeholder {
    color: var(--ink-600);
  }

  .join-manual {
    margin: 0;
  }

  .join-form {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 8px;
  }

  input {
    flex: 1 1 240px;
    border: 1px solid rgba(16, 27, 30, 0.2);
    border-radius: 12px;
    padding: 10px 12px;
    background: #fff;
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 8px 16px;
    font-weight: 600;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
  }

  .ghost {
    background: transparent;
    border: 1px solid rgba(16, 27, 30, 0.2);
    color: var(--ink-700);
  }
</style>