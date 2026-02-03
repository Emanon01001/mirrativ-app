use super::core::MirrativClient;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_mission_status(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/mission/status", Some("home.select"))
        .await
}

#[tauri::command]
pub async fn get_mission_tutorial(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/mission/tutorial", Some("mission.tutorial"))
        .await
}

#[tauri::command]
pub async fn get_mission_tutorial_status(
    state: tauri::State<'_, MirrativClient>,
) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/mission/tutorial_status", None)
        .await
}

#[tauri::command]
pub async fn get_current_login_bonus(
    state: tauri::State<'_, MirrativClient>,
) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/mission/current_login_bonus", None)
        .await
}

#[tauri::command]
pub async fn receive_mission_reward(
    state: tauri::State<'_, MirrativClient>,
    mission_period: String,
    mission_id: String,
    live_id: Option<String>,
    check_only: Option<bool>,
    is_ad_required_mission: Option<bool>,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("mission_period".to_string(), mission_period);
    form.insert("mission_id".to_string(), mission_id);
    if let Some(lid) = live_id {
        form.insert("live_id".to_string(), lid);
    }
    if let Some(check) = check_only {
        form.insert(
            "check_only".to_string(),
            if check { "1" } else { "0" }.to_string(),
        );
    }
    if let Some(ad_required) = is_ad_required_mission {
        form.insert(
            "is_ad_required_mission".to_string(),
            if ad_required { "1" } else { "0" }.to_string(),
        );
    }

    state
        .post_json(
            "https://www.mirrativ.com/api/mission/receive_reward",
            form,
            None,
        )
        .await
}
