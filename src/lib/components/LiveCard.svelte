<script lang="ts">
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
  const owner = $derived(
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
      <img src={image} alt={title} loading="lazy" />
    {:else}
      <div class="thumb-placeholder">NO IMAGE</div>
    {/if}
    {#if isLive}
      <span class="badge">LIVE</span>
    {/if}
    {#if isCollab}
      <span class="badge collab">COLLAB</span>
    {/if}
  </div>
  <div class="meta">
    <h3>{title}</h3>
    <div class="meta-line">
      {#if appIcon}
        <img class="app-icon-inline" src={appIcon} alt="" loading="lazy" />
      {/if}
      <span class="owner">{owner}</span>
      <span class="dot">•</span>
      <span class="viewers">{formatNumber(viewers)} 人</span>
    </div>
    {#if tags.length > 0}
      <div class="tags">
        {#each tags.slice(0, 2) as tag}
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
    gap: 12px;
    padding: 14px;
    border-radius: 16px;
    background: var(--card-surface);
    color: inherit;
    text-align: left;
    cursor: pointer;
    box-shadow: var(--shadow-soft);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
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
    border-radius: 12px;
    overflow: hidden;
    background: linear-gradient(140deg, rgba(15, 42, 39, 0.9), rgba(24, 68, 62, 0.9));
  }

  .thumb img {
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
    top: 10px;
    left: 10px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--accent-500);
    color: #fff;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  .badge.collab {
    left: auto;
    right: 10px;
    background: rgba(15, 42, 39, 0.75);
  }

  .app-icon-inline {
    width: 18px;
    height: 18px;
    border-radius: 6px;
    border: 1px solid rgba(16, 27, 30, 0.12);
    background: #fff;
    object-fit: cover;
  }

  .meta {
    display: grid;
    gap: 6px;
  }

  .meta h3 {
    font-size: 1rem;
    margin: 0;
    font-family: var(--font-display);
  }

  .meta-line {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--ink-600);
  }

  .owner {
    font-weight: 600;
  }

  .dot {
    opacity: 0.5;
  }

  .tags {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .tag {
    font-size: 0.7rem;
    padding: 2px 8px;
    border-radius: 999px;
    background: rgba(16, 27, 30, 0.08);
    color: var(--ink-700);
    font-weight: 600;
  }
</style>
