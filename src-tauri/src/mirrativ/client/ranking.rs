use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_ranking_user_detail(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/ranking/user_detail?live_id={}",
        live_id
    );
    state.fetch_json(&url, Some("live_view")).await
}
