<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";

  type Live = {
    started_at?: number;
    title?: string;
    live_id?: string;
    owner?: { name?: string };
    preview?: { streaming_url_hls?: string };
  };

  type FollowLog = {
    at: string;
    message: string;
  };

  let { mrId, unique, authed, user, catalogTabs } = $props<{
    mrId: string;
    unique: string;
    authed: boolean;
    user: any;
    catalogTabs: unknown;
  }>();

  let now = $state(new Date());
  let followCursor = $state("");
  let followLoading = $state(false);
  let followError = $state("");
  let followResponse = $state<any>(null);
  let followLogs = $state<FollowLog[]>([]);

  const lives = $derived.by(() => {
    const source = catalogTabs as any;
    const list =
      source?.lives ??
      source?.data?.lives ??
      source?.tab_list?.[0]?.lives ??
      source?.tabs?.[0]?.lives ??
      [];
    return Array.isArray(list) ? (list as Live[]) : [];
  });

  const first = $derived(lives[0]);

  const startedAtUnix = $derived(first?.started_at ?? null);

  const startedAtText = $derived(
    startedAtUnix != null
      ? new Date(startedAtUnix * 1000).toLocaleString("ja-JP")
      : "-"
  );

  const followResponseJson = $derived(
    followResponse ? JSON.stringify(followResponse, null, 2) : ""
  );

  const countFollowLives = (source: any): number => {
    const list =
      source?.list ??
      source?.lives ??
      source?.live_list ??
      source?.data?.list ??
      source?.data?.lives ??
      source?.data?.live_list ??
      source?.data ??
      [];
    return Array.isArray(list) ? list.length : 0;
  };

  const followLiveCount = $derived.by(() => countFollowLives(followResponse));

  const timer = setInterval(() => {
    now = new Date();
  }, 1000);

  const pad = (n: number) => String(n).padStart(2, "0");

  const formatted = $derived(
    `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ` +
      `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`
  );

  const addFollowLog = (message: string) => {
    followLogs = [
      { at: new Date().toLocaleTimeString("ja-JP"), message },
      ...followLogs
    ].slice(0, 200);
  };

  const handleFollowCursorInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    followCursor = target?.value ?? "";
  };

  const clearFollowLogs = () => {
    followLogs = [];
  };

  const fetchCatalogFollow = async () => {
    followLoading = true;
    followError = "";

    const cursor = followCursor.trim();
    addFollowLog(`[ui] invoke get_catalog_follow cursor=${cursor || "-"}`);

    try {
      const payload = await invoke<any>("get_catalog_follow", {
        cursor: cursor || null
      });
      followResponse = payload;
      addFollowLog(`[ui] success lives=${countFollowLives(payload)}`);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      followError = message;
      addFollowLog(`[ui] error ${message}`);
    } finally {
      followLoading = false;
    }
  };

  onMount(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    void listen<string>("catalog_follow://log", (event) => {
      addFollowLog(`[rust] ${event.payload}`);
    }).then((dispose) => {
      if (!active) {
        dispose();
        return;
      }
      unlisten = dispose;
    });

    return () => {
      active = false;
      if (unlisten) unlisten();
    };
  });

  onDestroy(() => {
    clearInterval(timer);
  });
</script>

<section class="testpage">
  <h2>Debug</h2>
  <h2>時間: {formatted}</h2>
  <p>状態: {authed ? "ログイン済み" : "ゲスト"}</p>
  <p>mr_id: {mrId}</p>
  <p>f(unique): {unique}</p>
  <p>user: {user?.name ?? "-"}</p>

  <p>lives: {lives.length} 件</p>
  <p>first title: {first?.title ?? "-"}</p>
  <p>first owner: {first?.owner?.name ?? "-"}</p>
  <p>started_at (unix): {startedAtUnix ?? "-"}</p>
  <p>started_at (local): {startedAtText}</p>

  <div class="follow-panel">
    <h3>get_catalog_follow debug</h3>
    <div class="row">
      <input
        placeholder="cursor (optional)"
        value={followCursor}
        oninput={handleFollowCursorInput}
      />
      <button type="button" onclick={fetchCatalogFollow} disabled={followLoading}>
        {followLoading ? "取得中..." : "get_catalog_follow"}
      </button>
      <button type="button" class="ghost" onclick={clearFollowLogs} disabled={followLogs.length === 0}>
        ログクリア
      </button>
    </div>

    <p>response lives: {followResponse ? followLiveCount : "-"}</p>

    {#if followError}
      <p class="error">follow error: {followError}</p>
    {/if}

    <details>
      <summary>debug logs ({followLogs.length})</summary>
      <div class="logs">
        {#if followLogs.length === 0}
          <p class="log-empty">ログはまだありません</p>
        {:else}
          {#each followLogs as entry}
            <p class="log-line">
              <span class="log-time">{entry.at}</span>
              <span>{entry.message}</span>
            </p>
          {/each}
        {/if}
      </div>
    </details>

    <details>
      <summary>get_catalog_follow response</summary>
      <pre>{followResponseJson}</pre>
    </details>
  </div>

  <details>
    <summary>catalogTabs raw</summary>
    <pre>{JSON.stringify(catalogTabs, null, 2)}</pre>
  </details>
</section>

<style>
  .testpage {
    display: grid;
    gap: 8px;
    padding: 16px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.8);
    border: 1px solid rgba(16, 27, 30, 0.12);
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  .follow-panel {
    display: grid;
    gap: 8px;
    margin-top: 8px;
    padding: 12px;
    border-radius: 10px;
    border: 1px solid rgba(16, 27, 30, 0.16);
    background: rgba(255, 255, 255, 0.72);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  input {
    min-width: 260px;
    border: 1px solid rgba(16, 27, 30, 0.24);
    border-radius: 8px;
    padding: 8px 10px;
    background: #fff;
    color: var(--ink-900);
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 8px 14px;
    font-weight: 700;
    background: var(--accent-500);
    color: #fff;
    cursor: pointer;
  }

  .ghost {
    background: rgba(16, 27, 30, 0.08);
    color: var(--ink-700);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error {
    color: var(--accent-700);
    font-weight: 600;
  }

  .logs {
    display: grid;
    gap: 6px;
    max-height: 240px;
    overflow: auto;
    padding: 10px;
    border-radius: 8px;
    background: rgba(16, 27, 30, 0.9);
  }

  .log-line {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    color: #f3f6f7;
    font-size: 0.76rem;
    line-height: 1.45;
  }

  .log-time {
    color: rgba(243, 246, 247, 0.65);
    min-width: 78px;
  }

  .log-empty {
    margin: 0;
    color: rgba(243, 246, 247, 0.65);
    font-size: 0.76rem;
  }

  pre {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px;
    border-radius: 8px;
    background: rgba(16, 27, 30, 0.9);
    color: #f3f6f7;
    font-size: 0.76rem;
    line-height: 1.45;
  }
</style>
