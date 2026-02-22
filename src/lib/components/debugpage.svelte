<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  type Live = {
    started_at?: number;
    title?: string;
    live_id?: string;
    owner?: { name?: string };
    preview?: { streaming_url_hls?: string };
  };

  type RelayStartResult = {
    playlist_url: string;
    mode: string;
    source: string;
  };

  // catalogTabs は「何でも来る」前提で unknown に寄せると事故減る
  let { mrId, unique, authed, user, catalogTabs } = $props<{
    mrId: string;
    unique: string;
    authed: boolean;
    user: any;
    catalogTabs: unknown;
  }>();

  let now = $state(new Date());
  let liveIdInput = $state("");
  let streamFetchLoading = $state(false);
  let streamStatus = $state<any>(null);
  let streamFetchError = $state("");

  let relayVideoWsUrl = $state("");
  let relayAudioWsUrl = $state("");
  let relayLocalUrl = $state("");
  let relayLoading = $state(false);
  let relayError = $state("");
  let stopRelayOnDestroy = $state(false);

  // props 更新に追従するよう $derived で計算する
  const lives = $derived.by(() => {
    const source = catalogTabs as any;
    const list =
      source?.lives ??
      source?.data?.lives ??
      source?.tab_list?.[0]?.lives ??
      source?.tabs?.[0]?.lives ??
      [];
    return Array.isArray(list) ? (list as Live[]) : [];
  });

  const first = $derived(lives[0]);

  const startedAtUnix = $derived(first?.started_at ?? null);

  const startedAtText = $derived(
    startedAtUnix != null
      ? new Date(startedAtUnix * 1000).toLocaleString("ja-JP")
      : "-"
  );

  const timer = setInterval(() => {
    now = new Date();
  }, 1000);

  const pad = (n: number) => String(n).padStart(2, "0");

  const formatted = $derived(
    `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ` +
      `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`
  );

  const pickFirstString = (...values: Array<unknown>) => {
    for (const value of values) {
      if (typeof value === "string" && value.trim()) return value;
    }
    return "";
  };

  const buildLlstreamWsUrl = (edge: string, streamKey: string, suffix: string) => {
    if (edge.startsWith("ws://") || edge.startsWith("wss://")) {
      const normalized = edge.replace(/\/+$/, "");
      return `${normalized}/ws/${streamKey}/${suffix}`;
    }
    const host = edge.includes(":") ? edge : `${edge}:1883`;
    return `ws://${host}/ws/${streamKey}/${suffix}`;
  };

  const extractLlstreamVideoUrl = (status: any) => {
    const direct = pickFirstString(
      status?.streaming_url_llstream_video,
      status?.live?.streaming_url_llstream_video,
      status?.data?.streaming_url_llstream_video
    );
    if (direct) return direct;

    const streamKey = pickFirstString(
      status?.streaming_key,
      status?.live?.streaming_key,
      status?.data?.streaming_key
    );
    const edge = pickFirstString(
      status?.streaming_url_edge,
      status?.live?.streaming_url_edge,
      status?.data?.streaming_url_edge
    );

    if (!streamKey || !edge) return "";
    return buildLlstreamWsUrl(edge, streamKey, "video/avc");
  };

  const extractLlstreamAudioUrl = (status: any) => {
    const direct = pickFirstString(
      status?.streaming_url_llstream_audio,
      status?.live?.streaming_url_llstream_audio,
      status?.data?.streaming_url_llstream_audio
    );
    if (direct) return direct;

    const streamKey = pickFirstString(
      status?.streaming_key,
      status?.live?.streaming_key,
      status?.data?.streaming_key
    );
    const edge = pickFirstString(
      status?.streaming_url_edge,
      status?.live?.streaming_url_edge,
      status?.data?.streaming_url_edge
    );

    if (!streamKey || !edge) return "";
    return buildLlstreamWsUrl(edge, streamKey, "audio/aac");
  };

  const extractHlsUrl = (status: any) => {
    return pickFirstString(
      status?.streaming_url_hls,
      status?.streaming_url,
      status?.hls_url,
      status?.playlist_url,
      status?.live?.streaming_url_hls,
      status?.data?.streaming_url_hls
    );
  };

  const streamStatusJson = $derived(
    streamStatus ? JSON.stringify(streamStatus, null, 2) : ""
  );

  const resolvedHlsUrl = $derived(extractHlsUrl(streamStatus));

  const handleLiveIdInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    liveIdInput = target?.value ?? "";
  };

  const handleVideoWsInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    relayVideoWsUrl = target?.value ?? "";
  };

  const handleAudioWsInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    relayAudioWsUrl = target?.value ?? "";
  };

  const useFirstLive = () => {
    liveIdInput = first?.live_id ?? "";
  };

  const fetchStreamStatus = async () => {
    const id = liveIdInput.trim();
    if (!id) return;

    streamFetchLoading = true;
    streamFetchError = "";

    try {
      const status = await invoke<any>("get_live_status", { liveId: id });
      streamStatus = status;
      const videoWs = extractLlstreamVideoUrl(status);
      const audioWs = extractLlstreamAudioUrl(status);
      if (videoWs) {
        relayVideoWsUrl = videoWs;
      }
      if (audioWs) {
        relayAudioWsUrl = audioWs;
      }
    } catch (e) {
      streamFetchError = e instanceof Error ? e.message : String(e);
    } finally {
      streamFetchLoading = false;
    }
  };

  const startRelay = async (): Promise<string | null> => {
    const videoWs = relayVideoWsUrl.trim();
    if (!videoWs) {
      relayError = "video ws URL が空です";
      return null;
    }
    const audioWs = relayAudioWsUrl.trim();
    if (!audioWs) {
      relayError = "audio ws URL が空です";
      return null;
    }

    relayLoading = true;
    relayError = "";

    try {
      const result = await invoke<RelayStartResult>("start_llstream_av_ts_relay", {
        videoWsUrl: videoWs,
        audioWsUrl: audioWs
      });
      relayLocalUrl = result.playlist_url;
      return result.playlist_url;
    } catch (e) {
      relayError = e instanceof Error ? e.message : String(e);
      return null;
    } finally {
      relayLoading = false;
    }
  };

  const stopRelay = async () => {
    try {
      await invoke("stop_llstream_relay");
    } catch {
      // noop
    }
    relayLocalUrl = "";
  };

  const playRelayInMpv = async (relayUrl?: string) => {
    const url = (relayUrl ?? relayLocalUrl).trim();
    if (!url) {
      relayError = "relay URL がありません";
      return;
    }

    try {
      await invoke("create_player_window");
      await invoke("start_mpv", {
        url,
        embedded: true,
        windowLabel: "player"
      });
      relayError = "";
    } catch (e) {
      relayError = e instanceof Error ? e.message : String(e);
    }
  };

  const startRelayAndPlay = async () => {
    const relayUrl = await startRelay();
    if (!relayUrl) return;
    await playRelayInMpv(relayUrl);
  };

  const stopMpv = async () => {
    try {
      await invoke("stop_mpv", { reason: "user" });
    } catch {
      // noop
    }
  };

  onDestroy(() => {
    clearInterval(timer);
    if (stopRelayOnDestroy) {
      void invoke("stop_llstream_relay").catch(() => {});
    }
  });
</script>

<section class="testpage">
  <h2>Debug</h2>
  <h2>時間: {formatted}</h2>
  <p>状態: {authed ? "ログイン済み" : "ゲスト"}</p>
  <p>mr_id: {mrId}</p>
  <p>f(unique): {unique}</p>
  <p>user: {user?.name ?? "-"}</p>

  <p>lives: {lives.length} 件</p>
  <p>first title: {first?.title ?? "-"}</p>
  <p>first owner: {first?.owner?.name ?? "-"}</p>
  <p>started_at (unix): {startedAtUnix ?? "-"}</p>
  <p>started_at (local): {startedAtText}</p>

  <div class="relay-panel">
    <h3>LLStream Video+Audio -> MPEG-TS -> HTTP -> MPV</h3>

    <div class="row">
      <input
        placeholder="live_id"
        value={liveIdInput}
        oninput={handleLiveIdInput}
      />
      <button type="button" class="ghost" onclick={useFirstLive} disabled={!first?.live_id}>
        first live_id を使う
      </button>
      <button
        type="button"
        onclick={fetchStreamStatus}
        disabled={streamFetchLoading || !liveIdInput.trim()}
      >
        {streamFetchLoading ? "取得中..." : "get_live_status"}
      </button>
    </div>

    <p>resolved hls: {resolvedHlsUrl || "-"}</p>

    <label>
      video ws URL
      <input
        placeholder="ws://edge-.../ws/[streaming_key]/video/avc"
        value={relayVideoWsUrl}
        oninput={handleVideoWsInput}
      />
    </label>

    <label>
      audio ws URL
      <input
        placeholder="ws://edge-.../ws/[streaming_key]/audio/aac"
        value={relayAudioWsUrl}
        oninput={handleAudioWsInput}
      />
    </label>

    <div class="row">
      <button type="button" onclick={startRelayAndPlay} disabled={relayLoading || !relayVideoWsUrl.trim() || !relayAudioWsUrl.trim()}>
        {relayLoading ? "起動中..." : "relay+MPV起動"}
      </button>
      <button type="button" class="ghost" onclick={startRelay} disabled={relayLoading || !relayVideoWsUrl.trim() || !relayAudioWsUrl.trim()}>
        {relayLoading ? "relay起動中..." : "relay起動"}
      </button>
      <button type="button" class="ghost" onclick={stopRelay}>relay停止</button>
      <button type="button" class="ghost" onclick={() => void playRelayInMpv()} disabled={!relayLocalUrl}>
        MPV再生
      </button>
      <button type="button" class="ghost" onclick={stopMpv}>MPV停止</button>
    </div>

    <label class="inline-toggle">
      <input type="checkbox" bind:checked={stopRelayOnDestroy} />
      ページ破棄時に relay 停止
    </label>

    <p>relay URL: {relayLocalUrl || "-"}</p>

    {#if streamFetchError}
      <p class="error">stream status error: {streamFetchError}</p>
    {/if}
    {#if relayError}
      <p class="error">relay error: {relayError}</p>
    {/if}

    <details>
      <summary>get_live_status response</summary>
      <pre>{streamStatusJson}</pre>
    </details>
  </div>

  <details>
    <summary>catalogTabs raw</summary>
    <pre>{JSON.stringify(catalogTabs, null, 2)}</pre>
  </details>
</section>

<style>
  .testpage {
    display: grid;
    gap: 8px;
    padding: 16px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.8);
    border: 1px solid rgba(16, 27, 30, 0.12);
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  .relay-panel {
    display: grid;
    gap: 8px;
    margin-top: 8px;
    padding: 12px;
    border-radius: 10px;
    border: 1px solid rgba(16, 27, 30, 0.16);
    background: rgba(255, 255, 255, 0.72);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  label {
    display: grid;
    gap: 4px;
    font-size: 0.84rem;
    color: var(--ink-600);
  }

  .inline-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  input {
    min-width: 260px;
    border: 1px solid rgba(16, 27, 30, 0.24);
    border-radius: 8px;
    padding: 8px 10px;
    background: #fff;
    color: var(--ink-900);
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 8px 14px;
    font-weight: 700;
    background: var(--accent-500);
    color: #fff;
    cursor: pointer;
  }

  .ghost {
    background: rgba(16, 27, 30, 0.08);
    color: var(--ink-700);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error {
    color: var(--accent-700);
    font-weight: 600;
  }

  pre {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px;
    border-radius: 8px;
    background: rgba(16, 27, 30, 0.9);
    color: #f3f6f7;
    font-size: 0.76rem;
    line-height: 1.45;
  }
</style>
