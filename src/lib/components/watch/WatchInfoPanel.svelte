<script lang="ts">
  import { formatUnix } from "$lib/components/watch/watch-utils";
  import { onDestroy } from "svelte";

  let { liveInfoView } = $props<{
    liveInfoView: {
      title: string;
      owner: string;
      viewers: number;
      totalViewers: number;
      onlineViewers: number;
      commentNum: number;
      startedAt: number;
      appTitle: string;
      collabVacancy: number | null;
      status: string;
      isLive: boolean;
      starCount: number;
      giftCount: number;
      liveId: string;
    } | null;
  }>();

  let elapsed = $state("");
  let elapsedTimer: ReturnType<typeof setInterval> | null = null;

  const formatElapsed = (startedAt: number) => {
    if (!startedAt) return "";
    const now = Math.floor(Date.now() / 1000);
    const diff = now - startedAt;
    if (diff < 0) return "";
    const h = Math.floor(diff / 3600);
    const m = Math.floor((diff % 3600) / 60);
    const s = diff % 60;
    if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
    return `${m}:${String(s).padStart(2, "0")}`;
  };

  $effect(() => {
    if (elapsedTimer) clearInterval(elapsedTimer);
    const startedAt = liveInfoView?.startedAt ?? 0;
    if (startedAt && liveInfoView?.isLive) {
      elapsed = formatElapsed(startedAt);
      elapsedTimer = setInterval(() => {
        elapsed = formatElapsed(startedAt);
      }, 1000);
    } else {
      elapsed = "";
      elapsedTimer = null;
    }
  });

  onDestroy(() => {
    if (elapsedTimer) clearInterval(elapsedTimer);
  });
</script>

<h3>配信情報</h3>
{#if liveInfoView}
  <div class="info">
    <p class="title">{liveInfoView.title || "タイトルなし"}</p>
    <div class="info-grid">
      <span class="label">配信者</span>
      <span class="value">{liveInfoView.owner || "不明"}</span>

      <span class="label">状態</span>
      <span class="value">
        {#if !liveInfoView.isLive}
          <span class="live-badge">LIVE</span>
          {#if elapsed}
            <span class="elapsed">{elapsed}</span>
          {/if}
        {:else}
          <span class="ended-badge">終了</span>
        {/if}
      </span>

      <span class="label">視聴者</span>
      <span>
        {#if liveInfoView.onlineViewers > 0}
          {liveInfoView.onlineViewers.toLocaleString()} 人
          {#if liveInfoView.totalViewers > 0 && liveInfoView.totalViewers !== liveInfoView.onlineViewers}
            <span class="sub-info">/ 累計 {liveInfoView.totalViewers.toLocaleString()}</span>
          {/if}
        {:else if liveInfoView.totalViewers > 0}
          {liveInfoView.totalViewers.toLocaleString()} 人
        {:else}
          0
        {/if}
      </span>

      <span class="label">コメント</span>
      <span class="value">{(liveInfoView.commentNum ?? 0).toLocaleString()} 件</span>

      {#if liveInfoView.starCount > 0}
        <span class="label">スター</span>
        <span class="value">{liveInfoView.starCount.toLocaleString()}</span>
      {/if}

      {#if liveInfoView.giftCount > 0}
        <span class="label">ギフト</span>
        <span class="value">{liveInfoView.giftCount.toLocaleString()}</span>
      {/if}

      <span class="label">開始</span>
      <span class="value">{formatUnix(liveInfoView.startedAt || 0)}</span>

      <span class="label">アプリ</span>
      <span class="value">{liveInfoView.appTitle || "不明"}</span>

      <span class="label">コラボ枠</span>
      <span class="value">
        {liveInfoView.collabVacancy === null
          ? "不明"
          : liveInfoView.collabVacancy
            ? "空きあり"
            : "満員"}
      </span>
    </div>
  </div>
{:else}
  <p class="muted">配信情報を取得してください</p>
{/if}

<style>
  h3 {
    margin: 0;
    font-size: 0.98rem;
    letter-spacing: 0.02em;
  }

  .info {
    display: grid;
    gap: 12px;
  }

  .title {
    font-family: var(--font-display);
    font-size: 1.02rem;
    font-weight: 700;
    margin: 0;
  }

  .info-grid {
    display: grid;
    grid-template-columns: 86px minmax(0, 1fr);
    gap: 8px 10px;
    font-size: 0.85rem;
    align-items: baseline;
  }

  .label {
    color: rgba(16, 27, 30, 0.58);
    font-size: 0.75rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .value {
    min-width: 0;
    font-weight: 600;
  }

  .live-badge {
    display: inline-block;
    font-size: 0.62rem;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 999px;
    background: #e53e3e;
    color: #fff;
    letter-spacing: 0.06em;
    vertical-align: middle;
  }

  .ended-badge {
    display: inline-block;
    font-size: 0.62rem;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(16, 27, 30, 0.12);
    color: rgba(16, 27, 30, 0.6);
    letter-spacing: 0.06em;
    vertical-align: middle;
  }

  .elapsed {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    margin-left: 8px;
    color: var(--accent-700);
  }

  .sub-info {
    color: rgba(16, 27, 30, 0.5);
    font-size: 0.78rem;
  }

  .muted {
    color: var(--ink-500);
  }

  @media (max-width: 640px) {
    .info-grid {
      grid-template-columns: 76px minmax(0, 1fr);
      font-size: 0.82rem;
    }
  }
</style>
