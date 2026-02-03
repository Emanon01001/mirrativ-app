<script lang="ts">
  let { mode, query, loading, onSwitchMode, onSubmit, onQueryChange } = $props<{
    mode: "live" | "user";
    query: string;
    loading: boolean;
    onSwitchMode: (mode: "live" | "user") => void;
    onSubmit: (event: Event) => void;
    onQueryChange: (value: string) => void;
  }>();

  let focused = $state(false);

  const handleInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    onQueryChange(target?.value ?? "");
  };
</script>

<div class="search-controls">
  <div class="search-tabs">
    <button class:active={mode === "live"} onclick={() => onSwitchMode("live")}>
      配信
    </button>
    <button class:active={mode === "user"} onclick={() => onSwitchMode("user")}>
      ユーザー
    </button>
  </div>

  <form class="search-bar" class:focused onsubmit={onSubmit}>
    <svg class="search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none"
         stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="11" cy="11" r="8"/>
      <line x1="21" y1="21" x2="16.65" y2="16.65"/>
    </svg>
    <input
      type="text"
      placeholder={mode === "live" ? "例: 雑談, 歌, ゲーム" : "例: 名前, ID"}
      value={query}
      oninput={handleInput}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      aria-label={mode === "live" ? "配信検索" : "ユーザー検索"}
    />
    {#if query}
      <button type="button" class="clear-btn" onclick={() => onQueryChange("")} aria-label="クリア">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    {/if}
    <button type="submit" class="submit-btn" disabled={loading}>
      {loading ? "検索中..." : "検索"}
    </button>
  </form>
</div>

<style>
  .search-controls {
    display: grid;
    gap: 10px;
  }

  .search-tabs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .search-tabs button {
    border: 1px solid rgba(16, 27, 30, 0.18);
    background: rgba(16, 27, 30, 0.04);
    color: var(--ink-700);
    border-radius: 999px;
    padding: 6px 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease, border-color 0.2s ease;
  }

  .search-tabs button.active {
    background: var(--accent-500);
    color: #fff;
    border-color: transparent;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 0;
    border: 1.5px solid rgba(16, 27, 30, 0.15);
    border-radius: 999px;
    padding: 4px 4px 4px 14px;
    background: var(--card-surface);
    box-shadow: var(--shadow-soft);
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .search-bar.focused {
    border-color: var(--accent-500);
    box-shadow: 0 0 0 3px rgba(242, 95, 76, 0.15);
  }

  .search-icon {
    flex-shrink: 0;
    color: var(--ink-500);
    margin-right: 8px;
  }

  .search-bar input {
    border: none;
    background: transparent;
    flex: 1;
    min-width: 0;
    padding: 6px 0;
    font-size: 0.9rem;
    line-height: 1.4;
    outline: none;
    color: inherit;
    font-family: inherit;
  }

  .search-bar input::placeholder {
    color: var(--ink-500);
  }

  .clear-btn {
    display: grid;
    place-items: center;
    border: none;
    background: rgba(16, 27, 30, 0.08);
    color: var(--ink-600);
    border-radius: 50%;
    width: 28px;
    height: 28px;
    cursor: pointer;
    flex-shrink: 0;
    margin-right: 4px;
    transition: background 0.2s ease;
  }

  .clear-btn:hover {
    background: rgba(16, 27, 30, 0.14);
  }

  .submit-btn {
    border: none;
    border-radius: 999px;
    padding: 8px 18px;
    font-weight: 700;
    background: var(--accent-500);
    color: #fff;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.2s ease;
  }

  .submit-btn:hover:not(:disabled) {
    background: var(--accent-700);
  }

  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
