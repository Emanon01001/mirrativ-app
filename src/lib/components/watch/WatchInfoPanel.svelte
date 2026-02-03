<script lang="ts">
  import { formatUnix } from "$lib/components/watch/watch-utils";

  let { liveInfoView } = $props<{
    liveInfoView: {
      title: string;
      owner: string;
      viewers: number;
      commentNum: number;
      startedAt: number;
      appTitle: string;
      collabVacancy: number | null;
      status: string;
    } | null;
  }>();
</script>

<h3>配信情報</h3>
{#if liveInfoView}
  <div class="info">
    <p class="title">{liveInfoView.title || "タイトルなし"}</p>
    <p>配信者: {liveInfoView.owner || "不明"}</p>
    <p>視聴者数: {liveInfoView.viewers ?? 0}</p>
    <p>コメント数: {liveInfoView.commentNum ?? 0}</p>
    <p>開始時刻: {formatUnix(liveInfoView.startedAt || 0)}</p>
    <p>アプリ: {liveInfoView.appTitle || "不明"}</p>
    <p>
      コラボ枠:
      {liveInfoView.collabVacancy === null
        ? "不明"
        : liveInfoView.collabVacancy
          ? "空きあり"
          : "満員"}
    </p>
    <p>状態: {liveInfoView.status || "不明"}</p>
  </div>
{:else}
  <p class="muted">配信情報を取得してください</p>
{/if}

<style>
  h3 {
    margin: 0;
    font-size: 1rem;
  }

  .info {
    display: grid;
    gap: 6px;
  }

  .title {
    font-weight: 700;
  }

  .muted {
    color: var(--ink-500);
  }
</style>