<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import LiveCard from "$lib/components/LiveCard.svelte";
  import Skeleton from "$lib/components/ui/Skeleton.svelte";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";

  let { onOpenLive } = $props<{ onOpenLive: (live: any) => void }>();
  type DetailItem = { label: string; value: string };
  type CapabilityItem = { key: string; label: string; enabled: boolean };

  const capabilityDefs = [
    { key: "pip_enabled", label: "PiP" },
    { key: "gift_ranking_enabled", label: "ギフトランキング" },
    { key: "is_llstream_v1_subscribe", label: "LLStream視聴" },
    { key: "is_llstream_v1_broadcast", label: "LLStream配信" },
    { key: "karaoke_enabled", label: "カラオケ" },
    { key: "moderator", label: "モデレーター" },
    { key: "onboarding_twitter_enabled", label: "Twitter連携" },
    { key: "game_app_icon_enabled", label: "ゲームアイコン" }
  ] as const;

  let profile = $state<any>(null);
  let currency = $state<any>(null);
  let liveHistory = $state<any[]>([]);
  let viewHistory = $state<any[]>([]);
  let loading = $state(false);
  let error = $state("");

  const extractLives = (res: any) => {
    const list = res?.lives ?? res?.live_list ?? res?.history ?? res?.data ?? [];
    return Array.isArray(list) ? list : [];
  };

  const toFiniteNumber = (value: unknown): number | null => {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && value.trim()) {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
    return null;
  };

  const formatNumber = (value: unknown) => {
    const num = toFiniteNumber(value);
    if (num === null) return "-";
    return num.toLocaleString();
  };

  const formatUnix = (value: unknown) => {
    const seconds = toFiniteNumber(value);
    if (seconds === null || seconds <= 0) return "-";
    const date = new Date(seconds * 1000);
    if (Number.isNaN(date.getTime())) return "-";
    return date.toLocaleString();
  };

  const formatBirthday = (birthday: unknown, visible: unknown) => {
    if (visible === false || visible === 0 || visible === "0") return "非公開";
    const raw = typeof birthday === "string" ? birthday.trim() : "";
    if (!raw) return "-";
    if (raw.length === 4) return `${raw.slice(0, 2)}/${raw.slice(2)}`;
    return raw;
  };

  const textOrDash = (value: unknown) => {
    if (typeof value === "string" && value.trim()) return value.trim();
    if (typeof value === "number" && Number.isFinite(value)) return String(value);
    return "-";
  };

  const yesNo = (value: unknown, onLabel = "有効", offLabel = "無効") => {
    if (value === true || value === 1 || value === "1") return onLabel;
    return offLabel;
  };

  const capabilityEnabled = (key: string) => {
    const raw = profile?.capabilities?.[key];
    return raw === true || raw === 1 || raw === "1";
  };

  const profileDetails = $derived.by<DetailItem[]>(() => {
    if (!profile) return [];
    return [
      { label: "ユーザーID", value: textOrDash(profile.user_id) },
      {
        label: "Twitter",
        value: profile.twitter_screen_name ? `@${profile.twitter_screen_name}` : "-"
      },
      { label: "世代", value: textOrDash(profile.generation) },
      { label: "誕生日", value: formatBirthday(profile.birthday, profile.is_visible_birthday) },
      { label: "登録日", value: formatUnix(profile.registered_at) },
      { label: "最終配信開始", value: formatUnix(profile.latest_live_started_at) }
    ];
  });

  const activityDetails = $derived.by<DetailItem[]>(() => {
    if (!profile) return [];
    return [
      { label: "累計視聴者", value: formatNumber(profile.total_viewer_num) },
      { label: "配信リクエスト", value: formatNumber(profile.live_request_num) },
      { label: "マイアプリ数", value: formatNumber(profile.my_app_num) },
      { label: "録画機能", value: yesNo(profile.recording_enabled, "ON", "OFF") },
      { label: "初配信済み", value: yesNo(profile.has_started_first_live, "はい", "いいえ") },
      { label: "VIP公開", value: yesNo(profile.is_vip_public, "公開", "非公開") }
    ];
  });

  const profileLinks = $derived.by<string[]>(() => {
    if (!Array.isArray(profile?.links)) return [];
    return profile.links
      .map((item: any) => (typeof item?.url === "string" ? item.url.trim() : ""))
      .filter(Boolean);
  });

  const capabilityDetails = $derived.by<CapabilityItem[]>(() =>
    capabilityDefs.map((item) => ({
      key: item.key,
      label: item.label,
      enabled: capabilityEnabled(item.key)
    }))
  );

  const loadProfile = async () => {
    loading = true;
    error = "";
    try {
      profile = await invoke("get_my_profile");
      currency = await invoke("get_user_currency");

      const userId = profile?.user_id ?? profile?.id ?? profile?.user?.user_id;
      if (userId) {
        const liveRes: any = await invoke("get_live_history", { userId: String(userId) });
        liveHistory = extractLives(liveRes);

        const viewRes: any = await invoke("get_view_history", { userId: String(userId) });
        viewHistory = extractLives(viewRes);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  };

  onMount(loadProfile);
</script>

<section class="profile">
  <header class="section-head">
    <div>
      <p class="kicker">Profile</p>
      <h2>アカウント概要</h2>
    </div>
    <button class="ghost" onclick={loadProfile} disabled={loading}>
      {loading ? "更新中..." : "更新"}
    </button>
  </header>

  {#if error}
    <ErrorMessage
      message={error}
      onRetry={loadProfile}
      onDismiss={() => (error = "")}
    />
  {/if}

  {#if loading}
    <div class="profile-card">
      <Skeleton variant="avatar" />
      <div class="info" style="flex: 1;">
        <Skeleton variant="text" height="28px" width="60%" />
        <Skeleton variant="text" height="20px" width="90%" />
        <Skeleton variant="text" height="20px" width="40%" />
      </div>
    </div>
  {:else if profile}
    <div class="profile-card">
      <div class="avatar">
        {#if profile.profile_image_url}
          <img src={profile.profile_image_url} alt="avatar" />
        {:else}
          <span>ME</span>
        {/if}
      </div>
      <div class="info">
        <h3>{profile.name ?? "名前なし"}</h3>
        <p class="desc">{profile.description ?? "自己紹介はまだありません"}</p>
        <div class="stats">
          <span>フォロワー {formatNumber(profile.follower_num)}</span>
          <span>フォロー {formatNumber(profile.following_num)}</span>
          <span>配信数 {formatNumber(profile.live_count)}</span>
        </div>
      </div>
    </div>
  {/if}

  {#if !loading && profile}
    <div class="detail-panels">
      <div class="detail-card">
        <h3>プロフィール詳細</h3>
        <div class="detail-grid">
          {#each profileDetails as item}
            <div class="detail-item">
              <span class="detail-label">{item.label}</span>
              <span class="detail-value">{item.value}</span>
            </div>
          {/each}
        </div>
      </div>

      <div class="detail-card">
        <h3>配信・活動</h3>
        <div class="detail-grid">
          {#each activityDetails as item}
            <div class="detail-item">
              <span class="detail-label">{item.label}</span>
              <span class="detail-value">{item.value}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <div class="detail-card">
      <h3>リンク</h3>
      {#if profileLinks.length === 0}
        <p class="muted">リンクは未設定です</p>
      {:else}
        <ul class="link-list">
          {#each profileLinks as url}
            <li>
              <a href={url} target="_blank" rel="noreferrer">{url}</a>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="detail-panels">
      <div class="detail-card">
        <h3>配信説明文</h3>
        <p class="multiline">{profile.live_description ?? "未設定"}</p>
      </div>
      <div class="detail-card">
        <h3>お礼メッセージ</h3>
        <p class="multiline">{profile.custom_thanks_message ?? "未設定"}</p>
      </div>
    </div>

    <div class="detail-card">
      <h3>主要機能フラグ</h3>
      <div class="capability-list">
        {#each capabilityDetails as item}
          <span class="capability-badge {item.enabled ? 'enabled' : 'disabled'}">
            {item.label}: {item.enabled ? "ON" : "OFF"}
          </span>
        {/each}
      </div>
    </div>
  {/if}

  <div class="wallet">
    <h3>ウォレット</h3>
    {#if currency}
      <div class="wallet-grid">
        {#each Object.entries(currency) as [key, value]}
          <div class="wallet-item">
            <span class="wallet-key">{key}</span>
            <span class="wallet-value">{value}</span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="muted">通貨データなし</p>
    {/if}
  </div>

  <div class="history">
    <h3>配信履歴</h3>
    <div class="live-grid">
      {#if loading}
        {#each Array(3) as _}
          <Skeleton variant="card" />
        {/each}
      {:else if liveHistory.length === 0}
        <div class="empty">履歴はありません</div>
      {:else}
        {#each liveHistory as item}
          <LiveCard live={item.live ?? item} onSelect={onOpenLive} />
        {/each}
      {/if}
    </div>
  </div>

  <div class="history">
    <h3>視聴履歴</h3>
    <div class="live-grid">
      {#if loading}
        {#each Array(3) as _}
          <Skeleton variant="card" />
        {/each}
      {:else if viewHistory.length === 0}
        <div class="empty">履歴はありません</div>
      {:else}
        {#each viewHistory as item}
          <LiveCard live={item.live ?? item} onSelect={onOpenLive} />
        {/each}
      {/if}
    </div>
  </div>
</section>

<style>
  .profile {
    display: grid;
    gap: 22px;
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

  .profile-card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 16px;
    padding: 18px;
    border-radius: 20px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
  }

  .avatar {
    width: 72px;
    height: 72px;
    border-radius: 20px;
    background: rgba(16, 27, 30, 0.1);
    display: grid;
    place-items: center;
    overflow: hidden;
    font-weight: 700;
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .info h3 {
    margin: 0 0 6px 0;
    font-size: 1.2rem;
    font-family: var(--font-display);
  }

  .desc {
    margin: 0 0 10px 0;
    color: var(--ink-600);
  }

  .stats {
    display: flex;
    gap: 12px;
    font-size: 0.85rem;
    color: var(--ink-600);
    flex-wrap: wrap;
  }

  .detail-panels {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 14px;
  }

  .detail-card {
    padding: 16px;
    border-radius: 18px;
    background: rgba(16, 27, 30, 0.04);
    display: grid;
    gap: 10px;
  }

  .detail-card h3 {
    margin: 0;
    font-size: 1rem;
  }

  .detail-grid {
    display: grid;
    gap: 8px;
  }

  .detail-item {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 8px;
    align-items: center;
    padding: 8px 10px;
    border-radius: 10px;
    background: #fff;
    box-shadow: var(--shadow-soft);
    min-width: 0;
  }

  .detail-label {
    color: var(--ink-500);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    font-weight: 600;
  }

  .detail-value {
    color: var(--ink-700);
    font-size: 0.9rem;
    font-weight: 600;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .link-list {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 6px;
  }

  .link-list a {
    color: var(--ink-700);
    text-decoration: underline;
    text-decoration-color: rgba(16, 27, 30, 0.35);
    overflow-wrap: anywhere;
  }

  .multiline {
    margin: 0;
    white-space: pre-wrap;
    line-height: 1.5;
    color: var(--ink-700);
    background: #fff;
    border-radius: 12px;
    padding: 12px;
    box-shadow: var(--shadow-soft);
  }

  .capability-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .capability-badge {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 6px 12px;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .capability-badge.enabled {
    background: rgba(36, 152, 126, 0.18);
    color: #0f5e4d;
  }

  .capability-badge.disabled {
    background: rgba(16, 27, 30, 0.1);
    color: var(--ink-600);
  }

  .wallet {
    padding: 18px;
    border-radius: 18px;
    background: rgba(16, 27, 30, 0.04);
  }

  .wallet h3 {
    margin: 0 0 12px 0;
    font-size: 1rem;
  }

  .wallet-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 12px;
  }

  .wallet-item {
    padding: 10px 12px;
    border-radius: 14px;
    background: #fff;
    box-shadow: var(--shadow-soft);
    display: grid;
    gap: 6px;
  }

  .wallet-key {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--ink-500);
  }

  .wallet-value {
    font-weight: 700;
    font-size: 1rem;
  }

  .history h3 {
    margin: 0 0 12px 0;
    font-size: 1rem;
  }

  .live-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
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

  .empty {
    padding: 16px;
    border-radius: 16px;
    background: rgba(16, 27, 30, 0.06);
    color: var(--ink-600);
  }

  .error {
    margin: 0;
    color: var(--accent-700);
    font-weight: 600;
  }

  .muted {
    color: var(--ink-500);
  }

  @media (max-width: 720px) {
    .profile-card {
      grid-template-columns: 1fr;
    }

    .detail-item {
      grid-template-columns: 1fr;
      gap: 4px;
    }
  }
</style>
