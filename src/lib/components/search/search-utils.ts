export const extractList = (res: any, keys: string[]) => {
  for (const key of keys) {
    const value = key.includes(".")
      ? key.split(".").reduce((acc, part) => acc?.[part], res as any)
      : (res as any)?.[key];
    if (Array.isArray(value)) return value;
  }
  return [];
};

export const extractUsers = (res: any) =>
  extractList(res, [
    "users",
    "user_list",
    "result",
    "search_result",
    "data.users",
    "data.user_list",
    "data.result",
    "data.search_result"
  ]);

export const extractLives = (res: any) => {
  const list = (res as any)?.lives ?? (res as any)?.live_list ?? (res as any)?.history ?? (res as any)?.data ?? [];
  return Array.isArray(list) ? list : [];
};

export const extractMeta = (res: any) => {
  const meta = (res as any)?.data ?? res;
  return {
    currentPage: meta?.current_page ?? meta?.currentPage ?? null,
    nextPage: meta?.next_page ?? meta?.nextPage ?? null,
    previousPage: meta?.previous_page ?? meta?.previousPage ?? null,
    totalEntries: meta?.total_entries ?? meta?.totalEntries ?? null,
    currentCursor: meta?.current_cursor ?? meta?.currentCursor ?? null,
    nextCursor: meta?.next_cursor ?? meta?.nextCursor ?? null
  };
};

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

export const formatNumber = (...values: Array<unknown>) => {
  const num = pickFirstNumber(...values);
  if (!num) return "0";
  return num.toLocaleString();
};

export const formatUnix = (value: unknown) => {
  const seconds = pickFirstNumber(value);
  if (!seconds) return "-";
  const date = new Date(seconds * 1000);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleDateString();
};

export const formatBirthday = (value: unknown, visible?: unknown) => {
  if (visible === false || visible === 0 || visible === "0") return "非公開";
  const raw = pickFirstString(value);
  if (!raw) return "非公開";
  if (raw.length === 4) {
    return `${raw.slice(0, 2)}/${raw.slice(2)}`;
  }
  return raw;
};

export const getUserId = (user: any) =>
  pickFirstString(user?.user_id, user?.id, user?.user?.user_id);

export const getUserName = (user: any) =>
  pickFirstString(user?.name, user?.user?.name, user?.username, user?.screen_name, "ユーザー");

export const getUserAvatar = (user: any) =>
  pickFirstString(
    user?.profile_image_url,
    user?.user?.profile_image_url,
    user?.avatar_image_url,
    user?.image_url
  );

export const getUserDescription = (user: any) =>
  pickFirstString(user?.description, user?.user?.description, user?.bio);

export const getUserLiveId = (user: any) =>
  pickFirstString(
    user?.onlive?.live_id,
    user?.onlive?.id,
    user?.live?.live_id,
    user?.live_id
  );