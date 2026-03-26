//! Terraform Registry Policy API client.
//!
//! Provides search and detail lookup for Sentinel/OPA policy libraries
//! from the public Terraform Registry.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, warn};

const REGISTRY_BASE: &str = "https://registry.terraform.io";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// A policy library entry from the Terraform Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInfo {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub full_name: String,
    pub title: String,
    pub description: String,
    pub source: String,
    pub downloads: u64,
    pub verified: bool,
}

/// Client for the Terraform Registry Policy API.
pub struct PolicyClient {
    client: Client,
}

impl Default for PolicyClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Search for policy libraries in the public registry.
    pub async fn search_policies(
        &self,
        query: &str,
        provider_filter: Option<&str>,
    ) -> anyhow::Result<Vec<PolicyInfo>> {
        let mut url = format!("{}/v2/policies?page[size]=20", REGISTRY_BASE);
        if let Some(provider) = provider_filter {
            url.push_str(&format!("&filter[provider]={}", provider));
        }

        debug!("Searching policies: {}", url);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            warn!("Policy search failed: HTTP {}", response.status());
            return Ok(Vec::new());
        }

        let body: Value = response.json().await?;
        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut results = Vec::new();
        for item in &data {
            if let Some(info) = parse_policy_item(item) {
                // Apply text query filter (API doesn't support text search natively)
                let q = query.to_lowercase();
                if info.name.to_lowercase().contains(&q)
                    || info.title.to_lowercase().contains(&q)
                    || info.namespace.to_lowercase().contains(&q)
                    || info.full_name.to_lowercase().contains(&q)
                {
                    results.push(info);
                }
            }
        }

        // If no matches with text filter, return all (query might be a provider name)
        if results.is_empty() && !data.is_empty() {
            for item in &data {
                if let Some(info) = parse_policy_item(item) {
                    results.push(info);
                }
            }
        }

        debug!("Found {} policies", results.len());
        Ok(results)
    }

    /// Get details for a specific policy library.
    pub async fn get_policy_details(
        &self,
        namespace: &str,
        name: &str,
    ) -> anyhow::Result<PolicyInfo> {
        let url = format!("{}/v2/policies/{}/{}", REGISTRY_BASE, namespace, name);
        debug!("Fetching policy details: {}", url);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Policy not found: {}/{} (HTTP {})",
                namespace,
                name,
                response.status()
            ));
        }

        let body: Value = response.json().await?;
        let item = body
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("Invalid response: missing 'data' field"))?;

        parse_policy_item(item).ok_or_else(|| {
            anyhow::anyhow!("Failed to parse policy details for {}/{}", namespace, name)
        })
    }
}

fn parse_policy_item(item: &Value) -> Option<PolicyInfo> {
    let attrs = item.get("attributes")?;
    Some(PolicyInfo {
        id: item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        name: attrs
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        namespace: attrs
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        full_name: attrs
            .get("full-name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        title: attrs
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        description: attrs
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        source: attrs
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        downloads: attrs.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
        verified: attrs
            .get("verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_policy_item() {
        let item = serde_json::json!({
            "type": "policy-libraries",
            "id": "140",
            "attributes": {
                "name": "CIS-Policy-Set-for-AWS",
                "namespace": "hashicorp",
                "full-name": "hashicorp/CIS-Policy-Set-for-AWS",
                "title": "CIS Policies for AWS",
                "source": "https://github.com/hashicorp/policy-library",
                "downloads": 1000,
                "verified": true
            }
        });

        let info = parse_policy_item(&item).expect("should parse");
        assert_eq!(info.name, "CIS-Policy-Set-for-AWS");
        assert_eq!(info.namespace, "hashicorp");
        assert_eq!(info.downloads, 1000);
        assert!(info.verified);
    }

    #[test]
    fn test_policy_client_creation() {
        let _client = PolicyClient::new();
    }
}
