/**
 * watch-broadcast.ts
 *
 * WebSocket ブロードキャストのメッセージ解析ユーティリティ
 * Mirrativ の bcsvr（ブロードキャストサーバー）から受信したメッセージを
 * UI 表示用のデータ構造に変換する純粋関数群。
 */
import { pickFirstString, pickNullableNumber } from "./watch-utils";

// ─────────────────────────────────────────────────────────────────────────────
// 型定義
// ─────────────────────────────────────────────────────────────────────────────

/** ブロードキャスト接続に必要な設定情報 */
export type BroadcastConfig = {
  /** bcsvr サーバーへの接続キー */
  bcsvrKey: string;
  /** ブロードキャストサーバーのホスト名 */
  host: string;
};

// ─────────────────────────────────────────────────────────────────────────────
// 設定抽出
// ─────────────────────────────────────────────────────────────────────────────

/**
 * liveInfo または streamStatus から bcsvr 接続設定を抽出する。
 * API バージョンによってフィールドの位置が異なるため複数のパスを試みる。
 * 必要な情報が揃わない場合は null を返す。
 */
export const extractBroadcastConfig = (info: any): BroadcastConfig | null => {
  const bcsvrKey = pickFirstString(
    info?.bcsvr_key,
    info?.broadcast_key,
    info?.live?.bcsvr_key,
    info?.live?.broadcast_key,
    info?.data?.bcsvr_key,
    info?.data?.broadcast_key
  );
  const host = pickFirstString(
    info?.broadcast_host,
    info?.live?.broadcast_host,
    info?.data?.broadcast_host
  );
  if (!bcsvrKey || !host) return null;
  return { bcsvrKey, host };
};

// ─────────────────────────────────────────────────────────────────────────────
// メッセージ変換
// ─────────────────────────────────────────────────────────────────────────────

/**
 * WS メッセージをコメントオブジェクトに変換する（t=1 のみ処理）。
 *
 * Mirrativ WS メッセージタイプ:
 *   t=1   : ユーザーコメント（テキスト）
 *   t=3   : システム通知（入室など）
 *   t=38  : キープアライブ（無視）
 *   t=123 : 配信終了通知
 *
 * t=1 以外は null を返す。
 */
export const toBroadcastComment = (msg: any) => {
  if (!msg || typeof msg !== "object") return null;

  // メッセージタイプを正規化（t または type フィールド）
  const msgType = pickNullableNumber(msg?.t, msg?.type);
  if (msgType !== 1) return null;

  const text = pickFirstString(msg?.cm, msg?.comment, msg?.speech, msg?.message);
  if (!text) return null;

  return {
    comment_id: msg.lci ?? msg.comment_id,
    user_id: msg.u ?? msg.user_id,
    user_name: msg.ac ?? msg.user_name ?? "",
    comment: text,
    created_at: msg.created_at ?? msg.createdAt,
    profile_image_url: msg.iurl ?? msg.profile_image_url ?? "",
    is_moderator: msg.is_moderator ?? 0,
    is_cheerleader: msg.is_cheerleader ?? 0,
    vip_rank: msg.vip_rank ?? 0,
    yell_rank: msg.yell_rank ?? 0,
    yell_level: msg.yell_level ?? 0,
    profile_frame_image_url: msg.profile_frame_image_url ?? "",
    push_image_url: msg.push_image_url ?? "",
    _raw: msg,
  };
};

/**
 * WS メッセージをシステム通知オブジェクトに変換する（t=3 のみ処理）。
 * ユーザー情報が含まれる場合は入室通知として扱い、
 * 含まれない場合はメッセージテキストをそのまま表示する。
 * t=3 以外は null を返す。
 */
export const toBroadcastSystemNotice = (msg: any) => {
  if (!msg || typeof msg !== "object") return null;

  const msgType = pickNullableNumber(msg?.t, msg?.type);
  if (msgType !== 3) return null;

  const userName = pickFirstString(msg?.ac, msg?.user_name);
  const userId = pickFirstString(msg?.u, msg?.user_id);
  const profileImageUrl = pickFirstString(msg?.iurl, msg?.profile_image_url);

  // ユーザー情報がある場合は入室通知として表示
  const text = userName
    ? `${userName} が入室しました`
    : pickFirstString(msg?.cm, msg?.message, msg?.speech, msg?.notice_text, msg?.text) ||
      "入室通知";

  const viewers = pickNullableNumber(
    msg?.online_viewer_num,
    msg?.online_user_num,
    msg?.viewer_num
  );
  const createdAt =
    pickNullableNumber(msg?.created_at, msg?.createdAt) ?? Math.floor(Date.now() / 1000);
  const commentId = pickFirstString(msg?.lci, msg?.comment_id);
  // 重複排除のためのキー（comment_id があればそれを使い、なければ時刻+ユーザー+テキストで生成）
  const key = commentId || `${createdAt}:${userId || ""}:${text}`;

  return {
    key,
    type: "join",
    text,
    userName,
    userId,
    profileImageUrl,
    viewers,
    created_at: createdAt,
    _raw: msg,
  };
};
