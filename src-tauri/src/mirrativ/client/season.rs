use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_season_rating(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json(
            "https://www.mirrativ.com/api/season_rating/status",
            Some("home.select"),
        )
        .await
}

#[tauri::command]
pub async fn get_season_yell_status(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
    streamer_id: Option<String>,
) -> Result<Value, String> {
    let mut url = format!(
        "https://www.mirrativ.com/api/season_yell/status?user_id={}",
        user_id
    );
    if let Some(sid) = streamer_id {
        url.push_str(&format!("&streamer_id={}", sid));
    }
    state.fetch_json(&url, None).await
}
