use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn post_analytics_log(
    state: tauri::State<'_, MirrativClient>,
    payload: Value,
) -> Result<Value, String> {
    state
        .post_json_body("https://clog.mirrativ.com/api/analytics/log", payload, None)
        .await
}
