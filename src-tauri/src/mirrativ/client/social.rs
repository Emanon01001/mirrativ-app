use super::core::MirrativClient;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn follow(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
) -> Result<Value, String> {
    if !state.is_authed().await {
        return Err("フォローはログイン済みユーザーのみ実行できます".to_string());
    }

    let mut form = HashMap::new();
    form.insert("user_id".to_string(), user_id);

    state
        .post_json(
            "https://www.mirrativ.com/api/graph/follow",
            form,
            Some("live_view"),
        )
        .await
}

#[tauri::command]
pub async fn unfollow(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("user_id".to_string(), user_id);

    state
        .post_json(
            "https://www.mirrativ.com/api/graph/unfollow",
            form,
            Some("live_view"),
        )
        .await
}

#[tauri::command]
pub async fn get_urge_users(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json(
            "https://www.mirrativ.com/api/graph/urge_users",
            Some("home.follow"),
        )
        .await
}

#[tauri::command]
pub async fn get_recommend_users(
    state: tauri::State<'_, MirrativClient>,
    page: Option<i32>,
) -> Result<Value, String> {
    let p = page.unwrap_or(1).max(1);
    let url = format!(
        "https://www.mirrativ.com/api/graph/recommend_users?page={}",
        p
    );
    state.fetch_json(&url, Some("search.recommend_users")).await
}

#[tauri::command]
pub async fn get_chat_threads(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/chat/threads", None)
        .await
}

#[tauri::command]
pub async fn get_talk_room_home(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/talk_room/home", None)
        .await
}
