/**
 * watch-comments.ts
 *
 * コメント管理のユーティリティ関数
 * コメントの識別キー生成とタイムスタンプ取得の純粋関数を提供する。
 * 重複排除・ソート順の推定など状態を持つロジックは WatchPage.svelte 側で管理する。
 */
import { pickNullableNumber } from "./watch-utils";

// ─────────────────────────────────────────────────────────────────────────────
// コメント識別
// ─────────────────────────────────────────────────────────────────────────────

/**
 * コメントの重複排除に使用するキーを返す。
 *
 * 優先度:
 *   1. comment_id / id / comment.id / commentId / comment_id_str
 *   2. フォールバック: "user_id|comment_text|created_at" の結合文字列
 *
 * 有効なキーが取得できない場合は null を返す（その場合は重複チェックをスキップ）。
 */
export const getCommentKey = (item: any): string | null => {
  const raw =
    item?.comment_id ??
    item?.id ??
    item?.comment?.id ??
    item?.commentId ??
    item?.comment_id_str ??
    null;
  if (raw !== null && raw !== undefined && raw !== "") return String(raw);

  // ID フィールドがない場合はユーザー・内容・時刻の組み合わせをキーとする
  const fallback = [
    item?.user_id ?? item?.user?.user_id ?? "",
    item?.comment ?? item?.message ?? "",
    item?.created_at ?? item?.createdAt ?? "",
  ]
    .join("|")
    .trim();
  return fallback || null;
};

// ─────────────────────────────────────────────────────────────────────────────
// タイムスタンプ取得
// ─────────────────────────────────────────────────────────────────────────────

/**
 * コメントオブジェクトからタイムスタンプ（Unix 秒）を取得する。
 * フィールド名が API バージョンによって異なるため複数のパスを試みる。
 * 取得できない場合は null を返す。
 */
export const getCommentTimestamp = (item: any): number | null =>
  pickNullableNumber(item?.created_at, item?.createdAt, item?.time, item?.timestamp);
