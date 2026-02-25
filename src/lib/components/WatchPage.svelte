<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";

  // ── サブコンポーネント ──────────────────────────────────────────────────────
  import WatchHeader from "$lib/components/watch/WatchHeader.svelte";
  import WatchJoin from "$lib/components/watch/WatchJoin.svelte";
  import WatchPlayer from "$lib/components/watch/WatchPlayer.svelte";
  import WatchInfoPanel from "$lib/components/watch/WatchInfoPanel.svelte";
  import WatchRankingPanel from "$lib/components/watch/WatchRankingPanel.svelte";
  import WatchPollingPanel from "$lib/components/watch/WatchPollingPanel.svelte";
  import WatchCommentsPanel from "$lib/components/watch/WatchCommentsPanel.svelte";

  // ── ユーティリティ ─────────────────────────────────────────────────────────
  import {
    extractComments,
    extractRanking,
    getStreamUrl,
    getLlstreamVideoWsUrl,
    getLlstreamAudioWsUrl,
    buildLiveInfoView,
    getGiftRankingUrl,
    getObfuscatedUserId,
    buildGiftRankingView,
    flattenForDisplay,
    pickNullableNumber,
  } from "$lib/components/watch/watch-utils";
  import { log, logWarn, logErr } from "$lib/components/watch/watch-logger";
  import {
    extractBroadcastConfig,
    toBroadcastComment,
    toBroadcastSystemNotice,
  } from "$lib/components/watch/watch-broadcast";
  import { getCommentKey, getCommentTimestamp } from "$lib/components/watch/watch-comments";

  // ─────────────────────────────────────────────────────────────────────────
  // Props
  // ─────────────────────────────────────────────────────────────────────────

  let { initialLiveId, initialLive, authed = false } = $props<{ initialLiveId: string; initialLive?: any; authed?: boolean }>();

  // ─────────────────────────────────────────────────────────────────────────
  // リアクティブ状態
  // ─────────────────────────────────────────────────────────────────────────

  // 配信情報
  let liveId = $state("");
  let liveInfo = $state<any>(null);
  let streamStatus = $state<any>(null);
  let streamError = $state("");
  let lastStreamUrl = $state("");
  let usingPreview = $state(false);
  let relayStreamUrl = $state("");

  // コメント
  let comments = $state<any[]>([]);
  let systemNotices = $state<any[]>([]);
  let commentText = $state("");
  let commentTotal = $state<number | null>(null);

  // ギフトランキング
  let rankingType = $state("live");
  let giftRanking = $state<any>(null);
  let giftRankingExtra = $state<any>(null);
  let giftRankingLoaded = $state(false);
  let giftRankingCursor = $state<string | null>(null);
  let giftRankingAll = $state(false);
  let giftRankingLoading = $state(false);

  // ポーリング・接続状態
  let polling = $state<any>(null);
  let loading = $state(false);
  let error = $state("");

  // プレイヤー状態
  let isPlaying = $state(false);
  let isPaused = $state(false);
  let volume = $state(70);
  let rotation = $state(0);
  const rotateOptions = [0, 90, 180, 270];
  let autoplayBlocked = $state(false);

  // WebSocket 接続状態
  let broadcastConnected = $state(false);
  let broadcastSubscribed = $state(false);

  // 視聴モード: "normal" | "no-join" | "silent"
  // normal: 全機能（WS・入室ログ・コメント送受信・ランキング・ポーリング）
  // no-join: 入室ログのみスキップ（WS・コメント等は有効）
  // silent: 受信のみ（WS接続・コメント受信・ランキング・ポーリングは有効、入室ログ・コメント送信は無効）
  let viewMode = $state<"normal" | "no-join" | "silent">("silent");
  type StreamSourceMode = "auto" | "hls" | "llstream";
  let streamSourceMode = $state<StreamSourceMode>("auto");

  // コメント送信クールダウン
  let commentCooldownMs = $state(500);
  let commentCoolingDown = $state(false);
  let commentCooldownTimer: ReturnType<typeof setTimeout> | null = null;

  // ─────────────────────────────────────────────────────────────────────────
  // 内部変数（リアクティブ不要）
  // ─────────────────────────────────────────────────────────────────────────

  // タイマー・クリーンアップ関数
  let pollingTimer: ReturnType<typeof setInterval> | null = null;
  let playerSyncTimer: ReturnType<typeof setInterval> | null = null;
  let playerUnlisten: (() => void) | null = null;
  let broadcastUnlisten: (() => void) | null = null;
  let broadcastStatusUnlisten: (() => void) | null = null;
  let broadcastLogUnlisten: (() => void) | null = null;

  // 接続管理
  let pollingInFlight = false;
  let bootstrapped = false;
  let connectSeq = 0;
  let prevLiveId = "";
  let lastGiftRankingUrl = "";

  // バッファ上限
  const COMMENT_BUFFER_LIMIT = 300;
  const SYSTEM_NOTICE_LIMIT = 30;

  // コメントの並び順（API/WS から自動推定）
  let commentOrder: "asc" | "desc" | "unknown" = "unknown";

  const stopRelay = async (reason = "") => {
    if (reason) log("relay", `stop relay: ${reason}`);
    try {
      await invoke("stop_llstream_relay");
    } catch {
      // noop
    } finally {
      relayStreamUrl = "";
    }
  };

  const startRelayFromStatus = async (status: any, seq: number) => {
    const videoWsUrl = getLlstreamVideoWsUrl(status);
    const audioWsUrl = getLlstreamAudioWsUrl(status);
    if (!videoWsUrl || !audioWsUrl) {
      logWarn("relay", "llstream ws urls missing, fallback to HLS");
      relayStreamUrl = "";
      return false;
    }

    try {
      log("relay", `starting av relay v=${videoWsUrl.slice(0, 64)}… a=${audioWsUrl.slice(0, 64)}…`);
      const res = await invoke<any>("start_llstream_av_ts_relay", { videoWsUrl, audioWsUrl });
      if (connectSeq !== seq) {
        logWarn("relay", "stale relay start result, stopping");
        await stopRelay("stale-seq");
        return false;
      }
      const url = typeof res?.playlist_url === "string" ? res.playlist_url : "";
      if (!url) {
        logWarn("relay", "relay started but playlist_url is empty");
        relayStreamUrl = "";
        return false;
      }
      relayStreamUrl = url;
      log("relay", `relay ready: ${url}`);
      return true;
    } catch (e) {
      relayStreamUrl = "";
      logWarn("relay", "relay start failed, fallback to HLS", e);
      return false;
    }
  };

  const getResolvedHlsUrl = (status: any) =>
    getStreamUrl(status) || liveInfo?.preview?.streaming_url_hls || "";

  const hasRelayWsUrls = (status: any) =>
    Boolean(getLlstreamVideoWsUrl(status) && getLlstreamAudioWsUrl(status));

  const hasCandidateForSource = (status: any) => {
    if (streamSourceMode === "hls") {
      return Boolean(getResolvedHlsUrl(status));
    }
    if (streamSourceMode === "llstream") {
      return hasRelayWsUrls(status);
    }
    return hasRelayWsUrls(status) || Boolean(getResolvedHlsUrl(status));
  };

  const selectStreamSource = async (status: any, seq: number, context: string) => {
    const hlsUrl = getResolvedHlsUrl(status);
    if (streamSourceMode === "hls") {
      if (relayStreamUrl) {
        await stopRelay(`${context}: hls-selected`);
      }
      if (hlsUrl) {
        log("hls", `${context}: selected source HLS ${hlsUrl.slice(0, 80)}…`);
        return true;
      }
      logWarn("hls", `${context}: HLS selected but URL is missing`);
      return false;
    }

    const relayReady = await startRelayFromStatus(status, seq);
    if (relayReady) {
      log("relay", `${context}: selected source LLStream relay`);
      return true;
    }

    if (streamSourceMode === "llstream") {
      logWarn("relay", `${context}: LLStream selected but relay unavailable`);
      return false;
    }

    if (hlsUrl) {
      log("hls", `${context}: fallback HLS ${hlsUrl.slice(0, 80)}…`);
      return true;
    }

    logWarn("hls", `${context}: no relay/HLS URL after all attempts`);
    return false;
  };

  // ─────────────────────────────────────────────────────────────────────────
  // 状態リセット
  // ─────────────────────────────────────────────────────────────────────────

  /** 新しい配信に切り替える際に全状態を初期化する */
  const resetForLive = () => {
    log("join", "resetForLive");
    stopKeepAlive();
    void disconnectBroadcast();
    void stopRelay("resetForLive");
    // cleanupPlayer は streamUrl $effect に任せる（二重呼び出し防止）
    comments = [];
    systemNotices = [];
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
    relayStreamUrl = "";
    pollingInFlight = false;
    streamError = "";
    error = "";
    usingPreview = false;
    autoplayBlocked = false;
  };

  // ─────────────────────────────────────────────────────────────────────────
  // API ユーティリティ
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Tauri コマンドをタイムアウト付きで実行する。
   * 規定時間内に応答がなければ Error をスローする。
   */
  const invokeWithTimeout = async (
    command: string,
    args: Record<string, any> | undefined,
    ms: number,
    label: string
  ) => {
    const t0 = performance.now();
    log("api", `→ ${label} (${command})`, args ?? {});
    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    const timeout = new Promise((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`${label} timeout`));
      }, ms);
    });
    try {
      const result = await Promise.race([invoke(command, args ?? {}), timeout]);
      log("api", `← ${label} OK (${(performance.now() - t0).toFixed(0)}ms)`);
      return result;
    } catch (e) {
      logErr("api", `← ${label} FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
      throw e;
    } finally {
      if (timeoutId) clearTimeout(timeoutId);
    }
  };

  // ─────────────────────────────────────────────────────────────────────────
  // コメント管理
  // ─────────────────────────────────────────────────────────────────────────

  /** システム通知を重複排除しながら先頭に追加する */
  const appendSystemNotice = (notice: any) => {
    const filtered = systemNotices.filter((item) => item?.key !== notice?.key);
    systemNotices = [notice, ...filtered].slice(0, SYSTEM_NOTICE_LIMIT);
  };

  /**
   * コメントリストの並び順を自動推定する（asc / desc）。
   * 先頭と末尾のタイムスタンプを比較して判定。
   * 一度確定したら変更しない。
   */
  const inferCommentOrder = (list: any[]) => {
    if (commentOrder !== "unknown") return;
    if (list.length < 2) return;
    const first = getCommentTimestamp(list[0]);
    const last = getCommentTimestamp(list[list.length - 1]);
    if (first === null || last === null || first === last) return;
    commentOrder = first < last ? "asc" : "desc";
  };

  /** APIレスポンスからコメント総数を更新する */
  const applyCommentTotal = (res: any) => {
    const meta = res?.data ?? res;
    const total = pickNullableNumber(
      meta?.comment_num,
      meta?.commentNum,
      meta?.comments_num,
      meta?.commentsNum
    );
    if (total !== null) {
      commentTotal = total;
    }
  };

  /**
   * コメントバッファが上限を超えた場合に古いものをトリムする。
   * 並び順に応じて先頭（desc）または末尾（asc）を保持する。
   */
  const trimComments = (list: any[]) => {
    if (list.length <= COMMENT_BUFFER_LIMIT) return list;
    if (commentOrder === "desc") {
      return list.slice(0, COMMENT_BUFFER_LIMIT);
    }
    return list.slice(-COMMENT_BUFFER_LIMIT);
  };

  /**
   * 新着コメントを既存リストにマージする。
   * - mode="latest": 最新コメントとして追加
   * - mode="older" : 古いコメントとして追加（ページング）
   * comment_id による重複排除を行う。
   */
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

  /** liveInfoView のコメント数でコメント総数を更新する */
  const updateCommentTotalFromLiveInfo = () => {
    if (typeof liveInfoView?.commentNum === "number") {
      commentTotal = liveInfoView.commentNum;
    }
  };

  // ─────────────────────────────────────────────────────────────────────────
  // 派生状態（$derived）
  // ─────────────────────────────────────────────────────────────────────────

  // ゲスト状態では silent モード固定
  $effect(() => {
    if (!authed && viewMode !== "silent") {
      viewMode = "silent";
    }
  });

  // サムネクリックからの遷移を検知して自動参加する
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
      if (viewMode === "silent") {
        void silentWatch(id);
      } else {
        joinLive(undefined, { silent: false });
      }
    }
  });

  // viewMode 変更を検知してWS接続を自動切り替え
  let prevViewMode: typeof viewMode | null = null;
  $effect(() => {
    const mode = viewMode;
    const id = liveId.trim();
    if (prevViewMode === null) {
      // 初回は記録のみ
      prevViewMode = mode;
      return;
    }
    if (mode === prevViewMode || !id) {
      prevViewMode = mode;
      return;
    }
    const wasSilent = prevViewMode === "silent";
    prevViewMode = mode;

    if (wasSilent && mode === "normal") {
      // silent → normal: 入室ログを送信（WS は既に接続済み）
      log("join", `viewMode changed to "normal", sending join log`);
      void invokeWithTimeout("join_live", { liveId: id }, 12000, "join_live").catch((e: any) =>
        logWarn("join", "join_live failed on mode switch", e)
      );
    }
    // silent ↔ no-join / normal ↔ no-join: WS は既に接続中なので何もしない
  });

  const hlsStreamUrl = $derived(getResolvedHlsUrl(streamStatus));

  /** 再生可能 URL（ソース選択に応じて切り替え） */
  const streamUrl = $derived.by(() => {
    if (streamSourceMode === "hls") return hlsStreamUrl;
    if (streamSourceMode === "llstream") return relayStreamUrl;
    return relayStreamUrl || hlsStreamUrl;
  });

  // HLS 固定に切り替えたら relay は停止して明示的に HLS を使う
  $effect(() => {
    if (streamSourceMode !== "hls") return;
    if (!relayStreamUrl) return;
    void stopRelay("source-mode-hls");
  });

  /** UI 表示用にマージされたライブ情報 */
  const liveInfoView = $derived.by(() => buildLiveInfoView(liveInfo, polling));

  $effect(() => {
    updateCommentTotalFromLiveInfo();
  });

  /** ギフトランキング取得用の URL */
  const giftRankingUrl = $derived.by(() => getGiftRankingUrl(polling, liveInfo));

  /** ランキング API の認証用難読化ユーザー ID */
  const obfuscatedUserId = $derived.by(() => getObfuscatedUserId(polling, liveInfo, giftRankingUrl));

  /** UI 表示用に正規化されたギフトランキング */
  const giftRankingView = $derived.by(() => buildGiftRankingView(giftRanking, giftRankingExtra));

  /** ポーリングデータをデバッグ表示用にフラット化 */
  const pollingDetails = $derived.by(() => (polling ? flattenForDisplay(polling) : []));

  /** 現在の接続状態をラベルで表現する */
  const watchStatus = $derived.by(() => {
    if (loading) return { label: "接続中", tone: "connecting" as const };
    if (error || streamError) return { label: "エラー", tone: "error" as const };
    if (broadcastConnected) return { label: "リアルタイム", tone: "live" as const };
    if (streamUrl) return { label: "再生準備OK", tone: "ready" as const };
    return { label: "待機中", tone: "idle" as const };
  });

  /** ヘッダーに表示する配信 ID */
  const activeLiveIdLabel = $derived.by(() => {
    const value = liveInfoView?.liveId || liveId.trim();
    return value || "-";
  });

  /** ヘッダーに表示する視聴者数サマリー */
  const viewerSummary = $derived.by(() => {
    if (!liveInfoView) return "-";
    if (liveInfoView.onlineViewers > 0) return `${liveInfoView.onlineViewers.toLocaleString()} 人`;
    if (liveInfoView.totalViewers > 0) return `${liveInfoView.totalViewers.toLocaleString()} 人`;
    return "0 人";
  });

  /** ヘッダーに表示するコメント数サマリー */
  const commentSummary = $derived.by(() => {
    if (commentTotal !== null) return `${commentTotal.toLocaleString()} 件`;
    return `${comments.length.toLocaleString()} 件`;
  });

  // ─────────────────────────────────────────────────────────────────────────
  // プレイヤー管理
  // ─────────────────────────────────────────────────────────────────────────

  /** MPV プレイヤーを停止してフロントエンドの状態をリセットする */
  const cleanupPlayer = async () => {
    log("hls", "stopping mpv…");
    try {
      await invoke("stop_mpv");
      isPlaying = false;
      isPaused = false;
      lastStreamUrl = "";
      log("hls", "mpv stopped");
    } catch (e) {
      logWarn("hls", "stop_mpv failed", e);
    }
  };

  /** プレイヤーウィンドウを開く（既に開いている場合はスキップ） */
  const openPlayerWindow = async () => {
    try {
      await invoke("create_player_window");
    } catch (e) {
      console.warn("Failed to open player window", e);
    }
  };

  /** 指定 URL で MPV プレイヤーを起動する（同一 URL の場合はスキップ） */
  const setupPlayer = async (url: string) => {
    if (!url || url === lastStreamUrl) return;

    log("hls", `starting mpv url=${url.slice(0, 80)}…`);
    const t0 = performance.now();
    try {
      await openPlayerWindow();
      await invoke("start_mpv", { url, embedded: true, windowLabel: "player" });
      lastStreamUrl = url;
      isPlaying = true;
      isPaused = false;
      streamError = "";
      log("hls", `mpv started (${(performance.now() - t0).toFixed(0)}ms)`);
    } catch (e) {
      logErr("hls", `start_mpv FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
      streamError = e instanceof Error ? e.message : String(e);
      isPlaying = false;
    }
  };

  /** ユーザー操作で再生を停止する（自動再生ブロックフラグをセット） */
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

  /** 再生の一時停止・再開を切り替える */
  const togglePause = async () => {
    if (!isPlaying) return;
    try {
      await invoke("mpv_command", { args: ["cycle", "pause"] });
      isPaused = !isPaused;
    } catch (e) {
      streamError = e instanceof Error ? e.message : String(e);
    }
  };

  /** 音量を変更する（0–100） */
  const updateVolume = async (value: number) => {
    volume = value;
    if (!isPlaying) return;
    try {
      await invoke("mpv_command", { args: ["set", "volume", String(value)] });
    } catch (e) {
      streamError = e instanceof Error ? e.message : String(e);
    }
  };

  /** 映像の回転角度を変更する */
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

  /** Rust 側から受け取ったプレイヤー状態をフロントエンドの状態に反映する */
  const applyPlayerInfo = (info: PlayerInfo | null | undefined) => {
    if (!info) return;
    log("hls", `playerInfo: playing=${info.is_playing} paused=${info.is_paused} blocked=${info.autoplay_blocked}`);
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

  /** Rust 側からプレイヤー情報をポーリングして状態を同期する */
  const syncPlayerInfo = async () => {
    try {
      const info = await invoke<PlayerInfo>("get_player_info");
      applyPlayerInfo(info);
    } catch {
      // プレイヤー未初期化時はエラーを無視（UI を汚さないため）
    }
  };

  // streamUrl が変化したらプレイヤーをセットアップ（またはクリーンアップ）する
  $effect(() => {
    const url = streamUrl;

    if (!url) {
      if (lastStreamUrl) {
        log("hls", "streamUrl cleared, cleaning up player");
        void cleanupPlayer();
      }
      return;
    }

    if (autoplayBlocked) {
      log("hls", "autoplay blocked, skipping setup");
      return;
    }

    if (url !== lastStreamUrl) {
      log("hls", `streamUrl changed → ${url.slice(0, 60)}…`);
      const timer = setTimeout(() => void setupPlayer(url), 50);
      return () => clearTimeout(timer);
    }
  });

  // ─────────────────────────────────────────────────────────────────────────
  // WebSocket（ブロードキャスト）管理
  // ─────────────────────────────────────────────────────────────────────────

  /** ポーリングタイマーを停止する */
  const stopKeepAlive = () => {
    if (pollingTimer) clearInterval(pollingTimer);
    pollingTimer = null;
  };

  /** WS 接続を切断してリスナーをクリーンアップする */
  const disconnectBroadcast = async () => {
    log("ws", "disconnecting…");
    broadcastConnected = false;
    broadcastSubscribed = false;
    if (broadcastUnlisten) {
      broadcastUnlisten();
      broadcastUnlisten = null;
    }
    if (broadcastStatusUnlisten) {
      broadcastStatusUnlisten();
      broadcastStatusUnlisten = null;
    }
    await invoke("disconnect_broadcast").catch(() => {});
    log("ws", "disconnected");
    // ログリスナーはページ離脱時まで維持（再接続ログも見えるように）
  };

  /**
   * ブロードキャストの subscribe 完了を待つ。
   * タイムアウトした場合は false を返す。
   */
  const waitForBroadcastSubscribed = async (timeoutMs = 10000) => {
    if (broadcastSubscribed) return true;
    const startedAt = Date.now();
    while (Date.now() - startedAt < timeoutMs) {
      await new Promise((resolve) => setTimeout(resolve, 50));
      if (broadcastSubscribed) return true;
    }
    return broadcastSubscribed;
  };

  /**
   * WebSocket でブロードキャストサーバーに接続する。
   * メッセージリスナーとステータスリスナーをセットアップしてから
   * Rust 側の connect_broadcast を呼び出す。
   * 失敗時はリスナーをクリーンアップしてエラーをスローする。
   */
  const connectBroadcast = async (info: any) => {
    const config = extractBroadcastConfig(info);
    if (!config) {
      throw new Error("broadcast config not found");
    }

    log("ws", `connecting host=${config.host} key=${config.bcsvrKey.slice(0, 12)}…`);
    await disconnectBroadcast();

    try {
      // メッセージリスナーを設定
      broadcastUnlisten = await listen<any>("broadcast://message", (event) => {
        const msg = event.payload;
        if (!msg) return;

        const msgType = pickNullableNumber(msg?.t, msg?.type);
        log("ws", `recv t=${msgType}`, msg);

        // t=123: 配信終了通知
        if (msgType === 123) {
          log("ws", "stream ended (t=123)");
          appendSystemNotice({
            key: `end:${Date.now()}`,
            type: "end",
            text: "配信が終了しました",
            viewers: null,
            created_at: Math.floor(Date.now() / 1000),
            _raw: msg,
          });
          return;
        }

        const systemNotice = toBroadcastSystemNotice(msg);
        if (systemNotice) {
          log("ws", `system notice: ${systemNotice.text}`);
          appendSystemNotice(systemNotice);
          return;
        }
        // t=38 などは除外し、コメントイベント(t=1)のみ反映
        const comment = toBroadcastComment(msg);
        if (!comment) return;
        log("ws", `comment: ${comment.user_name}: ${comment.comment.slice(0, 40)}`);
        mergeComments([comment], "latest");
      });

      // 接続ステータスリスナーを設定
      broadcastStatusUnlisten = await listen<string>("broadcast://status", (event) => {
        const status = event.payload;
        log("ws", `status → ${status}`);
        if (status === "subscribed") {
          broadcastConnected = true;
          broadcastSubscribed = true;
          return;
        }
        if (status === "connected") {
          broadcastConnected = true;
          broadcastSubscribed = false;
          return;
        }
        if (status === "disconnected" || status === "failed" || status === "error") {
          logWarn("ws", `connection lost: ${status}`);
          broadcastConnected = false;
          broadcastSubscribed = false;
        }
      });

      const t0 = performance.now();
      await invoke("connect_broadcast", {
        bcsvrKey: config.bcsvrKey,
        broadcastHost: config.host,
      });
      log("ws", `connect_broadcast OK (${(performance.now() - t0).toFixed(0)}ms)`);

      const subscribed = await waitForBroadcastSubscribed();
      if (!subscribed) {
        throw new Error("broadcast subscribe timeout");
      }
      log("ws", "subscribed OK");
    } catch (e) {
      // リスナーリーク防止: 失敗時にクリーンアップ
      broadcastUnlisten?.();
      broadcastUnlisten = null;
      broadcastStatusUnlisten?.();
      broadcastStatusUnlisten = null;
      logErr("ws", "connect failed", e);
      throw e;
    }
  };

  // ─────────────────────────────────────────────────────────────────────────
  // API フェッチ関数
  // ─────────────────────────────────────────────────────────────────────────

  /** ストリームステータス（LLStream/HLS URL を含む）を取得する */
  const fetchStreamStatus = async (targetLiveId?: string, seq?: number) => {
    const id = (targetLiveId ?? liveId).trim();
    if (!id) return;
    const t0 = performance.now();
    log("api", `→ get_live_status liveId=${id}`);
    try {
      const status = await invoke("get_live_status", { liveId: id });
      if (typeof seq === "number" && connectSeq !== seq) return;
      streamStatus = status;
      log("api", `← get_live_status OK (${(performance.now() - t0).toFixed(0)}ms)`);
    } catch (e) {
      logWarn("api", `← get_live_status FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
    }
  };

  /** ポーリングタイマーを開始する（15 秒間隔） */
  const startKeepAlive = () => {
    stopKeepAlive();
    pollingTimer = setInterval(() => {
      refreshPolling();
    }, 15_000);
  };

  /**
   * サイレント視聴: 映像再生の準備のみ行う。
   * joinLive を一切呼ばず、WS接続・入室ログ・コメント等すべてスキップ。
   */
  const silentWatch = async (targetLiveId: string) => {
    loading = true;
    error = "";
    const seq = ++connectSeq;
    try {
      log("join", `silent watch: liveId=${targetLiveId}`);
      if (!bootstrapped) {
        await invokeWithTimeout("bootstrap_guest", undefined, 12000, "bootstrap");
        bootstrapped = true;
      }
      if (connectSeq !== seq) return;

      const [infoRes, statusRes] = await Promise.allSettled([
        invokeWithTimeout("get_live_info", { liveId: targetLiveId }, 12000, "live_info"),
        invokeWithTimeout("get_live_status", { liveId: targetLiveId }, 12000, "live_status")
      ]);
      if (connectSeq !== seq) return;

      if (infoRes.status === "fulfilled") {
        liveInfo = infoRes.value;
      }
      if (statusRes.status === "fulfilled") {
        streamStatus = statusRes.value;
      }
      if (!hasCandidateForSource(streamStatus)) {
        await fetchStreamStatus(targetLiveId, seq);
      }

      const sourceReady = await selectStreamSource(streamStatus, seq, "silent mode");
      if (sourceReady) {
        startKeepAlive();
      } else {
        error =
          streamSourceMode === "llstream"
            ? "LLStream relay が開始できませんでした"
            : "再生URLが取得できませんでした";
      }

      // WS 接続（受信のみ）: liveInfo → streamStatus → polling の順で broadcast config を探す
      let broadcastSource: any = liveInfo;
      let bcConfig = extractBroadcastConfig(broadcastSource);
      if (!bcConfig) {
        broadcastSource = streamStatus;
        bcConfig = extractBroadcastConfig(broadcastSource);
      }
      if (!bcConfig) {
        log("ws", "[silent] broadcast config not in liveInfo/streamStatus, trying polling…");
        await refreshPolling(targetLiveId, seq);
        if (connectSeq !== seq) return;
        if (polling) {
          broadcastSource = polling;
          bcConfig = extractBroadcastConfig(broadcastSource);
        }
      }
      if (broadcastSource && bcConfig) {
        try {
          await connectBroadcast(broadcastSource);
          if (connectSeq !== seq) return;
          log("ws", "[silent] broadcast ready (receive-only)");
        } catch (wsErr) {
          logWarn("ws", "[silent] broadcast unavailable", wsErr);
        }
      }

      // コメント・ランキング・ポーリング取得（受信のみ）
      await refreshComments(targetLiveId, seq);
      if (connectSeq !== seq) return;
      void refreshRanking(targetLiveId, seq);
      void refreshPolling(targetLiveId, seq);

      log("join", "silentWatch complete");
    } catch (e) {
      logErr("join", "silentWatch error", e);
      error = e instanceof Error ? e.message : String(e);
    } finally {
      if (connectSeq === seq) loading = false;
    }
  };

  /**
   * 配信に参加する主要フロー。
   * 1. ゲストセッションのブートストラップ（初回のみ）
   * 2. liveInfo と streamStatus を並列取得
   * 3. WebSocket 接続（失敗時は REST ポーリングにフォールバック）
   * 4. join_live API を呼び出して参加完了（normal のみ）
   * 5. 初期コメント・ランキング・ポーリングを取得
   */
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
          if (streamUrl) return;
          loading = false;
          error = "接続がタイムアウトしました";
          connectSeq += 1;
        }
      }, 15000);
    }
    try {
      log("join", `start liveId=${targetLiveId} silent=${silent} seq=${seq}`);
      if (!bootstrapped) {
        await invokeWithTimeout("bootstrap_guest", undefined, 12000, "bootstrap");
        bootstrapped = true;
      }
      if (connectSeq !== seq) { log("join", "seq mismatch after bootstrap"); return; }

      log("join", "fetching live_info & live_status in parallel…");
      const [infoRes, statusRes] = await Promise.allSettled([
        invokeWithTimeout("get_live_info", { liveId: targetLiveId }, 12000, "live_info"),
        invokeWithTimeout("get_live_status", { liveId: targetLiveId }, 12000, "live_status")
      ]);

      if (connectSeq !== seq) { log("join", "seq mismatch after info/status"); return; }
      if (infoRes.status === "fulfilled") {
        liveInfo = infoRes.value;
        log("join", "liveInfo OK", { bcsvr_key: liveInfo?.bcsvr_key, broadcast_host: liveInfo?.broadcast_host });
      } else {
        logWarn("join", "liveInfo FAIL", infoRes.reason);
        if (!silent) {
          error = infoRes.reason instanceof Error ? infoRes.reason.message : String(infoRes.reason);
        }
      }

      if (statusRes.status === "fulfilled") {
        streamStatus = statusRes.value;
        const hlsUrl = getResolvedHlsUrl(streamStatus);
        log("hls", `streamStatus OK, hls=${hlsUrl ? hlsUrl.slice(0, 60) + "…" : "(none)"}`);
      } else {
        logWarn("join", "streamStatus FAIL", statusRes.reason);
        if (!silent && !error) {
          error = statusRes.reason instanceof Error ? statusRes.reason.message : String(statusRes.reason);
        }
      }

      if (!hasCandidateForSource(streamStatus)) {
        log("hls", "no selected-source candidate yet, retrying fetchStreamStatus…");
        await fetchStreamStatus(targetLiveId, seq);
      }

      if (relayStreamUrl || hasCandidateForSource(streamStatus)) {
        log("hls", "stream URL available, starting keepAlive");
        startKeepAlive();
      }

      // WebSocket 接続: liveInfo → streamStatus → polling の順で broadcast config を探す
      let broadcastSource = liveInfo;
      let bcConfig = extractBroadcastConfig(broadcastSource);
      if (!bcConfig) {
        broadcastSource = streamStatus;
        bcConfig = extractBroadcastConfig(broadcastSource);
      }
      if (!bcConfig && polling) {
        broadcastSource = polling;
        bcConfig = extractBroadcastConfig(broadcastSource);
      }
      // それでも取れない場合、polling を取得して再試行
      if (!bcConfig) {
        log("ws", "broadcast config not in liveInfo/streamStatus, trying polling…");
        await refreshPolling(targetLiveId, seq);
        if (connectSeq !== seq) return;
        if (polling) {
          broadcastSource = polling;
          bcConfig = extractBroadcastConfig(broadcastSource);
        }
      }
      if (broadcastSource && bcConfig) {
        try {
          await connectBroadcast(broadcastSource);
          if (connectSeq !== seq) { log("join", "seq mismatch after broadcast"); return; }
          log("ws", "broadcast ready");
        } catch (wsErr) {
          logWarn("ws", "broadcast unavailable, fallback to REST polling", wsErr);
          broadcastConnected = false;
          broadcastSubscribed = false;
        }
      } else {
        logWarn("ws", "broadcast config missing in all sources, fallback to REST polling");
        broadcastConnected = false;
        broadcastSubscribed = false;
      }

      // 入室ログ送信（normal のみ）
      if (viewMode === "normal") {
        const joinRes = await invokeWithTimeout("join_live", { liveId: targetLiveId }, 12000, "join_live");
        if (connectSeq !== seq) { log("join", "seq mismatch after join"); return; }
        if (!hasCandidateForSource(streamStatus)) {
          streamStatus = joinRes;
        }
      } else {
        log("join", "skipping join_live (入室ログなしモード)");
      }

      if (!hasCandidateForSource(streamStatus)) {
        await fetchStreamStatus(targetLiveId, seq);
      }
      const sourceReady = await selectStreamSource(streamStatus, seq, "join flow");
      if (sourceReady) {
        startKeepAlive();
      } else if (!silent) {
        error =
          streamSourceMode === "llstream"
            ? "LLStream relay が開始できませんでした"
            : "再生URLが取得できませんでした";
      }

      if (!silent) {
        await refreshComments(targetLiveId, seq);
      }
      if (connectSeq !== seq) return;

      if (!silent) {
        void refreshRanking(targetLiveId, seq);
        void refreshPolling(targetLiveId, seq);
      }
      log("join", "joinLive complete");
    } catch (e) {
      logErr("join", "joinLive error", e);
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

  /** 最新コメントを REST API で取得してマージする */
  const refreshComments = async (targetLiveId?: string | Event, seq?: number) => {
    const id = (typeof targetLiveId === "string" ? targetLiveId : liveId).trim();
    if (!id) return;
    const t0 = performance.now();
    log("api", `→ get_comments liveId=${id}`);
    try {
      const res: any = await invoke("get_comments", { liveId: id });
      if (typeof seq === "number" && connectSeq !== seq) return;
      const incoming = extractComments(res);
      applyCommentTotal(res);
      mergeComments(incoming, "latest");
      log("api", `← get_comments OK (${(performance.now() - t0).toFixed(0)}ms) +${incoming.length} comments, total=${commentTotal}`);
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      logWarn("api", `← get_comments FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
    }
  };

  /** コメントを送信する。WS 未接続時は送信後に REST でコメントを再取得する。 */
  const sendComment = async () => {
    if (!commentText.trim() || !liveId.trim()) return;
    if (commentCoolingDown) return;
    const text = commentText.trim();
    const t0 = performance.now();
    log("api", `→ comment liveId=${liveId.trim()} text="${text.slice(0, 30)}"`);
    try {
      // REST API で送信 → サーバーが WS で全員にブロードキャスト
      await invoke("comment", {
        liveId: liveId.trim(),
        message: text,
        commentType: 1
      });
      log("api", `← comment OK (${(performance.now() - t0).toFixed(0)}ms)`);
      commentText = "";
      // クールダウン開始
      commentCoolingDown = true;
      if (commentCooldownTimer) clearTimeout(commentCooldownTimer);
      commentCooldownTimer = setTimeout(() => {
        commentCoolingDown = false;
        commentCooldownTimer = null;
      }, commentCooldownMs);
      // WS 未接続時のみ REST API でコメント再取得
      if (!broadcastConnected) {
        log("api", "WS not connected, refreshing comments via REST");
        await refreshComments();
      }
    } catch (e) {
      logErr("api", `← comment FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
      error = e instanceof Error ? e.message : String(e);
    }
  };

  /** ギフトランキングの 1 ページ分を取得する（append=true でページング追加） */
  const fetchRankingPage = async (
    targetLiveId: string,
    cursor: string | null,
    seq?: number,
    append = false
  ) => {
    if (!targetLiveId) return;
    if (giftRankingLoading) return;
    giftRankingLoading = true;
    const t0 = performance.now();
    log("api", `→ get_gift_ranking type=${rankingType} cursor=${cursor ?? "(initial)"}`);
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
      log("api", `← get_gift_ranking OK (${(performance.now() - t0).toFixed(0)}ms) +${list.length} items`);
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      logWarn("api", `← get_gift_ranking FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
    } finally {
      giftRankingLoading = false;
    }
  };

  /** ギフトランキングを最初から取り直す */
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

  /** ランキング種別を切り替えて再取得する */
  const changeRankingType = (value: string) => {
    if (rankingType === value) return;
    rankingType = value;
    void refreshRanking();
  };

  /** ランキングの次ページを追加読み込みする */
  const loadMoreRanking = async () => {
    if (!liveId.trim() || giftRankingAll || giftRankingLoading) return;
    await fetchRankingPage(liveId.trim(), giftRankingCursor, connectSeq, true);
  };

  /** ギフトランキングを専用 URL から直接取得する（追加情報としてマージ） */
  const refreshRankingByUrl = async (url: string, seq?: number) => {
    const trimmed = url.trim();
    if (!trimmed) return;
    const t0 = performance.now();
    log("api", `→ get_gift_ranking_by_url`);
    try {
      const res: any = await invoke("get_gift_ranking_by_url", { url: trimmed });
      if (typeof seq === "number" && connectSeq !== seq) return;
      giftRankingExtra = extractRanking(res);
      giftRankingLoaded = true;
      log("api", `← get_gift_ranking_by_url OK (${(performance.now() - t0).toFixed(0)}ms)`);
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      logWarn("api", `← get_gift_ranking_by_url FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
    }
  };

  /** ライブポーリング API を呼び出してリアルタイム情報を更新する */
  const refreshPolling = async (targetLiveId?: string | Event, seq?: number) => {
    const id = (typeof targetLiveId === "string" ? targetLiveId : liveId).trim();
    if (!id) return;
    if (pollingInFlight) return;
    pollingInFlight = true;
    const t0 = performance.now();
    log("api", `→ live_polling liveId=${id}`);
    try {
      const res = await invoke("live_polling", { liveId: id });
      if (typeof seq === "number" && connectSeq !== seq) return;
      polling = res;
      log("api", `← live_polling OK (${(performance.now() - t0).toFixed(0)}ms)`);
    } catch (e) {
      if (typeof seq === "number" && connectSeq !== seq) return;
      logWarn("api", `← live_polling FAIL (${(performance.now() - t0).toFixed(0)}ms)`, e);
    } finally {
      pollingInFlight = false;
    }
  };

  // giftRankingUrl が変化したら追加ランキングを取得する
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

  // ─────────────────────────────────────────────────────────────────────────
  // ライフサイクル
  // ─────────────────────────────────────────────────────────────────────────

  onDestroy(() => {
    log("join", "onDestroy: teardown");
    stopKeepAlive();
    void disconnectBroadcast();
    void stopRelay("onDestroy");
    void cleanupPlayer();
    if (playerSyncTimer) clearInterval(playerSyncTimer);
    playerSyncTimer = null;
    if (playerUnlisten) {
      playerUnlisten();
      playerUnlisten = null;
    }
    if (commentCooldownTimer) clearTimeout(commentCooldownTimer);
    commentCooldownTimer = null;
  });

  onMount(() => {
    log("join", "onMount: setting up listeners");
    void (async () => {
      // MPV 状態イベントリスナーを登録
      try {
        playerUnlisten = await listen<PlayerInfo>("mpv://state", (event) => {
          applyPlayerInfo(event.payload);
        });
        log("hls", "mpv://state listener ready");
      } catch {
        playerUnlisten = null;
      }
      // ブロードキャストログリスナーを登録（Rust 側の WS ログをフロントで確認できる）
      try {
        broadcastLogUnlisten = await listen<string>("broadcast://log", (event) => {
          log("ws", `[rust] ${event.payload}`);
        });
        log("ws", "broadcast://log listener ready");
      } catch {
        broadcastLogUnlisten = null;
      }
      // プレイヤー状態を初回同期してから定期ポーリングを開始
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
      if (broadcastLogUnlisten) {
        broadcastLogUnlisten();
        broadcastLogUnlisten = null;
      }
    };
  });
</script>

<section class="watch">
  <div class="watch-hero">
    <div class="hero-copy">
      <p class="kicker">Watch Studio</p>
      <h2>配信モニタリング</h2>
      <p class="hero-sub">再生状態、コメント、ランキングを1画面で追跡</p>
    </div>
    <div class="hero-metrics">
      <div class="metric-card">
        <span class="metric-label">状態</span>
        <strong class={`metric-value status-${watchStatus.tone}`}>{watchStatus.label}</strong>
      </div>
      <div class="metric-card">
        <span class="metric-label">配信ID</span>
        <strong class="metric-value mono">{activeLiveIdLabel}</strong>
      </div>
      <div class="metric-card">
        <span class="metric-label">視聴者</span>
        <strong class="metric-value">{viewerSummary}</strong>
      </div>
      <div class="metric-card">
        <span class="metric-label">コメント</span>
        <strong class="metric-value">{commentSummary}</strong>
      </div>
    </div>
  </div>

  <div class="watch-layout">
    <div class="watch-main">
      <WatchHeader onRefreshComments={refreshComments} onRefreshPolling={refreshPolling} />

      <WatchJoin
        liveId={liveId}
        loading={loading}
        viewMode={viewMode}
        streamSourceMode={streamSourceMode}
        {authed}
        onJoin={(event) => {
          event?.preventDefault();
          const id = liveId.trim();
          if (!id) return;
          resetForLive();
          void openPlayerWindow();
          if (viewMode === "silent") {
            void silentWatch(id);
          } else {
            joinLive(undefined, { silent: false });
          }
        }}
        onLiveIdChange={(value) => (liveId = value)}
        onViewModeChange={(value) => (viewMode = value)}
        onStreamSourceModeChange={(value) => (streamSourceMode = value)}
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

      <div class="panel comments-panel">
        <WatchCommentsPanel
          comments={comments}
          systemNotices={systemNotices}
          commentText={commentText}
          commentTotal={commentTotal}
          broadcastConnected={broadcastConnected}
          commentCoolingDown={commentCoolingDown}
          commentCooldownMs={commentCooldownMs}
          sendDisabled={viewMode === "silent" || !authed}
          onCommentChange={(value) => (commentText = value)}
          onSend={sendComment}
          onCooldownMsChange={(value) => (commentCooldownMs = value)}
        />
      </div>
    </div>

    <aside class="watch-side">
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
    </aside>
  </div>
</section>

<style>
  .watch {
    position: relative;
    display: grid;
    gap: 16px;
  }

  .watch-hero {
    position: relative;
    overflow: hidden;
    display: grid;
    gap: 14px;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 420px);
    align-items: end;
    border-radius: 24px;
    padding: 20px;
    color: #f6fbfa;
    background:
      radial-gradient(180px 140px at 8% -10%, rgba(242, 95, 76, 0.35), transparent 70%),
      radial-gradient(220px 140px at 95% 10%, rgba(61, 176, 162, 0.35), transparent 72%),
      linear-gradient(145deg, #0f3234 0%, #17494b 58%, #1a5456 100%);
    box-shadow: var(--shadow-soft);
    border: 1px solid rgba(255, 255, 255, 0.12);
  }

  .watch-hero::after {
    content: "";
    position: absolute;
    inset: 0;
    background:
      linear-gradient(100deg, rgba(255, 255, 255, 0.08), transparent 40%),
      linear-gradient(180deg, transparent 55%, rgba(9, 20, 22, 0.2));
    pointer-events: none;
  }

  .hero-copy {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 6px;
  }

  .kicker {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.26em;
    font-size: 0.67rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.75);
  }

  h2 {
    margin: 0;
    font-size: clamp(1.28rem, 2.1vw, 1.95rem);
    font-family: var(--font-display);
    letter-spacing: 0.02em;
  }

  .hero-sub {
    margin: 0;
    color: rgba(240, 249, 247, 0.84);
    font-size: 0.86rem;
  }

  .hero-metrics {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .metric-card {
    display: grid;
    gap: 5px;
    border-radius: 14px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.16);
    backdrop-filter: blur(4px);
  }

  .metric-label {
    font-size: 0.67rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(240, 252, 249, 0.72);
  }

  .metric-value {
    font-size: 0.95rem;
    font-weight: 700;
    color: #fff;
  }

  .metric-value.mono {
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    font-size: 0.82rem;
    word-break: break-all;
  }

  .status-idle {
    color: rgba(246, 251, 250, 0.85);
  }

  .status-connecting {
    color: #ffd166;
  }

  .status-live {
    color: #7df9c1;
  }

  .status-ready {
    color: #9ad7ff;
  }

  .status-error {
    color: #ffb3a9;
  }

  .watch-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.58fr) minmax(290px, 1fr);
    gap: 16px;
    align-items: start;
  }

  .watch-main {
    display: grid;
    gap: 12px;
    min-width: 0;
  }

  .watch-side {
    display: grid;
    gap: 12px;
    align-items: start;
    min-width: 0;
  }

  .panel {
    position: relative;
    overflow: hidden;
    padding: 16px 14px;
    border-radius: 20px;
    background:
      linear-gradient(180deg, rgba(255, 255, 255, 0.98) 0%, rgba(247, 251, 250, 0.95) 100%);
    box-shadow: var(--shadow-soft);
    border: 1px solid rgba(16, 27, 30, 0.08);
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .panel::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(120deg, rgba(242, 95, 76, 0.03), transparent 38%);
    pointer-events: none;
  }

  .comments-panel {
    min-height: 260px;
  }

  .error {
    margin: 0;
    border-radius: 12px;
    padding: 10px 12px;
    color: var(--accent-700);
    font-weight: 600;
    background: rgba(212, 72, 58, 0.12);
    border: 1px solid rgba(212, 72, 58, 0.28);
  }

  @media (max-width: 1100px) {
    .watch-layout {
      grid-template-columns: 1fr;
    }

    .watch-side {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 860px) {
    .watch-hero {
      grid-template-columns: 1fr;
    }

    .hero-metrics {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .watch-side {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .watch-hero {
      padding: 16px;
      border-radius: 18px;
    }

    .hero-metrics {
      grid-template-columns: 1fr;
    }

    .panel {
      border-radius: 16px;
    }
  }
</style>
