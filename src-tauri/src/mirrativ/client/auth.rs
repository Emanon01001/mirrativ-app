use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri::webview::PageLoadEvent;

use super::core::MirrativClient;

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
        WebviewUrl::External(
            url::Url::parse(MIRRATIV_LOGIN_URL).map_err(|e| e.to_string())?,
        ),
    )
    .title("Mirrativ - Twitterでログイン")
    .inner_size(480.0, 720.0)
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
                let is_mirrativ = u.host_str()
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

            let mirrativ_url = url::Url::parse("https://www.mirrativ.com/")
                .expect("hardcoded URL");

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
