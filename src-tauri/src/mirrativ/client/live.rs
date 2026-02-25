use super::core::MirrativClient;
use reqwest::multipart::Form;
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

#[tauri::command]
pub async fn live_create(
    state: tauri::State<'_, MirrativClient>,
) -> Result<Value, String> {
    state
        .post_web_json(
            "https://www.mirrativ.com/api/live/live_create",
            serde_json::json!({}),
            Some("https://www.mirrativ.com/broadcast/history"),
        )
        .await
}

#[tauri::command]
pub async fn live_edit(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    title: Option<String>,
    description: Option<String>,
    app_id: Option<String>,
) -> Result<Value, String> {
    let referer = format!("https://www.mirrativ.com/broadcast/{}", live_id);
    let mut form = Form::new().text("live_id", live_id);
    if let Some(t) = title {
        form = form.text("title", t);
    }
    if let Some(d) = description {
        form = form.text("description", d);
    }
    if let Some(a) = app_id {
        form = form.text("app_id", a);
    }

    state
        .post_web_multipart(
            "https://www.mirrativ.com/api/live/live_edit",
            form,
            Some(&referer),
        )
        .await
}

#[tauri::command]
pub async fn live_capture_image(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    image_data: Vec<u8>,
    filename: Option<String>,
) -> Result<Value, String> {
    let referer = format!("https://www.mirrativ.com/broadcast/{}", live_id);
    let url = format!(
        "https://www.mirrativ.com/api/live/live_capture_image?live_id={}",
        live_id
    );
    let fname = filename.unwrap_or_else(|| "thumbnail.jpg".to_string());
    let part = reqwest::multipart::Part::bytes(image_data)
        .file_name(fname)
        .mime_str("image/jpeg")
        .map_err(|e| e.to_string())?;
    let form = Form::new().part("live_capture_image", part);

    state
        .post_web_multipart(&url, form, Some(&referer))
        .await
}

#[tauri::command]
pub async fn live_heartbeat(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let referer = format!("https://www.mirrativ.com/broadcast/{}", live_id);
    let url = format!(
        "https://www.mirrativ.com/api/live/live_heartbeat?live_id={}",
        live_id
    );
    state
        .post_web_json(&url, serde_json::json!(null), Some(&referer))
        .await
}

#[tauri::command]
pub async fn live_start(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let referer = format!("https://www.mirrativ.com/broadcast/{}", live_id);
    let url = format!(
        "https://www.mirrativ.com/api/live/live_start?live_id={}",
        live_id
    );
    state
        .post_web_json(&url, serde_json::json!(null), Some(&referer))
        .await
}

#[tauri::command]
pub async fn renew_streaming_key(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let referer = format!("https://www.mirrativ.com/broadcast/{}", live_id);
    let mut form = HashMap::new();
    form.insert("live_id".to_string(), live_id);

    state
        .post_web_form(
            "https://www.mirrativ.com/api/live/renew_streaming_key",
            form,
            Some(&referer),
        )
        .await
}

#[tauri::command]
pub async fn live_end(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let referer = format!("https://www.mirrativ.com/broadcast/{}", live_id);
    let url = format!(
        "https://www.mirrativ.com/api/live/live_end?live_id={}",
        live_id
    );
    state
        .post_web_json(&url, serde_json::json!(null), Some(&referer))
        .await
}
