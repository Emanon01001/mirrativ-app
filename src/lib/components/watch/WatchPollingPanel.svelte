<script lang="ts">
  let { pollingDetails, polling } = $props<{
    pollingDetails: Array<{ key: string; value: string }>;
    polling: any;
  }>();
</script>

<h3>ポーリング結果</h3>
{#if pollingDetails.length > 0}
  <div class="detail-list">
    {#each pollingDetails as row}
      <div class="detail-row">
        <span class="detail-key">{row.key}</span>
        <span class="detail-value">{row.value}</span>
      </div>
    {/each}
  </div>
  <details class="raw-json">
    <summary>raw json</summary>
    <pre>{JSON.stringify(polling, null, 2)}</pre>
  </details>
{:else}
  <p class="muted">まだ取得していません</p>
{/if}

<style>
  h3 {
    margin: 0;
    font-size: 0.98rem;
    letter-spacing: 0.02em;
  }

  .detail-list {
    display: grid;
    gap: 7px;
    max-height: 320px;
    overflow: auto;
    padding-right: 4px;
  }

  .detail-row {
    display: grid;
    grid-template-columns: minmax(0, 150px) minmax(0, 1fr);
    gap: 8px;
    font-size: 0.78rem;
    align-items: start;
    border-radius: 10px;
    padding: 6px 8px;
    background: rgba(16, 27, 30, 0.04);
    border: 1px solid rgba(16, 27, 30, 0.08);
  }

  .detail-key {
    color: var(--ink-600);
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    word-break: break-all;
    overflow-wrap: anywhere;
  }

  .detail-value {
    color: var(--ink-800);
    word-break: break-all;
    overflow-wrap: anywhere;
  }

  .raw-json {
    margin-top: 4px;
    max-width: 100%;
    min-width: 0;
    padding-top: 6px;
    border-top: 1px solid rgba(16, 27, 30, 0.09);
  }

  .raw-json summary {
    cursor: pointer;
    font-size: 0.76rem;
    color: var(--ink-600);
  }

  .raw-json pre {
    width: 100%;
    max-height: 260px;
    overflow-x: auto;
    overflow-y: auto;
    scrollbar-gutter: stable both-edges;
  }

  pre {
    margin: 0;
    font-size: 0.75rem;
    background: rgba(15, 42, 39, 0.08);
    padding: 10px;
    border-radius: 12px;
    overflow: auto;
    max-width: 100%;
    white-space: pre;
  }

  .muted {
    color: var(--ink-500);
  }
</style>
