use super::core::MirrativClient;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn join_live(
    state: tauri::State<'_, MirrativClient>,
    live_id: String,
) -> Result<Value, String> {
    // 入室通知トリガー: live_comment(type=3)
    let mut join_form = HashMap::new();
    join_form.insert("live_id".to_string(), live_id.clone());
    join_form.insert("comment".to_string(), String::new());
    join_form.insert("type".to_string(), "3".to_string());
    state
        .post_json(
            "https://www.mirrativ.com/api/live/live_comment",
            join_form,
            Some("live_view"),
        )
        .await?;

    let info_url = format!("https://www.mirrativ.com/api/live/live?live_id={}", live_id);
    let notice_url = format!(
        "https://www.mirrativ.com/api/event/notice?type=2&live_id={}",
        live_id
    );
    let comments_url = format!(
        "https://www.mirrativ.com/api/live/live_comments?live_id={}",
        live_id
    );
    let stream_url = format!(
        "https://www.mirrativ.com/api/live/get_streaming_url?live_id={}",
        live_id
    );

    let (_info, _notice, _comments, status) = tokio::join!(
        state.fetch_json(&info_url, Some("live_view")),
        state.fetch_json(&notice_url, Some("live_view")),
        state.fetch_json(&comments_url, Some("live_view")),
        state.fetch_json(&stream_url, Some("live_view")),
    );

    let status = status?;
    Ok(status)
}
