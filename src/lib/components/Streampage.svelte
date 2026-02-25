<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import WatchCommentsPanel from "$lib/components/watch/WatchCommentsPanel.svelte";
  import {
    extractBroadcastConfig,
    toBroadcastComment,
    toBroadcastSystemNotice,
  } from "$lib/components/watch/watch-broadcast";

  // ── Props ──────────────────────────────────────────────────────────────────
  let { authed = false } = $props<{ authed?: boolean }>();

  // ── 配信状態 ────────────────────────────────────────────────────────────────
  // idle → creating → ready（接続情報確認）→ live（WS接続・コメント受送信）→ ending → idle
  type StreamPhase = "idle" | "creating" | "ready" | "live" | "ending";
  let phase = $state<StreamPhase>("idle");
  let error = $state("");
  let liveData = $state<any>(null);

  // フォーム
  let title = $state("");
  let description = $state("");

  // アプリ選択
  let appSearchQuery = $state("");
  let appSearchResults = $state<any[]>([]);
  let appSearching = $state(false);
  let selectedApp = $state<any>(null);
  let appSearchTimer: ReturnType<typeof setTimeout> | null = null;

  // サムネイル
  let thumbnailFile = $state<File | null>(null);
  let thumbnailPreview = $state<string | null>(null);
  let uploadingThumbnail = $state(false);

  // コメント
  let comments = $state<any[]>([]);
  let systemNotices = $state<any[]>([]);
  let commentText = $state("");
  let commentTotal = $state<number | null>(null);
  let commentCoolingDown = $state(false);
  let commentCooldownMs = $state(500);
  let commentCooldownTimer: ReturnType<typeof setTimeout> | null = null;

  // ブロードキャスト WS
  let broadcastConnected = $state(false);
  let broadcastSubscribed = $state(false);
  let broadcastUnlisten: (() => void) | null = null;
  let broadcastStatusUnlisten: (() => void) | null = null;

  // ポーリング
  let pollingTimer: ReturnType<typeof setInterval> | null = null;

  // キー再発行
  let renewingKey = $state(false);

  const COMMENT_BUFFER_LIMIT = 300;
  const SYSTEM_NOTICE_LIMIT = 30;

  // ── 導出値 ──────────────────────────────────────────────────────────────────
  const liveId = $derived(liveData?.live_id ?? "");
  const streamingKey = $derived(liveData?.streaming_key ?? "");
  const rtmpUrl = $derived(liveData?.streaming_upload_url2 ?? "");
  const wsUploadUrl = $derived(liveData?.streaming_upload_url3 ?? "");
  const hlsUrl = $derived(liveData?.streaming_url_hls ?? "");
  const bcsvrKey = $derived(liveData?.bcsvr_key ?? "");
  const broadcastKey = $derived(liveData?.broadcast_key ?? "");
  const broadcastHost = $derived(liveData?.broadcast_host ?? "");
  const broadcastPort = $derived(liveData?.broadcast_port ?? "");
  const llstreamVideoUrl = $derived(liveData?.streaming_url_llstream_video ?? "");
  const llstreamAudioUrl = $derived(liveData?.streaming_url_llstream_audio ?? "");
  const ownerName = $derived(liveData?.owner?.name ?? "");

  // ── ストリーミング情報の補完取得 ──────────────────────────────────────────
  // live_create でストリーミングURLが空の場合、get_live_status / get_live_info で取得
  const fetchStreamingInfo = async (id: string) => {
    const STREAM_FIELDS = [
      "streaming_key", "streaming_upload_url2", "streaming_upload_url3",
      "streaming_url_hls", "streaming_url_llstream_video", "streaming_url_llstream_audio",
    ];
    const mergeFields = (src: any) => {
      if (!src || typeof src !== "object") return;
      for (const key of STREAM_FIELDS) {
        if (src[key] && !liveData?.[key]) {
          liveData = { ...liveData, [key]: src[key] };
        }
      }
    };

    // まず get_live_status（get_streaming_url）を試す
    try {
      const status = await invoke<any>("get_live_status", { liveId: id });
      mergeFields(status);
    } catch {
      // ignore
    }

    // まだ streaming_key が空なら get_live_info を試す
    if (!liveData?.streaming_key) {
      try {
        const info = await invoke<any>("get_live_info", { liveId: id });
        mergeFields(info);
      } catch {
        // ignore
      }
    }
  };

  // ── アプリ検索 ────────────────────────────────────────────────────────────────
  const handleAppSearch = (query: string) => {
    appSearchQuery = query;
    if (appSearchTimer) clearTimeout(appSearchTimer);
    if (!query.trim()) {
      appSearchResults = [];
      return;
    }
    appSearchTimer = setTimeout(async () => {
      appSearching = true;
      try {
        const res = await invoke<any>("get_app_search", { query: query.trim() });
        appSearchResults = res?.apps ?? [];
      } catch {
        appSearchResults = [];
      } finally {
        appSearching = false;
      }
    }, 300);
  };

  const selectApp = (app: any) => {
    selectedApp = app;
    appSearchQuery = "";
    appSearchResults = [];
  };

  const clearApp = () => {
    selectedApp = null;
  };

  // ── サムネイル ────────────────────────────────────────────────────────────────
  const handleThumbnailSelect = (e: Event) => {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    thumbnailFile = file;
    // プレビュー生成
    const reader = new FileReader();
    reader.onload = () => { thumbnailPreview = reader.result as string; };
    reader.readAsDataURL(file);
  };

  const clearThumbnail = () => {
    thumbnailFile = null;
    thumbnailPreview = null;
  };

  const uploadThumbnail = async (id: string) => {
    if (!thumbnailFile) return;
    uploadingThumbnail = true;
    try {
      const buf = await thumbnailFile.arrayBuffer();
      const data = Array.from(new Uint8Array(buf));
      await invoke("live_capture_image", {
        liveId: id,
        imageData: data,
        filename: thumbnailFile.name,
      });
    } catch {
      // サムネイルアップロード失敗は致命的でない
    } finally {
      uploadingThumbnail = false;
    }
  };

  // ── 配信作成 ────────────────────────────────────────────────────────────────
  const handleCreate = async () => {
    if (!authed) {
      error = "配信するにはログインが必要です";
      return;
    }
    error = "";
    phase = "creating";

    try {
      const res = await invoke<any>("live_create");

      if (res?.status?.ok !== 1 && res?.status?.error) {
        throw new Error(res.status.error || res.status.message || "配信の作成に失敗しました");
      }

      liveData = res;
      phase = "ready";

      // タイトル/説明/アプリを live_edit で設定（multipart/form-data）
      if (title || description || selectedApp) {
        try {
          const editRes = await invoke<any>("live_edit", {
            liveId: res.live_id,
            title: title || null,
            description: description || null,
            appId: selectedApp?.app_id ?? null,
          });
          // live_edit のレスポンスには最新の接続情報が含まれる
          if (editRes?.live_id) {
            liveData = editRes;
          }
        } catch {
          // 更新失敗しても配信は続行
        }
      }

      // サムネイル画像をアップロード
      if (thumbnailFile) {
        await uploadThumbnail(res.live_id);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      phase = "idle";
    }
  };

  // ── 配信開始（ready → live） ────────────────────────────────────────────────
  const handleGoLive = async () => {
    error = "";
    try {
      // live_start API で実際に配信を開始する
      const res = await invoke<any>("live_start", { liveId });
      if (res?.live_id) {
        liveData = res;
      }
      phase = "live";
      await connectBroadcast(liveData);
      startPolling();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  };

  // ── ストリーミングキー再発行 ────────────────────────────────────────────────
  const handleRenewKey = async () => {
    if (!liveId || renewingKey) return;
    renewingKey = true;
    error = "";

    try {
      const res = await invoke<any>("renew_streaming_key", { liveId });
      if (res?.status?.ok === 1 && res?.streaming_key) {
        liveData = { ...liveData, streaming_key: res.streaming_key };
        // キー再発行後にストリーミングURL等も再取得
        await fetchStreamingInfo(liveId);
      } else {
        throw new Error(res?.status?.error || "キーの再発行に失敗しました");
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      renewingKey = false;
    }
  };

  // ── 配信終了 ────────────────────────────────────────────────────────────────
  const handleEnd = async () => {
    if (!liveId) return;
    phase = "ending";
    error = "";

    try {
      await invoke("live_end", { liveId });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      await cleanup();
      phase = "idle";
    }
  };

  // ── ブロードキャスト WS ──────────────────────────────────────────────────────
  const connectBroadcast = async (info: any) => {
    const config = extractBroadcastConfig(info);
    if (!config) return;

    await disconnectBroadcast();

    try {
      broadcastUnlisten = await listen<any>("broadcast://message", (event) => {
        const msg = event.payload;
        const cm = toBroadcastComment(msg);
        if (cm) {
          comments = [cm, ...comments].slice(0, COMMENT_BUFFER_LIMIT);
          commentTotal = (commentTotal ?? 0) + 1;
          return;
        }
        const sn = toBroadcastSystemNotice(msg);
        if (sn) {
          systemNotices = [sn, ...systemNotices].slice(0, SYSTEM_NOTICE_LIMIT);
        }
      });

      broadcastStatusUnlisten = await listen<any>("broadcast://status", (event) => {
        const status = String(event.payload?.status ?? event.payload ?? "");
        if (status === "connected" || status === "subscribed") {
          broadcastConnected = true;
          if (status === "subscribed") broadcastSubscribed = true;
        } else if (status === "disconnected" || status === "error") {
          broadcastConnected = false;
          broadcastSubscribed = false;
        }
      });

      await invoke("connect_broadcast", {
        bcsvrKey: config.bcsvrKey,
        broadcastHost: config.host,
      });
    } catch {
      broadcastConnected = false;
    }
  };

  const disconnectBroadcast = async () => {
    broadcastConnected = false;
    broadcastSubscribed = false;
    if (broadcastUnlisten) { broadcastUnlisten(); broadcastUnlisten = null; }
    if (broadcastStatusUnlisten) { broadcastStatusUnlisten(); broadcastStatusUnlisten = null; }
    await invoke("disconnect_broadcast").catch(() => {});
  };

  // ── ポーリング ──────────────────────────────────────────────────────────────
  const startPolling = () => {
    stopPolling();
    pollingTimer = setInterval(async () => {
      if (!liveId) return;
      try {
        await invoke("live_heartbeat", { liveId });
      } catch {
        // ポーリング失敗は無視
      }
    }, 30_000);
  };

  const stopPolling = () => {
    if (pollingTimer) { clearInterval(pollingTimer); pollingTimer = null; }
  };

  // ── コメント送信 ────────────────────────────────────────────────────────────
  const handleSendComment = async () => {
    if (!commentText.trim() || !liveId || commentCoolingDown) return;
    const msg = commentText.trim();
    commentText = "";
    commentCoolingDown = true;

    try {
      await invoke("comment", { liveId, message: msg });
    } catch {
      // 送信失敗
    }

    commentCooldownTimer = setTimeout(() => {
      commentCoolingDown = false;
      commentCooldownTimer = null;
    }, commentCooldownMs);
  };

  // ── クリップボードコピー ────────────────────────────────────────────────────
  let copiedField = $state<string | null>(null);
  const copyToClipboard = async (text: string, field: string) => {
    try {
      await navigator.clipboard.writeText(text);
      copiedField = field;
      setTimeout(() => { copiedField = null; }, 2000);
    } catch {
      // フォールバック
    }
  };

  // ── クリーンアップ ──────────────────────────────────────────────────────────
  const cleanup = async () => {
    stopPolling();
    await disconnectBroadcast();
    if (commentCooldownTimer) { clearTimeout(commentCooldownTimer); commentCooldownTimer = null; }
    if (appSearchTimer) { clearTimeout(appSearchTimer); appSearchTimer = null; }
    comments = [];
    systemNotices = [];
    commentTotal = null;
    liveData = null;
    broadcastConnected = false;
    commentCoolingDown = false;
  };

  onDestroy(() => {
    void cleanup();
  });
</script>

<!-- ────────────────────────────────────────────────────────────────────────── -->
<!-- 接続情報スニペット（ready / live 共通） -->
<!-- ────────────────────────────────────────────────────────────────────────── -->
{#snippet streamInfoBlock()}
  <div class="stream-info">
    <h3>配信接続情報</h3>

    <div class="info-row">
      <span class="info-label">Live ID</span>
      <div class="info-value-row">
        <code class="info-value">{liveId}</code>
        <button class="btn-copy" onclick={() => copyToClipboard(liveId, "liveId")}>
          {copiedField === "liveId" ? "Copied" : "Copy"}
        </button>
      </div>
    </div>

    <div class="info-row">
      <span class="info-label">ストリーミングキー</span>
      <div class="info-value-row">
        <code class="info-value">{streamingKey}</code>
        <button class="btn-copy" onclick={() => copyToClipboard(streamingKey, "streamingKey")}>
          {copiedField === "streamingKey" ? "Copied" : "Copy"}
        </button>
        <button class="btn-renew" onclick={handleRenewKey} disabled={renewingKey}>
          {renewingKey ? "再発行中..." : "キー再発行"}
        </button>
      </div>
    </div>

    {#if rtmpUrl}
      <div class="info-row">
        <span class="info-label">RTMP URL</span>
        <div class="info-value-row">
          <code class="info-value url">{rtmpUrl}</code>
          <button class="btn-copy" onclick={() => copyToClipboard(rtmpUrl, "rtmpUrl")}>
            {copiedField === "rtmpUrl" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
    {/if}

    {#if wsUploadUrl}
      <div class="info-row">
        <span class="info-label">WebSocket Upload URL</span>
        <div class="info-value-row">
          <code class="info-value url">{wsUploadUrl}</code>
          <button class="btn-copy" onclick={() => copyToClipboard(wsUploadUrl, "wsUrl")}>
            {copiedField === "wsUrl" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
    {/if}

    {#if hlsUrl}
      <div class="info-row">
        <span class="info-label">HLS URL (視聴用)</span>
        <div class="info-value-row">
          <code class="info-value url">{hlsUrl}</code>
          <button class="btn-copy" onclick={() => copyToClipboard(hlsUrl, "hlsUrl")}>
            {copiedField === "hlsUrl" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
    {/if}

    {#if llstreamVideoUrl}
      <div class="info-row">
        <span class="info-label">LLStream Video URL</span>
        <div class="info-value-row">
          <code class="info-value url">{llstreamVideoUrl}</code>
          <button class="btn-copy" onclick={() => copyToClipboard(llstreamVideoUrl, "llVideo")}>
            {copiedField === "llVideo" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
    {/if}

    {#if llstreamAudioUrl}
      <div class="info-row">
        <span class="info-label">LLStream Audio URL</span>
        <div class="info-value-row">
          <code class="info-value url">{llstreamAudioUrl}</code>
          <button class="btn-copy" onclick={() => copyToClipboard(llstreamAudioUrl, "llAudio")}>
            {copiedField === "llAudio" ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
    {/if}

    <div class="info-row">
      <span class="info-label">Broadcast Key</span>
      <div class="info-value-row">
        <code class="info-value">{broadcastKey}</code>
        <button class="btn-copy" onclick={() => copyToClipboard(broadcastKey, "broadcastKey")}>
          {copiedField === "broadcastKey" ? "Copied" : "Copy"}
        </button>
      </div>
    </div>

    <div class="info-row">
      <span class="info-label">BCSVR Key</span>
      <div class="info-value-row">
        <code class="info-value">{bcsvrKey}</code>
        <button class="btn-copy" onclick={() => copyToClipboard(bcsvrKey, "bcsvrKey")}>
          {copiedField === "bcsvrKey" ? "Copied" : "Copy"}
        </button>
      </div>
    </div>

    <div class="info-row">
      <span class="info-label">Broadcast Host</span>
      <div class="info-value-row">
        <code class="info-value">{broadcastHost}:{broadcastPort}</code>
        <button class="btn-copy" onclick={() => copyToClipboard(`${broadcastHost}:${broadcastPort}`, "bcHost")}>
          {copiedField === "bcHost" ? "Copied" : "Copy"}
        </button>
      </div>
    </div>

    {#if liveData}
      <details class="raw-response">
        <summary>Raw Response (debug)</summary>
        <pre>{JSON.stringify(liveData, null, 2)}</pre>
      </details>
    {/if}
  </div>
{/snippet}

<!-- ────────────────────────────────────────────────────────────────────────── -->
<!-- メインUI -->
<!-- ────────────────────────────────────────────────────────────────────────── -->
<section class="streampage">
  {#if !authed}
    <!-- 未ログイン -->
    <div class="card center-card">
      <div class="icon-large">📡</div>
      <h2>配信機能</h2>
      <p class="muted">配信するにはログインが必要です。設定ページからログインしてください。</p>
    </div>

  {:else if phase === "idle" || phase === "creating"}
    <!-- 配信準備画面 -->
    <div class="card setup-card">
      <div class="card-header">
        <div class="icon-large">📡</div>
        <div>
          <h2>配信を開始する</h2>
          <p class="muted">タイトルと説明を設定して配信を作成できます</p>
        </div>
      </div>

      <div class="form-group">
        <label for="stream-title">タイトル</label>
        <input
          id="stream-title"
          type="text"
          placeholder="配信タイトルを入力"
          bind:value={title}
          disabled={phase === "creating"}
        />
      </div>

      <div class="form-group">
        <label for="stream-desc">説明</label>
        <textarea
          id="stream-desc"
          placeholder="配信の説明を入力（任意）"
          rows="3"
          bind:value={description}
          disabled={phase === "creating"}
        ></textarea>
      </div>

      <!-- 配信アプリ選択 -->
      <div class="form-group">
        <label>配信アプリ</label>
        {#if selectedApp}
          <div class="selected-app">
            <img class="app-icon" src={selectedApp.icon_url} alt="" />
            <span class="app-name">{selectedApp.name}</span>
            <button class="btn-clear" onclick={clearApp} disabled={phase === "creating"}>✕</button>
          </div>
        {:else}
          <div class="app-search-wrap">
            <input
              type="text"
              placeholder="アプリ名を検索..."
              value={appSearchQuery}
              oninput={(e) => handleAppSearch((e.target as HTMLInputElement).value)}
              disabled={phase === "creating"}
            />
            {#if appSearching}
              <span class="search-indicator">検索中...</span>
            {/if}
          </div>
          {#if appSearchResults.length > 0}
            <ul class="app-results">
              {#each appSearchResults as app}
                <li>
                  <button class="app-result-btn" onclick={() => selectApp(app)}>
                    <img class="app-icon" src={app.icon_url} alt="" />
                    <span>{app.name}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      </div>

      <!-- サムネイル画像 -->
      <div class="form-group">
        <label>サムネイル画像</label>
        {#if thumbnailPreview}
          <div class="thumbnail-preview-wrap">
            <img class="thumbnail-preview" src={thumbnailPreview} alt="サムネイル" />
            <button class="btn-clear" onclick={clearThumbnail} disabled={phase === "creating"}>✕</button>
          </div>
        {:else}
          <label class="file-drop" for="thumbnail-input">
            <span class="file-drop-text">画像を選択</span>
            <input
              id="thumbnail-input"
              type="file"
              accept="image/jpeg,image/png,image/webp"
              onchange={handleThumbnailSelect}
              disabled={phase === "creating"}
              hidden
            />
          </label>
        {/if}
      </div>

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <button
        class="btn-primary btn-large"
        onclick={handleCreate}
        disabled={phase === "creating"}
      >
        {#if phase === "creating"}
          {uploadingThumbnail ? "サムネイルをアップロード中..." : "配信を作成中..."}
        {:else}
          配信を作成
        {/if}
      </button>
    </div>

  {:else if phase === "ready"}
    <!-- 配信準備完了画面（接続情報を確認してから配信開始） -->
    <div class="card ready-card">
      <div class="ready-header">
        <div class="ready-badge">準備完了</div>
        <div>
          <h2>{title || "無題の配信"}</h2>
          <p class="muted">OBS等の配信ソフトにRTMP URLとストリーミングキーを設定してください</p>
        </div>
      </div>

      {@render streamInfoBlock()}

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="ready-actions">
        <button class="btn-primary btn-large" onclick={handleGoLive}>
          配信を開始（コメント接続）
        </button>
        <button class="btn-danger" onclick={handleEnd}>
          配信を終了
        </button>
      </div>
    </div>

  {:else if phase === "live" || phase === "ending"}
    <!-- 配信中画面 -->
    <div class="live-layout">
      <!-- 配信情報パネル -->
      <div class="card info-card">
        <div class="live-header">
          <div class="live-badge">LIVE</div>
          <h2>{title || "無題の配信"}</h2>
          {#if ownerName}
            <span class="owner-name">{ownerName}</span>
          {/if}
        </div>

        {@render streamInfoBlock()}

        {#if error}
          <p class="error">{error}</p>
        {/if}

        <button
          class="btn-danger btn-large"
          onclick={handleEnd}
          disabled={phase === "ending"}
        >
          {phase === "ending" ? "配信を終了中..." : "配信を終了"}
        </button>
      </div>

      <!-- コメントパネル -->
      <div class="card comment-card">
        <WatchCommentsPanel
          {comments}
          {systemNotices}
          {commentText}
          {commentTotal}
          {broadcastConnected}
          {commentCoolingDown}
          {commentCooldownMs}
          sendDisabled={false}
          onCommentChange={(v) => commentText = v}
          onSend={handleSendComment}
          onCooldownMsChange={(v) => commentCooldownMs = v}
        />
      </div>
    </div>
  {/if}
</section>

<style>
  .streampage {
    display: grid;
    gap: 16px;
    align-content: start;
  }

  .card {
    padding: 24px;
    border-radius: 20px;
    background: var(--panel-surface);
    box-shadow: var(--shadow-soft);
  }

  .center-card {
    text-align: center;
    max-width: 480px;
    margin: 40px auto;
    display: grid;
    gap: 8px;
    justify-items: center;
  }

  .setup-card {
    max-width: 600px;
    display: grid;
    gap: 18px;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .card-header h2 {
    margin: 0;
  }

  .icon-large {
    font-size: 2.4rem;
    line-height: 1;
  }

  h2 {
    margin: 0 0 4px 0;
    font-family: var(--font-display);
  }

  h3 {
    margin: 0 0 10px 0;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
  }

  .muted {
    margin: 0;
    color: var(--ink-500);
    font-size: 0.9rem;
  }

  .form-group {
    display: grid;
    gap: 6px;
  }

  .form-group label {
    font-weight: 600;
    font-size: 0.88rem;
    color: var(--ink-700);
  }

  .form-group input,
  .form-group textarea {
    border: 1px solid rgba(16, 27, 30, 0.18);
    border-radius: 12px;
    padding: 10px 14px;
    background: #fff;
    font-size: 0.92rem;
    resize: vertical;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    border-color: var(--accent-500);
    outline: none;
    box-shadow: 0 0 0 3px rgba(242, 95, 76, 0.12);
  }

  /* ── アプリ検索 ──────────────────────────── */

  .app-search-wrap {
    position: relative;
  }

  .search-indicator {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.78rem;
    color: var(--ink-500);
  }

  .app-results {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 180px;
    overflow-y: auto;
    border: 1px solid rgba(16, 27, 30, 0.12);
    border-radius: 10px;
    background: #fff;
  }

  .app-result-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 0.88rem;
    text-align: left;
    transition: background 0.12s;
  }

  .app-result-btn:hover {
    background: rgba(16, 27, 30, 0.04);
  }

  .app-icon {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .selected-app {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border: 1px solid rgba(16, 27, 30, 0.12);
    border-radius: 12px;
    background: rgba(16, 27, 30, 0.02);
  }

  .app-name {
    flex: 1;
    font-size: 0.9rem;
    font-weight: 500;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn-clear {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 50%;
    background: rgba(16, 27, 30, 0.08);
    cursor: pointer;
    font-size: 0.72rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s;
  }

  .btn-clear:hover:not(:disabled) {
    background: rgba(16, 27, 30, 0.16);
  }

  .btn-clear:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* ── サムネイル ──────────────────────────── */

  .file-drop {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    border: 2px dashed rgba(16, 27, 30, 0.15);
    border-radius: 12px;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }

  .file-drop:hover {
    border-color: var(--accent-500);
    background: rgba(242, 95, 76, 0.04);
  }

  .file-drop-text {
    font-size: 0.88rem;
    color: var(--ink-500);
    font-weight: 500;
  }

  .thumbnail-preview-wrap {
    position: relative;
    display: inline-block;
  }

  .thumbnail-preview {
    max-width: 100%;
    max-height: 160px;
    border-radius: 10px;
    object-fit: contain;
    border: 1px solid rgba(16, 27, 30, 0.1);
  }

  .thumbnail-preview-wrap .btn-clear {
    position: absolute;
    top: 6px;
    right: 6px;
    background: rgba(0, 0, 0, 0.5);
    color: #fff;
  }

  .thumbnail-preview-wrap .btn-clear:hover:not(:disabled) {
    background: rgba(0, 0, 0, 0.7);
  }

  .btn-primary {
    border: none;
    border-radius: 999px;
    padding: 12px 24px;
    font-weight: 700;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
    transition: transform 0.15s, box-shadow 0.15s;
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 8px 18px rgba(242, 95, 76, 0.25);
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-danger {
    border: none;
    border-radius: 999px;
    padding: 12px 24px;
    font-weight: 700;
    cursor: pointer;
    background: #e53e3e;
    color: #fff;
    transition: transform 0.15s, box-shadow 0.15s;
  }

  .btn-danger:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 8px 18px rgba(229, 62, 62, 0.25);
  }

  .btn-danger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-large {
    font-size: 1rem;
    padding: 14px 28px;
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
    font-size: 0.9rem;
  }

  /* ── 準備完了画面 ──────────────────────────── */

  .ready-card {
    display: grid;
    gap: 20px;
  }

  .ready-header {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .ready-header h2 {
    margin: 0;
  }

  .ready-badge {
    display: inline-flex;
    align-items: center;
    padding: 6px 14px;
    border-radius: 8px;
    background: rgba(56, 161, 105, 0.15);
    color: #276749;
    font-size: 0.8rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    white-space: nowrap;
  }

  .ready-actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    align-items: center;
  }

  /* ── 配信中レイアウト ──────────────────────────── */

  .live-layout {
    display: grid;
    grid-template-columns: 1fr 380px;
    gap: 16px;
    align-items: start;
  }

  .info-card {
    display: grid;
    gap: 18px;
  }

  .live-header {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .live-badge {
    display: inline-flex;
    align-items: center;
    padding: 4px 12px;
    border-radius: 6px;
    background: #e53e3e;
    color: #fff;
    font-size: 0.75rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    animation: pulse-badge 2s ease-in-out infinite;
  }

  @keyframes pulse-badge {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }

  .owner-name {
    color: var(--ink-500);
    font-size: 0.88rem;
  }

  /* ── 接続情報共通 ──────────────────────────── */

  .stream-info {
    padding: 16px;
    border-radius: 14px;
    background: rgba(16, 27, 30, 0.03);
    border: 1px solid rgba(16, 27, 30, 0.08);
    display: grid;
    gap: 12px;
  }

  .info-row {
    display: grid;
    gap: 4px;
  }

  .info-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--ink-500);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .info-value-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .info-value {
    flex: 1;
    font-size: 0.82rem;
    padding: 6px 10px;
    border-radius: 8px;
    background: rgba(16, 27, 30, 0.05);
    border: 1px solid rgba(16, 27, 30, 0.1);
    word-break: break-all;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .info-value.url {
    font-size: 0.72rem;
    max-height: 48px;
    overflow: hidden;
  }

  .btn-copy {
    flex-shrink: 0;
    border: 1px solid rgba(16, 27, 30, 0.15);
    border-radius: 8px;
    padding: 5px 10px;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    background: #fff;
    color: var(--ink-700);
    transition: background 0.15s;
  }

  .btn-copy:hover {
    background: rgba(16, 27, 30, 0.05);
  }

  .btn-renew {
    flex-shrink: 0;
    border: 1px solid rgba(221, 107, 32, 0.4);
    border-radius: 8px;
    padding: 5px 10px;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    background: rgba(221, 107, 32, 0.08);
    color: #b7511a;
    transition: background 0.15s;
  }

  .btn-renew:hover:not(:disabled) {
    background: rgba(221, 107, 32, 0.16);
  }

  .btn-renew:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .comment-card {
    max-height: calc(100vh - 160px);
    overflow-y: auto;
  }

  .raw-response {
    margin-top: 4px;
  }

  .raw-response summary {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--ink-500);
    cursor: pointer;
  }

  .raw-response pre {
    margin: 8px 0 0;
    padding: 12px;
    border-radius: 10px;
    background: rgba(16, 27, 30, 0.05);
    border: 1px solid rgba(16, 27, 30, 0.1);
    font-size: 0.72rem;
    line-height: 1.5;
    max-height: 400px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }

  @media (max-width: 900px) {
    .live-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
