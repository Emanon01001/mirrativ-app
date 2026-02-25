<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let { onOpenLive } = $props<{ onOpenLive: (live: any) => void }>();

  type FollowOwner = {
    name?: string;
    user_id?: string;
    profile_image_url?: string;
  };

  type FollowArchive = {
    live_id?: string;
    title?: string;
    image_url?: string;
    blur_image_url?: string;
    joined_live_thumbnail_image_url?: string;
    app_icon_urls?: string[];
    archive_status?: number;
    started_at?: number;
    ended_at?: number;
    total_viewer_num?: number;
    is_live?: boolean;
    owner?: FollowOwner;
    user_label_image_url?: string;
  };

  type FollowAnnouncement = {
    body?: string;
    created_at?: number;
    start_at?: number;
    app_icon_url?: string;
    owner?: FollowOwner;
    user_label_image_url?: string;
  };

  type FollowEntry = {
    type?: string;
    log_id?: string;
    archive?: FollowArchive;
    live_announcement?: FollowAnnouncement;
  };

  let entries = $state<FollowEntry[]>([]);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state("");
  let currentCursor = $state<string | null>(null);
  let nextCursor = $state<string | null>(null);

  const archiveCount = $derived.by(
    () => entries.filter((entry) => entry?.type === "archive").length
  );
  const announcementCount = $derived.by(
    () => entries.filter((entry) => entry?.type === "live_announcement").length
  );

  const toRecord = (value: unknown): Record<string, unknown> | null => {
    if (!value || typeof value !== "object") return null;
    return value as Record<string, unknown>;
  };

  const pickFirstString = (...values: unknown[]): string => {
    for (const value of values) {
      if (typeof value === "string" && value.trim()) return value;
    }
    return "";
  };

  const toUnixSeconds = (value: unknown): number | null => {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && value.trim()) {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
    return null;
  };

  const formatDateTime = (value: unknown): string => {
    const unix = toUnixSeconds(value);
    if (!unix || unix <= 0) return "-";
    return new Date(unix * 1000).toLocaleString("ja-JP", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit"
    });
  };

  const formatViewerCount = (value: unknown): string => {
    const num = toUnixSeconds(value);
    if (num === null) return "0";
    return Math.max(0, Math.floor(num)).toLocaleString("ja-JP");
  };

  const normalizeCursor = (value: unknown): string | null => {
    if (typeof value !== "string") return null;
    const trimmed = value.trim();
    return trimmed ? trimmed : null;
  };

  const extractFollowEntries = (res: unknown): FollowEntry[] => {
    const source = toRecord(res);
    if (!source) return [];

    const sourceData = toRecord(source.data);
    const list =
      (Array.isArray(source.list) ? source.list : null) ??
      (Array.isArray(sourceData?.list) ? sourceData.list : null) ??
      (Array.isArray(source.lives) ? source.lives : null) ??
      (Array.isArray(sourceData?.lives) ? sourceData.lives : null) ??
      [];

    return Array.isArray(list) ? (list as FollowEntry[]) : [];
  };

  const getEntryKey = (entry: FollowEntry, index = 0): string => {
    const logId = pickFirstString(entry?.log_id);
    if (logId) return `log:${logId}`;

    if (entry?.type === "archive") {
      const liveId = pickFirstString(entry.archive?.live_id);
      const endedAt = toUnixSeconds(entry.archive?.ended_at) ?? 0;
      return `archive:${liveId}:${endedAt}:${index}`;
    }

    if (entry?.type === "live_announcement") {
      const userId = pickFirstString(entry.live_announcement?.owner?.user_id);
      const createdAt = toUnixSeconds(entry.live_announcement?.created_at) ?? 0;
      return `announcement:${userId}:${createdAt}:${index}`;
    }

    return `unknown:${index}`;
  };

  const dedupeEntries = (items: FollowEntry[]): FollowEntry[] => {
    const seen = new Set<string>();
    const unique: FollowEntry[] = [];
    items.forEach((entry, index) => {
      const key = getEntryKey(entry, index);
      if (seen.has(key)) return;
      seen.add(key);
      unique.push(entry);
    });
    return unique;
  };

  const mergeEntries = (base: FollowEntry[], incoming: FollowEntry[]): FollowEntry[] => {
    const merged = [...base];
    const seen = new Set(base.map((entry, index) => getEntryKey(entry, index)));
    incoming.forEach((entry, index) => {
      const key = getEntryKey(entry, index);
      if (seen.has(key)) return;
      seen.add(key);
      merged.push(entry);
    });
    return merged;
  };

  const applyResponse = (res: unknown, replace: boolean) => {
    const source = toRecord(res);
    const list = dedupeEntries(extractFollowEntries(source));
    entries = replace ? list : mergeEntries(entries, list);

    const current = normalizeCursor(source?.current_cursor ?? source?.currentCursor);
    const next = normalizeCursor(source?.next_cursor ?? source?.nextCursor);
    if (replace) {
      currentCursor = current;
    } else if (current !== null) {
      currentCursor = current;
    }
    nextCursor = next;
  };

  const fetchFollow = async (cursor: string | null, replace: boolean) => {
    if (replace) {
      loading = true;
      entries = [];
      currentCursor = null;
      nextCursor = null;
    } else {
      loadingMore = true;
    }

    error = "";
    try {
      const res = await invoke<unknown>("get_catalog_follow", { cursor });
      applyResponse(res, replace);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
      loadingMore = false;
    }
  };

  const loadFollow = async () => {
    await fetchFollow(null, true);
  };

  const loadMore = async () => {
    if (!nextCursor || loading || loadingMore) return;
    await fetchFollow(nextCursor, false);
  };

  const getArchiveImage = (archive: FollowArchive | undefined): string =>
    pickFirstString(
      archive?.image_url,
      archive?.joined_live_thumbnail_image_url,
      archive?.blur_image_url
    );

  const getOwnerName = (owner: FollowOwner | undefined): string =>
    pickFirstString(owner?.name) || "ユーザー未設定";

  const getAnnouncementBody = (announcement: FollowAnnouncement | undefined): string => {
    const text = pickFirstString(announcement?.body);
    return text || "配信予告が投稿されました";
  };

  const getArchiveStatusLabel = (value: unknown): string => {
    const status = toUnixSeconds(value);
    if (status === 1) return "配信ログ";
    if (status === 2) return "アーカイブ";
    if (status !== null) return `status:${status}`;
    return "status:-";
  };

  const openArchive = (archive: FollowArchive | undefined) => {
    const liveId = pickFirstString(archive?.live_id);
    if (!liveId) return;
    onOpenLive(archive);
  };

  onMount(() => {
    void loadFollow();
  });
</script>

<section class="follow">
  <header class="section-head">
    <div>
      <p class="kicker">Following</p>
      <h2>フォロータイムライン</h2>
      <div class="summary">
        <span class="pill">全 {entries.length}</span>
        <span class="pill">アーカイブ {archiveCount}</span>
        <span class="pill">予告 {announcementCount}</span>
      </div>
    </div>
    <button class="ghost" onclick={loadFollow} disabled={loading}>
      {loading ? "更新中..." : "更新"}
    </button>
  </header>

  {#if error}
    <ErrorMessage
      message={error}
      onRetry={loadFollow}
      onDismiss={() => (error = "")}
    />
  {/if}

  <div class="timeline">
    {#if loading}
      {#each Array(6) as _}
        <Skeleton variant="card" />
      {/each}
    {:else if entries.length === 0}
      <div class="empty">フォロー情報がありません</div>
    {:else}
      {#each entries as entry, index (getEntryKey(entry, index))}
        {#if entry?.type === "archive" && entry.archive}
          <article class="entry archive-entry">
            <div class="entry-head">
              <span class="chip archive-chip">ARCHIVE</span>
              <span class="entry-time">終了: {formatDateTime(entry.archive.ended_at)}</span>
            </div>

            <div class="archive-layout">
              <button
                class="thumb-button"
                type="button"
                onclick={() => openArchive(entry.archive)}
                disabled={!entry.archive?.live_id}
                aria-label="配信を開く"
              >
                {#if getArchiveImage(entry.archive)}
                  <img
                    class="thumb"
                    src={getArchiveImage(entry.archive)}
                    alt={entry.archive.title ?? "archive thumbnail"}
                    loading="lazy"
                  />
                {:else}
                  <div class="thumb-empty">NO IMAGE</div>
                {/if}
                {#if entry.archive.is_live}
                  <span class="live-badge">LIVE</span>
                {:else}
                  <span class="status-badge">{getArchiveStatusLabel(entry.archive.archive_status)}</span>
                {/if}
              </button>

              <div class="entry-body">
                <h3>{entry.archive.title ?? "タイトル未設定"}</h3>
                <p class="owner">
                  {getOwnerName(entry.archive.owner)} ・ {formatViewerCount(entry.archive.total_viewer_num)} 人
                </p>
                <p class="meta-line">
                  開始: {formatDateTime(entry.archive.started_at)}
                </p>
                <div class="icon-row">
                  {#each entry.archive.app_icon_urls ?? [] as icon, appIndex (icon)}
                    {#if appIndex < 3}
                      <img class="app-icon" src={icon} alt="" loading="lazy" />
                    {/if}
                  {/each}
                  {#if entry.archive.user_label_image_url}
                    <img
                      class="ribbon"
                      src={entry.archive.user_label_image_url}
                      alt=""
                      loading="lazy"
                    />
                  {/if}
                </div>
                {#if entry.archive.live_id}
                  <button class="ghost open-btn" type="button" onclick={() => openArchive(entry.archive)}>
                    視聴ページを開く
                  </button>
                {/if}
              </div>
            </div>
          </article>
        {:else if entry?.type === "live_announcement" && entry.live_announcement}
          <article class="entry announcement-entry">
            <div class="entry-head">
              <span class="chip announce-chip">ANNOUNCEMENT</span>
              <span class="entry-time">投稿: {formatDateTime(entry.live_announcement.created_at)}</span>
            </div>

            <div class="entry-body">
              <div class="announce-owner">
                {#if entry.live_announcement.owner?.profile_image_url}
                  <img
                    class="avatar"
                    src={entry.live_announcement.owner.profile_image_url}
                    alt={getOwnerName(entry.live_announcement.owner)}
                    loading="lazy"
                  />
                {:else}
                  <div class="avatar avatar-fallback"></div>
                {/if}
                <div>
                  <p class="owner">{getOwnerName(entry.live_announcement.owner)}</p>
                  <p class="meta-line">@{entry.live_announcement.owner?.user_id ?? "-"}</p>
                </div>
                {#if entry.live_announcement.app_icon_url}
                  <img class="app-icon" src={entry.live_announcement.app_icon_url} alt="" loading="lazy" />
                {/if}
              </div>

              <p class="announce-text">{getAnnouncementBody(entry.live_announcement)}</p>

              <div class="announce-meta">
                <span>開始予定: {entry.live_announcement.start_at ? formatDateTime(entry.live_announcement.start_at) : "未定"}</span>
                {#if entry.live_announcement.user_label_image_url}
                  <img
                    class="ribbon"
                    src={entry.live_announcement.user_label_image_url}
                    alt=""
                    loading="lazy"
                  />
                {/if}
              </div>
            </div>
          </article>
        {:else}
          <article class="entry unknown-entry">
            <p>未対応のタイプ: {entry?.type ?? "unknown"}</p>
          </article>
        {/if}
      {/each}
    {/if}
  </div>

  <div class="load-more">
    {#if currentCursor}
      <span class="cursor">current_cursor: {currentCursor}</span>
    {/if}
    {#if nextCursor}
      <button class="ghost" onclick={loadMore} disabled={loading || loadingMore}>
        {loadingMore ? "読み込み中..." : "さらに読み込む"}
      </button>
      <span class="cursor">next_cursor: {nextCursor}</span>
    {:else if !loading && entries.length > 0}
      <span class="muted">これ以上の項目はありません</span>
    {/if}
  </div>
</section>

<style>
  .follow {
    display: grid;
    gap: 20px;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .kicker {
    text-transform: uppercase;
    letter-spacing: 0.2em;
    font-size: 0.7rem;
    color: var(--accent-500);
    margin: 0 0 6px 0;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(1.2rem, 2vw, 1.8rem);
  }

  .summary {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 10px;
  }

  .pill {
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 0.74rem;
    font-weight: 700;
    color: var(--ink-700);
    background: rgba(16, 27, 30, 0.08);
  }

  .ghost {
    border: 1px solid rgba(16, 27, 30, 0.2);
    background: transparent;
    color: var(--ink-700);
    border-radius: 999px;
    padding: 8px 16px;
    font-weight: 600;
    cursor: pointer;
  }

  .ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .timeline {
    display: grid;
    gap: 16px;
  }

  .entry {
    padding: 14px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.82);
    border: 1px solid rgba(16, 27, 30, 0.08);
    box-shadow: var(--shadow-soft);
    display: grid;
    gap: 10px;
  }

  .entry-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 0.78rem;
  }

  .chip {
    padding: 4px 8px;
    border-radius: 999px;
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  .archive-chip {
    color: #0f5d4f;
    background: rgba(41, 157, 132, 0.17);
  }

  .announce-chip {
    color: #8b3a20;
    background: rgba(242, 95, 76, 0.2);
  }

  .entry-time {
    color: var(--ink-500);
    font-variant-numeric: tabular-nums;
  }

  .archive-layout {
    display: grid;
    grid-template-columns: minmax(220px, 280px) minmax(0, 1fr);
    gap: 14px;
  }

  .thumb-button {
    position: relative;
    border: none;
    border-radius: 12px;
    padding: 0;
    overflow: hidden;
    cursor: pointer;
    background: rgba(16, 27, 30, 0.08);
    aspect-ratio: 16 / 9;
  }

  .thumb-button:disabled {
    cursor: default;
    opacity: 0.85;
  }

  .thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .thumb-empty {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    letter-spacing: 0.18em;
    font-size: 0.7rem;
    color: var(--ink-600);
  }

  .live-badge,
  .status-badge {
    position: absolute;
    left: 8px;
    top: 8px;
    font-size: 0.65rem;
    font-weight: 700;
    color: #fff;
    padding: 3px 8px;
    border-radius: 999px;
  }

  .live-badge {
    background: #d4483a;
  }

  .status-badge {
    background: rgba(16, 27, 30, 0.65);
  }

  .entry-body {
    display: grid;
    gap: 8px;
    min-width: 0;
  }

  h3 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .owner {
    margin: 0;
    color: var(--ink-700);
    font-weight: 600;
  }

  .meta-line {
    margin: 0;
    color: var(--ink-500);
    font-size: 0.82rem;
  }

  .icon-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .app-icon {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    object-fit: cover;
    background: #fff;
    border: 1px solid rgba(16, 27, 30, 0.1);
  }

  .ribbon {
    height: 18px;
    max-width: 180px;
    object-fit: contain;
  }

  .open-btn {
    justify-self: start;
    padding: 6px 12px;
    font-size: 0.8rem;
  }

  .announce-owner {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .avatar {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .avatar-fallback {
    background: rgba(16, 27, 30, 0.14);
  }

  .announce-text {
    margin: 0;
    line-height: 1.45;
    color: var(--ink-700);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .announce-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    color: var(--ink-500);
    font-size: 0.82rem;
  }

  .unknown-entry {
    color: var(--ink-600);
    font-size: 0.85rem;
  }

  .empty {
    padding: 20px;
    border-radius: 16px;
    background: rgba(16, 27, 30, 0.06);
    color: var(--ink-600);
  }

  .load-more {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .cursor {
    font-size: 0.75rem;
    color: var(--ink-500);
    word-break: break-all;
  }

  .muted {
    color: var(--ink-500);
  }

  @media (max-width: 860px) {
    .archive-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
