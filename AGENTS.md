# Mirrativ Desktop Studio (Tauri + Svelte) — agent.md

このドキュメントは「スマホ向け配信アプリ、ミラティブのPCソフト作成(tauri app)」の全体像を、開発/保守/機能追加を担当するエージェント向けにまとめたものです。

---

## 1. 目的 / スコープ

- **Mirrativ（ミラティブ）APIを叩いて**、配信の発見（Catalog）、フォロー、視聴、コメント、プロフィール閲覧などを **デスクトップUIで再現**する。
- 視聴は基本 **HLS**（`streaming_url_hls` 等）を再生。
- HLSが不安定/使えないケース向けに、Mirrativが返す **LLStream（WebSocket）** を **ローカルHLSへリレー**してブラウザ側（hls.js）で再生できるようにする（PoC/デバッグ）。

---

## 2. 技術スタック

- **Tauri**（Rust backend / WebView frontend）
- **Rust**
  - `reqwest`（Cookie store付きHTTPクライアント）
  - `tokio`, `tokio_tungstenite`（LLStream WS受信）
  - `serde_json`（APIレスポンスをValueで扱う）
  - `libloading`（libmvpの動的ライブラリを使用するため）
- **Svelte（runes構文） + TypeScript**
  - `@tauri-apps/api/core` の `invoke` でRustコマンドを呼ぶ
  - `libmpv` でHLS再生

---

## 3. リポジトリ構造（主要ファイル）

> ※ここでは、今回確認できたファイル群（抜粋）に基づいて整理しています。

### 3.1 Rust（Tauri側）

- `src-tauri/src/main.rs`
  - `mirrativ_app_lib::run()` を呼ぶだけのエントリポイント。

- `src-tauri/src/lib.rs`
  - モジュール登録と **Tauri command 登録の中心**。
  - `MirrativClient`（APIクライアント）と `VideoHlsManager / AudioHlsManager`（LLStream→HLSリレー）を `manage()` している。

- `src-tauri/src/mirrativ/mod.rs`
  - `pub mod client;` + `MirrativClient` のre-export。

- `src-tauri/src/mirrativ/client.rs`
  - Mirrativ APIクライアント実装（reqwest + Cookie）
  - **GET/POSTのTauri commandが大量に定義**されている。
  - `join_live` は複数エンドポイントを並列に叩き、最後に `type=3` の `live_comment` を投げる。

- `src-tauri/src/llstream_video.rs`
  - **LLStream video (WS) → ffmpeg → HLS(ts) → ローカルHTTPサーバで配信**。
  - WSの `MR` ヘッダ（18 bytes）を剥がし、payloadのNALをAnnexBへ整形してffmpegへ `pipe:0` で流す。

- `src-tauri/src/llstream_audio.rs`
  - **LLStream audio (WS) → ffmpeg → HLS(fMP4) → ローカルHTTPサーバで配信**。
  - LOAS/LATMのsync（`0x56` + 次byte上位3bitが`111`）を探して ffmpeg に `-f loas` として流す。
  - ffmpeg stderr は `ffmpeg.log` に保存して、UIから参照できるようにしている。

### 3.2 Frontend（Svelte側）

- `LoginPage.svelte`
  - Cookieの `mr_id` と `f(unique)` を入力してセッション開始するUI（Android側から取得して貼る想定）。

- `Sidebar.svelte`
  - ページ遷移UI + 通知表示
  - **Debug: WS → HLS リレー** の操作UI（video/audio）を持つ  
    - `start_llstream_video_hls` / `start_llstream_audio_hls` を `invoke`
    - 返ってきた `playlist_url` を `llstreamRelayUrl` storeへ入れる

- `HomePage.svelte`
  - catalog tabs / banners / lives を表示。`LiveCard`クリックで視聴へ。

- `FollowPage.svelte`
  - `get_catalog_follow` でフォロー中の配信一覧。

- `WatchPage.svelte`
  - `join_live`, `get_live_info`, `get_live_status` などで視聴準備
  - `streamUrl`（通常HLS or llstreamRelayUrl）を `hls.js` で再生
  - 再生監視（watchdog）とリカバリー（再接続）ロジックあり
  - コメント取得/送信、ギフトランキング、ポーリング取得もここに集約

- `ProfilePage.svelte`
  - 自分のプロフィール/通貨/履歴（配信・視聴）を表示。

- `LiveCard.svelte`
  - サムネ、配信者、視聴者数、タグ等のカードUI。

---

## 4. 主要データフロー

### 4.1 認証（セッション）

- 基本は **AndroidアプリのCookie値を流用**する方式。
  - `mr_id` と `f`（unique）を入力 → Rust側 `login(mr_id, unique)` を呼ぶ想定
- `MirrativClient` は `reqwest::Client` を **cookie_store有効で保持**しつつ、必要に応じて `Cookie:` ヘッダも組み立てる。

### 4.2 配信一覧（Catalog）

- `HomePage`:
  - `bootstrap_guest` → `get_catalog_tabs` → `get_catalog_lives` / `get_catalog_banners`
- `FollowPage`:
  - `get_catalog_follow`

### 4.3 視聴（HLS）

- `WatchPage`:
  - `join_live`（軽く叩いて視聴状態を作る）
  - `get_live_info`, `get_live_status`（HLS URLを得る）
  - `streamUrl = llstreamRelayUrl || streaming_url_hls || preview...` を hls.js で再生

### 4.4 視聴（LLStream WS → ローカルHLSリレー）

- UI（`Sidebar` Debug）で WS URL を入力し、Rustへrelay開始指示
- Rust側はローカル一時ディレクトリを作り、
  - `ffmpeg` を起動（stdinにWS payloadを流し、HLSを生成）
  - 同じディレクトリを簡易HTTPサーバで配信
  - UIへ `http://127.0.0.1:<port>/index.m3u8` を返す
- UI側は `llstreamRelayUrl` を `streamUrl` として扱い、hls.js で再生

---

## 5. Tauri Commands（概観）

`src-tauri/src/lib.rs` で登録されている主なコマンド群：

- ユーザー: `get_profile`, `get_my_profile`, `bootstrap_guest`, `get_user_currency`
- 配信: `get_live_info`, `get_live_status`, `get_comments`, `get_online_users`, `get_collaborators`
- 履歴: `get_live_history`, `get_view_history`
- カタログ/ディスカバリー: `get_catalog`, `get_catalog_tabs`, `get_catalog_lives`, `get_catalog_banners`, `get_catalog_follow`, `get_recommend_live`, `get_onlive_apps`, `get_recommend_apps`
- ギフト/ランキング: `get_gift_ranking`, `get_coin_box_status`, `get_ranking_user_detail`, `get_season_rating`, `get_season_yell_status`
- 通知: `get_notice_counts`, `get_notice_popups`
- ミッション: `get_mission_status`
- ソーシャル: `get_urge_users`, `get_chat_threads`, `get_my_app`
- イベント: `get_event_notice`
- POST: `comment`, `follow`, `unfollow`, `request_live`, `leave_live`, `profile_edit`, `post_demographic`, `live_polling`
- 認証: `login`, `reset_session`
- 複雑: `join_live`

---

## 6. LLStream実装メモ（重要）

### 6.1 MRフレーム

- WSから来る `Binary` payload は先頭に **"MR"** を持つ独自ヘッダがあり、現在は **18 bytes固定**として扱っている。
- `bytes[2]` をタイプとして判定:
  - `0x01` = video
  - `0x02` = audio

### 6.2 Video payload（H.264）

- payloadがAnnexB開始コード (`00 00 01` or `00 00 00 01`) を持つ場合はそのまま
- そうでない場合は「**3バイト長のNAL length prefix**」と仮定し、AnnexBへ変換してffmpegへ投入

ffmpegは以下に近い形で起動（copyでHLS化）:
- `-f h264 -i pipe:0 -c:v copy -f hls ... index.m3u8`

### 6.3 Audio payload（LOAS/LATM → AAC）

- LOAS/LATM sync を探して ffmpeg stdinへ流す（`-f loas`）
- HLSは `fmp4`（init.mp4 + segXXXX.m4s）

---

## 7. 開発・実行メモ

### 7.1 ffmpeg の扱い

- ffmpeg の探索順（video/audio共通）
  1. 環境変数 `MIRRATIV_FFMPEG_PATH`
  2. Tauriの `resource_dir` 内に `ffmpeg*` があればそれ
  3. 最後に `ffmpeg(.exe)` をPATHから探す

### 7.2 ログ

- audio relayは `ffmpeg.log` にstderrを追記保存し、UIへパスを返す。

---

## 8. 既知の課題 / 注意点

- **HLSが不安定な配信がある**  
  `WatchPage` には再接続・watchdogがあるが、根本原因（セグメント欠落/HTTP周り/ストール）は要調査。
- **音声LOAS/LATMの互換性**  
  ストリームによっては ffmpeg が `aac_latm` で未実装機能に当たる可能性がある（ログで確認し、入力形式の再検討が必要）。
- **videoとaudioを別々にリレーしている**  
  現状は「video-only」「audio-only」デバッグ。最終的に **A/V同時**（mux）で1本のHLSにしたい。

---

## 9. 次にやるなら（優先順の叩き台）

1. **LLStream A/V 同時リレー（mux）**
   - video (h264) + audio (loas/latm) を受けて **1本のHLS** にする
   - まずは ffmpeg に2入力（pipeを2本 or 一時fifo/localhost tcp）を与える構成を検討

2. **再生基盤の強化**
   - hls.jsの設定（buffer/timeout/retry）を調整
   - 例外時にURL再取得（`get_live_status` 再叩き）をより強化

3. **mpv/libmpv埋め込み**
   - WebViewの限界（HLS.jsの不安定さ）を避けるため、プレイヤー専用ウィンドウ案を含めて検討
   - Tauri側で動画ウィンドウ（ネイティブ）を扱う方針を固める

4. **セッション/UIの完成度**
   - ログイン情報の安全な保存（remember）
   - エラーメッセージの整理、ユーザーフローの安定化

---

## 10. 変更時の指針（エージェント用）

- UI ↔ Rust は `invoke()` が境界。  
  追加APIは原則 `client.rs` に `#[tauri::command]` として追加し、`lib.rs` の `invoke_handler!` に登録する。
- WSリレーは「停止・再起動・クリーンアップ」が重要。
  - `stop_tx` で停止通知
  - ffmpeg子プロセスのkill
  - temp dir削除
  - UI storeのクリア（`llstreamRelayUrl`, `llstreamError`）
- 互換性のないストリームを見つけたら、まずは **ffmpeg.log / WS payloadの先頭数十バイト** を取得して分類する。

---

## 付録: 今回参照したファイル一覧（この環境にあるもの）

- `/mnt/data/main.rs`
- `/mnt/data/lib.rs`
- `/mnt/data/mod.rs`
- `/mnt/data/client.rs`
- `/mnt/data/llstream_video.rs`
- `/mnt/data/llstream_audio.rs`
- `/mnt/data/LoginPage.svelte`
- `/mnt/data/Sidebar.svelte`
- `/mnt/data/HomePage.svelte`
- `/mnt/data/FollowPage.svelte`
- `/mnt/data/WatchPage.svelte`
- `/mnt/data/ProfilePage.svelte`
- `/mnt/data/LiveCard.svelte`
- `/mnt/data/WatchTab.svelte`, `/mnt/data/ProfileTab.svelte`, `/mnt/data/LiveTab.svelte`（UI構成補助）
