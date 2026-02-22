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
  <div class="player-head">
    <div>
      <p class="kicker">Player</p>
      <h3>再生コントロール</h3>
    </div>
    <span class="player-note">映像は別ウィンドウに表示されます</span>
  </div>

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
          <button class="danger" onclick={onStop}>再生停止</button>
        {:else}
          <button onclick={onStart}>再生開始</button>
        {/if}
        <button class="ghost" onclick={onTogglePause} disabled={!isPlaying}>
          {isPaused ? "再開" : "一時停止"}
        </button>
      </div>
      <div class="player-controls">
        <div class="control-group">
          <div class="control-head">
            <span>音量</span>
            <span class="value">{volume}%</span>
          </div>
          <input
            type="range"
            min="0"
            max="500"
            step="1"
            value={volume}
            oninput={handleVolume}
            disabled={!isPlaying}
          />
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
    </div>
  {:else}
    <div class="player-placeholder">
      <p>{loading ? "配信URLを取得中..." : "配信URLがまだ取得できていません"}</p>
      <div class="placeholder-actions">
        <button class="ghost" onclick={onOpenPlayer}>プレイヤーを開く</button>
      </div>
    </div>
  {/if}

  {#if streamError}
    <p class="error">{streamError}</p>
  {/if}
</div>

<style>
  .player {
    position: relative;
    overflow: hidden;
    display: grid;
    gap: 12px;
    padding: 16px;
    border-radius: 22px;
    background:
      radial-gradient(240px 160px at 92% -20%, rgba(61, 176, 162, 0.24), transparent 70%),
      linear-gradient(170deg, #0f3133 0%, #154647 52%, #174f51 100%);
    box-shadow: var(--shadow-soft);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .player::after {
    content: "";
    position: absolute;
    inset: 0;
    background:
      linear-gradient(130deg, rgba(255, 255, 255, 0.06), transparent 48%),
      linear-gradient(180deg, transparent 58%, rgba(7, 17, 19, 0.22));
    pointer-events: none;
  }

  .player-head {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
  }

  .kicker {
    margin: 0 0 4px;
    color: rgba(220, 241, 238, 0.85);
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.62rem;
    font-weight: 700;
  }

  h3 {
    margin: 0;
    color: #f4fbfa;
    font-size: 1.05rem;
    font-family: var(--font-display);
  }

  .player-note {
    color: rgba(220, 239, 237, 0.8);
    font-size: 0.72rem;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 999px;
    padding: 4px 10px;
    white-space: nowrap;
  }

  .player-info {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 12px;
    padding: 14px;
    border-radius: 14px;
    background: rgba(246, 252, 251, 0.95);
    border: 1px solid rgba(16, 27, 30, 0.12);
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
    background: var(--ink-500);
  }

  .status-indicator.playing {
    background: #22c55e;
    animation: pulse 2s ease-in-out infinite;
  }

  .status-indicator.stopped {
    background: var(--ink-500);
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
    font-size: 0.76rem;
    color: rgba(15, 42, 39, 0.76);
    background: rgba(16, 27, 30, 0.07);
    padding: 8px 10px;
    border-radius: 10px;
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    margin: 0;
  }

  .player-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .player-controls {
    display: grid;
    gap: 10px;
  }

  .control-group {
    display: grid;
    gap: 6px;
    font-size: 0.85rem;
    color: var(--ink-600);
    background: rgba(16, 27, 30, 0.04);
    border: 1px solid rgba(16, 27, 30, 0.08);
    border-radius: 12px;
    padding: 8px 10px;
  }

  .control-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
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
    font-size: 0.82rem;
  }

  .rotation-buttons {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .rotation-buttons button {
    background: rgba(16, 27, 30, 0.08);
    color: var(--ink-700);
    padding: 6px 10px;
    border: 1px solid rgba(16, 27, 30, 0.14);
    font-size: 0.76rem;
  }

  .rotation-buttons button.active {
    background: var(--accent-500);
    border-color: var(--accent-500);
    color: #fff;
  }

  .player-placeholder {
    position: relative;
    z-index: 1;
    padding: 20px;
    border-radius: 14px;
    border: 1px dashed rgba(221, 240, 236, 0.36);
    background: rgba(7, 22, 24, 0.4);
    color: rgba(234, 248, 245, 0.9);
    display: grid;
    gap: 12px;
  }

  .player-placeholder p {
    margin: 0;
  }

  .placeholder-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  button {
    border: 1px solid transparent;
    border-radius: 999px;
    padding: 8px 16px;
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
    transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
  }

  button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 16px rgba(7, 22, 24, 0.16);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .ghost {
    background: rgba(255, 255, 255, 0.84);
    border-color: rgba(16, 27, 30, 0.22);
    color: var(--ink-700);
  }

  .danger {
    background: #d4483a;
  }

  .placeholder-actions .ghost {
    background: rgba(255, 255, 255, 0.16);
    border-color: rgba(255, 255, 255, 0.3);
    color: #f4fbfa;
  }

  .error {
    margin: 0;
    position: relative;
    z-index: 1;
    color: #ffd5cf;
    background: rgba(212, 72, 58, 0.2);
    border: 1px solid rgba(255, 179, 169, 0.35);
    border-radius: 10px;
    padding: 8px 10px;
    font-weight: 600;
    font-size: 0.84rem;
  }

  @media (max-width: 760px) {
    .player {
      border-radius: 16px;
      padding: 12px;
    }

    .player-head {
      flex-direction: column;
      align-items: flex-start;
    }

    .player-note {
      white-space: normal;
    }

    .player-actions button {
      flex: 1 1 120px;
    }
  }
</style>
