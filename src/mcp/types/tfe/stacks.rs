use schemars::JsonSchema;
use serde::Deserialize;

/// Organization-scoped stack list request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeStacksInput {
    /// Organization name
    pub organization: String,
    /// Page number (default: 1)
    pub page_number: Option<u16>,
    /// Page size, clamped to 1..=100 (default: 20)
    pub page_size: Option<u16>,
}

/// Stack details request
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TfeStackInput {
    /// Stack ID (e.g., st-...)
    pub stack_id: String,
}
