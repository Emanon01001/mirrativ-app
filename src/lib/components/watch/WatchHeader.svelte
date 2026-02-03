<script lang="ts">
  let { onRefreshComments, onRefreshPolling } = $props<{
    onRefreshComments: () => void;
    onRefreshPolling: () => void;
  }>();
</script>

<header class="section-head">
  <div>
    <p class="kicker">Watch</p>
    <h2>配信視聴</h2>
  </div>
  <div class="actions">
    <button class="ghost" onclick={onRefreshComments}>コメント更新</button>
    <button class="ghost" onclick={onRefreshPolling}>ポーリング</button>
  </div>
</header>

<style>
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

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
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