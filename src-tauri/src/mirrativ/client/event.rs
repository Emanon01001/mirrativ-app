use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_event_notice(
    state: tauri::State<'_, MirrativClient>,
    notice_type: String,
    live_id: Option<String>,
) -> Result<Value, String> {
    let mut url = format!(
        "https://www.mirrativ.com/api/event/notice?type={}",
        notice_type
    );
    if let Some(lid) = live_id {
        url.push_str(&format!("&live_id={}", lid));
    }
    state.fetch_json(&url, None).await
}
