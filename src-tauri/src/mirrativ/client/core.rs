// ─────────────────────────────────────────────────────────────────────────────
// core.rs
//
// Mirrativ API との HTTP 通信を担うコアクライアント。
//
// 主な責務:
//   - reqwest をベースにした HTTP クライアントの構築
//   - Android アプリを模したカスタムヘッダーの設定
//   - Cookie ベースのセッション管理（mr_id / f）
//   - GET/POST/マルチパートリクエストの送信と自動リトライ
//   - ゲストセッションのブートストラップ
// ─────────────────────────────────────────────────────────────────────────────

use reqwest::{
    header::{HeaderMap, HeaderValue, SET_COOKIE, USER_AGENT},
    multipart::Form,
    Client,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::sleep;

// ─────────────────────────────────────────────────────────────────────────────
// クライアント構造体
// ─────────────────────────────────────────────────────────────────────────────

/// Mirrativ API クライアント。Tauri の管理状態として登録して使用する。
///
/// セッション情報（mr_id / unique）は RwLock で保護しており、
/// 複数の非同期タスクから安全にアクセスできる。
pub struct MirrativClient {
    pub(crate) client: Arc<Client>,
    /// Accept-Language に使用する言語コード
    lang: String,
    /// セッション Cookie: Mirrativ のユーザー識別子
    mr_id: RwLock<String>,
    /// セッション Cookie: 端末識別子（f パラメータ）
    unique: RwLock<String>,
    /// ログイン済みフラグ（mr_id と unique が両方揃っているか）
    authed: RwLock<bool>,
    /// すべてのリクエストに付加するカスタムヘッダー（起動時に一度生成）
    custom_headers: HeaderMap,
    /// Web ブラウザ風リクエスト用の CSRF トークン（<meta name="csrf-token"> から取得）
    csrf_token: RwLock<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// クライアント初期化
// ─────────────────────────────────────────────────────────────────────────────

impl MirrativClient {
    /// クライアントを初期化する。
    /// reqwest クライアントを構築し、Android アプリを模したカスタムヘッダーを生成する。
    pub fn new() -> Self {
        let client = Client::builder()
            .cookie_store(false)  // Cookie の自動管理は無効（手動で制御するため）
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        let custom_headers = Self::create_custom_headers();

        Self {
            client: Arc::new(client),
            lang: "ja".to_string(),
            mr_id: RwLock::new(String::new()),
            unique: RwLock::new(String::new()),
            authed: RwLock::new(false),
            custom_headers,
            csrf_token: RwLock::new(String::new()),
        }
    }

    /// Mirrativ Android アプリを模したカスタムヘッダーセットを生成する。
    /// 端末 ID 系のヘッダーはランダム値を使用し、起動ごとに異なる値になる。
    fn create_custom_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();

        // Android アプリとして識別されるための User-Agent
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("MR_APP/11.56.0/Android/PGEM10/9"),
        );

        headers.insert("Accept-Language", HeaderValue::from_static("ja-JP"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));

        // Mirrativ 固有ヘッダー
        headers.insert("HTTP_X_TIMEZONE", HeaderValue::from_static("Asia/Tokyo"));
        // 端末識別子（IDFV に相当、起動ごとにランダム生成）
        if let Ok(value) = HeaderValue::from_str(&random_hex(16)) {
            headers.insert("x-idfv", value);
        }
        // 広告識別子（起動ごとにランダム生成）
        if let Ok(value) = HeaderValue::from_str(&uuid::Uuid::new_v4().to_string()) {
            headers.insert("x-ad", value);
        }
        headers.insert("x-hw", HeaderValue::from_static("qcom")); // ハードウェアプラットフォーム
        headers.insert("x-widevine-id", HeaderValue::from_static("")); // DRM ID（空でOK）
        headers.insert("x-network-status", HeaderValue::from_static("2")); // Wi-Fi
        headers.insert("x-os-push", HeaderValue::from_static("1")); // プッシュ通知対応
        // Adjust SDK の広告追跡ID（起動ごとにランダム生成）
        if let Ok(value) = HeaderValue::from_str(&random_hex(32)) {
            headers.insert("x-adjust-adid", value);
        }
        headers.insert("x-unity-framework", HeaderValue::from_static("6.4.0"));

        headers
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ヘッダー構築
    // ─────────────────────────────────────────────────────────────────────────

    /// リクエスト用のヘッダーを構築する。
    /// カスタムヘッダーにタイムスタンプと（必要な場合）Cookie を追加して返す。
    pub(crate) async fn get_headers_for_url(&self, target_url: &str) -> HeaderMap {
        let mut headers = self.custom_headers.clone();

        // リクエスト時刻のタイムスタンプ（ミリ秒精度）
        headers.insert(
            "x-client-unixtime",
            HeaderValue::from_str(&current_unixtime())
                .unwrap_or_else(|_| HeaderValue::from_static("0")),
        );

        // Mirrativ ドメイン向けのリクエストにのみ Cookie を付加する
        let mr_id = self.mr_id.read().await;
        let unique = self.unique.read().await;
        if should_attach_session_cookie(target_url) && (!mr_id.is_empty() || !unique.is_empty()) {
            let mut cookie_parts = vec![format!("lang={}", self.lang)];
            if !mr_id.is_empty() {
                cookie_parts.push(format!("mr_id={}", *mr_id));
            }
            if !unique.is_empty() {
                cookie_parts.push(format!("f={}", *unique));
            }
            let cookie = format!("{};", cookie_parts.join("; "));
            headers.insert(
                "Cookie",
                HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }

        headers
    }

    // ─────────────────────────────────────────────────────────────────────────
    // HTTP メソッド
    // ─────────────────────────────────────────────────────────────────────────

    /// GET リクエストを送信して JSON レスポンスを取得する。
    /// 一時的なエラー（429, 5xx, タイムアウト）は最大 3 回まで自動リトライする。
    pub(crate) async fn fetch_json(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers_for_url(url).await;
        add_referer_header(&mut headers, referer);

        for attempt in 0..3 {
            let resp = match self.client.get(url).headers(headers.clone()).send().await {
                Ok(resp) => resp,
                Err(err) => {
                    if attempt < 2 && is_retryable_transport_error(&err) {
                        sleep(retry_delay(attempt)).await;
                        continue;
                    }
                    return Err(err.to_string());
                }
            };

            let status = resp.status();
            if status.is_success() {
                return resp.json::<Value>().await.map_err(|e| e.to_string());
            }

            if attempt < 2 && is_retryable_status(status.as_u16()) {
                sleep(retry_delay(attempt)).await;
                continue;
            }

            return Err(format!("HTTP {}", status));
        }

        Err("HTTP request failed".to_string())
    }

    /// フォームエンコードされた POST リクエストを送信して JSON レスポンスを取得する。
    /// リトライロジックは fetch_json と同じ。
    pub(crate) async fn post_json(
        &self,
        url: &str,
        form: HashMap<String, String>,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers_for_url(url).await;
        add_referer_header(&mut headers, referer);
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
        );

        let body = encode_form(&form);
        for attempt in 0..3 {
            let resp = match self
                .client
                .post(url)
                .headers(headers.clone())
                .body(body.clone())
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    if attempt < 2 && is_retryable_transport_error(&err) {
                        sleep(retry_delay(attempt)).await;
                        continue;
                    }
                    return Err(err.to_string());
                }
            };

            let status = resp.status();
            if status.is_success() {
                return resp.json::<Value>().await.map_err(|e| e.to_string());
            }

            if attempt < 2 && is_retryable_status(status.as_u16()) {
                sleep(retry_delay(attempt)).await;
                continue;
            }

            return Err(format!("HTTP {}", status));
        }

        Err("HTTP request failed".to_string())
    }

    /// ブロードキャストページの HTML から CSRF トークンを取得してキャッシュする。
    /// `<meta name="csrf-token" content="...">` タグからトークンを抽出する。
    pub(crate) async fn fetch_csrf_token(&self) -> Result<String, String> {
        // キャッシュ済みなら返す
        {
            let cached = self.csrf_token.read().await;
            if !cached.is_empty() {
                return Ok(cached.clone());
            }
        }

        // ブロードキャストページを Chrome UA で GET して HTML を取得
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36"
            ),
        );
        headers.insert("Accept", HeaderValue::from_static("text/html,application/xhtml+xml"));
        headers.insert("Accept-Language", HeaderValue::from_static("ja"));

        // Cookie を付加
        let mr_id = self.mr_id.read().await;
        let unique = self.unique.read().await;
        if !mr_id.is_empty() || !unique.is_empty() {
            let mut cookie_parts = vec![format!("lang={}", self.lang)];
            if !mr_id.is_empty() {
                cookie_parts.push(format!("mr_id={}", *mr_id));
            }
            if !unique.is_empty() {
                cookie_parts.push(format!("f={}", *unique));
            }
            let cookie = format!("{};", cookie_parts.join("; "));
            headers.insert(
                "Cookie",
                HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }
        drop(mr_id);
        drop(unique);

        let resp = self
            .client
            .get("https://www.mirrativ.com/broadcast/history")
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("CSRF fetch failed: {}", e))?;

        let html = resp.text().await.map_err(|e| format!("CSRF read failed: {}", e))?;

        // <meta name="csrf-token" content="TOKEN"> を抽出
        let token = extract_csrf_token(&html)
            .ok_or_else(|| "CSRF token not found in HTML".to_string())?;

        *self.csrf_token.write().await = token.clone();
        Ok(token)
    }

    /// キャッシュ済みの CSRF トークンをクリアする（セッションリセット時に使用）。
    pub(crate) async fn clear_csrf_token(&self) {
        *self.csrf_token.write().await = String::new();
    }

    /// Web ブラウザ風のヘッダーを構築する（配信関連 API 用）。
    /// Android アプリヘッダーの代わりに Chrome ベースの User-Agent と CSRF トークンを使用する。
    pub(crate) async fn get_web_headers_for_url(&self, target_url: &str, referer: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();

        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36"
            ),
        );
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        headers.insert("Accept-Language", HeaderValue::from_static("ja"));
        headers.insert("X-Timezone", HeaderValue::from_static("Asia/Tokyo"));
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Origin", HeaderValue::from_static("https://www.mirrativ.com"));
        headers.insert("DNT", HeaderValue::from_static("1"));

        // CSRF トークン
        if let Ok(token) = self.fetch_csrf_token().await {
            if let Ok(val) = HeaderValue::from_str(&token) {
                headers.insert("X-CSRF-Token", val);
            }
        }

        // Referer（完全 URL をそのまま使用）
        if let Some(ref_url) = referer {
            if let Ok(val) = HeaderValue::from_str(ref_url) {
                headers.insert("Referer", val);
            }
        }

        // Cookie は Android と同じセッション Cookie を使う
        let mr_id = self.mr_id.read().await;
        let unique = self.unique.read().await;
        if should_attach_session_cookie(target_url) && (!mr_id.is_empty() || !unique.is_empty()) {
            let mut cookie_parts = vec![format!("lang={}", self.lang)];
            if !mr_id.is_empty() {
                cookie_parts.push(format!("mr_id={}", *mr_id));
            }
            if !unique.is_empty() {
                cookie_parts.push(format!("f={}", *unique));
            }
            let cookie = format!("{};", cookie_parts.join("; "));
            headers.insert(
                "Cookie",
                HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static("")),
            );
        }

        headers
    }

    /// Web ブラウザ風ヘッダーで JSON POST リクエストを送信する（配信作成等に使用）。
    pub(crate) async fn post_web_json(
        &self,
        url: &str,
        body: Value,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let headers = self.get_web_headers_for_url(url, referer).await;
        for attempt in 0..3 {
            let resp = match self
                .client
                .post(url)
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    if attempt < 2 && is_retryable_transport_error(&err) {
                        sleep(retry_delay(attempt)).await;
                        continue;
                    }
                    return Err(err.to_string());
                }
            };

            let status = resp.status();
            if status.is_success() {
                return resp.json::<Value>().await.map_err(|e| e.to_string());
            }

            if attempt < 2 && is_retryable_status(status.as_u16()) {
                sleep(retry_delay(attempt)).await;
                continue;
            }

            return Err(format!("HTTP {}", status));
        }

        Err("HTTP request failed".to_string())
    }

    /// Web ブラウザ風ヘッダーで multipart/form-data POST リクエストを送信する（live_edit 等に使用）。
    pub(crate) async fn post_web_multipart(
        &self,
        url: &str,
        form: Form,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_web_headers_for_url(url, referer).await;
        // Content-Type は reqwest が multipart boundary 付きで自動設定するので削除
        headers.remove("Content-Type");

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        if status.is_success() {
            return resp.json::<Value>().await.map_err(|e| e.to_string());
        }
        Err(format!("HTTP {}", status))
    }

    /// Web ブラウザ風ヘッダーで form-encoded POST リクエストを送信する。
    pub(crate) async fn post_web_form(
        &self,
        url: &str,
        form: HashMap<String, String>,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_web_headers_for_url(url, referer).await;
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded;charset=UTF-8"),
        );

        let body = encode_form(&form);
        for attempt in 0..3 {
            let resp = match self
                .client
                .post(url)
                .headers(headers.clone())
                .body(body.clone())
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    if attempt < 2 && is_retryable_transport_error(&err) {
                        sleep(retry_delay(attempt)).await;
                        continue;
                    }
                    return Err(err.to_string());
                }
            };

            let status = resp.status();
            if status.is_success() {
                return resp.json::<Value>().await.map_err(|e| e.to_string());
            }

            if attempt < 2 && is_retryable_status(status.as_u16()) {
                sleep(retry_delay(attempt)).await;
                continue;
            }

            return Err(format!("HTTP {}", status));
        }

        Err("HTTP request failed".to_string())
    }

    /// JSON ボディの POST リクエストを送信して JSON レスポンスを取得する。
    pub(crate) async fn post_json_body(
        &self,
        url: &str,
        body: Value,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers_for_url(url).await;
        add_referer_header(&mut headers, referer);
        for attempt in 0..3 {
            let resp = match self
                .client
                .post(url)
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(err) => {
                    if attempt < 2 && is_retryable_transport_error(&err) {
                        sleep(retry_delay(attempt)).await;
                        continue;
                    }
                    return Err(err.to_string());
                }
            };

            let status = resp.status();
            if status.is_success() {
                return resp.json::<Value>().await.map_err(|e| e.to_string());
            }

            if attempt < 2 && is_retryable_status(status.as_u16()) {
                sleep(retry_delay(attempt)).await;
                continue;
            }

            return Err(format!("HTTP {}", status));
        }

        Err("HTTP request failed".to_string())
    }

    /// マルチパートフォームの POST リクエストを送信して JSON レスポンスを取得する。
    /// ファイルアップロード（プロフィール画像など）に使用する。リトライなし。
    pub(crate) async fn post_multipart_json(
        &self,
        url: &str,
        form: Form,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers_for_url(url).await;
        add_referer_header(&mut headers, referer);
        let resp = self
            .client
            .post(url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        if status.is_success() {
            return resp.json::<Value>().await.map_err(|e| e.to_string());
        }
        Err(format!("HTTP {}", status))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // セッション管理
    // ─────────────────────────────────────────────────────────────────────────

    /// セッション Cookie を設定してログイン状態にする。
    /// mr_id と unique の両方が揃っている場合のみ authed フラグを立てる。
    pub async fn login(&self, mr_id: String, unique: String) {
        let is_authed = !mr_id.is_empty() && !unique.is_empty();
        *self.mr_id.write().await = mr_id;
        *self.unique.write().await = unique;
        *self.authed.write().await = is_authed;
    }

    /// セッションをリセットしてゲスト状態に戻す
    pub async fn reset(&self) {
        *self.mr_id.write().await = String::new();
        *self.unique.write().await = String::new();
        *self.authed.write().await = false;
        self.clear_csrf_token().await;
    }

    /// ログイン済みかどうかを返す（mr_id と unique の両方が揃っているか）
    pub(crate) async fn is_authed(&self) -> bool {
        *self.authed.read().await
    }

    /// セッション Cookie が設定されているかどうかを返す
    pub(crate) async fn has_session(&self) -> bool {
        let mr_id = self.mr_id.read().await;
        let unique = self.unique.read().await;
        !mr_id.is_empty() && !unique.is_empty()
    }

    /// セッション Cookie が未設定の場合のみ値をセットする（上書きしない）。
    /// ゲストセッション取得時に既存セッションを保護するために使用する。
    pub(crate) async fn set_session_if_empty(&self, mr_id: Option<String>, unique: Option<String>) {
        if let Some(mr) = mr_id {
            let mut current = self.mr_id.write().await;
            if current.is_empty() {
                *current = mr;
            }
        }
        if let Some(f) = unique {
            let mut current = self.unique.write().await;
            if current.is_empty() {
                *current = f;
            }
        }
    }

    /// ゲストセッションを取得する。
    ///
    /// /api/user/me にリクエストを送り、Set-Cookie ヘッダーから
    /// mr_id と f（unique）を抽出してセッションを初期化する。
    /// 既にセッションが設定済みの場合は上書きしない。
    pub(crate) async fn bootstrap_guest_session(&self) -> Result<(), String> {
        let mut headers = self
            .get_headers_for_url("https://www.mirrativ.com/api/user/me")
            .await;
        headers.insert("x-referer", HeaderValue::from_static("my_page"));

        let resp = self
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

        // Set-Cookie ヘッダーをすべて検査して必要な Cookie を取得
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

        self.set_session_if_empty(mr_id_cookie, f_cookie).await;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ヘルパー関数
// ─────────────────────────────────────────────────────────────────────────────

/// x-referer ヘッダーを追加する（None の場合は何もしない）
fn add_referer_header(headers: &mut HeaderMap, referer: Option<&str>) {
    if let Some(value) = referer {
        headers.insert(
            "x-referer",
            HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
    }
}

/// HashMap をフォームエンコードされた文字列に変換する（key=value&... 形式）
pub(crate) fn encode_form(form: &HashMap<String, String>) -> String {
    form.iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// リトライすべき HTTP ステータスコードかどうかを判定する
/// 対象: 429 (Too Many Requests), 500/502/503/504 (サーバーエラー)
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// リトライすべきトランスポート層のエラーかどうかを判定する
fn is_retryable_transport_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

/// 指定 URL が Mirrativ ドメイン向けかどうかを判定する
/// （Cookie を付加するかどうかの判断に使用）
fn should_attach_session_cookie(target_url: &str) -> bool {
    url::Url::parse(target_url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(is_mirrativ_host))
        .unwrap_or(false)
}

/// ホスト名が Mirrativ のドメインかどうかを判定する
fn is_mirrativ_host(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();
    host == "mirrativ.com" || host.ends_with(".mirrativ.com")
}

/// リトライ時の待機時間を返す（指数バックオフ）
/// attempt 0: 200ms, 1: 500ms, 2以上: 900ms
fn retry_delay(attempt: usize) -> Duration {
    match attempt {
        0 => Duration::from_millis(200),
        1 => Duration::from_millis(500),
        _ => Duration::from_millis(900),
    }
}

/// 現在時刻を Unix タイムスタンプ（秒.ミリ秒）の文字列で返す
fn current_unixtime() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{}.{}",
        now.as_secs(),
        format!("{:03}", now.subsec_millis())
    )
}

/// Set-Cookie ヘッダーの値から "name=value" ペアを抽出する
/// セミコロン以降の属性（Path, Expires など）は無視する
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

/// HTML から `<meta name="csrf-token" content="...">` の値を抽出する
fn extract_csrf_token(html: &str) -> Option<String> {
    // パターン: <meta name="csrf-token" content="TOKEN">
    // name と content の順序は問わない
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(idx) = lower[pos..].find("<meta ") {
        let start = pos + idx;
        let end = match lower[start..].find('>') {
            Some(e) => start + e + 1,
            None => break,
        };
        let tag = &html[start..end];
        let tag_lower = &lower[start..end];

        if tag_lower.contains("csrf-token") {
            // content="..." を抽出
            if let Some(ci) = tag_lower.find("content=") {
                let rest = &tag[ci + 8..];
                let quote = rest.chars().next()?;
                if quote == '"' || quote == '\'' {
                    let inner = &rest[1..];
                    if let Some(end_q) = inner.find(quote) {
                        return Some(inner[..end_q].to_string());
                    }
                }
            }
        }
        pos = end;
    }
    None
}

/// 指定長のランダムな 16 進文字列を生成する（端末 ID 等に使用）
fn random_hex(length: usize) -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    (0..length)
        .map(|_| format!("{:x}", rng.random::<u8>() % 16))
        .collect()
}
