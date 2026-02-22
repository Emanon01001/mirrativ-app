use serde::{Deserialize, Serialize};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use super::core::MirrativClient;

const SESSION_FILE: &str = "session.json";

const AUTH_WINDOW_LABEL: &str = "twitter-auth";
const MIRRATIV_LOGIN_URL: &str = "https://www.mirrativ.com/social/twitter";

#[derive(Clone, Serialize)]
pub struct AuthResult {
    pub success: bool,
    pub mr_id: String,
    pub unique: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn open_twitter_login(app: AppHandle) -> Result<(), String> {
    // 既存の認証ウィンドウがあれば閉じる
    if let Some(existing) = app.get_webview_window(AUTH_WINDOW_LABEL) {
        let _ = existing.close();
    }

    let app_handle = app.clone();
    let app_new_window = app.clone();

    WebviewWindowBuilder::new(
        &app,
        AUTH_WINDOW_LABEL,
        WebviewUrl::External(url::Url::parse(MIRRATIV_LOGIN_URL).map_err(|e| e.to_string())?),
    )
    .title("Mirrativ - Twitterでログイン")
    .inner_size(1080.0, 720.0)
    .resizable(true)
    .on_new_window(move |url, _features| {
        // window.open() や target="_blank" のリンクを同じウィンドウ内で遷移させる
        let url_str = url.to_string();
        if let Some(auth_window) = app_new_window.get_webview_window(AUTH_WINDOW_LABEL) {
            let js = format!(
                "window.location.href = '{}';",
                url_str.replace('\\', "\\\\").replace('\'', "\\'")
            );
            let _ = auth_window.eval(&js);
        }
        tauri::webview::NewWindowResponse::Deny
    })
    .on_page_load(move |window, payload| {
        if payload.event() != PageLoadEvent::Finished {
            return;
        }

        let url_str = payload.url().to_string();

        // Mirrativ ページでは window.open をオーバーライドして
        // ポップアップではなく同じウィンドウ内で遷移させる
        inject_popup_override(&window);

        // Twitter OAuth コールバック URL にのみ反応する
        // https://www.mirrativ.com/social/twitter/callback?oauth_token=...
        let is_callback = url::Url::parse(&url_str)
            .ok()
            .map(|u| {
                let is_mirrativ = u
                    .host_str()
                    .map(|h| h.contains("mirrativ.com"))
                    .unwrap_or(false);
                is_mirrativ && u.path().starts_with("/social/twitter/callback")
            })
            .unwrap_or(false);

        if !is_callback {
            return;
        }

        let window_clone = window.clone();
        let app_clone = app_handle.clone();

        // Windows での deadlock 回避のため async_runtime::spawn を使用
        tauri::async_runtime::spawn(async move {
            // Cookie が設定されるまで少し待つ
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let mirrativ_url = url::Url::parse("https://www.mirrativ.com/").expect("hardcoded URL");

            match window_clone.cookies_for_url(mirrativ_url) {
                Ok(cookies) => {
                    let mut mr_id = String::new();
                    let mut unique = String::new();

                    for cookie in &cookies {
                        match cookie.name() {
                            "mr_id" => mr_id = cookie.value().to_string(),
                            "f" => unique = cookie.value().to_string(),
                            _ => {}
                        }
                    }

                    if !mr_id.is_empty() && !unique.is_empty() {
                        // MirrativClient にセッションを設定
                        let client = app_clone.state::<MirrativClient>();
                        client.login(mr_id.clone(), unique.clone()).await;

                        let result = AuthResult {
                            success: true,
                            mr_id,
                            unique,
                            error: None,
                        };
                        let _ = app_clone.emit("auth://login-success", result);

                        // 認証ウィンドウを閉じる
                        if let Some(w) = app_clone.get_webview_window(AUTH_WINDOW_LABEL) {
                            let _ = w.close();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Cookie取得失敗: {}", e);
                }
            }
        });
    })
    .build()
    .map_err(|e| format!("認証ウィンドウの作成に失敗: {}", e))?;

    Ok(())
}

/// ポップアップや target="_blank" リンクを同じウィンドウ内で遷移させる
fn inject_popup_override(window: &WebviewWindow) {
    let js = r#"
        (function() {
            if (window.__popupOverrideInstalled) return;
            window.__popupOverrideInstalled = true;

            // window.open をオーバーライド
            window.open = function(url) {
                if (url) window.location.href = url;
                return window;
            };

            // target="_blank" のリンククリックを捕捉して同じウィンドウで開く
            document.addEventListener('click', function(e) {
                var link = e.target.closest ? e.target.closest('a') : null;
                if (!link) return;
                var href = link.getAttribute('href');
                if (!href) return;
                if (link.target === '_blank' || link.target === '_new' || link.rel && link.rel.indexOf('noopener') !== -1) {
                    e.preventDefault();
                    e.stopPropagation();
                    window.location.href = href;
                }
            }, true);
        })();
    "#;
    let _ = window.eval(js);
}

#[tauri::command]
pub async fn close_twitter_login(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(AUTH_WINDOW_LABEL) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ----- セッション永続化 -----

#[derive(Serialize, Deserialize)]
struct SavedSession {
    mr_id: String,
    unique: String,
}

#[cfg(windows)]
fn encrypt_session_bytes(plain: &[u8]) -> Result<Vec<u8>, String> {
    use windows::core::w;
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptProtectData(
            &input,
            w!("Mirrativ Session"),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| format!("failed to encrypt session: {}", e))?;

        let encrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as _)));
        Ok(encrypted)
    }
}

#[cfg(windows)]
fn decrypt_session_bytes(cipher: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: cipher.len() as u32,
        pbData: cipher.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|e| format!("failed to decrypt session: {}", e))?;

        let decrypted = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as _)));
        Ok(decrypted)
    }
}

fn encode_saved_session(data: &SavedSession) -> Result<Vec<u8>, String> {
    let serialized = serde_json::to_vec(data).map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        return encrypt_session_bytes(&serialized);
    }

    #[cfg(not(windows))]
    {
        Ok(serialized)
    }
}

fn decode_saved_session(bytes: &[u8]) -> Result<SavedSession, String> {
    #[cfg(windows)]
    {
        if let Ok(decrypted) = decrypt_session_bytes(bytes) {
            if let Ok(data) = serde_json::from_slice::<SavedSession>(&decrypted) {
                return Ok(data);
            }
        }
    }

    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

fn session_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join(SESSION_FILE))
}

#[tauri::command]
pub async fn save_session(app: AppHandle, mr_id: String, unique: String) -> Result<(), String> {
    let path = session_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = SavedSession { mr_id, unique };
    let encoded = encode_saved_session(&data)?;
    std::fs::write(&path, encoded).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn load_session(app: AppHandle) -> Result<Option<(String, String)>, String> {
    let path = session_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let data = decode_saved_session(&bytes)?;
    if data.mr_id.is_empty() || data.unique.is_empty() {
        return Ok(None);
    }
    Ok(Some((data.mr_id, data.unique)))
}

#[tauri::command]
pub async fn delete_session(app: AppHandle) -> Result<(), String> {
    let path = session_path(&app)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
