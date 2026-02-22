// ─────────────────────────────────────────────────────────────────────────────
// プリミティブ値ピックヘルパー
// 複数の候補値から最初の有効な値を返す汎用ユーティリティ
// ─────────────────────────────────────────────────────────────────────────────

/** 複数の候補から最初の有効な文字列を返す（見つからない場合は空文字） */
export const pickFirstString = (...values: Array<unknown>) => {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) return value;
  }
  return "";
};

/** 複数の候補から最初の有効な数値を返す（見つからない場合は 0） */
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

/**
 * 複数の候補から最初の有効な数値を返す（見つからない場合は null）
 * 値がないことを明示的に区別したい場合（例: 視聴者数が未取得）に使用する
 */
export const pickNullableNumber = (...values: Array<unknown>): number | null => {
  for (const value of values) {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && value.trim()) {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
  }
  return null;
};

// ─────────────────────────────────────────────────────────────────────────────
// フォーマットヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/** Unix タイムスタンプ（秒）をロケール文字列にフォーマットする */
export const formatUnix = (value: unknown) => {
  const seconds = pickFirstNumber(value);
  if (!seconds) return "-";
  const date = new Date(seconds * 1000);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString();
};

/** 複数の候補から最初の有効な数値を取得し、カンマ区切りの文字列にフォーマットする */
export const formatNumber = (...values: Array<unknown>) => {
  const num = pickFirstNumber(...values);
  if (!num) return "0";
  return num.toLocaleString();
};

// ─────────────────────────────────────────────────────────────────────────────
// デバッグ表示用フラット化
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 任意のオブジェクトをデバッグ表示用にフラットなキー・値ペアの配列へ変換する。
 * ネストされたオブジェクトはドット区切りのパスで表現される。
 */
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

// ─────────────────────────────────────────────────────────────────────────────
// APIレスポンス正規化
// ─────────────────────────────────────────────────────────────────────────────

/** API レスポンスからコメント配列を抽出する（複数フィールド名に対応） */
export const extractComments = (res: any) => {
  const list = res?.comments ?? res?.live_comments ?? res?.data ?? [];
  return Array.isArray(list) ? list : [];
};

/**
 * API レスポンスからランキング配列を抽出する。
 * ギフトランキングはネスト構造が API バージョンによって異なるため、
 * 複数のパスを試みる。
 */
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

// ─────────────────────────────────────────────────────────────────────────────
// ストリーム URL 解決
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ストリームステータスオブジェクトから HLS URL を取得する。
 * APIバージョンによってフィールド名が異なるため、複数のパスを試みる。
 */
export const getStreamUrl = (status: any) => {
  // 直接フィールドを優先
  const direct =
    status?.streaming_url_hls ??
    status?.streaming_url ??
    status?.hls_url ??
    status?.playlist_url ??
    "";
  if (direct) return direct;

  // リスト形式の場合、最初の有効な URL を返す
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

/**
 * LLStream の edge/stream_key から WS URL を組み立てる。
 * edge が ws:// / wss:// の場合はそのまま利用し、そうでなければ ws://<edge>:1883 を使う。
 */
export const buildLlstreamWsUrl = (edge: string, streamKey: string, suffix: string) => {
  if (!edge || !streamKey) return "";
  if (edge.startsWith("ws://") || edge.startsWith("wss://")) {
    const normalized = edge.replace(/\/+$/, "");
    return `${normalized}/ws/${streamKey}/${suffix}`;
  }
  const host = edge.includes(":") ? edge : `${edge}:1883`;
  return `ws://${host}/ws/${streamKey}/${suffix}`;
};

/** streamStatus から LLStream video WS URL を解決する */
export const getLlstreamVideoWsUrl = (status: any) => {
  const direct = pickFirstString(
    status?.streaming_url_llstream_video,
    status?.live?.streaming_url_llstream_video,
    status?.data?.streaming_url_llstream_video
  );
  if (direct) return direct;

  const streamKey = pickFirstString(
    status?.streaming_key,
    status?.live?.streaming_key,
    status?.data?.streaming_key
  );
  const edge = pickFirstString(
    status?.streaming_url_edge,
    status?.live?.streaming_url_edge,
    status?.data?.streaming_url_edge
  );
  return buildLlstreamWsUrl(edge, streamKey, "video/avc");
};

/** streamStatus から LLStream audio WS URL を解決する */
export const getLlstreamAudioWsUrl = (status: any) => {
  const direct = pickFirstString(
    status?.streaming_url_llstream_audio,
    status?.live?.streaming_url_llstream_audio,
    status?.data?.streaming_url_llstream_audio
  );
  if (direct) return direct;

  const streamKey = pickFirstString(
    status?.streaming_key,
    status?.live?.streaming_key,
    status?.data?.streaming_key
  );
  const edge = pickFirstString(
    status?.streaming_url_edge,
    status?.live?.streaming_url_edge,
    status?.data?.streaming_url_edge
  );
  return buildLlstreamWsUrl(edge, streamKey, "audio/aac");
};

// ─────────────────────────────────────────────────────────────────────────────
// ライブ情報ビルダー
// ─────────────────────────────────────────────────────────────────────────────

/**
 * liveInfo と polling の両方を統合してUI表示用のビューモデルを構築する。
 * polling の値はリアルタイム更新されるため、liveInfo より優先される。
 */
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
  // pollingの値を優先（リアルタイム更新されるため）
  const totalViewers = pickFirstNumber(
    pollSrc?.total_viewer_num,
    src?.total_viewer_num
  );
  const onlineViewers = pickFirstNumber(
    pollSrc?.online_user_num,
    src?.online_user_num
  );
  const viewers = onlineViewers || totalViewers;
  const commentNum = pickFirstNumber(pollSrc?.comment_num, src?.comment_num);
  const startedAt = pickFirstNumber(src?.started_at, pollSrc?.started_at);
  const appTitle = pickFirstString(
    src?.app_title,
    src?.app_short_title,
    pollSrc?.app_title,
    pollSrc?.app_short_title
  );
  const collabVacancy =
    typeof pollSrc?.collab_has_vacancy === "number"
      ? pollSrc.collab_has_vacancy
      : typeof src?.collab_has_vacancy === "number"
        ? src.collab_has_vacancy
        : null;

  const isLiveValue =
    typeof pollSrc?.is_live === "boolean"
      ? pollSrc.is_live
      : typeof src?.is_live === "boolean"
        ? src.is_live
        : typeof liveInfo?.is_live === "boolean"
          ? liveInfo.is_live
          : src?.ended_at === 0;

  const starCount = pickFirstNumber(pollSrc?.star_num, src?.star_num);
  const giftCount = pickFirstNumber(pollSrc?.gift_num, src?.gift_num);
  const liveId = pickFirstString(src?.live_id, pollSrc?.live_id);
  const ownerUserId = pickFirstString(
    src?.owner?.user_id,
    src?.user?.user_id,
    pollSrc?.owner?.user_id,
    pollSrc?.user?.user_id
  );
  const isFollowing =
    typeof src?.owner?.is_following === "number"
      ? src.owner.is_following
      : typeof src?.is_following === "number"
        ? src.is_following
        : null;

  return {
    title: title || "タイトルなし",
    owner: ownerName || "不明",
    ownerUserId,
    isFollowing,
    viewers,
    totalViewers,
    onlineViewers,
    commentNum,
    startedAt,
    appTitle: appTitle || "不明",
    collabVacancy,
    status: isLiveValue ? "配信中" : "終了",
    isLive: isLiveValue,
    starCount,
    giftCount,
    liveId,
  };
};

// ─────────────────────────────────────────────────────────────────────────────
// ギフトランキング URL / ユーザー ID 解決
// ─────────────────────────────────────────────────────────────────────────────

/** polling または liveInfo からギフトランキング API の URL を取得する */
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

/**
 * ランキング取得に必要な obfuscated_user_id を取得する。
 * URL クエリパラメータ、polling、liveInfo の順に検索する。
 */
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

// ─────────────────────────────────────────────────────────────────────────────
// ランキングリストのマージ・正規化
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ベースランキングと追加ランキングをマージする。
 * rank / user_id をキーに重複排除し、追加情報で補完する。
 */
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

/**
 * ランキングアイテムを API 構造の差異を吸収してUI表示用に正規化する。
 * ユーザー情報・ギフト情報のネスト位置がバージョンによって異なるため、
 * 複数のパスから取得する。
 */
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

/** ギフトランキングのベース・追加リストをマージしてUI表示用に変換する */
export const buildGiftRankingView = (giftRanking: any, giftRankingExtra: any) => {
  const base = Array.isArray(giftRanking) ? giftRanking : [];
  const extra = Array.isArray(giftRankingExtra) ? giftRankingExtra : [];
  const merged = mergeRankingLists(base, extra);
  return merged.map(resolveRankingItem);
};
