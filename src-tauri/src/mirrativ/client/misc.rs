use super::core::MirrativClient;
use serde_json::Value;

#[tauri::command]
pub async fn get_live_game_new_counts(
    state: tauri::State<'_, MirrativClient>,
    live_games: Option<i32>,
) -> Result<Value, String> {
    let live_games = live_games.unwrap_or(0);
    let url = format!(
        "https://www.mirrativ.com/api/live_game/new_counts?live_games={}",
        live_games
    );
    state.fetch_json(&url, None).await
}

#[tauri::command]
pub async fn get_jack_home(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/jack/home", None)
        .await
}

#[tauri::command]
pub async fn get_tooltip_start_live_button(
    state: tauri::State<'_, MirrativClient>,
) -> Result<Value, String> {
    state
        .fetch_json(
            "https://www.mirrativ.com/api/tooltip/start_live_button",
            None,
        )
        .await
}
