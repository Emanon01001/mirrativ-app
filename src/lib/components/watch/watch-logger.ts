/**
 * watch-logger.ts
 *
 * 構造化ロガー
 * フロントエンドのログを Tauri の frontend_log コマンド経由でターミナルへ転送する。
 * タグによりどのサブシステム（WS/HLS/API/JOIN）からの出力かを識別できる。
 */
import { invoke } from "@tauri-apps/api/core";

// ─────────────────────────────────────────────────────────────────────────────
// 型定義
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ログのサブシステムタグ
 * - ws  : WebSocket / ブロードキャスト関連
 * - hls : HLS ストリーム / MPV プレイヤー関連
 * - api : REST API 呼び出し関連
 * - join: 配信参加フロー関連
 * - relay: LLStream relay 関連
 */
export type LogTag = "ws" | "hls" | "api" | "join" | "relay";

// ─────────────────────────────────────────────────────────────────────────────
// 内部送信関数
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ログを Rust 側へ送信する内部関数。
 * 追加引数を JSON 文字列化してメッセージに結合する。
 */
const _emit = (level: string, tag: LogTag, msg: string, args: unknown[]) => {
  const extra = args.length
    ? " " +
      args
        .map((a) => {
          try {
            return typeof a === "string" ? a : JSON.stringify(a);
          } catch {
            return String(a);
          }
        })
        .join(" ")
    : "";
  invoke("frontend_log", { level, tag, message: msg + extra }).catch(() => {});
};

// ─────────────────────────────────────────────────────────────────────────────
// 公開ロガー関数
// ─────────────────────────────────────────────────────────────────────────────

/** INFO レベルのログを出力する */
export const log = (tag: LogTag, msg: string, ...args: unknown[]) =>
  _emit("info", tag, msg, args);

/** WARN レベルのログを出力する */
export const logWarn = (tag: LogTag, msg: string, ...args: unknown[]) =>
  _emit("warn", tag, msg, args);

/** ERROR レベルのログを出力する */
export const logErr = (tag: LogTag, msg: string, ...args: unknown[]) =>
  _emit("error", tag, msg, args);
