use super::core::{encode_form, MirrativClient};
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_notice_counts(
    state: tauri::State<'_, MirrativClient>,
    params: Option<HashMap<String, String>>,
) -> Result<Value, String> {
    let mut url = "https://www.mirrativ.com/api/notice/counts".to_string();
    if let Some(values) = params {
        if !values.is_empty() {
            url.push('?');
            url.push_str(&encode_form(&values));
        }
    }
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn get_notice_popups(
    state: tauri::State<'_, MirrativClient>,
    position: Option<String>,
) -> Result<Value, String> {
    let url = if let Some(pos) = position {
        format!(
            "https://www.mirrativ.com/api/notice/popups?position={}",
            pos
        )
    } else {
        "https://www.mirrativ.com/api/notice/popups".to_string()
    };
    state.fetch_json(&url, None).await
}
