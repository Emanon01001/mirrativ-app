<script lang="ts">
  type ViewMode = "normal" | "no-join" | "silent";
  type StreamSourceMode = "auto" | "hls" | "llstream";

  let { liveId, loading, viewMode, streamSourceMode, authed, onJoin, onLiveIdChange, onViewModeChange, onStreamSourceModeChange } = $props<{
    liveId: string;
    loading: boolean;
    viewMode: ViewMode;
    streamSourceMode: StreamSourceMode;
    authed: boolean;
    onJoin: (event?: Event) => void;
    onLiveIdChange: (value: string) => void;
    onViewModeChange: (value: ViewMode) => void;
    onStreamSourceModeChange: (value: StreamSourceMode) => void;
  }>();

  const handleInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    onLiveIdChange(target?.value ?? "");
  };

  const handleModeChange = (event: Event) => {
    const target = event.currentTarget as HTMLSelectElement | null;
    onViewModeChange((target?.value as ViewMode) ?? "silent");
  };

  const handleStreamSourceModeChange = (event: Event) => {
    const target = event.currentTarget as HTMLSelectElement | null;
    onStreamSourceModeChange((target?.value as StreamSourceMode) ?? "auto");
  };

  const modeLabels: Record<ViewMode, string> = {
    normal: "通常（全機能）",
    "no-join": "入室ログなし",
    silent: "サイレント（受信のみ）",
  };

  const streamSourceLabels: Record<StreamSourceMode, string> = {
    auto: "自動（LLStream優先）",
    hls: "HLS 固定",
    llstream: "LLStream 固定",
  };
</script>

<div class="join">
  {#if liveId}
    <div class="join-status">
      <span class="label">視聴対象</span>
      <strong class="join-code">{liveId}</strong>
      <span class:connecting={loading} class="state">
        {loading ? "接続中..." : "視聴中"}
      </span>
    </div>
    <div class="join-options">
      <label class="mode-label">
        <span>視聴モード</span>
        <select class="mode-select" value={viewMode} onchange={handleModeChange} disabled={!authed}>
          {#each Object.entries(modeLabels) as [value, label]}
            <option {value} disabled={!authed && value !== "silent"}>{label}</option>
          {/each}
        </select>
        {#if !authed}
          <span class="mode-hint">ログインで全モード解放</span>
        {/if}
      </label>
      <label class="mode-label">
        <span>再生ソース</span>
        <select class="mode-select" value={streamSourceMode} onchange={handleStreamSourceModeChange}>
          {#each Object.entries(streamSourceLabels) as [value, label]}
            <option {value}>{label}</option>
          {/each}
        </select>
      </label>
    </div>
    <button type="button" class="ghost reconnect" onclick={onJoin} disabled={loading}>
      {loading ? "接続中..." : "再接続"}
    </button>
  {:else}
    <div class="join-placeholder">
      ホームの配信サムネをクリックすると、ここから視聴セッションが開始されます
    </div>
    <details class="join-manual">
      <summary>手動で live_id を入力</summary>
      <form class="join-form" onsubmit={onJoin}>
        <input placeholder="live_id を入力" value={liveId} oninput={handleInput} />
        <button type="submit" disabled={loading}>
          {loading ? "接続中..." : "参加"}
        </button>
      </form>
      <div class="join-options">
        <label class="mode-label">
          <span>視聴モード</span>
          <select class="mode-select" value={viewMode} onchange={handleModeChange} disabled={!authed}>
            {#each Object.entries(modeLabels) as [value, label]}
              <option {value} disabled={!authed && value !== "silent"}>{label}</option>
            {/each}
          </select>
          {#if !authed}
            <span class="mode-hint">ログインで全モード解放</span>
          {/if}
        </label>
        <label class="mode-label">
          <span>再生ソース</span>
          <select class="mode-select" value={streamSourceMode} onchange={handleStreamSourceModeChange}>
            {#each Object.entries(streamSourceLabels) as [value, label]}
              <option {value}>{label}</option>
            {/each}
          </select>
        </label>
      </div>
    </details>
  {/if}
</div>

<style>
  .join {
    position: relative;
    overflow: hidden;
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border-radius: 18px;
    background:
      linear-gradient(160deg, rgba(255, 255, 255, 0.95), rgba(246, 252, 250, 0.9));
    border: 1px solid rgba(16, 27, 30, 0.1);
    box-shadow: var(--shadow-soft);
  }

  .join::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(120deg, rgba(242, 95, 76, 0.05), transparent 40%);
    pointer-events: none;
  }

  .join-status {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    font-size: 0.95rem;
  }

  .label {
    font-size: 0.72rem;
    color: var(--ink-600);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .join-code {
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    background: rgba(13, 42, 43, 0.9);
    color: #f4fbfa;
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 0.82rem;
    font-weight: 700;
  }

  .state {
    font-size: 0.74rem;
    color: var(--ink-600);
    font-weight: 600;
    background: rgba(15, 42, 39, 0.08);
    border-radius: 999px;
    padding: 2px 8px;
  }

  .state.connecting {
    color: #c2512e;
    background: rgba(242, 95, 76, 0.16);
  }

  .join-placeholder {
    position: relative;
    z-index: 1;
    color: var(--ink-700);
    font-size: 0.88rem;
    line-height: 1.5;
  }

  .join-manual {
    position: relative;
    z-index: 1;
    margin: 0;
    padding-top: 2px;
  }

  .join-manual summary {
    cursor: pointer;
    font-size: 0.82rem;
    color: var(--ink-600);
  }

  .join-form {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 8px;
  }

  input:not([type="checkbox"]) {
    flex: 1 1 240px;
    border: 1px solid rgba(16, 27, 30, 0.2);
    border-radius: 12px;
    padding: 10px 12px;
    background: #fff;
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
    background: rgba(255, 255, 255, 0.84);
    border-color: rgba(16, 27, 30, 0.2);
    color: var(--ink-700);
  }

  .join-options {
    position: relative;
    z-index: 1;
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .mode-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.78rem;
    color: var(--ink-600);
  }

  .mode-select {
    padding: 4px 8px;
    border: 1px solid rgba(16, 27, 30, 0.2);
    border-radius: 8px;
    background: #fff;
    font-size: 0.78rem;
    color: var(--ink-700);
    cursor: pointer;
  }

  .mode-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .mode-hint {
    font-size: 0.68rem;
    color: var(--accent-500, #c2512e);
    font-weight: 500;
  }

  .reconnect {
    justify-self: start;
  }

  @media (max-width: 640px) {
    .join {
      border-radius: 14px;
      padding: 12px;
    }

    .reconnect {
      width: 100%;
      justify-self: stretch;
    }
  }
</style>
