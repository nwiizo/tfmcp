use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// Flexible provider info structure that can handle multiple API versions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderInfo {
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub id: String,
    // Additional fields for API compatibility
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub trusted: Option<bool>,
    // Catch unknown fields to avoid parsing failures
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIdResult {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub subcategory: Option<String>,
    // Catch unknown fields
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderVersions {
    /// Registry returns versions as either `["1.0"]` or `[{"version":"1.0",...}]`
    #[serde(default, deserialize_with = "deserialize_versions")]
    pub versions: Vec<String>,
    // Handle alternative response formats
    #[serde(default)]
    pub data: Option<Vec<VersionInfo>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Deserializes versions from either a string array or an array of objects
/// with a `version` field. The Terraform Registry API returns the latter.
fn deserialize_versions<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Vec<Value> = Vec::deserialize(deserializer).unwrap_or_default();
    Ok(raw
        .into_iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s),
            Value::Object(ref map) => map
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub protocols: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySearchResponse {
    #[serde(default)]
    pub providers: Vec<ProviderInfo>,
    #[serde(default)]
    pub meta: HashMap<String, Value>,
    // Handle alternative response formats
    #[serde(default)]
    pub data: Option<Vec<ProviderInfo>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDocsResponse {
    #[serde(default)]
    pub data: Vec<DocIdResult>,
    // Handle alternative response formats
    #[serde(default)]
    pub docs: Option<Vec<DocIdResult>>,
    #[serde(default)]
    pub documentation: Option<Vec<DocIdResult>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
