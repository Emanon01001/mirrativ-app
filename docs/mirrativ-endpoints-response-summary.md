# Mirrativ Endpoint & Response Summary

作成日: 2026-02-18

## 1. 収集元
- `src-tauri/src/mirrativ/client/*.rs` (Tauri command -> endpoint)
- `src/lib/components/watch/watch-utils.ts` (レスポンス正規化/参照フィールド)
- `src/lib/components/watch/watch-broadcast.ts` (WSメッセージ項目)
- `src/lib/components/WatchPage.svelte` (join/watchフローで実際に使用している項目)

## 2. 主要レスポンス項目（視聴系）

### 2.1 HLS URL (`get_live_status`)
- URL候補の優先順: `streaming_url_hls` -> `streaming_url` -> `hls_url` -> `playlist_url` -> `streaming_url_list[]`
- 配信によっては `bcsvr_key` / `broadcast_host` が含まれ、WSコメント接続に使う

### 2.2 Live情報 (`get_live_info`, `live_polling`)
- 主な利用項目: `title`, `owner.name`, `total_viewer_num`, `online_user_num`, `comment_num`, `started_at`, `app_title`, `collab_has_vacancy`, `is_live`, `ended_at`, `star_num`, `gift_num`, `live_id`
- ギフト/ユーザー補助: `gift_ranking_url`, `obfuscated_user_id`（polling/liveInfo/URLクエリから解決）

### 2.3 コメント (`get_comments`, WS)
- REST配列候補: `comments` / `live_comments` / `data`
- WS `t=1`: コメント (`cm`, `comment`, `speech`, `message`)
- WS `t=3`: 入室通知、`t=123`: 配信終了通知、`t=38`: keepalive

### 2.4 ランキング (`get_gift_ranking`, `get_gift_ranking_by_url`)
- 配列候補: `ranking`, `rankings`, `gift_ranking.*`, `ranks`, `items`, `list`, `results`, `data`
- ページング: `next_cursor`

### 2.5 WS接続 (`connect_broadcast`)
- 接続先: `wss://{broadcast_host}/`
- Subscribe: `SUB\t{bcsvr_key}`
- 必須入力: `bcsvr_key`, `broadcast_host`

## 3. Endpoint一覧（tauri::command 抽出）

注意: Rust 側の大半は `serde_json::Value` をそのまま返すため、下表の「レスポンス要約」は主にフロント実利用に基づく。未記載は「生JSON返却（型未固定）」とする。

| Module | Command | Method | Endpoint | レスポンス要約 |
|---|---|---|---|---|
| analytics.rs | post_analytics_log | POST_JSON | https://clog.mirrativ.com/api/analytics/log | 生JSON返却（`serde_json::Value` / 型未固定） |
| app.rs | add_my_app | POST_JSON | https://www.mirrativ.com/api/app/add_my_app | 生JSON返却（`serde_json::Value` / 型未固定） |
| app.rs | get_app_appeal_banners | GET | https://www.mirrativ.com/api/app/appeal_banners | 生JSON返却（`serde_json::Value` / 型未固定） |
| app.rs | get_my_app | GET | https://www.mirrativ.com/api/app/my_app?user_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| app.rs | get_onlive_apps | GET | https://www.mirrativ.com/api/app/onlive_apps | 生JSON返却（`serde_json::Value` / 型未固定） |
| app.rs | get_recommend_apps | GET | https://www.mirrativ.com/api/app/recommend_apps | 生JSON返却（`serde_json::Value` / 型未固定） |
| app.rs | get_recommend_apps | GET | https://www.mirrativ.com/api/app/recommend_apps?type={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| auth.rs | open_twitter_login | N/A | https://www.mirrativ.com/ | Twitter OAuth連携のCookie取得用途（APIレスポンスではなくWebログインフロー）。 |
| auth.rs | open_twitter_login | N/A | https://www.mirrativ.com/social/twitter/callback?oauth_token=... | ログインコールバックURL（判定条件）。 |
| broadcast.rs | send_broadcast | N/A | https://www.mirrativ.com | WebSocket接続準備で Origin 相当として参照。実接続先は `wss://{broadcast_host}/`。 |
| catalog.rs | get_catalog | GET | https://www.mirrativ.com/api/live/catalog?id=2 | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog | GET | https://www.mirrativ.com/api/live/catalog?id=2&cursor={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog_banners | GET | https://www.mirrativ.com/api/catalog/banners?tab_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog_follow | GET | https://www.mirrativ.com/api/catalog/follow | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog_follow | GET | https://www.mirrativ.com/api/catalog/follow?cursor={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog_lives | GET | https://www.mirrativ.com/api/catalog/lives?tab_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog_tabs | GET | https://www.mirrativ.com/api/catalog/tabs | 生JSON返却（`serde_json::Value` / 型未固定） |
| catalog.rs | get_catalog_tabs | GET | https://www.mirrativ.com/api/user/me | tabs取得前のセッション補完用呼び出し。Set-Cookie (mr_id/f) を取り込む用途あり。 |
| closet.rs | apply_preset | POST_FORM | https://www.mirrativ.com/api/closet/apply_preset | 生JSON返却（`serde_json::Value` / 型未固定） |
| closet.rs | get_closet_avatar | GET | https://www.mirrativ.com/api/closet/avatar?user_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| closet.rs | get_closet_part_avatar_parts | GET | https://www.mirrativ.com/api/closet/part_avatar_parts?part_type_id={}&gender_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| closet.rs | get_closet_presets | GET | https://www.mirrativ.com/api/closet/presets | 生JSON返却（`serde_json::Value` / 型未固定） |
| complex.rs | join_live | GET+POST_FORM | https://www.mirrativ.com/api/event/notice?type=2&live_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| complex.rs | join_live | GET+POST_FORM | https://www.mirrativ.com/api/live/get_streaming_url?live_id={} | join_live の戻り値は最終的にこの streaming_url レスポンス。 |
| complex.rs | join_live | GET+POST_FORM | https://www.mirrativ.com/api/live/live_comment | まず type=3 で入室通知 POST。続いて live/info/comments/stream URL を並列 GET。 |
| complex.rs | join_live | GET+POST_FORM | https://www.mirrativ.com/api/live/live_comments?live_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| complex.rs | join_live | GET+POST_FORM | https://www.mirrativ.com/api/live/live?live_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| device.rs | adjust_attribute | POST_FORM | https://www.mirrativ.com/api/adjust/attribute | 生JSON返却（`serde_json::Value` / 型未固定） |
| device.rs | register_token_android | POST_FORM | https://www.mirrativ.com/api/notification/register_token_android | 生JSON返却（`serde_json::Value` / 型未固定） |
| event.rs | get_event_notice | GET | https://www.mirrativ.com/api/event/notice?type={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| gift.rs | get_coin_box_status | GET | https://www.mirrativ.com/api/coin_box/status?live_id={} | 生JSON返却（UI側で必要項目のみ参照）。 |
| gift.rs | get_emomo_run_gifts | GET | https://www.mirrativ.com/api/gift/emomo_run_gifts | 生JSON返却（`serde_json::Value` / 型未固定） |
| gift.rs | get_gift_ranking | GET | https://www.mirrativ.com/api/gift/ranking?live_id={}&type={} | ランキング配列抽出キー: ranking / rankings / gift_ranking.* / ranks / items / list / results / data。next_cursor も利用。 |
| gift.rs | get_reward_ad_ids | GET | https://www.mirrativ.com/api/reward_ad/available_reward_ad_ids?mode={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | comment | POST_FORM | https://www.mirrativ.com/api/live/live_comment | POST body: live_id, comment(message), type(comment_type)。レスポンスは生JSON。 |
| live.rs | get_collaborators | GET | https://www.mirrativ.com/api/collab/collaborating_users?live_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | get_comments | GET | https://www.mirrativ.com/api/live/live_comments?live_id={} | コメント配列抽出キー: comments / live_comments / data。各要素は comment_id, user_id, user_name, comment, created_at 等を想定。 |
| live.rs | get_live_appeal_links | GET | https://www.mirrativ.com/api/live/appeal_links?live_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | get_live_campaign | GET | https://www.mirrativ.com/api/live/campaign?live_id={}&completed_at={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | get_live_history | GET | https://www.mirrativ.com/api/live/live_history?user_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | get_live_info | GET | https://www.mirrativ.com/api/live/live?live_id={} | UIで利用: live.title, owner.name, total_viewer_num, online_user_num, comment_num, started_at, app_title, collab_has_vacancy, is_live, ended_at, star_num, gift_num, live_id, gift_ranking_url, (場合により)bcsvr_key,broadcast_host。 |
| live.rs | get_live_search | GET | https://www.mirrativ.com/api/live/search?q={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | get_live_status | GET | https://www.mirrativ.com/api/live/get_streaming_url?live_id={} | HLS URL候補: streaming_url_hls / streaming_url / hls_url / playlist_url / streaming_url_list[]。配信によって bcsvr_key, broadcast_host も含まれる。 |
| live.rs | get_online_users | GET | https://www.mirrativ.com/api/live/online_users?live_id={}&page={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | get_view_history | GET | https://www.mirrativ.com/api/live/view_history?user_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | leave_live | POST_FORM | https://www.mirrativ.com/api/live/leave | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | live_polling | POST_FORM | https://www.mirrativ.com/api/live/live_polling | UIで利用: online_user_num, total_viewer_num, comment_num, star_num, gift_num, gift_ranking_url, current_user_rank.user.obfuscated_user_id 等。 |
| live.rs | preview_end | POST_FORM | https://www.mirrativ.com/api/live/preview_end | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | preview_polling | POST_FORM | https://www.mirrativ.com/api/live/preview_polling | 生JSON返却（`serde_json::Value` / 型未固定） |
| live.rs | preview_start | POST_FORM | https://www.mirrativ.com/api/live/preview_start | 生JSON返却（`serde_json::Value` / 型未固定） |
| misc.rs | get_jack_home | GET | https://www.mirrativ.com/api/jack/home | 生JSON返却（`serde_json::Value` / 型未固定） |
| misc.rs | get_live_game_new_counts | GET | https://www.mirrativ.com/api/live_game/new_counts?live_games={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| misc.rs | get_tooltip_start_live_button | GET | https://www.mirrativ.com/api/tooltip/start_live_button | 生JSON返却（`serde_json::Value` / 型未固定） |
| mission.rs | get_current_login_bonus | GET | https://www.mirrativ.com/api/mission/current_login_bonus | 生JSON返却（`serde_json::Value` / 型未固定） |
| mission.rs | get_mission_status | GET | https://www.mirrativ.com/api/mission/status | 生JSON返却（`serde_json::Value` / 型未固定） |
| mission.rs | get_mission_tutorial | GET | https://www.mirrativ.com/api/mission/tutorial | 生JSON返却（`serde_json::Value` / 型未固定） |
| mission.rs | get_mission_tutorial_status | GET | https://www.mirrativ.com/api/mission/tutorial_status | 生JSON返却（`serde_json::Value` / 型未固定） |
| mission.rs | receive_mission_reward | POST_FORM | https://www.mirrativ.com/api/mission/receive_reward | 生JSON返却（`serde_json::Value` / 型未固定） |
| notice.rs | get_notice_counts | GET | https://www.mirrativ.com/api/notice/counts | 生JSON返却（`serde_json::Value` / 型未固定） |
| notice.rs | get_notice_popups | GET | https://www.mirrativ.com/api/notice/popups | 生JSON返却（`serde_json::Value` / 型未固定） |
| notice.rs | get_notice_popups | GET | https://www.mirrativ.com/api/notice/popups?position={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| onboarding.rs | get_onboarding_redirect | GET | https://www.mirrativ.com/api/onboarding/redirect | 生JSON返却（`serde_json::Value` / 型未固定） |
| onboarding.rs | get_recommend_live | GET | https://www.mirrativ.com/api/onboarding/recommend_live | 生JSON返却（`serde_json::Value` / 型未固定） |
| ranking.rs | get_ranking_user_detail | GET | https://www.mirrativ.com/api/ranking/user_detail?live_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| season.rs | get_season_rating | GET | https://www.mirrativ.com/api/season_rating/status | 生JSON返却（`serde_json::Value` / 型未固定） |
| season.rs | get_season_yell_status | GET | https://www.mirrativ.com/api/season_yell/status?user_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| social.rs | follow | POST_FORM | https://www.mirrativ.com/api/graph/follow | 生JSON返却（`serde_json::Value` / 型未固定） |
| social.rs | get_chat_threads | GET | https://www.mirrativ.com/api/chat/threads | 生JSON返却（`serde_json::Value` / 型未固定） |
| social.rs | get_recommend_users | GET | https://www.mirrativ.com/api/graph/recommend_users?page={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| social.rs | get_talk_room_home | GET | https://www.mirrativ.com/api/talk_room/home | 生JSON返却（`serde_json::Value` / 型未固定） |
| social.rs | get_urge_users | GET | https://www.mirrativ.com/api/graph/urge_users | 生JSON返却（`serde_json::Value` / 型未固定） |
| social.rs | unfollow | POST_FORM | https://www.mirrativ.com/api/graph/unfollow | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | check_minor | POST_FORM | https://www.mirrativ.com/api/user/check_minor | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | get_my_page_banner | GET | https://www.mirrativ.com/api/user/my_page_banner | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | get_my_profile | GET | https://www.mirrativ.com/api/user/me | セッション確認・プロフィール表示の基礎データ。 |
| user.rs | get_profile | GET | https://www.mirrativ.com/api/user/profile?user_id={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | get_user_currency | GET | https://www.mirrativ.com/api/user/currency | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | get_user_search | GET | https://www.mirrativ.com/api/user/search?q={} | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | get_user_tos | GET | https://www.mirrativ.com/api/user/tos | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | post_demographic | POST_FORM | https://www.mirrativ.com/api/user/post_demographic | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | post_user_demographic | POST_FORM | https://www.mirrativ.com/api/user/demographic | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | profile_edit | POST_MULTIPART | https://www.mirrativ.com/api/user/profile_edit | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | profile_edit_tutorial | POST_FORM | https://www.mirrativ.com/api/user/profile_edit | 生JSON返却（`serde_json::Value` / 型未固定） |
| user.rs | request_live | POST_FORM | https://www.mirrativ.com/api/user/post_live_request | 生JSON返却（`serde_json::Value` / 型未固定） |

## 4. 補足
- `bootstrap_guest` は内部で `/api/user/me` を呼び、`Set-Cookie` から `mr_id`/`f` をセッションへ補完する。
- `get_gift_ranking_by_url` は URLバリデーションあり（`http/https`, `mirrativ.com` 配下, パスが `/api/gift/ranking`）。
- `join_live` は `live_comment(type=3)` を送った後に複数 GET を実行し、最終的に streaming_url を返す。
