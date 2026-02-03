use super::core::MirrativClient;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_closet_avatar(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/closet/avatar?user_id={}",
        user_id
    );
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn get_closet_presets(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/closet/presets", None)
        .await
}

#[tauri::command]
pub async fn get_closet_part_avatar_parts(
    state: tauri::State<'_, MirrativClient>,
    part_type_id: String,
    gender_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/closet/part_avatar_parts?part_type_id={}&gender_id={}",
        part_type_id, gender_id
    );
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn apply_preset(
    state: tauri::State<'_, MirrativClient>,
    preset_id: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("id".to_string(), preset_id);

    state
        .post_json(
            "https://www.mirrativ.com/api/closet/apply_preset",
            form,
            None,
        )
        .await
}
