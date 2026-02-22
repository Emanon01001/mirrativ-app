use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_gift_ranking(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
    ranking_type: Option<String>,
    cursor: Option<String>,
    obfuscated_user_id: Option<String>,
    is_force_update: Option<i32>,
) -> Result<Value, String> {
    let t = ranking_type.unwrap_or_else(|| "live".to_string());
    let mut url = format!(
        "https://www.mirrativ.com/api/gift/ranking?live_id={}&type={}",
        live_id, t
    );
    if let Some(cursor) = cursor {
        if !cursor.trim().is_empty() {
            url.push_str("&cursor=");
            url.push_str(&cursor);
        }
    }
    if let Some(obfuscated_user_id) = obfuscated_user_id {
        if !obfuscated_user_id.trim().is_empty() {
            url.push_str("&obfuscated_user_id=");
            url.push_str(&obfuscated_user_id);
        }
    }
    if let Some(is_force_update) = is_force_update {
        if is_force_update != 0 {
            url.push_str("&is_force_update=1");
        }
    }
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_emomo_run_gifts(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/gift/emomo_run_gifts", None)
        .await
}

#[tauri::command]
pub async fn get_coin_box_status(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/coin_box/status?live_id={}",
        live_id
    );
    state.fetch_json(&url, Some("live_view")).await
}

#[tauri::command]
pub async fn get_reward_ad_ids(
    state: tauri::State<'_, MirrativClient>,
    mode: Option<i32>,
) -> Result<Value, String> {
    let mode = mode.unwrap_or(1);
    let url = format!(
        "https://www.mirrativ.com/api/reward_ad/available_reward_ad_ids?mode={}",
        mode
    );
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn get_gift_ranking_by_url(
    state: tauri::State<'_, MirrativClient>,
    url: String,
) -> Result<Value, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("gift_ranking_url is empty".to_string());
    }

    let parsed = url::Url::parse(trimmed).map_err(|_| "gift_ranking_url is invalid".to_string())?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err("gift_ranking_url must be http(s)".to_string());
    }

    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    if host != "mirrativ.com" && !host.ends_with(".mirrativ.com") {
        return Err("gift_ranking_url must be a mirrativ.com domain".to_string());
    }

    if !parsed.path().starts_with("/api/gift/ranking") {
        return Err("gift_ranking_url path is not allowed".to_string());
    }

    state.fetch_json(trimmed, Some("live_view")).await
}
