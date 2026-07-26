//! Environment-derived TFE client configuration.

pub(super) fn normalize_address(address: &str) -> String {
    address.trim().trim_end_matches('/').to_string()
}

pub(super) fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

pub(super) fn response_byte_limit() -> usize {
    std::env::var("TFE_MAX_RESPONSE_BYTES")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|limit| *limit > 0)
        .unwrap_or(super::DEFAULT_MAX_RESPONSE_BYTES)
}

pub(super) fn operations_enabled() -> bool {
    env_bool("ENABLE_TF_OPERATIONS")
}
