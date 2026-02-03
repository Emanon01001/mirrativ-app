<script lang="ts">
  let { user, onOpenLive, onSelect } = $props<{
    user: any;
    onOpenLive?: (live: any) => void;
    onSelect?: (user: any) => void;
  }>();

  const pickFirstString = (...values: Array<unknown>) => {
    for (const value of values) {
      if (typeof value === "string" && value.trim()) return value;
    }
    return "";
  };

  const name = $derived(
    pickFirstString(
      user?.name,
      user?.user?.name,
      user?.username,
      user?.screen_name,
      "ユーザー"
    )
  );
  const userId = $derived(
    pickFirstString(user?.user_id, user?.id, user?.user?.user_id)
  );
  const avatar = $derived(
    pickFirstString(
      user?.profile_image_url,
      user?.user?.profile_image_url,
      user?.avatar_image_url,
      user?.image_url
    )
  );
  const description = $derived(
    pickFirstString(user?.description, user?.user?.description, user?.bio)
  );
  const liveId = $derived(
    pickFirstString(
      user?.onlive?.live_id,
      user?.onlive?.id,
      user?.live?.live_id,
      user?.live_id
    )
  );
  const isLive = $derived(Boolean(liveId));

  const handleOpen = () => {
    if (!isLive || !onOpenLive) return;
    onOpenLive({ live_id: liveId, owner: user });
  };

  const handleSelect = () => {
    if (!onSelect) return;
    onSelect(user);
  };
</script>

<div class="user-card" role="button" tabindex="0"
     onclick={handleSelect}
     onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSelect(); } }}>
  <div class="avatar">
    {#if avatar}
      <img src={avatar} alt={name} loading="lazy" />
    {:else}
      <span>U</span>
    {/if}
    {#if isLive}
      <span class="badge">LIVE</span>
    {/if}
  </div>
  <div class="meta">
    <div class="name-row">
      <h3>{name}</h3>
      {#if userId}
        <span class="user-id">@{userId}</span>
      {/if}
    </div>
    {#if description}
      <p class="desc">{description}</p>
    {/if}
    {#if isLive}
      <button class="ghost" onclick={(e) => { e.stopPropagation(); handleOpen(); }}>
        視聴する
      </button>
    {/if}
  </div>
</div>

<style>
  .user-card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 12px;
    padding: 14px;
    border-radius: 16px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
    min-width: 0;
    cursor: pointer;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  .user-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow-bold);
  }

  .user-card:focus-visible {
    outline: 2px solid var(--accent-500);
    outline-offset: 2px;
  }

  .avatar {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: rgba(16, 27, 30, 0.08);
    overflow: hidden;
    display: grid;
    place-items: center;
    font-weight: 700;
    color: var(--ink-600);
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .badge {
    position: absolute;
    bottom: -6px;
    right: -6px;
    background: var(--accent-500);
    color: #fff;
    font-size: 0.65rem;
    padding: 2px 6px;
    border-radius: 999px;
    box-shadow: var(--shadow-soft);
  }

  .meta {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .name-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }

  h3 {
    margin: 0;
    font-size: 1rem;
    font-family: var(--font-display);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .user-id {
    font-size: 0.75rem;
    color: var(--ink-600);
    white-space: nowrap;
  }

  .desc {
    margin: 0;
    font-size: 0.8rem;
    color: var(--ink-600);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .ghost {
    justify-self: flex-start;
    border: 1px solid rgba(16, 27, 30, 0.2);
    background: transparent;
    color: var(--ink-700);
    border-radius: 999px;
    padding: 6px 12px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.2s ease, background 0.2s ease, color 0.2s ease;
  }

  .ghost:hover {
    border-color: var(--accent-500);
    background: rgba(242, 95, 76, 0.06);
    color: var(--accent-700);
  }
</style>
