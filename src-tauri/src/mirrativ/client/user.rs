use super::core::MirrativClient;
use reqwest::header::{HeaderValue, SET_COOKIE};
use reqwest::multipart::Form;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_profile(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
) -> Result<Value, String> {
    let url = format!(
        "https://www.mirrativ.com/api/user/profile?user_id={}",
        user_id
    );
    state.fetch_json(&url, Some("profile")).await
}

#[tauri::command]
pub async fn get_my_profile(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/user/me", Some("my_page"))
        .await
}

#[tauri::command]
pub async fn get_user_tos(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/user/tos", None)
        .await
}

#[tauri::command]
pub async fn get_my_page_banner(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/user/my_page_banner", Some("my_page"))
        .await
}

#[tauri::command]
pub async fn bootstrap_guest(state: tauri::State<'_, MirrativClient>) -> Result<(), String> {
    let mut headers = state.get_headers().await;
    headers.insert("x-referer", HeaderValue::from_static("my_page"));

    let resp = state
        .client
        .get("https://www.mirrativ.com/api/user/me")
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let headers = resp.headers().clone();
    let _ = resp
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    let mut mr_id_cookie: Option<String> = None;
    let mut f_cookie: Option<String> = None;

    for value in headers.get_all(SET_COOKIE).iter() {
        if let Ok(raw) = value.to_str() {
            if let Some((name, val)) = parse_set_cookie(raw) {
                match name.as_str() {
                    "mr_id" => mr_id_cookie = Some(val),
                    "f" => f_cookie = Some(val),
                    _ => {}
                }
            }
        }
    }

    state
        .set_session_if_empty(mr_id_cookie, f_cookie)
        .await;
    Ok(())
}

fn parse_set_cookie(value: &str) -> Option<(String, String)> {
    let first = value.split(';').next()?.trim();
    let mut parts = first.splitn(2, '=');
    let name = parts.next()?.trim();
    let val = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), val.to_string()))
}

#[tauri::command]
pub async fn profile_edit(
    state: tauri::State<'_, MirrativClient>,
    name: String,
    description: Option<String>,
    url: Option<String>,
) -> Result<Value, String> {
    let links = format!("[{{\"url\":\"{}\"}}]", url.unwrap_or_default());

    let form = Form::new()
        .text("name", name)
        .text("description", description.unwrap_or_default())
        .text("links", links);

    state
        .post_multipart_json(
            "https://www.mirrativ.com/api/user/profile_edit",
            form,
            Some("live_view"),
        )
        .await
}

#[tauri::command]
pub async fn profile_edit_tutorial(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
    name: String,
    description: Option<String>,
    include_urge_users: Option<bool>,
    dynamic_link: Option<String>,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("user_id".to_string(), user_id);
    form.insert("name".to_string(), name);
    form.insert("description".to_string(), description.unwrap_or_default());
    if let Some(include) = include_urge_users {
        form.insert(
            "include_urge_users".to_string(),
            if include { "1" } else { "0" }.to_string(),
        );
    }
    if let Some(link) = dynamic_link {
        form.insert("dynamic_link".to_string(), link);
    }

    state
        .post_json(
            "https://www.mirrativ.com/api/user/profile_edit",
            form,
            Some("tutorial"),
        )
        .await
}

#[tauri::command]
pub async fn post_demographic(
    state: tauri::State<'_, MirrativClient>,
    gender_type: i32,
    generation: i32,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("gender_type".to_string(), gender_type.to_string());
    form.insert("generation".to_string(), generation.to_string());

    state
        .post_json(
            "https://www.mirrativ.com/api/user/post_demographic",
            form,
            Some("tutorial"),
        )
        .await
}

#[tauri::command]
pub async fn post_user_demographic(
    state: tauri::State<'_, MirrativClient>,
    gender_type: i32,
    generation: i32,
    birthday: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("gender_type".to_string(), gender_type.to_string());
    form.insert("generation".to_string(), generation.to_string());
    form.insert("birthday".to_string(), birthday);

    state
        .post_json(
            "https://www.mirrativ.com/api/user/demographic",
            form,
            Some("tutorial"),
        )
        .await
}

#[tauri::command]
pub async fn check_minor(
    state: tauri::State<'_, MirrativClient>,
    generation: i32,
    birthday: String,
) -> Result<Value, String> {
    let mut form = HashMap::new();
    form.insert("generation".to_string(), generation.to_string());
    form.insert("birthday".to_string(), birthday);

    state
        .post_json("https://www.mirrativ.com/api/user/check_minor", form, None)
        .await
}

#[tauri::command]
pub async fn get_user_currency(state: tauri::State<'_, MirrativClient>) -> Result<Value, String> {
    state
        .fetch_json("https://www.mirrativ.com/api/user/currency", None)
        .await
}

#[tauri::command]
pub async fn get_user_search(
    state: tauri::State<'_, MirrativClient>,
    query: String,
    page: Option<i32>,
    cursor: Option<String>,
) -> Result<Value, String> {
    let q = urlencoding::encode(&query);
    let mut url = format!("https://www.mirrativ.com/api/user/search?q={}", q);
    if let Some(c) = cursor {
        if !c.trim().is_empty() {
            url.push_str(&format!("&cursor={}", c));
        }
    } else if let Some(p) = page {
        if p > 1 {
            url.push_str(&format!("&page={}", p));
        }
    }
    state.fetch_json(&url, Some("search.user")).await
}

#[tauri::command]
pub async fn request_live(
    state: tauri::State<'_, MirrativClient>,
    user_id: String,
    count: i32,
) -> Result<Value, String> {
    let safe_count = count.max(1).min(10_000);

    let mut form = HashMap::new();
    form.insert("count".to_string(), safe_count.to_string());
    form.insert("user_id".to_string(), user_id);
    form.insert("where".to_string(), "profile".to_string());

    state
        .post_json(
            "https://www.mirrativ.com/api/user/post_live_request",
            form,
            Some("profile"),
        )
        .await
}

// ----- 認証 -----

#[tauri::command]
pub async fn login(
    state: tauri::State<'_, MirrativClient>,
    mr_id: String,
    unique: String,
) -> Result<(), String> {
    state.login(mr_id, unique).await;
    Ok(())
}

#[tauri::command]
pub async fn reset_session(state: tauri::State<'_, MirrativClient>) -> Result<(), String> {
    state.reset().await;
    Ok(())
}
