<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";

  let playerElement: HTMLDivElement | null = null;
  let isPlaying = $state(false);
  let currentUrl = $state<string | null>(null);
  let statusMessage = $state("");

  let resizeObserver: ResizeObserver | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let unlisten: (() => void) | null = null;
  let refreshInFlight = false;
  let positionUpdateScheduled = false;

  type PlayerInfo = {
    is_playing: boolean;
    is_paused?: boolean;
    autoplay_blocked?: boolean;
    current_url?: string | null;
  };

  const fetchPlayerInfo = async () => {
    try {
      const info = await invoke<PlayerInfo>("get_player_info");
      applyPlayerInfo(info);
      statusMessage = "";
    } catch (e) {
      statusMessage = e instanceof Error ? e.message : String(e);
    }
  };

  const applyPlayerInfo = (info: PlayerInfo | null | undefined) => {
    isPlaying = Boolean(info?.is_playing);
    currentUrl = info?.current_url ?? null;
    if (isPlaying) {
      scheduleUpdatePlayerPosition();
    }
  };

  const scheduleUpdatePlayerPosition = () => {
    if (positionUpdateScheduled) return;
    positionUpdateScheduled = true;
    requestAnimationFrame(() => {
      positionUpdateScheduled = false;
      void updatePlayerPosition();
    });
  };

  const updatePlayerPosition = async () => {
    if (!playerElement || !isPlaying) return;

    const rect = playerElement.getBoundingClientRect();
    const scale = window.devicePixelRatio || 1;
    const params = {
      x: Math.round(rect.left * scale),
      y: Math.round(rect.top * scale),
      width: Math.round(rect.width * scale),
      height: Math.round(rect.height * scale)
    };

    try {
      await invoke("position_mpv_window", params);
    } catch (e) {
      statusMessage = e instanceof Error ? e.message : String(e);
    }
  };

  const refresh = async () => {
    if (refreshInFlight) return;
    refreshInFlight = true;
    try {
      await fetchPlayerInfo();
      if (isPlaying) {
        await updatePlayerPosition();
      }
    } finally {
      refreshInFlight = false;
    }
  };

  const stopPlayback = async () => {
    try {
      await invoke("stop_mpv", { reason: "user" });
      await fetchPlayerInfo();
    } catch (e) {
      statusMessage = e instanceof Error ? e.message : String(e);
    }
  };

  const togglePause = async () => {
    try {
      await invoke("mpv_command", { args: ["cycle", "pause"] });
    } catch (e) {
      statusMessage = e instanceof Error ? e.message : String(e);
    }
  };

  const closeWindow = async () => {
    try {
      await invoke("close_player_window");
    } catch (e) {
      statusMessage = e instanceof Error ? e.message : String(e);
    }
  };

  onMount(() => {
    void refresh();

    const handleUpdate = () => scheduleUpdatePlayerPosition();
    window.addEventListener("resize", handleUpdate);
    window.addEventListener("scroll", handleUpdate, true);

    if (playerElement) {
      resizeObserver = new ResizeObserver(handleUpdate);
      resizeObserver.observe(playerElement);
    }

    pollTimer = setInterval(() => {
      if (document.visibilityState !== "visible") return;
      void refresh();
    }, 4000);

    void (async () => {
      try {
        unlisten = await listen<PlayerInfo>("mpv://state", (event) => {
          applyPlayerInfo(event.payload);
          statusMessage = "";
        });
      } catch (e) {
        statusMessage = e instanceof Error ? e.message : String(e);
      }
    })();

    return () => {
      window.removeEventListener("resize", handleUpdate);
      window.removeEventListener("scroll", handleUpdate, true);
      resizeObserver?.disconnect();
      resizeObserver = null;
      if (pollTimer) clearInterval(pollTimer);
      pollTimer = null;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    if (pollTimer) clearInterval(pollTimer);
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  });
</script>

<section class="player-shell">
  <header class="player-topbar">
    <div class="title">
      <p class="kicker">Player</p>
      <h1>映像ウィンドウ</h1>
    </div>
    <div class="controls">
      <span class="status {isPlaying ? 'live' : 'stopped'}">
        {isPlaying ? "再生中" : "停止中"}
      </span>
      <button class="ghost" onclick={togglePause} disabled={!isPlaying}>
        一時停止/再開
      </button>
      <button class="ghost" onclick={stopPlayback} disabled={!isPlaying}>
        再生停止
      </button>
      <button onclick={closeWindow}>閉じる</button>
    </div>
  </header>

  <div class="video-area" bind:this={playerElement}>
    <div class="video-placeholder">
      {#if isPlaying}
        mpv で再生中
      {:else}
        再生待機中
      {/if}
    </div>
  </div>

  <div class="info-row">
    <span class="label">Stream URL</span>
    <span class="url">{currentUrl ?? "まだ設定されていません"}</span>
  </div>

  {#if statusMessage}
    <p class="error">{statusMessage}</p>
  {/if}
</section>

<style>
  :global(:root) {
    font-family: "Noto Sans JP", "Space Grotesk", sans-serif;
    color: #101b1e;
    background-color: #f7f3ea;
    --font-display: "Space Grotesk", "Noto Sans JP", sans-serif;
    --ink-900: #0b1b1e;
    --ink-700: #1f3739;
    --ink-600: #3a5152;
    --ink-500: #607173;
    --ink-50: #f7f3ea;
    --accent-500: #f25f4c;
    --accent-700: #d4483a;
    --card-surface: #fff;
    --panel-surface: rgba(255, 255, 255, 0.85);
    --shadow-soft: 0 12px 28px rgba(15, 42, 39, 0.08);
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    min-height: 100vh;
    background:
      radial-gradient(1200px 600px at 12% -10%, rgba(242, 95, 76, 0.2), transparent 60%),
      radial-gradient(900px 600px at 90% 10%, rgba(61, 176, 162, 0.2), transparent 55%),
      linear-gradient(160deg, #f7f3ea 0%, #e7f4f1 100%);
    color: var(--ink-900);
    overflow: hidden;
  }

  .player-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    gap: 12px;
    padding: 16px;
  }

  .player-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 14px;
    border-radius: 16px;
    background: var(--panel-surface);
    box-shadow: var(--shadow-soft);
  }

  .kicker {
    margin: 0 0 6px 0;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.65rem;
    color: var(--accent-500);
  }

  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.2rem;
  }

  .controls {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .status {
    padding: 6px 12px;
    border-radius: 999px;
    font-weight: 700;
    font-size: 0.8rem;
  }

  .status.live {
    background: rgba(34, 197, 94, 0.15);
    color: #15803d;
  }

  .status.stopped {
    background: rgba(15, 42, 39, 0.1);
    color: var(--ink-600);
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 8px 14px;
    font-weight: 700;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .ghost {
    background: transparent;
    border: 1px solid rgba(16, 27, 30, 0.2);
    color: var(--ink-700);
  }

  .video-area {
    border-radius: 18px;
    background: #000;
    position: relative;
    overflow: hidden;
  }

  .video-placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.6);
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: 0.75rem;
  }

  .info-row {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 12px;
    background: rgba(16, 27, 30, 0.06);
    font-size: 0.8rem;
  }

  .label {
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.6rem;
    color: var(--ink-500);
  }

  .url {
    word-break: break-all;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New",
      monospace;
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }
</style>
