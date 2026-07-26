use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TfeError {
    #[error("TFE_TOKEN is not configured")]
    MissingToken,

    #[error("Invalid TFE_ADDRESS: {0}")]
    InvalidAddress(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("TFE API returned {status}: {body}")]
    Api { status: StatusCode, body: String },

    #[error("JSON parsing failed: {0}")]
    Json(String),

    #[error("Response did not contain a log-read-url")]
    MissingLogReadUrl,

    #[error(
        "Terraform operations are disabled for {operation}. Set ENABLE_TF_OPERATIONS=true to enable gated HCP Terraform/TFE write tools."
    )]
    OperationDisabled { operation: String },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<reqwest::Error> for TfeError {
    fn from(error: reqwest::Error) -> Self {
        TfeError::Http(error.to_string())
    }
}

impl From<serde_json::Error> for TfeError {
    fn from(error: serde_json::Error) -> Self {
        TfeError::Json(error.to_string())
    }
}
