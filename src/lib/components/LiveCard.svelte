<script lang="ts">
  import { onDestroy } from "svelte";

  let { live, onSelect } = $props<{
    live: any;
    onSelect?: (live: any) => void;
  }>();

  const pickFirstString = (...values: Array<unknown>) => {
    for (const value of values) {
      if (typeof value === "string" && value.trim()) return value;
    }
    return "";
  };

  const title = $derived(
    live?.title ?? live?.name ?? live?.live?.title ?? "タイトル未設定"
  );
  const ownerName = $derived(
    live?.owner?.name ?? live?.user?.name ?? live?.live?.owner?.name ?? "配信者未設定"
  );
  const image = $derived(
    pickFirstString(
      live?.thumbnail_image_url,
      live?.live?.thumbnail_image_url,
      live?.thumbnail_blur_image_url,
      live?.live?.thumbnail_blur_image_url,
      live?.joined_live_thumbnail_image_url,
      live?.live?.joined_live_thumbnail_image_url,
      live?.blur_image_url,
      live?.live?.blur_image_url,
      live?.preview_blur_image_url,
      live?.live?.preview_blur_image_url,
      live?.share_image_url,
      live?.live?.share_image_url,
      live?.image_url_without_letterbox,
      live?.live?.image_url_without_letterbox,
      live?.live_thumbnail_image_url,
      live?.live?.live_thumbnail_image_url,
      live?.thumbnail_url,
      live?.live?.thumbnail_url,
      live?.image_url,
      live?.live?.image_url,
      live?.banner_image_url,
      live?.live?.banner_image_url,
      live?.preview_image_url,
      live?.live?.preview_image_url
    )
  );
  const appIcon = $derived(
    pickFirstString(live?.app_icon_urls?.[0], live?.app_icon_url, live?.app?.icon_url)
  );
  const profileImage = $derived(
    pickFirstString(
      live?.owner?.profile_image_url,
      live?.user?.profile_image_url,
      live?.live?.owner?.profile_image_url
    )
  );
  const profileFrame = $derived(
    pickFirstString(
      live?.owner?.profile_frame_image_url,
      live?.user?.profile_frame_image_url,
      live?.live?.owner?.profile_frame_image_url
    )
  );
  const seasonIcon = $derived(
    pickFirstString(
      live?.owner?.season_rating?.icon_url,
      live?.user?.season_rating?.icon_url,
      live?.live?.owner?.season_rating?.icon_url
    )
  );
  const viewers = $derived(
    live?.total_viewer_num ??
      live?.current_viewer_num ??
      live?.viewer_num ??
      live?.live?.total_viewer_num ??
      0
  );
  const isLive = $derived(Boolean(live?.is_live ?? live?.live?.is_live ?? false));
  const isCollab = $derived(Boolean(live?.is_collab ?? live?.live?.is_collab ?? false));
  const tags = $derived(Array.isArray(live?.live_tags) ? live.live_tags : []);

  const startedAt = $derived(
    Number(live?.started_at ?? live?.live?.started_at ?? 0)
  );

  let now = $state(Math.floor(Date.now() / 1000));
  const timer = setInterval(() => {
    now = Math.floor(Date.now() / 1000);
  }, 1000);
  onDestroy(() => clearInterval(timer));

  const elapsedSeconds = $derived(
    startedAt > 0 && isLive ? Math.max(0, now - startedAt) : 0
  );

  const formatDuration = (totalSeconds: number): string => {
    if (totalSeconds <= 0) return "";
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    const s = totalSeconds % 60;
    const pad = (n: number) => n.toString().padStart(2, "0");
    if (h > 0) return `${h}:${pad(m)}:${pad(s)}`;
    return `${m}:${pad(s)}`;
  };

  const durationText = $derived(formatDuration(elapsedSeconds));

  const formatNumber = (value: any) => {
    if (typeof value === "number") return value.toLocaleString();
    if (typeof value === "string" && value.trim()) return value;
    return "0";
  };

  const handleSelect = () => {
    if (onSelect) onSelect(live);
  };
</script>

<button type="button" class="live-card" onclick={handleSelect}>
  <div class="thumb">
    {#if image}
      <img class="thumb-image" src={image} alt={title} loading="lazy" />
    {:else}
      <div class="thumb-placeholder">NO IMAGE</div>
    {/if}
    {#if isLive}
      <span class="badge">LIVE</span>
    {/if}
    {#if isCollab}
      <span class="badge collab">COLLAB</span>
    {/if}
    {#if durationText}
      <span class="duration">{durationText}</span>
    {/if}
    {#if appIcon}
      <img class="app-icon-corner" src={appIcon} alt="" loading="lazy" />
    {/if}
  </div>

  <div class="meta">
    <div class="meta-row">
      <div class="avatar-wrap">
        {#if profileImage}
          <img class="avatar" src={profileImage} alt={ownerName} loading="lazy" />
        {:else}
          <div class="avatar avatar-fallback"></div>
        {/if}
        {#if profileFrame}
          <img class="avatar-frame" src={profileFrame} alt="" loading="lazy" />
        {/if}
        {#if seasonIcon}
          <img class="season-icon" src={seasonIcon} alt="" loading="lazy" />
        {/if}
      </div>

      <div class="meta-text">
        <h3>{title}</h3>
        <div class="meta-line">
          <span class="owner">{ownerName}</span>
          <span class="dot">·</span>
          <span class="viewers">{formatNumber(viewers)} 人</span>
        </div>
      </div>
    </div>

    {#if tags.length > 0}
      <div class="tags">
        {#each tags.slice(0, 3) as tag}
          <span class="tag">{tag?.tag_text ?? tag?.text ?? "タグ"}</span>
        {/each}
      </div>
    {/if}
  </div>
</button>

<style>
  .live-card {
    border: none;
    display: grid;
    gap: 0;
    padding: 0;
    border-radius: 14px;
    background: var(--card-surface);
    color: inherit;
    text-align: left;
    cursor: pointer;
    box-shadow: var(--shadow-soft);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
    overflow: hidden;
  }

  .live-card:focus-visible {
    outline: 2px solid var(--accent-500);
    outline-offset: 2px;
  }

  .live-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-bold);
  }

  .thumb {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    background: linear-gradient(140deg, rgba(15, 42, 39, 0.9), rgba(24, 68, 62, 0.9));
  }

  .thumb-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .thumb-placeholder {
    color: rgba(255, 255, 255, 0.7);
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    font-size: 0.7rem;
    display: grid;
    place-items: center;
    height: 100%;
  }

  .badge {
    position: absolute;
    top: 8px;
    left: 8px;
    padding: 3px 8px;
    border-radius: 4px;
    background: #e53935;
    color: #fff;
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    line-height: 1;
  }

  .badge.collab {
    left: auto;
    right: 8px;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
  }

  .duration {
    position: absolute;
    bottom: 6px;
    right: 6px;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.78);
    color: #fff;
    font-size: 0.7rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.03em;
    line-height: 1.3;
  }

  .app-icon-corner {
    position: absolute;
    bottom: 6px;
    left: 6px;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    background: #fff;
    object-fit: cover;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  /* --- Avatar with frame overlay --- */
  .avatar-wrap {
    position: relative;
    width: 40px;
    height: 40px;
    flex-shrink: 0;
  }

  .avatar {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    display: block;
  }

  .avatar-fallback {
    background: linear-gradient(135deg, #3a7c6e, #2c5f54);
  }

  .avatar-frame {
    position: absolute;
    top: -4px;
    left: -4px;
    width: calc(100% + 8px);
    height: calc(100% + 8px);
    object-fit: contain;
    pointer-events: none;
  }

  .season-icon {
    position: absolute;
    bottom: -2px;
    right: -4px;
    width: 18px;
    height: 18px;
    object-fit: contain;
    pointer-events: none;
    filter: drop-shadow(0 1px 2px rgba(0,0,0,0.3));
  }

  /* --- Meta section --- */
  .meta {
    display: grid;
    gap: 8px;
    padding: 10px 12px 12px;
  }

  .meta-row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }

  .meta-text {
    min-width: 0;
    display: grid;
    gap: 3px;
  }

  .meta h3 {
    font-size: 0.88rem;
    margin: 0;
    font-family: var(--font-display);
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .meta-line {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.78rem;
    color: var(--ink-600);
  }

  .owner {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dot {
    opacity: 0.4;
    flex-shrink: 0;
  }

  .viewers {
    white-space: nowrap;
    flex-shrink: 0;
  }

  .tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    padding-left: 50px;
  }

  .tag {
    font-size: 0.65rem;
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(16, 27, 30, 0.07);
    color: var(--ink-700);
    font-weight: 600;
  }
</style>
