use crate::registry::client::{DocIdResult, ProviderInfo};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Output formatter for structured, user-friendly results
#[allow(dead_code)]
pub struct OutputFormatter;

impl OutputFormatter {
    /// Format provider search results in a structured way
    pub fn format_provider_list(providers: Vec<ProviderInfo>) -> Value {
        json!({
            "summary": {
                "total_providers": providers.len(),
                "description": "List of Terraform providers matching your search criteria"
            },
            "providers": providers.iter().map(|provider| {
                json!({
                    "id": format!("{}/{}", provider.namespace, provider.name),
                    "name": provider.name,
                    "namespace": provider.namespace,
                    "version": provider.version,
                    "description": provider.description,
                    "downloads": provider.downloads,
                    "published_at": provider.published_at,
                    "registry_url": format!("https://registry.terraform.io/providers/{}/{}", provider.namespace, provider.name)
                })
            }).collect::<Vec<_>>(),
            "usage_note": "Use the 'id' field to reference these providers in your Terraform configuration"
        })
    }

    /// Format provider information with comprehensive details
    pub fn format_provider_details(
        provider: &ProviderInfo,
        versions: Option<Vec<String>>,
        docs: Option<Vec<DocIdResult>>,
    ) -> Value {
        let mut result = json!({
            "provider": {
                "id": format!("{}/{}", provider.namespace, provider.name),
                "name": provider.name,
                "namespace": provider.namespace,
                "current_version": provider.version,
                "description": provider.description,
                "downloads": provider.downloads,
                "published_at": provider.published_at,
                "registry_url": format!("https://registry.terraform.io/providers/{}/{}", provider.namespace, provider.name)
            }
        });

        if let Some(versions) = versions {
            result["versions"] = json!({
                "available": versions,
                "latest": versions.first().unwrap_or(&provider.version),
                "count": versions.len()
            });
        }

        if let Some(docs) = docs {
            result["documentation"] = json!({
                "available_docs": docs.iter().map(|doc| {
                    json!({
                        "id": doc.id,
                        "title": doc.title,
                        "category": doc.category,
                        "description": doc.description
                    })
                }).collect::<Vec<_>>(),
                "doc_count": docs.len()
            });
        }

        result["terraform_config_example"] = json!(format!(
            r#"terraform {{
  required_providers {{
    {} = {{
      source  = "{}/{}"
      version = "~> {}"
    }}
  }}
}}"#,
            provider.name, provider.namespace, provider.name, provider.version
        ));

        result
    }

    /// Format documentation search results
    pub fn format_documentation_results(docs: Vec<DocIdResult>, provider_name: &str) -> Value {
        json!({
            "summary": {
                "provider": provider_name,
                "total_docs": docs.len(),
                "description": format!("Documentation available for {} provider", provider_name)
            },
            "documentation": docs.iter().map(|doc| {
                json!({
                    "id": doc.id,
                    "title": doc.title,
                    "category": doc.category,
                    "description": doc.description,
                    "url": format!("https://registry.terraform.io/providers/{}/latest/docs/{}", provider_name, doc.id)
                })
            }).collect::<Vec<_>>(),
            "usage_note": "Use the 'id' field to fetch detailed documentation content"
        })
    }

    /// Format error messages with helpful suggestions
    pub fn format_error_with_suggestions(
        error: &str,
        suggestions: Option<Vec<String>>,
        provider_hints: Option<Vec<ProviderInfo>>,
    ) -> Value {
        let mut result = json!({
            "error": {
                "message": error,
                "type": "provider_resolution_error"
            }
        });

        if let Some(suggestions) = suggestions {
            result["suggestions"] = json!({
                "recommended_actions": suggestions,
                "description": "Try these steps to resolve the issue"
            });
        }

        if let Some(providers) = provider_hints {
            result["similar_providers"] = json!({
                "description": "Did you mean one of these providers?",
                "providers": providers.iter().take(5).map(|p| {
                    json!({
                        "id": format!("{}/{}", p.namespace, p.name),
                        "name": p.name,
                        "namespace": p.namespace,
                        "description": p.description
                    })
                }).collect::<Vec<_>>()
            });
        }

        result
    }

    /// Format validation results with structured details
    pub fn format_validation_results(
        valid: bool,
        errors: Vec<String>,
        warnings: Vec<String>,
        suggestions: Vec<String>,
    ) -> Value {
        json!({
            "validation": {
                "status": if valid { "valid" } else { "invalid" },
                "summary": {
                    "error_count": errors.len(),
                    "warning_count": warnings.len(),
                    "suggestion_count": suggestions.len()
                }
            },
            "details": {
                "errors": errors,
                "warnings": warnings,
                "suggestions": suggestions
            },
            "next_steps": if valid {
                vec!["Configuration is valid and ready to use"]
            } else {
                vec!["Fix the errors listed above", "Review warnings for best practices", "Consider implementing the suggestions"]
            }
        })
    }


    /// Format comprehensive provider comparison
    pub fn format_provider_comparison(providers: Vec<(ProviderInfo, Vec<String>)>) -> Value {
        json!({
            "comparison": {
                "total_providers": providers.len(),
                "description": "Comparison of multiple Terraform providers"
            },
            "providers": providers.iter().map(|(provider, versions)| {
                json!({
                    "id": format!("{}/{}", provider.namespace, provider.name),
                    "name": provider.name,
                    "namespace": provider.namespace,
                    "current_version": provider.version,
                    "description": provider.description,
                    "downloads": provider.downloads,
                    "version_count": versions.len(),
                    "latest_versions": versions.iter().take(3).collect::<Vec<_>>(),
                    "popularity_score": Self::calculate_popularity_score(provider.downloads),
                    "maturity": Self::assess_maturity(versions)
                })
            }).collect::<Vec<_>>(),
            "recommendations": Self::generate_provider_recommendations(&providers)
        })
    }

    /// Format cache statistics
    pub fn format_cache_stats(stats: HashMap<String, crate::registry::cache::CacheStats>) -> Value {
        json!({
            "cache_statistics": {
                "summary": {
                    "total_caches": stats.len(),
                    "description": "Performance statistics for Registry API caching"
                },
                "caches": stats.iter().map(|(name, stat)| {
                    json!({
                        "cache_name": name,
                        "total_entries": stat.total_entries,
                        "valid_entries": stat.valid_entries,
                        "expired_entries": stat.expired_entries,
                        "hit_rate": if stat.total_entries > 0 {
                            (stat.valid_entries as f64 / stat.total_entries as f64) * 100.0
                        } else { 0.0 }
                    })
                }).collect::<Vec<_>>(),
                "overall_efficiency": Self::calculate_overall_cache_efficiency(&stats)
            }
        })
    }

    // Helper methods
    fn calculate_popularity_score(downloads: u64) -> &'static str {
        match downloads {
            0..=1000 => "low",
            1001..=10000 => "moderate",
            10001..=100000 => "high",
            _ => "very_high",
        }
    }

    fn assess_maturity(versions: &[String]) -> &'static str {
        match versions.len() {
            0..=5 => "early",
            6..=20 => "developing",
            21..=50 => "mature",
            _ => "established",
        }
    }

    fn generate_provider_recommendations(providers: &[(ProviderInfo, Vec<String>)]) -> Vec<Value> {
        let mut recommendations = Vec::new();

        if let Some((best_provider, _)) = providers.iter().max_by_key(|(p, _)| p.downloads) {
            recommendations.push(json!({
                "type": "most_popular",
                "provider": format!("{}/{}", best_provider.namespace, best_provider.name),
                "reason": "Highest download count indicates strong community adoption"
            }));
        }

        if let Some((most_versions, versions)) = providers.iter().max_by_key(|(_, v)| v.len()) {
            recommendations.push(json!({
                "type": "most_mature",
                "provider": format!("{}/{}", most_versions.namespace, most_versions.name),
                "reason": format!("Has {} versions, indicating active development", versions.len())
            }));
        }

        recommendations
    }

    fn calculate_overall_cache_efficiency(
        stats: &HashMap<String, crate::registry::cache::CacheStats>,
    ) -> f64 {
        let total_entries: usize = stats.values().map(|s| s.total_entries).sum();
        let valid_entries: usize = stats.values().map(|s| s.valid_entries).sum();

        if total_entries > 0 {
            (valid_entries as f64 / total_entries as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_provider_list() {
        let providers = vec![ProviderInfo {
            name: "aws".to_string(),
            namespace: "hashicorp".to_string(),
            version: "5.0.0".to_string(),
            description: "AWS Provider".to_string(),
            downloads: 1000000,
            published_at: "2023-01-01".to_string(),
            id: "hashicorp/aws".to_string(),
            source: None,
            tag: None,
            logo_url: None,
            owner: None,
            tier: None,
            verified: None,
            trusted: None,
            extra: std::collections::HashMap::new(),
        }];

        let formatted = OutputFormatter::format_provider_list(providers);
        assert_eq!(formatted["summary"]["total_providers"], 1);
        assert_eq!(formatted["providers"][0]["name"], "aws");
    }

    #[test]
    fn test_format_error_with_suggestions() {
        let error = "Provider not found";
        let suggestions = Some(vec!["Check provider name".to_string()]);

        let formatted = OutputFormatter::format_error_with_suggestions(error, suggestions, None);
        assert_eq!(formatted["error"]["message"], "Provider not found");
        assert!(formatted["suggestions"].is_object());
    }

    #[test]
    fn test_popularity_score() {
        assert_eq!(OutputFormatter::calculate_popularity_score(500), "low");
        assert_eq!(
            OutputFormatter::calculate_popularity_score(5000),
            "moderate"
        );
        assert_eq!(OutputFormatter::calculate_popularity_score(50000), "high");
        assert_eq!(
            OutputFormatter::calculate_popularity_score(500000),
            "very_high"
        );
    }
}
