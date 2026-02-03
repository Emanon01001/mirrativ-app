use super::core::MirrativClient;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn register_token_android(
    state: tauri::State<'_, MirrativClient>,
    app_identifier: String,
    token: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("app_identifier".to_string(), app_identifier);
    form.insert("token".to_string(), token);

    state
        .post_json(
            "https://www.mirrativ.com/api/notification/register_token_android",
            form,
            None,
        )
        .await
}

#[tauri::command]
pub async fn adjust_attribute(
    state: tauri::State<'_, MirrativClient>,
    tracker_name: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("tracker_name".to_string(), tracker_name);

    state
        .post_json("https://www.mirrativ.com/api/adjust/attribute", form, None)
        .await
}
