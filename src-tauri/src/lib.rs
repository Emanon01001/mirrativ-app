mod mirrativ;
// mod llstream_video;
// mod llstream_audio;
mod mpv_player;
use mirrativ::MirrativClient;
// use llstream_video::VideoHlsManager;
// use llstream_audio::AudioHlsManager;
use mpv_player::MpvPlayerManager;
use tauri::{Emitter, Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    mpv_player::init_main_thread_id();
    let client = MirrativClient::new();
    // let video_hls = VideoHlsManager::default();
    // let audio_hls = AudioHlsManager::default();
    let mpv_player = MpvPlayerManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(client)
        // .manage(video_hls)
        // .manage(audio_hls)
        .manage(mpv_player)
        .on_window_event(|window, event| {
            match window.label() {
                "player" => {
                    if let WindowEvent::CloseRequested { .. } = event {
                        let app = window.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            let manager = app.state::<MpvPlayerManager>();
                            let _ = mpv_player::stop_mpv(
                                app.clone(),
                                manager,
                                Some("window-close".to_string()),
                            )
                            .await;
                        });
                    }
                }
                "twitter-auth" => {
                    if let WindowEvent::Destroyed = event {
                        let _ = window.app_handle().emit("auth://login-cancelled", ());
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            // GET API - ユーザー
            mirrativ::client::user::get_profile,
            mirrativ::client::user::get_my_profile,
            mirrativ::client::user::get_user_tos,
            mirrativ::client::user::get_my_page_banner,
            mirrativ::client::user::bootstrap_guest,
            mirrativ::client::user::get_user_currency,
            mirrativ::client::user::get_user_search,
            // GET API - 配信
            mirrativ::client::live::get_live_info,
            mirrativ::client::live::get_live_status,
            mirrativ::client::live::get_live_search,
            mirrativ::client::live::get_comments,
            mirrativ::client::live::get_live_appeal_links,
            mirrativ::client::live::get_live_campaign,
            mirrativ::client::live::get_online_users,
            mirrativ::client::live::get_collaborators,
            mirrativ::client::live::get_live_history,
            mirrativ::client::live::get_view_history,
            // GET API - カタログ/ディスカバリー
            mirrativ::client::catalog::get_catalog,
            mirrativ::client::catalog::get_catalog_tabs,
            mirrativ::client::catalog::get_catalog_lives,
            mirrativ::client::catalog::get_catalog_banners,
            mirrativ::client::catalog::get_catalog_follow,
            mirrativ::client::onboarding::get_recommend_live,
            mirrativ::client::onboarding::get_onboarding_redirect,
            // GET API - アプリ
            mirrativ::client::app::get_onlive_apps,
            mirrativ::client::app::get_my_app,
            mirrativ::client::app::get_recommend_apps,
            mirrativ::client::app::get_app_appeal_banners,
            // GET API - ギフト/ランキング
            mirrativ::client::gift::get_gift_ranking,
            mirrativ::client::gift::get_gift_ranking_by_url,
            mirrativ::client::gift::get_emomo_run_gifts,
            mirrativ::client::gift::get_coin_box_status,
            mirrativ::client::gift::get_reward_ad_ids,
            mirrativ::client::ranking::get_ranking_user_detail,
            mirrativ::client::season::get_season_rating,
            mirrativ::client::season::get_season_yell_status,
            // GET API - 通知
            mirrativ::client::notice::get_notice_counts,
            mirrativ::client::notice::get_notice_popups,
            // GET API - ミッション
            mirrativ::client::mission::get_mission_status,
            mirrativ::client::mission::get_mission_tutorial,
            mirrativ::client::mission::get_mission_tutorial_status,
            mirrativ::client::mission::get_current_login_bonus,
            // GET API - ソーシャル
            mirrativ::client::social::get_urge_users,
            mirrativ::client::social::get_recommend_users,
            mirrativ::client::social::get_chat_threads,
            mirrativ::client::social::get_talk_room_home,
            // GET API - エモモ/クローゼット
            mirrativ::client::closet::get_closet_avatar,
            mirrativ::client::closet::get_closet_presets,
            mirrativ::client::closet::get_closet_part_avatar_parts,
            // GET API - イベント
            mirrativ::client::event::get_event_notice,
            // GET API - その他
            mirrativ::client::misc::get_live_game_new_counts,
            mirrativ::client::misc::get_jack_home,
            mirrativ::client::misc::get_tooltip_start_live_button,
            // POST API
            mirrativ::client::live::comment,
            mirrativ::client::social::follow,
            mirrativ::client::social::unfollow,
            mirrativ::client::user::request_live,
            mirrativ::client::live::leave_live,
            mirrativ::client::live::preview_start,
            mirrativ::client::live::preview_polling,
            mirrativ::client::live::preview_end,
            mirrativ::client::user::profile_edit,
            mirrativ::client::user::profile_edit_tutorial,
            mirrativ::client::user::post_demographic,
            mirrativ::client::user::post_user_demographic,
            mirrativ::client::user::check_minor,
            mirrativ::client::live::live_polling,
            mirrativ::client::mission::receive_mission_reward,
            mirrativ::client::app::add_my_app,
            mirrativ::client::closet::apply_preset,
            mirrativ::client::device::register_token_android,
            mirrativ::client::device::adjust_attribute,
            mirrativ::client::analytics::post_analytics_log,
            // 認証
            mirrativ::client::user::login,
            mirrativ::client::user::reset_session,
            mirrativ::client::auth::open_twitter_login,
            mirrativ::client::auth::close_twitter_login,
            // 複雑な操作
            mirrativ::client::complex::join_live,
            // MPV Player
            mpv_player::create_player_window,
            mpv_player::start_mpv,
            mpv_player::stop_mpv,
            mpv_player::mpv_command,
            mpv_player::get_player_info,
            mpv_player::close_player_window,
            mpv_player::position_mpv_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
