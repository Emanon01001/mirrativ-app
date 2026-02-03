<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
  import ErrorMessage from "$lib/components/ui/ErrorMessage.svelte";
  import SearchControls from "$lib/components/search/SearchControls.svelte";
  import SearchRecommend from "$lib/components/search/SearchRecommend.svelte";
  import SearchResults from "$lib/components/search/SearchResults.svelte";
  import SearchUserDetail from "$lib/components/search/SearchUserDetail.svelte";
  import { searchState } from "$lib/stores/search";
  import {
    extractList,
    extractUsers,
    extractLives,
    extractMeta,
    pickFirstString,
    pickFirstNumber,
    formatNumber,
    formatUnix,
    formatBirthday,
    getUserId,
    getUserName,
    getUserAvatar,
    getUserDescription,
    getUserLiveId
  } from "$lib/components/search/search-utils";

  let { onOpenLive } = $props<{ onOpenLive: (live: any) => void }>();

  const initialState = get(searchState);

  let query = $state(initialState.query);
  let mode = $state<"live" | "user">(initialState.mode);
  let results = $state<any[]>(initialState.results);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state(initialState.error);
  let searched = $state(initialState.searched);
  let userHasMore = $state(initialState.userHasMore);
  let currentPage = $state<number | null>(initialState.currentPage);
  let nextPage = $state<number | null>(initialState.nextPage);
  let previousPage = $state<number | null>(initialState.previousPage);
  let totalEntries = $state<number | null>(initialState.totalEntries);
  let currentCursor = $state<string | null>(initialState.currentCursor);
  let nextCursor = $state<string | null>(initialState.nextCursor);
  let recommendUsers = $state<any[]>(initialState.recommendUsers);
  let recommendPage = $state(initialState.recommendPage);
  let recommendHasMore = $state(initialState.recommendHasMore);
  let recommendLoading = $state(false);
  let recommendLoadingMore = $state(false);
  let recommendError = $state(initialState.recommendError);
  let selectedUser = $state<any>(initialState.selectedUser);
  let selectedUserDetail = $state<any>(initialState.selectedUserDetail);
  let selectedUserLiveHistory = $state<any[]>(initialState.selectedUserLiveHistory);
  let userHistoryPage = $state(initialState.userHistoryPage);
  let userHistoryHasMore = $state(initialState.userHistoryHasMore);
  let userHistoryTotal = $state<number | null>(initialState.userHistoryTotal);
  let userHistoryCurrentPage = $state<number | null>(initialState.userHistoryCurrentPage);
  let userHistoryNextPage = $state<number | null>(initialState.userHistoryNextPage);
  let userHistoryPreviousPage = $state<number | null>(initialState.userHistoryPreviousPage);
  let userHistoryLoadingMore = $state(false);
  let userDetailLoading = $state(false);
  let userDetailError = $state(initialState.userDetailError);
  let userHistoryError = $state(initialState.userHistoryError);
  let sentinel: HTMLDivElement | null = $state(null);
  let observer: IntersectionObserver | null = null;
  let userSentinel: HTMLDivElement | null = $state(null);
  let userObserver: IntersectionObserver | null = null;

  const applyMeta = (res: any) => {
    const meta = extractMeta(res);
    currentPage = meta.currentPage;
    nextPage = meta.nextPage;
    previousPage = meta.previousPage;
    totalEntries = meta.totalEntries;
    currentCursor = meta.currentCursor;
    nextCursor = meta.nextCursor;
  };

  const pickNumber = (...values: Array<unknown>) => {
    for (const value of values) {
      if (typeof value === "number" && Number.isFinite(value)) return value;
      if (typeof value === "string" && value.trim()) {
        const parsed = Number(value);
        if (Number.isFinite(parsed)) return parsed;
      }
    }
    return null;
  };

  const applyHistoryMeta = (res: any) => {
    const meta = (res as any)?.data ?? res;
    userHistoryCurrentPage = pickNumber(meta?.current_page, meta?.currentPage);
    userHistoryNextPage = pickNumber(meta?.next_page, meta?.nextPage);
    userHistoryPreviousPage = pickNumber(meta?.previous_page, meta?.previousPage);
    userHistoryTotal = pickNumber(meta?.lives_num, meta?.total_entries, meta?.totalEntries);
    userHistoryHasMore = userHistoryNextPage !== null && userHistoryNextPage > 0;
  };

  const runSearch = async (options: {
    page?: number;
    cursor?: string | null;
    append?: boolean;
  } = {}) => {
    const { page = 1, cursor = null, append = false } = options;
    const q = query.trim();
    if (!q) {
      error = "検索ワードを入力してください";
      results = [];
      searched = false;
      currentPage = null;
      nextPage = null;
      previousPage = null;
      totalEntries = null;
      currentCursor = null;
      nextCursor = null;
      return;
    }
    if (append) {
      loadingMore = true;
    } else {
      loading = true;
      if (mode === "user") {
        selectedUser = null;
        selectedUserDetail = null;
        selectedUserLiveHistory = [];
        userHistoryPage = 1;
        userHistoryHasMore = true;
        userHistoryLoadingMore = false;
        userHistoryTotal = null;
        userHistoryCurrentPage = null;
        userHistoryNextPage = null;
        userHistoryPreviousPage = null;
        userDetailError = "";
        userHistoryError = "";
        userDetailLoading = false;
      }
    }
    error = "";
    searched = true;
    try {
      const res: any =
        mode === "live"
          ? await invoke("get_live_search", { query: q, page })
          : await invoke("get_user_search", { query: q, page, cursor });
      const incoming = extractList(
        res,
        mode === "live"
          ? ["live_smalls", "lives", "live_list", "data", "data.lives", "search_result"]
          : [
              "users",
              "user_list",
              "search_result",
              "result",
              "data.users",
              "data.user_list",
              "data.search_result",
              "data.result"
            ]
      );
      results = append ? [...results, ...incoming] : incoming;
      applyMeta(res);
      if (mode === "user") {
        if (currentPage === null) {
          currentPage = page;
        }
        if (!append) userHasMore = true;
        if (incoming.length === 0 && append) {
          userHasMore = false;
        } else if (nextCursor || nextPage) {
          userHasMore = true;
        } else if (totalEntries !== null && results.length >= totalEntries) {
          userHasMore = false;
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
      loadingMore = false;
    }
  };

  const handleSubmit = (event: Event) => {
    event.preventDefault();
    void runSearch({ page: 1 });
  };

  const loadNext = () => {
    if (loadingMore || loading) return;
    if (!searched || !query.trim()) return;
    if (mode === "user" && nextCursor) {
      void runSearch({ cursor: nextCursor, append: true });
      return;
    }
    if (mode === "user") {
      if (!userHasMore) return;
      const next = nextPage ?? (currentPage ? currentPage + 1 : 2);
      void runSearch({ page: next, append: true });
      return;
    }
    if (!nextPage) return;
    void runSearch({ page: nextPage, append: true });
  };

  const switchMode = (next: "live" | "user") => {
    if (mode === next) return;
    mode = next;
    results = [];
    searched = false;
    userHasMore = true;
    currentPage = null;
    nextPage = null;
    previousPage = null;
    totalEntries = null;
    currentCursor = null;
    nextCursor = null;
    selectedUser = null;
    selectedUserDetail = null;
    selectedUserLiveHistory = [];
    userHistoryPage = 1;
    userHistoryHasMore = true;
    userHistoryLoadingMore = false;
    userHistoryTotal = null;
    userHistoryCurrentPage = null;
    userHistoryNextPage = null;
    userHistoryPreviousPage = null;
    userDetailError = "";
    userHistoryError = "";
    userDetailLoading = false;
    error = "";
    if (next === "user" && recommendUsers.length === 0) {
      void fetchRecommend(1, false);
    }
  };

  const fetchRecommend = async (page = 1, append = false) => {
    if (append) {
      recommendLoadingMore = true;
    } else {
      recommendLoading = true;
    }
    recommendError = "";
    try {
      const res: any = await invoke("get_recommend_users", { page });
      const users = extractUsers(res);
      recommendUsers = append ? [...recommendUsers, ...users] : users;
      recommendPage = page;
      recommendHasMore = users.length > 0;
    } catch (e) {
      recommendError = e instanceof Error ? e.message : String(e);
    } finally {
      recommendLoading = false;
      recommendLoadingMore = false;
    }
  };

  const loadMoreRecommend = () => {
    if (recommendLoading || recommendLoadingMore || !recommendHasMore) return;
    void fetchRecommend(recommendPage + 1, true);
  };

  const fetchUserHistory = async (userId: string, page = 1, append = false) => {
    if (!userId) return;
    if (append) {
      userHistoryLoadingMore = true;
    } else {
      userHistoryLoadingMore = false;
    }
    try {
      const historyResult = await invoke("get_live_history", { userId: String(userId), page });
      const lives = extractLives(historyResult);
      applyHistoryMeta(historyResult);
      selectedUserLiveHistory = append
        ? [...selectedUserLiveHistory, ...lives]
        : lives;
      userHistoryPage = userHistoryCurrentPage ?? page;
    } catch (e) {
      userHistoryError = e instanceof Error ? e.message : String(e);
    } finally {
      userHistoryLoadingMore = false;
    }
  };

  const openUserDetail = async (user: any) => {
    if (!user) return;
    document.querySelector(".search")?.scrollIntoView({ behavior: "smooth", block: "start" });
    selectedUser = user;
    selectedUserDetail = null;
    selectedUserLiveHistory = [];
    userHistoryPage = 1;
    userHistoryHasMore = true;
    userHistoryTotal = null;
    userHistoryCurrentPage = null;
    userHistoryNextPage = null;
    userHistoryPreviousPage = null;
    userDetailError = "";
    userHistoryError = "";
    const userId = getUserId(user);
    if (!userId) {
      userDetailError = "user_id が見つかりません";
      return;
    }
    userDetailLoading = true;
    try {
      const [profileResult, historyResult] = await Promise.allSettled([
        invoke("get_profile", { userId: String(userId) }),
        invoke("get_live_history", { userId: String(userId), page: 1 })
      ]);

      if (profileResult.status === "fulfilled") {
        selectedUserDetail = profileResult.value;
      } else {
        userDetailError =
          profileResult.reason instanceof Error
            ? profileResult.reason.message
            : String(profileResult.reason);
      }

      if (historyResult.status === "fulfilled") {
        const lives = extractLives(historyResult.value);
        selectedUserLiveHistory = lives;
        applyHistoryMeta(historyResult.value);
        userHistoryPage = userHistoryCurrentPage ?? 1;
      } else {
        userHistoryError =
          historyResult.reason instanceof Error
            ? historyResult.reason.message
            : String(historyResult.reason);
      }
    } finally {
      userDetailLoading = false;
    }
  };

  const closeUserDetail = () => {
    selectedUser = null;
    selectedUserDetail = null;
    selectedUserLiveHistory = [];
    userHistoryPage = 1;
    userHistoryHasMore = true;
    userHistoryLoadingMore = false;
    userHistoryTotal = null;
    userHistoryCurrentPage = null;
    userHistoryNextPage = null;
    userHistoryPreviousPage = null;
    userDetailError = "";
    userHistoryError = "";
    userDetailLoading = false;
  };

  const activeUser = $derived(selectedUserDetail ?? selectedUser);
  const activeUserId = $derived(getUserId(activeUser));
  const activeUserName = $derived(getUserName(activeUser));
  const activeUserAvatar = $derived(getUserAvatar(activeUser));
  const activeUserDesc = $derived(getUserDescription(activeUser));
  const activeUserLiveId = $derived(getUserLiveId(activeUser));
  const activeUserFollowers = $derived(
    pickFirstNumber(
      activeUser?.follower_num,
      activeUser?.followers_count,
      activeUser?.followers,
      activeUser?.user?.follower_num
    )
  );
  const activeUserFollowing = $derived(
    pickFirstNumber(
      activeUser?.following_num,
      activeUser?.followings_count,
      activeUser?.following,
      activeUser?.user?.following_num
    )
  );
  const activeUserLiveCount = $derived.by(() => {
    if (typeof userHistoryTotal === "number" && userHistoryTotal > 0) {
      return userHistoryTotal;
    }
    return pickFirstNumber(
      activeUser?.live_count,
      activeUser?.lives_count,
      activeUser?.user?.live_count
    );
  });
  const activeUserDetailRows = $derived.by(() => {
    if (!activeUser) return [];
    const rows = [
      { label: "登録日", value: formatUnix(activeUser?.registered_at) },
      {
        label: "総視聴者数",
        value: formatNumber(activeUser?.total_viewer_num, activeUser?.total_viewer_count)
      },
      {
        label: "連続配信記録",
        value: `${formatNumber(activeUser?.current_continuous_record)}/${formatNumber(
          activeUser?.max_continuous_record
        )}`
      },
      {
        label: "連続配信まで",
        value: formatNumber(activeUser?.remaining_days_for_continuous_streamer)
      },
      { label: "アプリ登録数", value: formatNumber(activeUser?.my_app_num) },
      { label: "配信リクエスト数", value: formatNumber(activeUser?.live_request_num) },
      { label: "Twitter", value: pickFirstString(activeUser?.twitter_screen_name) },
      {
        label: "誕生日",
        value: formatBirthday(activeUser?.birthday, activeUser?.is_visible_birthday)
      },
      { label: "ID", value: activeUserId || "-" }
    ];
    return rows.filter((row) => row.value && row.value !== "0");
  });

  const initObserver = () => {
    if (observer) observer.disconnect();
    if (mode === "user") return;
    observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          loadNext();
        }
      },
      { root: null, rootMargin: "200px", threshold: 0 }
    );
    if (sentinel) observer.observe(sentinel);
  };

  const initUserObserver = () => {
    if (userObserver) userObserver.disconnect();
    if (mode !== "user") return;
    userObserver = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          loadNext();
        }
      },
      { root: null, rootMargin: "200px", threshold: 0 }
    );
    if (userSentinel) userObserver.observe(userSentinel);
  };

  $effect(() => {
    if (sentinel) {
      initObserver();
    } else if (observer) {
      observer.disconnect();
    }
  });

  $effect(() => {
    if (userSentinel) {
      initUserObserver();
    } else if (userObserver) {
      userObserver.disconnect();
    }
  });

  $effect(() => {
    searchState.set({
      query,
      mode,
      results,
      searched,
      userHasMore,
      currentPage,
      nextPage,
      previousPage,
      totalEntries,
      currentCursor,
      nextCursor,
      recommendUsers,
      recommendPage,
      recommendHasMore,
      selectedUser,
      selectedUserDetail,
      selectedUserLiveHistory,
      userHistoryPage,
      userHistoryHasMore,
      userHistoryTotal,
      userHistoryCurrentPage,
      userHistoryNextPage,
      userHistoryPreviousPage,
      userDetailError,
      userHistoryError,
      error,
      recommendError
    });
  });

  onMount(() => {
    initObserver();
    if (mode === "user" && recommendUsers.length === 0) {
      void fetchRecommend(1, false);
    }
  });

  onDestroy(() => {
    if (observer) observer.disconnect();
    if (userObserver) userObserver.disconnect();
  });
</script>

<section class="search">
  <header class="section-head">
    <div>
      <p class="kicker">Search</p>
      <h2>{mode === "live" ? "配信を検索" : "ユーザーを検索"}</h2>
    </div>
  </header>

  <SearchControls
    mode={mode}
    query={query}
    loading={loading}
    onSwitchMode={switchMode}
    onSubmit={handleSubmit}
    onQueryChange={(value) => (query = value)}
  />

  {#if error}
    <ErrorMessage message={error} onRetry={runSearch} onDismiss={() => (error = "")} />
  {/if}

  {#if mode === "user" && (selectedUser || userDetailLoading || userDetailError)}
    <SearchUserDetail
      activeUser={activeUser}
      activeUserName={activeUserName}
      activeUserId={activeUserId}
      activeUserAvatar={activeUserAvatar}
      activeUserDesc={activeUserDesc}
      activeUserFollowers={activeUserFollowers}
      activeUserFollowing={activeUserFollowing}
      activeUserLiveCount={activeUserLiveCount}
      activeUserDetailRows={activeUserDetailRows}
      activeUserLiveId={activeUserLiveId}
      selectedUserLiveHistory={selectedUserLiveHistory}
      userHistoryLoadingMore={userHistoryLoadingMore}
      userHistoryHasMore={userHistoryHasMore}
      userHistoryTotal={userHistoryTotal}
      userHistoryCurrentPage={userHistoryCurrentPage}
      userHistoryNextPage={userHistoryNextPage}
      userHistoryPreviousPage={userHistoryPreviousPage}
      userDetailLoading={userDetailLoading}
      userDetailError={userDetailError}
      userHistoryError={userHistoryError}
      onRetry={() => selectedUser && openUserDetail(selectedUser)}
      onClearDetailError={() => (userDetailError = "")}
      onClearHistoryError={() => (userHistoryError = "")}
      onClose={closeUserDetail}
      onLoadMoreHistory={() => {
        const userId = activeUserId;
        if (!userId || userHistoryLoadingMore || !userHistoryHasMore) return;
        const next = userHistoryNextPage ?? userHistoryPage + 1;
        void fetchUserHistory(userId, next, true);
      }}
      onOpenLive={onOpenLive}
    />
  {/if}

  {#if mode === "user" && !searched}
    <SearchRecommend
      users={recommendUsers}
      loading={recommendLoading}
      loadingMore={recommendLoadingMore}
      error={recommendError}
      hasMore={recommendHasMore}
      onRefresh={() => fetchRecommend(1, false)}
      onSelect={openUserDetail}
      onOpenLive={onOpenLive}
      onLoadMore={loadMoreRecommend}
      onClearError={() => (recommendError = "")}
    />
  {/if}

  {#if mode !== "user" || searched}
    <SearchResults
      mode={mode}
      results={results}
      loading={loading}
      searched={searched}
      totalEntries={totalEntries}
      onOpenLive={onOpenLive}
      onSelectUser={openUserDetail}
    />
  {/if}

  {#if searched && (currentPage || totalEntries || loadingMore)}
    <footer class="pager">
      {#if currentPage}
        <span class="pager-pill">ページ {currentPage}</span>
      {/if}
      {#if totalEntries !== null}
        <span class="pager-pill">合計 {totalEntries.toLocaleString()}件</span>
      {/if}
      {#if loadingMore}
        <span class="pager-loading">
          <svg class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none"
               stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
          </svg>
          読み込み中...
        </span>
      {/if}
    </footer>
  {/if}

  {#if mode === "user" && searched}
    <div bind:this={userSentinel} style="height:1px" />
  {:else}
    <div bind:this={sentinel} style="height:1px" />
  {/if}
</section>

<style>
  .search {
    display: grid;
    gap: 16px;
    align-content: start;
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
    margin: 0;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: clamp(1.2rem, 2vw, 1.8rem);
  }

  .pager {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    font-size: 0.8rem;
    color: var(--ink-600);
  }

  .pager-pill {
    background: rgba(16, 27, 30, 0.06);
    padding: 4px 12px;
    border-radius: 999px;
    font-weight: 600;
    color: var(--ink-700);
  }

  .pager-loading {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--ink-500);
  }

  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
