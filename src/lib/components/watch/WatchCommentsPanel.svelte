<script lang="ts">
  let {
    comments,
    commentText,
    commentTotal,
    onCommentChange,
    onSend
  } = $props<{
    comments: any[];
    commentText: string;
    commentTotal: number | null;
    onCommentChange: (value: string) => void;
    onSend: () => void;
  }>();

  const handleInput = (event: Event) => {
    const target = event.currentTarget as HTMLInputElement | null;
    onCommentChange(target?.value ?? "");
  };
</script>

<div class="comments">
  <div class="comments-head">
    <h3>コメント</h3>
    <span>
      {comments.length.toLocaleString()} 件
      {#if commentTotal !== null}
        / {commentTotal.toLocaleString()} 件
      {/if}
    </span>
  </div>
  <div class="comment-form">
    <input placeholder="コメントを入力" value={commentText} oninput={handleInput} />
    <button onclick={onSend}>送信</button>
  </div>
  <div class="comment-list">
    {#each comments as comment}
      <div class="comment">
        <span class="comment-user">
          {comment.user_name ?? comment.user?.name ?? comment.user?.username ?? "匿名"}
        </span>
        <span>{comment.comment ?? comment.message ?? ""}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .comments {
    display: grid;
    gap: 12px;
  }

  .comments-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .comment-form {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .comment-form input {
    flex: 1 1 240px;
  }

  .comment-list {
    display: grid;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
  }

  .comment {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 8px;
    font-size: 0.85rem;
    padding-bottom: 6px;
    border-bottom: 1px solid rgba(16, 27, 30, 0.1);
  }

  .comment-user {
    font-weight: 600;
  }

  input {
    flex: 1 1 240px;
    border: 1px solid rgba(16, 27, 30, 0.2);
    border-radius: 12px;
    padding: 10px 12px;
    background: #fff;
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 8px 16px;
    font-weight: 600;
    cursor: pointer;
    background: var(--accent-500);
    color: #fff;
  }

</style>
