use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    multipart::Form,
    Client,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tokio::sync::RwLock;

/// Mirrativ APIクライアント
pub struct MirrativClient {
    pub(crate) client: Arc<Client>,
    lang: String,
    mr_id: RwLock<String>,
    unique: RwLock<String>,
    authed: RwLock<bool>,
    custom_headers: HeaderMap,
}

impl MirrativClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .cookie_store(false)
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
        }
    }

    /// カスタムヘッダーを生成（Mirrativ API用）
    fn create_custom_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();

        // User-Agent
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("MR_APP/11.56.0/Android/PGEM10/9"),
        );

        headers.insert("Accept-Language", HeaderValue::from_static("ja-JP"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));

        // Mirrativ固有ヘッダー
        headers.insert("HTTP_X_TIMEZONE", HeaderValue::from_static("Asia/Tokyo"));
        headers.insert("x-idfv", HeaderValue::from_str(&random_hex(16)).unwrap());
        headers.insert(
            "x-ad",
            HeaderValue::from_str(&uuid::Uuid::new_v4().to_string()).unwrap(),
        );
        headers.insert("x-hw", HeaderValue::from_static("qcom"));
        headers.insert("x-widevine-id", HeaderValue::from_static(""));
        headers.insert("x-network-status", HeaderValue::from_static("2"));
        headers.insert("x-os-push", HeaderValue::from_static("1"));
        headers.insert(
            "x-adjust-adid",
            HeaderValue::from_str(&random_hex(32)).unwrap(),
        );
        headers.insert("x-unity-framework", HeaderValue::from_static("6.4.0"));

        headers
    }

    /// ヘッダーを取得（カスタムヘッダー + Cookie）
    pub(crate) async fn get_headers(&self) -> HeaderMap {
        let mut headers = self.custom_headers.clone();

        headers.insert(
            "x-client-unixtime",
            HeaderValue::from_str(&current_unixtime())
                .unwrap_or_else(|_| HeaderValue::from_static("0")),
        );

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

        headers
    }

    /// GETリクエストを送信してJSONを取得
    pub(crate) async fn fetch_json(
        &self,
        url: &str,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers().await;
        add_referer_header(&mut headers, referer);

        for attempt in 0..3 {
            let resp = self
                .client
                .get(url)
                .headers(headers.clone())
                .send()
                .await
                .map_err(|e| e.to_string())?;

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

    /// POSTリクエストを送信してJSONを取得
    pub(crate) async fn post_json(
        &self,
        url: &str,
        form: HashMap<String, String>,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers().await;
        add_referer_header(&mut headers, referer);
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
        );

        let body = encode_form(&form);
        for attempt in 0..3 {
            let resp = self
                .client
                .post(url)
                .headers(headers.clone())
                .body(body.clone())
                .send()
                .await
                .map_err(|e| e.to_string())?;

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

    pub(crate) async fn post_json_body(
        &self,
        url: &str,
        body: Value,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers().await;
        add_referer_header(&mut headers, referer);
        for attempt in 0..3 {
            let resp = self
                .client
                .post(url)
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

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

    pub(crate) async fn post_multipart_json(
        &self,
        url: &str,
        form: Form,
        referer: Option<&str>,
    ) -> Result<Value, String> {
        let mut headers = self.get_headers().await;
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

    /// ログイン（Cookie設定）
    pub async fn login(&self, mr_id: String, unique: String) {
        let is_authed = !mr_id.is_empty() && !unique.is_empty();
        *self.mr_id.write().await = mr_id;
        *self.unique.write().await = unique;
        *self.authed.write().await = is_authed;
    }

    /// 認証済みかどうか（ゲストセッションは false）
    pub(crate) async fn is_authed(&self) -> bool {
        *self.authed.read().await
    }

    /// セッションをリセット（authed フラグもクリア）
    pub async fn reset(&self) {
        *self.mr_id.write().await = String::new();
        *self.unique.write().await = String::new();
        *self.authed.write().await = false;
    }

    pub(crate) async fn has_session(&self) -> bool {
        let mr_id = self.mr_id.read().await;
        let unique = self.unique.read().await;
        !mr_id.is_empty() || !unique.is_empty()
    }

    pub(crate) async fn set_session_if_empty(
        &self,
        mr_id: Option<String>,
        unique: Option<String>,
    ) {
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
}

fn add_referer_header(headers: &mut HeaderMap, referer: Option<&str>) {
    if let Some(value) = referer {
        headers.insert(
            "x-referer",
            HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
    }
}

pub(crate) fn encode_form(form: &HashMap<String, String>) -> String {
    form.iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn is_retryable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

fn retry_delay(attempt: usize) -> Duration {
    match attempt {
        0 => Duration::from_millis(200),
        1 => Duration::from_millis(500),
        _ => Duration::from_millis(900),
    }
}

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

/// ランダムな16進文字列を生成
fn random_hex(length: usize) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..length)
        .map(|_| format!("{:x}", rng.random::<u8>() % 16))
        .collect()
}
