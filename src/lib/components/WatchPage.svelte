<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import WatchHeader from "$lib/components/watch/WatchHeader.svelte";
  import WatchJoin from "$lib/components/watch/WatchJoin.svelte";
  import WatchPlayer from "$lib/components/watch/WatchPlayer.svelte";
  import WatchInfoPanel from "$lib/components/watch/WatchInfoPanel.svelte";
  import WatchRankingPanel from "$lib/components/watch/WatchRankingPanel.svelte";
  import WatchPollingPanel from "$lib/components/watch/WatchPollingPanel.svelte";
  import WatchCommentsPanel from "$lib/components/watch/WatchCommentsPanel.svelte";
  import {
    extractComments,
    extractRanking,
    getStreamUrl,
    buildLiveInfoView,
    getGiftRankingUrl,
    getObfuscatedUserId,
    buildGiftRankingView,
    flattenForDisplay
  } from "$lib/components/watch/watch-utils";

  let { initialLiveId, initialLive } = $props<{ initialLiveId: string; initialLive?: any }>();

  let liveId = $state("");
  let liveInfo = $state<any>(null);
  let streamStatus = $state<any>(null);
  let streamError = $state("");
  let lastStreamUrl = $state("");
  let pollingTimer: ReturnType<typeof setInterval> | null = null;
  let commentTimer: ReturnType<typeof setInterval> | null = null;
  let didLogCommentSample = false;
  let bootstrapped = false;
  let connectSeq = 0;
  let prevLiveId = "";
  let usingPreview = $state(false);
  let comments = $state<any[]>([]);
  let commentText = $state("");
  let commentTotal = $state<number | null>(null);
  let rankingType = $state("live");
  let giftRanking = $state<any>(null);
  let giftRankingExtra = $state<any>(null);
  let giftRankingLoaded = $state(false);
  let giftRankingCursor = $state<string | null>(null);
  let giftRankingAll = $state(false);
  let giftRankingLoading = $state(false);
  let polling = $state<any>(null);
  let loading = $state(false);
  let error = $state("");
  let isPlaying = $state(false);
  let isPaused = $state(false);
  let volume = $state(70);
  let rotation = $state(0);
  const rotateOptions = [0, 90, 180, 270];
  let autoplayBlocked = $state(false);
  let playerUnlisten: (() => void) | null = null;
  let playerSyncTimer: ReturnType<typeof setInterval> | null = null;
  let lastGiftRankingUrl = "";
  const COMMENT_BUFFER_LIMIT = 300;
  let commentOrder: "asc" | "desc" | "unknown" = "unknown";

  const resetForLive = () => {
    stopKeepAlive();
    cleanupPlayer();
    comments = [];
    commentTotal = null;
    commentOrder = "unknown";
    giftRanking = null;
    giftRankingExtra = null;
    giftRankingLoaded = false;
    giftRankingCursor = null;
    giftRankingAll = false;
    giftRankingLoading = false;
    lastGiftRankingUrl = "";
    polling = null;
    streamStatus = null;
    liveInfo = null;
    streamError = "";
    error = "";
    usingPreview = false;
    autoplayBlocked = false;
  };

  const invokeWithTimeout = async (
    command: string,
    args: Record<string, any> | undefined,
    ms: number,
    label: string
  ) => {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    const timeout = new Promise((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`${label} timeout`));
      }, ms);
    });
    try {
      return await Promise.race([invoke(command, args ?? {}), timeout]);
    } finally {
      if (timeoutId) clearTimeout(timeoutId);
    }
  };

  const pickNumber = (...values: Array<unknown>) => {
    for (const value of values) {
      if (typeof value === "number" && Number.isFinite(value)) return value;
      if (typeof value === "string" && value.trim()) {
        const parsed = Number(value);
        if (Number.isFinite(parsed)) return parsed;
      }
    }
    return null;
  };

  const getCommentKey = (item: any) => {
    const raw =
      item?.comment_id ??
      item?.id ??
      item?.comment?.id ??
      item?.commentId ??
      item?.comment_id_str ??
      null;
    if (raw !== null && raw !== undefined && raw !== "") return String(raw);
    const fallback = [
      item?.user_id ?? item?.user?.user_id ?? "",
      item?.comment ?? item?.message ?? "",
      item?.created_at ?? item?.createdAt ?? ""
    ]
      .join("|")
      .trim();
    return fallback || null;
  };

  const getCommentTimestamp = (item: any) =>
    pickNumber(item?.created_at, item?.createdAt, item?.time, item?.timestamp);

  const inferCommentOrder = (list: any[]) => {
    if (commentOrder !== "unknown") return;
    if (list.length < 2) return;
    const first = getCommentTimestamp(list[0]);
    const last = getCommentTimestamp(list[list.length - 1]);
    if (first === null || last === null || first === last) return;
    commentOrder = first < last ? "asc" : "desc";
  };

  const applyCommentTotal = (res: any) => {
    const meta = res?.data ?? res;
    const total = pickNumber(
      meta?.comment_num,
      meta?.commentNum,
      meta?.comments_num,
      meta?.commentsNum
    );
    if (total !== null) {
      commentTotal = total;
    }
  };

  const trimComments = (list: any[]) => {
    if (list.length <= COMMENT_BUFFER_LIMIT) return list;
    if (commentOrder === "desc") {
      return list.slice(0, COMMENT_BUFFER_LIMIT);
    }
    return list.slice(-COMMENT_BUFFER_LIMIT);
  };

  const mergeComments = (incoming: any[], mode: "latest" | "older") => {
    if (!incoming.length) return;
    inferCommentOrder(incoming);
    const prepend =
      commentOrder === "desc"
        ? mode === "latest"
        : mode === "older";
    const merged = prepend ? [...incoming, ...comments] : [...comments, ...incoming];
    const seen = new Set<string>();
    const deduped: any[] = [];
    for (const item of merged) {
      const key = getCommentKey(item);
      if (key) {
        if (seen.has(key)) continue;
        seen.add(key);
      }
      deduped.push(item);
    }
    comments = trimComments(deduped);
  };

  const updateCommentTotalFromLiveInfo = () => {
    if (typeof liveInfoView?.commentNum === "number") {
      commentTotal = liveInfoView.commentNum;
    }
  };

  // サムネクリックから遷移の検知
  $effect(() => {
    const id = initialLiveId;
    if (id && id !== prevLiveId) {
      prevLiveId = id;
      liveId = id;
      resetForLive();
      if (initialLive) {
        liveInfo = initialLive;
      }
      void openPlayerWindow();
      joinLive(undefined, { silent: false });
    }
  });

  const streamUrl = $derived(
    getStreamUrl(streamStatus) || liveInfo?.preview?.streaming_url_hls || ""
  );

  const liveInfoView = $derived.by(() => buildLiveInfoView(liveInfo, polling));

  $effect(() => {
    updateCommentTotalFromLiveInfo();
  });

  const giftRankingUrl = $derived.by(() => getGiftRankingUrl(polling, liveInfo));

  const obfuscatedUserId = $derived.by(() => getObfuscatedUserId(polling, liveInfo, giftRankingUrl));

  const giftRankingView = $derived.by(() => buildGiftRankingView(giftRanking, giftRankingExtra));

  const pollingDetails = $derived.by(() => (polling ? flattenForDisplay(polling) : []));

  const cleanupPlayer = async () => {
    try {
      await invoke("stop_mpv");
      isPlaying = false;
      isPaused = false;
      lastStreamUrl = "";
    } catch (e) {
      console.warn("Failed to stop mpv", e);
    }
  };

  const openPlayerWindow = async () => {
    try {
      await invoke("create_player_window");
    } catch (e) {
      console.warn("Failed to open player window", e);
    }
  };

  const setupPlayer = async (url: string) => {
    if (!url || url === lastStreamUrl) return;

    try {
      await openPlayerWindow();
      await invoke("start_mpv", { url, embedded: true, windowLabel: "player" });
      lastStreamUrl = url;
      isPlaying = true;
      isPaused = false;
      streamError = "";
    } catch (e) {
      console.error("Failed to start mpv:", e);
      streamError = e instanceof Error ? e.message : String(e);
      isPlaying = false;
    }
  };

  const stopPlayback = async () => {
    try {
      await invoke("stop_mpv", { reason: "user" });
    } catch (e) {
      console.warn("Failed to stop mpv", e);
    } finally {
      isPlaying = false;
      isPaused = false;
      lastStreamUrl = "";
      autoplayBlocked = true;
    }
  };

  const togglePause = async () => {
    if (!isPlaying) return;
    try {
      await invoke("mpv_command", { args: ["cycle", "pause"] });
      isPaused = !isPaused;
    } catch (e) {
      streamError = e instanceof Error ? e.message : String(e);
    }
  };

  const updateVolume = async (value: number) => {
    volume = value;
    if (!isPlaying) return;
    try {
      await invoke("mpv_command", { args: ["set", "volume", String(value)] });
    } catch (e) {
      streamError = e instanceof Error ? e.message : String(e);
    }
  };

  const applyRotation = async (value: number) => {
    rotation = value;
    if (!isPlaying) return;
    try {
      await invoke("mpv_command", { args: ["set", "video-rotate", String(value)] });
    } catch (e) {
      streamError = e instanceof Error ? e.message : String(e);
    }
  };

  type PlayerInfo = {
    is_playing: boolean;
    is_paused?: boolean;
    autoplay_blocked?: boolean;
    current_url?: string | null;
  };

  const applyPlayerInfo = (info: PlayerInfo | null | undefined) => {
    if (!info) return;
    isPlaying = Boolean(info.is_playing);
    if (!isPlaying) {
      isPaused = false;
    } else if (typeof info.is_paused === "boolean") {
      isPaused = info.is_paused;
    }
    if (typeof info.autoplay_blocked === "boolean") {
      autoplayBlocked = info.autoplay_blocked;
    }
    if (isPlaying) {
      autoplayBlocked = false;
    }
    if (info.current_url !== undefined) {
      lastStreamUrl = info.current_url ?? "";
    }
  };

  const syncPlayerInfo = async () => {
    try {
      const info = await invoke<PlayerInfo>("get_player_info");
      applyPlayerInfo(info);
    } catch {
      // ignore to avoid noisy UI when player isn't initialized yet
    }
  };

  const stopKeepAlive = () => {
    if (pollingTimer) clearInterval(pollingTimer);
    pollingTimer = null;
    if (commentTimer) clearInterval(commentTimer);
    commentTimer = null;
  };

  const fetchStreamStatus = async (targetLiveId?: string, seq?: number) => {
    const id = (targetLiveId ?? liveId).trim();
    if (!id) return;
    try {
      const status = await invoke("get_live_status", { liveId: id });
      if (typeof seq === "number" && connectSeq !== seq) return;
      streamStatus = status;
    } catch (e) {
      console.warn("get_live_status failed", e);
    }
  };

  const startKeepAlive = () => {
    stopKeepAlive();
    pollingTimer = setInterval(() => {
      refreshPolling();
    }, 15_000);
    commentTimer = setInterval(() => {
      refreshComments();
    }, 6_000);
  };

  const joinLive = async (event?: Event, opts?: { silent?: boolean }) => {
    event?.preventDefault();
    const targetLiveId = liveId.trim();
    if (!targetLiveId) return;

    const silent = Boolean(opts?.silent);
    if (!silent) {
      loading = true;
    }
    usingPreview = false;
    error = "";
    const seq = ++connectSeq;
    let connectTimer: ReturnType<typeof setTimeout> | null = null;
    if (!silent) {
      connectTimer = setTimeout(() => {
        if (connectSeq === seq) {
          if (getStreamUrl(streamStatus)) return;
          loading = false;
          error = "接続がタイムアウトしました";
          connectSeq += 1;
        }
      }, 15000);
    }
    try {
      if (!bootstrapped) {
        await invokeWithTimeout("bootstrap_guest", undefined, 12000, "bootstrap");
        bootstrapped = true;
      }
      if (connectSeq !== seq) return;

      const joinPromise = invoke("join_live", { liveId: targetLiveId })
        .then((res) => {
          if (connectSeq !== seq) return;
          if (!getStreamUrl(streamStatus)) {
            streamStatus = res;
          }
          if (!getStreamUrl(streamStatus)) {
            void fetchStreamStatus(targetLiveId, seq);
          }
          if (getStreamUrl(streamStatus)) {
            startKeepAlive();
          }
        })
        .catch((e) => {
          console.warn("join_live failed", e);
        });

      const [infoRes, statusRes] = await Promise.allSettled([
        invokeWithTimeout("get_live_info", { liveId: targetLiveId }, 12000, "live_info"),
        invokeWithTimeout("get_live_status", { liveId: targetLiveId }, 12000, "live_status")
      ]);

      if (connectSeq !== seq) return;
      if (infoRes.status === "fulfilled") {
        liveInfo = infoRes.value;
      } else if (!silent) {
        error = infoRes.reason instanceof Error ? infoRes.reason.message : String(infoRes.reason);
      }

      if (statusRes.status === "fulfilled") {
        streamStatus = statusRes.value;
      } else if (!silent && !error) {
        error = statusRes.reason instanceof Error ? statusRes.reason.message : String(statusRes.reason);
      }

      if (!getStreamUrl(streamStatus)) {
        await fetchStreamStatus(targetLiveId, seq);
      }

      if (!silent) {
        void refreshComments(targetLiveId, seq);
        void refreshRanking(targetLiveId, seq);
        void refreshPolling(targetLiveId, seq);
      }
      if (getStreamUrl(streamStatus)) {
        startKeepAlive();
      }

      void joinPromise;
    } catch (e) {
      if (!silent && connectSeq === seq) {
        error = e instanceof Error ? e.message : String(e);
      }
    } finally {
      if (connectTimer) clearTimeout(connectTimer);
      connectTimer = null;
      if (!silent && connectSeq === seq) {
        loading = false;
      }
    }
  };

  const refreshComments = async (targetLiveId?: string | Event, seq?: number) => {
    const id = (typeof targetLiveId === "string" ? targetLiveId : liveId).trim();
    if (!id) return;
    try {
      const res: any = await invoke("get_comments", { liveId: id });
      if (!didLogCommentSample) {
        console.log(res?.comments?.[0]);
        didLogCommentSample = true;
      }
      if (typeof seq === "number" && connectSeq !== seq) return;
      const incoming = extractComments(res);
      applyCommentTotal(res);
      mergeComments(incoming, "latest");
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      error = e instanceof Error ? e.message : String(e);
    }
  };

  const sendComment = async () => {
    if (!commentText.trim() || !liveId.trim()) return;
    try {
      await invoke("comment", {
        liveId: liveId.trim(),
        message: commentText.trim(),
        commentType: 1
      });
      commentText = "";
      await refreshComments();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  };

  const fetchRankingPage = async (
    targetLiveId: string,
    cursor: string | null,
    seq?: number,
    append = false
  ) => {
    if (!targetLiveId) return;
    if (giftRankingLoading) return;
    giftRankingLoading = true;
    try {
      const res: any = await invoke("get_gift_ranking", {
        liveId: targetLiveId,
        rankingType: rankingType,
        cursor: cursor ?? undefined,
        obfuscatedUserId: obfuscatedUserId || undefined
      });
      if (typeof seq === "number" && connectSeq !== seq) return;
      const list = extractRanking(res);
      giftRanking = append && Array.isArray(giftRanking) ? [...giftRanking, ...list] : list;
      giftRankingCursor = res?.next_cursor ?? res?.nextCursor ?? null;
      giftRankingAll = !giftRankingCursor || list.length === 0;
      giftRankingLoaded = true;
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      giftRankingLoading = false;
    }
  };

  const refreshRanking = async (targetLiveId?: string | Event, seq?: number) => {
    const id = (typeof targetLiveId === "string" ? targetLiveId : liveId).trim();
    if (!id) return;
    giftRanking = [];
    giftRankingExtra = null;
    giftRankingLoaded = false;
    giftRankingCursor = null;
    giftRankingAll = false;
    await fetchRankingPage(id, null, seq, false);
  };

  const changeRankingType = (value: string) => {
    if (rankingType === value) return;
    rankingType = value;
    void refreshRanking();
  };

  const loadMoreRanking = async () => {
    if (!liveId.trim() || giftRankingAll || giftRankingLoading) return;
    await fetchRankingPage(liveId.trim(), giftRankingCursor, connectSeq, true);
  };

  const refreshRankingByUrl = async (url: string, seq?: number) => {
    const trimmed = url.trim();
    if (!trimmed) return;
    try {
      const res: any = await invoke("get_gift_ranking_by_url", { url: trimmed });
      if (typeof seq === "number" && connectSeq !== seq) return;
      giftRankingExtra = extractRanking(res);
      giftRankingLoaded = true;
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      console.warn("get_gift_ranking_by_url failed", e);
    }
  };

  const refreshPolling = async (targetLiveId?: string | Event, seq?: number) => {
    const id = (typeof targetLiveId === "string" ? targetLiveId : liveId).trim();
    if (!id) return;
    try {
      const res = await invoke("live_polling", { liveId: id });
      if (typeof seq === "number" && connectSeq !== seq) return;
      polling = res;
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      error = e instanceof Error ? e.message : String(e);
    }
  };

  $effect(() => {
    const url = giftRankingUrl;
    if (!url) {
      giftRankingExtra = null;
      lastGiftRankingUrl = "";
      return;
    }
    if (!url.includes("/api/gift/ranking")) return;
    if (url === lastGiftRankingUrl) return;
    lastGiftRankingUrl = url;
    const seq = connectSeq;
    void refreshRankingByUrl(url, seq);
  });

  // streamUrlが変わったらプレイヤーをセットアップ
  $effect(() => {
    const url = streamUrl;

    if (!url) {
      if (lastStreamUrl) {
        void cleanupPlayer();
      }
      return;
    }

    if (autoplayBlocked) {
      return;
    }

    if (url !== lastStreamUrl) {
      const timer = setTimeout(() => void setupPlayer(url), 50);
      return () => clearTimeout(timer);
    }
  });

  onDestroy(() => {
    stopKeepAlive();
    void invoke("stop_mpv").catch(() => {});
    void cleanupPlayer();
    if (playerSyncTimer) clearInterval(playerSyncTimer);
    playerSyncTimer = null;
    if (playerUnlisten) {
      playerUnlisten();
      playerUnlisten = null;
    }
  });

  onMount(() => {
    void (async () => {
      try {
        playerUnlisten = await listen<PlayerInfo>("mpv://state", (event) => {
          applyPlayerInfo(event.payload);
        });
      } catch {
        playerUnlisten = null;
      }
      await syncPlayerInfo();
      playerSyncTimer = setInterval(() => {
        void syncPlayerInfo();
      }, 4000);
    })();

    return () => {
      if (playerSyncTimer) clearInterval(playerSyncTimer);
      playerSyncTimer = null;
      if (playerUnlisten) {
        playerUnlisten();
        playerUnlisten = null;
      }
    };
  });
</script>

<section class="watch">
  <WatchHeader onRefreshComments={refreshComments} onRefreshPolling={refreshPolling} />

  <WatchJoin
    liveId={liveId}
    loading={loading}
    onJoin={joinLive}
    onLiveIdChange={(value) => (liveId = value)}
  />

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <WatchPlayer
    streamUrl={streamUrl}
    isPlaying={isPlaying}
    isPaused={isPaused}
    volume={volume}
    rotation={rotation}
    rotateOptions={rotateOptions}
    loading={loading}
    streamError={streamError}
    onOpenPlayer={openPlayerWindow}
    onStart={() => setupPlayer(streamUrl)}
    onStop={stopPlayback}
    onTogglePause={togglePause}
    onVolumeChange={updateVolume}
    onRotate={applyRotation}
  />

  <div class="watch-grid">
    <div class="panel">
      <WatchInfoPanel liveInfoView={liveInfoView} />
    </div>

    <div class="panel">
      <WatchRankingPanel
        rankingType={rankingType}
        giftRankingLoaded={giftRankingLoaded}
        giftRankingView={giftRankingView}
        giftRankingAll={giftRankingAll}
        giftRankingLoading={giftRankingLoading}
        onChangeRankingType={changeRankingType}
        onLoadMore={loadMoreRanking}
      />
    </div>

    <div class="panel">
      <WatchPollingPanel pollingDetails={pollingDetails} polling={polling} />
    </div>
  </div>

  <div class="panel">
    <WatchCommentsPanel
      comments={comments}
      commentText={commentText}
      commentTotal={commentTotal}
      onCommentChange={(value) => (commentText = value)}
      onSend={sendComment}
    />
  </div>
</section>

<style>
  .watch {
    display: grid;
    gap: 20px;
  }

  .watch-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 16px;
    align-items: start;
  }

  .panel {
    padding: 16px;
    border-radius: 18px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }
</style>
