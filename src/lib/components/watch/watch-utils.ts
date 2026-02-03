export const pickFirstString = (...values: Array<unknown>) => {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) return value;
  }
  return "";
};

export const pickFirstNumber = (...values: Array<unknown>) => {
  for (const value of values) {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && value.trim()) {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
  }
  return 0;
};

export const formatUnix = (value: unknown) => {
  const seconds = pickFirstNumber(value);
  if (!seconds) return "-";
  const date = new Date(seconds * 1000);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString();
};

export const formatNumber = (...values: Array<unknown>) => {
  const num = pickFirstNumber(...values);
  if (!num) return "0";
  return num.toLocaleString();
};

export const flattenForDisplay = (
  value: unknown,
  options: { maxEntries?: number; maxDepth?: number; maxArray?: number } = {}
) => {
  const { maxEntries = 240, maxDepth = 4, maxArray = 8 } = options;
  const rows: Array<{ key: string; value: string }> = [];

  const pushRow = (key: string, val: unknown) => {
    if (!key || rows.length >= maxEntries) return;
    if (typeof val === "number" && Number.isFinite(val)) {
      rows.push({ key, value: val.toLocaleString() });
    } else if (typeof val === "boolean") {
      rows.push({ key, value: val ? "true" : "false" });
    } else if (val === null || val === undefined) {
      rows.push({ key, value: "-" });
    } else {
      rows.push({ key, value: String(val) });
    }
  };

  const walk = (val: unknown, path: string, depth: number) => {
    if (rows.length >= maxEntries) return;
    if (depth > maxDepth) {
      pushRow(path, "[depth]");
      return;
    }
    if (val === null || val === undefined) {
      pushRow(path, "-");
      return;
    }
    const t = typeof val;
    if (t === "string" || t === "number" || t === "boolean") {
      pushRow(path, val);
      return;
    }
    if (Array.isArray(val)) {
      if (val.length === 0) {
        pushRow(path, "[]");
        return;
      }
      const limit = Math.min(val.length, maxArray);
      for (let i = 0; i < limit; i++) {
        walk(val[i], `${path}[${i}]`, depth + 1);
      }
      if (val.length > limit) {
        pushRow(`${path}[+]`, `+${val.length - limit} items`);
      }
      return;
    }
    if (t === "object") {
      const entries = Object.entries(val as Record<string, unknown>);
      if (entries.length === 0) {
        pushRow(path, "{}");
        return;
      }
      for (const [key, inner] of entries) {
        walk(inner, path ? `${path}.${key}` : key, depth + 1);
        if (rows.length >= maxEntries) break;
      }
      return;
    }
    pushRow(path, val);
  };

  if (value && typeof value === "object" && !Array.isArray(value)) {
    for (const [key, inner] of Object.entries(value as Record<string, unknown>)) {
      walk(inner, key, 0);
      if (rows.length >= maxEntries) break;
    }
  } else {
    walk(value, "value", 0);
  }

  return rows;
};

export const extractComments = (res: any) => {
  const list = res?.comments ?? res?.live_comments ?? res?.data ?? [];
  return Array.isArray(list) ? list : [];
};

export const extractRanking = (res: any) => {
  const candidates = [
    res?.ranking,
    res?.rankings,
    res?.gift_ranking,
    res?.gift_ranking?.ranking,
    res?.gift_ranking?.rankings,
    res?.gift_ranking?.ranks,
    res?.data?.ranking,
    res?.data?.rankings,
    res?.data?.gift_ranking,
    res?.data?.gift_ranking?.ranking,
    res?.data?.gift_ranking?.rankings,
    res?.data?.gift_ranking?.ranks,
    res?.ranks,
    res?.items,
    res?.list,
    res?.results,
    res?.data
  ];
  for (const candidate of candidates) {
    if (Array.isArray(candidate)) return candidate;
  }
  return [];
};

export const getStreamUrl = (status: any) => {
  const direct =
    status?.streaming_url_hls ??
    status?.streaming_url ??
    status?.hls_url ??
    status?.playlist_url ??
    "";
  if (direct) return direct;

  const list = status?.streaming_url_list ?? status?.streaming_urls ?? status?.url_list ?? [];
  if (Array.isArray(list)) {
    for (const item of list) {
      if (typeof item === "string" && item) return item;
      if (item?.url) return item.url;
      if (item?.streaming_url) return item.streaming_url;
      if (item?.hls_url) return item.hls_url;
    }
  }
  return "";
};

export const buildLiveInfoView = (liveInfo: any, polling: any) => {
  if (!liveInfo && !polling) return null;
  const src = liveInfo?.live ?? liveInfo ?? {};
  const pollSrc = polling?.live ?? polling ?? {};

  const title = pickFirstString(src?.title, src?.name, pollSrc?.title, pollSrc?.name);
  const ownerName = pickFirstString(
    src?.owner?.name,
    src?.user?.name,
    pollSrc?.owner?.name,
    pollSrc?.user?.name
  );
  const viewers = pickFirstNumber(
    src?.total_viewer_num,
    src?.online_user_num,
    pollSrc?.total_viewer_num,
    pollSrc?.online_user_num
  );
  const commentNum = pickFirstNumber(src?.comment_num, pollSrc?.comment_num);
  const startedAt = pickFirstNumber(src?.started_at, pollSrc?.started_at);
  const appTitle = pickFirstString(
    src?.app_title,
    src?.app_short_title,
    pollSrc?.app_title,
    pollSrc?.app_short_title
  );
  const collabVacancy =
    typeof src?.collab_has_vacancy === "number"
      ? src.collab_has_vacancy
      : typeof pollSrc?.collab_has_vacancy === "number"
        ? pollSrc.collab_has_vacancy
        : null;

  const isLiveValue =
    typeof src?.is_live === "boolean"
      ? src.is_live
      : typeof pollSrc?.is_live === "boolean"
        ? pollSrc.is_live
        : typeof liveInfo?.is_live === "boolean"
          ? liveInfo.is_live
          : src?.ended_at === 0;

  return {
    title: title || "タイトルなし",
    owner: ownerName || "不明",
    viewers,
    commentNum,
    startedAt,
    appTitle: appTitle || "不明",
    collabVacancy,
    status: isLiveValue ? "配信中" : "終了"
  };
};

export const getGiftRankingUrl = (polling: any, liveInfo: any) =>
  pickFirstString(
    polling?.gift_ranking_url,
    polling?.giftRankingUrl,
    polling?.gift_ranking?.url,
    polling?.gift?.ranking_url,
    polling?.live?.gift_ranking_url,
    liveInfo?.gift_ranking_url,
    liveInfo?.live?.gift_ranking_url
  );

export const getObfuscatedUserId = (polling: any, liveInfo: any, giftRankingUrl: string) => {
  let fromUrl = "";
  if (giftRankingUrl) {
    try {
      const u = new URL(giftRankingUrl);
      fromUrl = u.searchParams.get("obfuscated_user_id") ?? "";
    } catch {
      fromUrl = "";
    }
  }
  return pickFirstString(
    polling?.obfuscated_user_id,
    polling?.current_user_rank?.user?.obfuscated_user_id,
    polling?.user?.obfuscated_user_id,
    liveInfo?.owner?.obfuscated_user_id,
    liveInfo?.user?.obfuscated_user_id,
    liveInfo?.live?.owner?.obfuscated_user_id,
    liveInfo?.live?.user?.obfuscated_user_id,
    fromUrl
  );
};

export const mergeRankingLists = (base: any[], extra: any[]) => {
  if (!base.length) return extra;
  if (!extra.length) return base;
  const extraByKey = new Map<string, any>();
  const extrasWithoutKey: any[] = [];

  for (const item of extra) {
    const rank = item?.rank ?? item?.rank_no ?? item?.rankNo;
    const userId = item?.user?.user_id ?? item?.user_id;
    const key = rank !== undefined && rank !== null ? `rank:${rank}` : userId ? `user:${userId}` : "";
    if (key) {
      extraByKey.set(key, item);
    } else {
      extrasWithoutKey.push(item);
    }
  }

  const used = new Set<string>();
  const merged = base.map((item) => {
    const rank = item?.rank ?? item?.rank_no ?? item?.rankNo;
    const userId = item?.user?.user_id ?? item?.user_id;
    const key = rank !== undefined && rank !== null ? `rank:${rank}` : userId ? `user:${userId}` : "";
    const extraItem = key ? extraByKey.get(key) : null;
    if (extraItem && key) used.add(key);
    return extraItem ? { ...item, ...extraItem } : item;
  });

  for (const [key, item] of extraByKey.entries()) {
    if (!used.has(key)) merged.push(item);
  }
  if (extrasWithoutKey.length) merged.push(...extrasWithoutKey);
  return merged;
};

export const resolveRankingItem = (item: any) => {
  const user = item?.user ?? item?.owner ?? item?.sender ?? item?.viewer ?? item?.account ?? {};
  const gift =
    item?.gift ?? item?.gift_master ?? item?.gift_detail ?? item?.gift_item ?? item?.present ?? {};
  const rank = item?.rank ?? item?.rank_no ?? item?.rankNo ?? "-";
  const userName = pickFirstString(user?.name, item?.user_name, item?.name, "ユーザー");
  const userId = pickFirstString(user?.user_id, item?.user_id);
  const points = formatNumber(
    item?.gift_point,
    item?.point,
    item?.amount,
    item?.total_point,
    item?.total,
    item?.points
  );
  const giftName = pickFirstString(
    gift?.name,
    gift?.title,
    item?.gift_name,
    item?.gift_title,
    item?.present_name
  );
  const giftImage = pickFirstString(
    item?.gift_image_url,
    gift?.image_url,
    gift?.icon_url,
    gift?.thumbnail_url,
    gift?.image,
    item?.image_url,
    item?.thumbnail_url
  );
  const userImage = pickFirstString(
    user?.profile_image_url,
    user?.avatar_image_url,
    user?.image_url,
    item?.user_image_url,
    item?.profile_image_url
  );
  return { rank, userName, userId, points, giftName, giftImage, userImage };
};

export const buildGiftRankingView = (giftRanking: any, giftRankingExtra: any) => {
  const base = Array.isArray(giftRanking) ? giftRanking : [];
  const extra = Array.isArray(giftRankingExtra) ? giftRankingExtra : [];
  const merged = mergeRankingLists(base, extra);
  return merged.map(resolveRankingItem);
};