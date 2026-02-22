<script lang="ts">
  let {
    comments,
    systemNotices = [],
    commentText,
    commentTotal,
    broadcastConnected = false,
    commentCoolingDown = false,
    commentCooldownMs = 500,
    onCommentChange,
    onSend,
    onCooldownMsChange
  } = $props<{
    comments: any[];
    systemNotices?: any[];
    commentText: string;
    commentTotal: number | null;
    broadcastConnected?: boolean;
    commentCoolingDown?: boolean;
    commentCooldownMs?: number;
    onCommentChange: (value: string) => void;
    onSend: () => void;
    onCooldownMsChange?: (value: number) => void;
  }>();

  const handleInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    onCommentChange(target?.value ?? "");
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      onSend();
    }
  };

  const handleCooldownInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    const value = Number(target?.value ?? 500);
    if (Number.isFinite(value) && value >= 0) {
      onCooldownMsChange?.(value);
    }
  };

  const toNum = (value: unknown) => {
    if (typeof value === "number" && Number.isFinite(value)) return value;
    if (typeof value === "string" && value.trim()) {
      const n = Number(value);
      if (Number.isFinite(n)) return n;
    }
    return 0;
  };

  const formatCommentTime = (value: unknown) => {
    const unix = toNum(value);
    if (!unix) return "";
    const date = new Date(unix * 1000);
    if (Number.isNaN(date.getTime())) return "";
    return date.toLocaleTimeString();
  };
</script>

<div class="comments">
  <div class="comments-head">
    <h3>
      コメント
      {#if broadcastConnected}
        <span class="ws-badge" title="WebSocket リアルタイム接続中">LIVE</span>
      {/if}
    </h3>
    <span class="count">
      {comments.length.toLocaleString()} 件
      {#if commentTotal !== null}
        / {commentTotal.toLocaleString()} 件
      {/if}
    </span>
  </div>
  <div class="comment-form">
    <input
      placeholder="コメントを入力（Enterで送信）"
      value={commentText}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      disabled={commentCoolingDown}
    />
    <button onclick={onSend} disabled={commentCoolingDown}>
      {commentCoolingDown ? "待機中…" : "送信"}
    </button>
  </div>
  <div class="cooldown-config">
    <label class="cooldown-label">
      <span>送信間隔</span>
      <input
        type="number"
        class="cooldown-input"
        min="0"
        max="10000"
        step="100"
        value={commentCooldownMs}
        oninput={handleCooldownInput}
      />
      <span class="cooldown-unit">ms</span>
    </label>
  </div>
  {#if systemNotices.length}
    <div class="system-list">
      {#each systemNotices as notice (notice.key)}
        <div class="system-item">
          {#if notice.type === "end"}
            <span class="system-label end-label">END</span>
            <span class="end-text">配信が終了しました</span>
          {:else if notice.userName}
            <span class="system-label join-label">JOIN</span>
            {#if notice.profileImageUrl}
              <img class="join-avatar" src={notice.profileImageUrl} alt="" />
            {/if}
            <span class="join-name">{notice.userName}</span>
            <span>が入室しました</span>
          {:else}
            <span class="system-label">SYS</span>
            <span>{notice.text ?? "入室通知"}</span>
          {/if}
          {#if toNum(notice.viewers) > 0}
            <span class="viewer-count">{toNum(notice.viewers).toLocaleString()} 視聴中</span>
          {/if}
          {#if formatCommentTime(notice.created_at)}
            <span>{formatCommentTime(notice.created_at)}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
  <div class="comment-list">
    {#each comments as comment}
      <div class="comment">
        <span class="comment-user">
          {comment.user_name ?? comment.user?.name ?? comment.user?.username ?? "匿名"}
        </span>
        <div class="comment-body">
          <span>{comment.comment ?? comment.message ?? ""}</span>
          <div class="comment-meta">
            {#if formatCommentTime(comment.created_at)}
              <span>{formatCommentTime(comment.created_at)}</span>
            {/if}
            {#if toNum(comment.vip_rank) > 0}
              <span>VIP {toNum(comment.vip_rank)}</span>
            {/if}
            {#if toNum(comment.yell_level) > 0}
              <span>チアリーダーレベル: {toNum(comment.yell_level)}</span>
            {/if}
            {#if toNum(comment.is_moderator) > 0}
              <span>MOD</span>
            {/if}
          </div>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .comments {
    position: relative;
    display: grid;
    gap: 12px;
  }

  .comments-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .comments-head h3 {
    margin: 0;
    font-size: 0.98rem;
    letter-spacing: 0.02em;
  }

  .count {
    font-size: 0.78rem;
    color: var(--ink-600);
    font-weight: 600;
  }

  .ws-badge {
    display: inline-block;
    font-size: 0.6rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: #e53e3e;
    color: #fff;
    vertical-align: middle;
    margin-left: 6px;
    letter-spacing: 0.5px;
  }

  .comment-form {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    padding: 8px;
    border-radius: 12px;
    background: rgba(16, 27, 30, 0.04);
    border: 1px solid rgba(16, 27, 30, 0.1);
  }

  .comment-form input {
    flex: 1 1 240px;
  }

  .comment-form input:disabled,
  .comment-form button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .cooldown-config {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cooldown-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.74rem;
    color: var(--ink-600);
  }

  .cooldown-input {
    width: 70px;
    flex: 0 0 auto;
    padding: 4px 6px;
    font-size: 0.74rem;
    border: 1px solid rgba(16, 27, 30, 0.2);
    border-radius: 8px;
    background: #fff;
    text-align: center;
  }

  .cooldown-unit {
    font-size: 0.72rem;
    color: var(--ink-500);
  }

  .system-list {
    display: grid;
    gap: 6px;
    max-height: 120px;
    overflow-y: auto;
    padding: 6px;
    border-radius: 10px;
    background: rgba(16, 27, 30, 0.035);
    border: 1px solid rgba(16, 27, 30, 0.07);
  }

  .system-item {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    font-size: 0.74rem;
    color: rgba(16, 27, 30, 0.72);
  }

  .system-label {
    display: inline-block;
    border-radius: 999px;
    background: rgba(16, 27, 30, 0.08);
    padding: 1px 7px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .join-label {
    background: rgba(56, 161, 105, 0.15);
    color: #276749;
  }

  .end-label {
    background: rgba(229, 62, 62, 0.15);
    color: #c53030;
  }

  .end-text {
    font-weight: 600;
    color: #c53030;
  }

  .join-avatar {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .join-name {
    font-weight: 600;
  }

  .viewer-count {
    opacity: 0.7;
  }

  .comment-list {
    display: grid;
    gap: 7px;
    max-height: 280px;
    overflow-y: auto;
  }

  .comment {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 8px;
    font-size: 0.85rem;
    border-radius: 12px;
    padding: 9px 10px;
    background: rgba(16, 27, 30, 0.045);
    border: 1px solid rgba(16, 27, 30, 0.08);
  }

  .comment-user {
    font-weight: 600;
  }

  .comment-body {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .comment-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 0.72rem;
    color: rgba(16, 27, 30, 0.6);
  }

  input {
    flex: 1 1 240px;
    border: 1px solid rgba(16, 27, 30, 0.2);
    border-radius: 12px;
    padding: 9px 11px;
    background: #fff;
  }

  button {
    border: 1px solid transparent;
    border-radius: 999px;
    padding: 8px 16px;
    font-weight: 700;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 14px rgba(15, 42, 39, 0.14);
  }

  @media (max-width: 720px) {
    .comment {
      grid-template-columns: 1fr;
      gap: 6px;
    }
  }
</style>
