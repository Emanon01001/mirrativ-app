<script lang="ts">
  let { onRefreshComments, onRefreshPolling } = $props<{
    onRefreshComments: () => void;
    onRefreshPolling: () => void;
  }>();
</script>

<header class="section-head">
  <div class="head-copy">
    <p class="kicker">Watch</p>
    <h2>配信視聴</h2>
    <p class="sub">コメントとポーリングを手動同期できます</p>
  </div>
  <div class="actions">
    <button class="ghost" onclick={onRefreshComments}>コメント更新</button>
    <button class="ghost" onclick={onRefreshPolling}>ポーリング更新</button>
  </div>
</header>

<style>
  .section-head {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px;
    border-radius: 18px;
    background:
      linear-gradient(165deg, rgba(255, 255, 255, 0.96) 0%, rgba(245, 250, 249, 0.92) 100%);
    border: 1px solid rgba(16, 27, 30, 0.1);
    box-shadow: var(--shadow-soft);
  }

  .section-head::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(120deg, rgba(61, 176, 162, 0.09), transparent 45%);
    pointer-events: none;
  }

  .head-copy {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 4px;
  }

  .kicker {
    text-transform: uppercase;
    letter-spacing: 0.24em;
    font-size: 0.64rem;
    font-weight: 700;
    color: var(--accent-500);
    margin: 0 0 6px 0;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(1.08rem, 1.8vw, 1.45rem);
    letter-spacing: 0.02em;
  }

  .sub {
    margin: 0;
    font-size: 0.78rem;
    color: var(--ink-600);
  }

  .actions {
    position: relative;
    z-index: 1;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  button {
    border: 1px solid transparent;
    border-radius: 999px;
    padding: 8px 14px;
    font-weight: 700;
    font-size: 0.78rem;
    letter-spacing: 0.01em;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
    transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
  }

  button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 16px rgba(15, 42, 39, 0.14);
  }

  .ghost {
    color: var(--ink-700);
    background: rgba(255, 255, 255, 0.82);
    border-color: rgba(16, 27, 30, 0.22);
  }

  .ghost:hover {
    border-color: rgba(242, 95, 76, 0.5);
  }

  @media (max-width: 760px) {
    .section-head {
      align-items: flex-start;
      flex-direction: column;
      border-radius: 14px;
      padding: 14px;
    }

    .actions {
      width: 100%;
    }

    .actions button {
      flex: 1 1 auto;
      text-align: center;
    }
  }
</style>
