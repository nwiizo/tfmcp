//! Bounded TFE API response handling.

use serde_json::Value;

pub(super) struct BoundedResponseBody {
    pub body: String,
    pub truncated: bool,
    pub original_bytes: u64,
}

pub(super) async fn read_bounded_response(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<BoundedResponseBody, reqwest::Error> {
    let content_length = response.content_length();
    let mut bytes = Vec::with_capacity(max_bytes.min(8 * 1024));
    let mut observed_bytes = 0_u64;
    let mut truncated = false;

    while let Some(chunk) = response.chunk().await? {
        observed_bytes = observed_bytes.saturating_add(chunk.len() as u64);
        let remaining = max_bytes.saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..remaining.min(chunk.len())]);
        if observed_bytes > max_bytes as u64 {
            truncated = true;
            break;
        }
    }

    Ok(BoundedResponseBody {
        body: String::from_utf8_lossy(&bytes).into_owned(),
        truncated,
        original_bytes: content_length.unwrap_or(observed_bytes),
    })
}

pub(super) fn truncated_json_response_with_original(
    body: &str,
    max_bytes: usize,
    original_bytes: u64,
) -> Value {
    serde_json::json!({
        "truncated": true,
        "original_bytes": original_bytes,
        "max_bytes": max_bytes,
        "preview": utf8_prefix(body, max_bytes)
    })
}

pub(super) fn mark_truncated_text(body: String, original_bytes: u64, max_bytes: usize) -> String {
    format!(
        "{body}\n\n[tfmcp: truncated TFE response from at least {original_bytes} bytes to {max_bytes} bytes; set TFE_MAX_RESPONSE_BYTES to adjust]"
    )
}

fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = 0;
    for (index, _) in value.char_indices() {
        if index > max_bytes {
            break;
        }
        end = index;
    }
    &value[..end]
}

pub(super) fn extract_log_read_url(value: &Value) -> Option<String> {
    value
        .pointer("/data/attributes/log-read-url")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}
