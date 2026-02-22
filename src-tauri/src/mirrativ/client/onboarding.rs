use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_recommend_live(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json(
            "https://www.mirrativ.com/api/onboarding/recommend_live",
            None,
        )
        .await
}

#[tauri::command]
pub async fn get_onboarding_redirect(
    state: tauri::State<'_, MirrativClient>,
) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/onboarding/redirect", None)
        .await
}
