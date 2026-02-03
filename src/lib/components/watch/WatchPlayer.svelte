<script lang="ts">
  let {
    streamUrl,
    isPlaying,
    isPaused,
    volume,
    rotation,
    rotateOptions,
    loading,
    streamError,
    llstreamError,
    onOpenPlayer,
    onStart,
    onStop,
    onTogglePause,
    onVolumeChange,
    onRotate
  } = $props<{
    streamUrl: string;
    isPlaying: boolean;
    isPaused: boolean;
    volume: number;
    rotation: number;
    rotateOptions: number[];
    loading: boolean;
    streamError: string;
    llstreamError: string;
    onOpenPlayer: () => void;
    onStart: () => void;
    onStop: () => void;
    onTogglePause: () => void;
    onVolumeChange: (value: number) => void;
    onRotate: (value: number) => void;
  }>();

  const handleVolume = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    const value = target ? Number(target.value) : 0;
    onVolumeChange(value);
  };
</script>

<div class="player">
  {#if streamUrl}
    <div class="player-info">
      <div class="player-status">
        <div class="status-indicator {isPlaying ? 'playing' : 'stopped'}"></div>
        <p>
          {#if isPlaying}
            {isPaused ? "一時停止中" : "プレイヤーで再生中"}
          {:else}
            停止中
          {/if}
        </p>
      </div>
      <p class="stream-url-display">{streamUrl}</p>
      <div class="player-actions">
        <button class="ghost" onclick={onOpenPlayer}>プレイヤーを開く</button>
        {#if isPlaying}
          <button class="ghost" onclick={onStop}>再生停止</button>
        {:else}
          <button onclick={onStart}>再生開始</button>
        {/if}
        <button class="ghost" onclick={onTogglePause} disabled={!isPlaying}>
          {isPaused ? "再開" : "一時停止"}
        </button>
      </div>
      <div class="player-controls">
        <div class="control-group">
          <span>音量</span>
          <input
            type="range"
            min="0"
            max="500"
            step="1"
            value={volume}
            oninput={handleVolume}
            disabled={!isPlaying}
          />
          <span class="value">{volume}%</span>
        </div>
        <div class="control-group">
          <span>回転</span>
          <div class="rotation-buttons">
            {#each rotateOptions as option}
              <button class:active={rotation === option} onclick={() => onRotate(option)} disabled={!isPlaying}>
                {option}°
              </button>
            {/each}
          </div>
        </div>
      </div>
      <p class="player-hint">※ 映像は別ウィンドウに表示されます</p>
    </div>
  {:else}
    <div class="player-placeholder">
      {loading ? "配信URLを取得中..." : "配信URLが取得できていません"}
      <div class="placeholder-actions">
        <button class="ghost" onclick={onOpenPlayer}>プレイヤーを開く</button>
      </div>
    </div>
  {/if}
  {#if streamError}
    <p class="error">{streamError}</p>
  {/if}
  {#if llstreamError}
    <p class="error">{llstreamError}</p>
  {/if}
</div>

<style>
  .player {
    display: grid;
    gap: 8px;
    padding: 16px;
    border-radius: 18px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
  }

  .player-info {
    display: grid;
    gap: 12px;
    padding: 16px;
    border-radius: 12px;
    background: rgba(16, 27, 30, 0.06);
  }

  .player-status {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 600;
  }

  .status-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--ink-400);
  }

  .status-indicator.playing {
    background: #22c55e;
    animation: pulse 2s ease-in-out infinite;
  }

  .status-indicator.stopped {
    background: var(--ink-400);
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  .stream-url-display {
    word-break: break-all;
    font-size: 0.85rem;
    background: rgba(16, 27, 30, 0.1);
    padding: 8px;
    border-radius: 8px;
    font-family: monospace;
  }

  .player-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .player-controls {
    display: grid;
    gap: 12px;
  }

  .control-group {
    display: grid;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--ink-600);
  }

  .control-group input[type="range"] {
    width: 100%;
    padding: 0;
    border: none;
    background: transparent;
  }

  .value {
    font-weight: 700;
    color: var(--ink-700);
  }

  .rotation-buttons {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .rotation-buttons button {
    background: rgba(16, 27, 30, 0.08);
    color: var(--ink-700);
    padding: 6px 12px;
  }

  .rotation-buttons button.active {
    background: var(--accent-500);
    color: #fff;
  }

  .player-hint {
    font-size: 0.8rem;
    color: var(--ink-600);
    margin: 0;
  }

  .player-placeholder {
    padding: 24px;
    border-radius: 12px;
    background: rgba(16, 27, 30, 0.06);
    color: var(--ink-600);
    display: grid;
    gap: 12px;
  }

  .placeholder-actions {
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

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }
</style>
