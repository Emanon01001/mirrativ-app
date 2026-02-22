use super::core::MirrativClient;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_live_info(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!("https://www.mirrativ.com/api/live/live?live_id={}", live_id);
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_live_status(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/live/get_streaming_url?live_id={}",
        live_id
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_live_search(
    state: tauri::State<'_, MirrativClient>,
    query: String,
    page: Option<i32>,
) -> Result<Value, String> {
    let q = urlencoding::encode(&query);
    let mut url = format!("https://www.mirrativ.com/api/live/search?q={}", q);
    if let Some(p) = page {
        if p > 1 {
            url.push_str(&format!("&page={}", p));
        }
    }
    state.fetch_json(&url, Some("search.live")).await
}

#[tauri::command]
pub async fn get_comments(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/live/live_comments?live_id={}",
        live_id
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_live_appeal_links(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/live/appeal_links?live_id={}",
        live_id
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_live_campaign(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    completed_at: Option<i64>,
) -> Result<Value, String> {
    let completed = completed_at.unwrap_or(0);
    let url = format!(
        "https://www.mirrativ.com/api/live/campaign?live_id={}&completed_at={}",
        live_id, completed
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_live_history(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
    page: Option<i32>,
) -> Result<Value, String> {
    let mut url = format!(
        "https://www.mirrativ.com/api/live/live_history?user_id={}",
        user_id
    );
    if let Some(p) = page {
        if p > 1 {
            url.push_str(&format!("&page={}", p));
        }
    }
    state.fetch_json(&url, Some("profile")).await
}

#[tauri::command]
pub async fn get_view_history(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/live/view_history?user_id={}",
        user_id
    );
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn get_online_users(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    page: i32,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/live/online_users?live_id={}&page={}",
        live_id, page
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_collaborators(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/collab/collaborating_users?live_id={}",
        live_id
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn comment(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    message: String,
    comment_type: Option<i32>,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);
    form.insert("comment".to_string(), message);
    form.insert("type".to_string(), comment_type.unwrap_or(1).to_string());

    state
        .post_json(
            "https://www.mirrativ.com/api/live/live_comment",
            form,
            Some("live_view"),
        )
        .await
}

#[tauri::command]
pub async fn leave_live(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);

    state
        .post_json(
            "https://www.mirrativ.com/api/live/leave",
            form,
            Some("live_view"),
        )
        .await
}

#[tauri::command]
pub async fn live_polling(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    live_user_key: Option<String>,
    is_ui_hidden: Option<i32>,
    screen_status: Option<i32>,
    screen_settings: Option<String>,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);
    if let Some(key) = live_user_key {
        form.insert("live_user_key".to_string(), key);
    }
    if let Some(hidden) = is_ui_hidden {
        form.insert("is_ui_hidden".to_string(), hidden.to_string());
    }
    if let Some(status) = screen_status {
        form.insert("screen_status".to_string(), status.to_string());
    }
    if let Some(settings) = screen_settings {
        form.insert("screen_settings".to_string(), settings);
    }

    state
        .post_json(
            "https://www.mirrativ.com/api/live/live_polling",
            form,
            Some("live_view"),
        )
        .await
}

#[tauri::command]
pub async fn preview_start(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);

    state
        .post_json(
            "https://www.mirrativ.com/api/live/preview_start",
            form,
            None,
        )
        .await
}

#[tauri::command]
pub async fn preview_polling(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);

    state
        .post_json(
            "https://www.mirrativ.com/api/live/preview_polling",
            form,
            None,
        )
        .await
}

#[tauri::command]
pub async fn preview_end(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);

    state
        .post_json("https://www.mirrativ.com/api/live/preview_end", form, None)
        .await
}
