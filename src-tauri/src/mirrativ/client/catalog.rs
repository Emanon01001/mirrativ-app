use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_catalog(
    state: tauri::State<'_, MirrativClient>,
    cursor: Option<String>,
) -> Result<Value, String> {
    let url = if let Some(c) = cursor {
        format!(
            "https://www.mirrativ.com/api/live/catalog?id=2&cursor={}",
            c
        )
    } else {
        "https://www.mirrativ.com/api/live/catalog?id=2".to_string()
    };
    state.fetch_json(&url, Some("home")).await
}

#[tauri::command]
pub async fn get_catalog_tabs(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    if !state.has_session().await {
        let _ = state
            .fetch_json("https://www.mirrativ.com/api/user/me", Some("my_page"))
            .await;
    }
    let res = state
        .fetch_json("https://www.mirrativ.com/api/catalog/tabs", Some("home"))
        .await;
    if res.is_ok() {
        return res;
    }
    state
        .fetch_json(
            "https://www.mirrativ.com/api/catalog/tabs",
            Some("home.select"),
        )
        .await
}

#[tauri::command]
pub async fn get_catalog_lives(
    state: tauri::State<'_, MirrativClient>,
    tab_id: String,
    app_id: Option<String>,
    cursor: Option<String>,
    referer: Option<String>,
) -> Result<Value, String> {
    let mut url = format!(
        "https://www.mirrativ.com/api/catalog/lives?tab_id={}",
        tab_id
    );
    if let Some(aid) = app_id {
        url.push_str(&format!("&app_id={}", aid));
    }
    if let Some(cur) = cursor {
        url.push_str(&format!("&cursor={}", cur));
    }
    let referer = referer.as_deref().unwrap_or("home_select");
    state.fetch_json(&url, Some(referer)).await
}

#[tauri::command]
pub async fn get_catalog_banners(
    state: tauri::State<'_, MirrativClient>,
    tab_id: String,
    app_id: Option<String>,
) -> Result<Value, String> {
    let mut url = format!(
        "https://www.mirrativ.com/api/catalog/banners?tab_id={}",
        tab_id
    );
    if let Some(aid) = app_id {
        url.push_str(&format!("&app_id={}", aid));
    }
    state.fetch_json(&url, Some("home")).await
}

#[tauri::command]
pub async fn get_catalog_follow(
    state: tauri::State<'_, MirrativClient>,
    cursor: Option<String>,
) -> Result<Value, String> {
    let url = if let Some(cur) = cursor {
        format!("https://www.mirrativ.com/api/catalog/follow?cursor={}", cur)
    } else {
        "https://www.mirrativ.com/api/catalog/follow".to_string()
    };
    state.fetch_json(&url, Some("home.follow")).await
}
