// src-tauri/src/mirrativ/error.rs
use std::fmt;

/// Mirrativ API のカスタムエラー型（将来の使用のため準備）
#[allow(dead_code)]
#[derive(Debug)]
pub enum MirrativError {
    /// ネットワークエラー
    Network(reqwest::Error),
    /// JSONパースエラー
    Parse(serde_json::Error),
    /// 無効なレスポンス
    InvalidResponse(String),
    /// タイムアウト
    Timeout(String),
    /// リソースが見つからない（404）
    NotFound(String),
    /// 認証エラー（401/403）
    Unauthorized,
    /// 内部エラー
    Internal(String),
}

impl fmt::Display for MirrativError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(e) => write!(f, "ネットワークエラー: {}", e),
            Self::Parse(e) => write!(f, "パースエラー: {}", e),
            Self::InvalidResponse(msg) => write!(f, "無効なレスポンス: {}", msg),
            Self::Timeout(msg) => write!(f, "タイムアウト: {}", msg),
            Self::NotFound(msg) => write!(f, "見つかりません: {}", msg),
            Self::Unauthorized => write!(f, "認証エラー"),
            Self::Internal(msg) => write!(f, "内部エラー: {}", msg),
        }
    }
}

impl std::error::Error for MirrativError {}

// reqwest::Error からの自動変換
impl From<reqwest::Error> for MirrativError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout(e.to_string())
        } else if e.is_status() {
            if let Some(status) = e.status() {
                if status == 401 || status == 403 {
                    return Self::Unauthorized;
                } else if status == 404 {
                    return Self::NotFound(e.to_string());
                }
            }
            Self::Network(e)
        } else {
            Self::Network(e)
        }
    }
}

// serde_json::Error からの自動変換
impl From<serde_json::Error> for MirrativError {
    fn from(e: serde_json::Error) -> Self {
        Self::Parse(e)
    }
}

// Tauri commands用にSerialize実装（Stringに変換）
impl serde::Serialize for MirrativError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
