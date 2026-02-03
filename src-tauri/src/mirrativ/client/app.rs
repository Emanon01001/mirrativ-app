use super::core::MirrativClient;
use serde_json::{json, Value};

#[tauri::command]
pub async fn get_onlive_apps(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/app/onlive_apps", Some("home.select"))
        .await
}

#[tauri::command]
pub async fn get_my_app(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/app/my_app?user_id={}",
        user_id
    );
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn get_recommend_apps(
    state: tauri::State<'_, MirrativClient>,
    app_type: Option<String>,
) -> Result<Value, String> {
    let url = if let Some(t) = app_type {
        format!("https://www.mirrativ.com/api/app/recommend_apps?type={}", t)
    } else {
        "https://www.mirrativ.com/api/app/recommend_apps".to_string()
    };
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn add_my_app(
    state: tauri::State<'_, MirrativClient>,
    app_ids: Vec<String>,
) -> Result<Value, String> {
    let body = json!({ "app_ids": app_ids });
    state
        .post_json_body("https://www.mirrativ.com/api/app/add_my_app", body, None)
        .await
}

#[tauri::command]
pub async fn get_app_appeal_banners(
    state: tauri::State<'_, MirrativClient>,
) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/app/appeal_banners", None)
        .await
}
