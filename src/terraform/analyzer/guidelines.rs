//! Future Architect Terraform guideline checks.

use crate::terraform::model::core::TerraformAnalysis;
use crate::terraform::model::validation::{
    CountUsageWarning, GuidelineCheckResult, SecretDetection,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static ANY_TYPE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"type\s*=\s*any\b"#).expect("Invalid any type regex"));

static COUNT_VALUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"resource\s+"([^"]+)"\s+"([^"]+)"\s*\{[^}]*count\s*=\s*([^\n]+)"#)
        .expect("Invalid count value regex")
});

static DEFAULT_TAGS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"default_tags\s*\{"#).expect("Invalid default_tags regex"));

static LIFECYCLE_PREVENT_DESTROY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"lifecycle\s*\{[^}]*prevent_destroy\s*=\s*true"#).expect("Invalid lifecycle regex")
});

static SECRET_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        (
            "AWS Access Key",
            Regex::new(r#"(?i)(aws_access_key_id|access_key)\s*=\s*"[A-Z0-9]{20}""#)
                .expect("Invalid AWS access key regex"),
        ),
        (
            "AWS Secret Key",
            Regex::new(r#"(?i)(aws_secret_access_key|secret_key)\s*=\s*"[A-Za-z0-9/+=]{40}""#)
                .expect("Invalid AWS secret key regex"),
        ),
        (
            "Generic API Key",
            Regex::new(r#"(?i)(api_key|apikey)\s*=\s*"[A-Za-z0-9_-]{20,}""#)
                .expect("Invalid API key regex"),
        ),
        (
            "Generic Secret",
            Regex::new(r#"(?i)(password|secret|token)\s*=\s*"[^"]{8,}""#)
                .expect("Invalid secret regex"),
        ),
        (
            "Private Key",
            Regex::new(r#"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----"#)
                .expect("Invalid private key regex"),
        ),
    ]
});

static CRITICAL_RESOURCE_TYPES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "aws_db_instance",
        "aws_rds_cluster",
        "aws_s3_bucket",
        "aws_dynamodb_table",
        "aws_elasticsearch_domain",
        "aws_elasticache_cluster",
        "aws_kms_key",
        "google_sql_database_instance",
        "google_storage_bucket",
        "azurerm_sql_database",
        "azurerm_storage_account",
    ]
    .into_iter()
    .collect()
});

// ==================== Future Architect Guideline Checks ====================

/// Check Terraform configurations against Future Architect guidelines
/// Reference: https://future-architect.github.io/coding-standards/documents/forTerraform/
pub fn check_guidelines(
    analysis: &TerraformAnalysis,
    file_contents: &HashMap<String, String>,
) -> GuidelineCheckResult {
    let mut result = GuidelineCheckResult::default();

    // Check variables missing type
    for var in &analysis.variables {
        if var.type_.is_none() {
            result.variables_missing_type.push(var.name.clone());
        }
    }

    // Check variables missing description
    for var in &analysis.variables {
        if var.description.is_none()
            || var
                .description
                .as_ref()
                .is_some_and(|d| d.trim().is_empty())
        {
            result.variables_missing_description.push(var.name.clone());
        }
    }

    // Check outputs missing description
    for output in &analysis.outputs {
        if output.description.is_none()
            || output
                .description
                .as_ref()
                .is_some_and(|d| d.trim().is_empty())
        {
            result.outputs_missing_description.push(output.name.clone());
        }
    }

    // Check providers missing version
    for provider in &analysis.providers {
        if provider.version.is_none() {
            result.providers_missing_version.push(provider.name.clone());
        }
    }

    // Check file contents for patterns
    let mut has_aws_provider = false;
    let mut has_default_tags = false;
    let mut resources_with_lifecycle: HashSet<String> = HashSet::new();

    for (filename, content) in file_contents {
        // Check for AWS provider
        if content.contains("provider \"aws\"") || content.contains("aws_") {
            has_aws_provider = true;
        }

        // Check for default_tags
        if DEFAULT_TAGS_REGEX.is_match(content) {
            has_default_tags = true;
        }

        // Check for any type usage
        if ANY_TYPE_REGEX.is_match(content) {
            // Find variable names using any type
            for line in content.lines() {
                if line.contains("type") && line.contains("any") {
                    // Try to find the variable name
                    if let Some(start) = content.find("variable \"")
                        && let Some(end) = content[start + 10..].find('"')
                    {
                        let var_name = &content[start + 10..start + 10 + end];
                        if !result.any_type_usage.contains(&var_name.to_string()) {
                            result.any_type_usage.push(var_name.to_string());
                        }
                    }
                }
            }
        }

        // Check for count usage that should be for_each
        check_count_usage(content, filename, &mut result.count_instead_of_foreach);

        // Check for hardcoded secrets
        check_secrets(content, filename, &mut result.hardcoded_secrets);

        // Check for lifecycle prevent_destroy
        if LIFECYCLE_PREVENT_DESTROY_REGEX.is_match(content) {
            for resource in &analysis.resources {
                if content.contains(&format!(
                    "resource \"{}\" \"{}\"",
                    resource.resource_type, resource.name
                )) {
                    resources_with_lifecycle
                        .insert(format!("{}.{}", resource.resource_type, resource.name));
                }
            }
        }
    }

    // Set missing_default_tags for AWS projects
    result.missing_default_tags = has_aws_provider && !has_default_tags;

    // Check for critical resources missing lifecycle protection
    for resource in &analysis.resources {
        if CRITICAL_RESOURCE_TYPES.contains(resource.resource_type.as_str()) {
            let resource_id = format!("{}.{}", resource.resource_type, resource.name);
            if !resources_with_lifecycle.contains(&resource_id) {
                result.missing_lifecycle_protection.push(resource_id);
            }
        }
    }

    // Calculate compliance score
    result.compliance_score = calculate_compliance_score(&result, analysis);

    result
}

/// Check for count usage that should be for_each
fn check_count_usage(content: &str, filename: &str, warnings: &mut Vec<CountUsageWarning>) {
    // Simple heuristic: if count is used with a variable or expression that isn't 0 or 1,
    // it might be better as for_each
    for cap in COUNT_VALUE_REGEX.captures_iter(content) {
        let resource_type = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let resource_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let count_value = cap.get(3).map(|m| m.as_str()).unwrap_or("").trim();

        // Skip if count is used for 0/1 toggle (conditional creation)
        let is_toggle = count_value == "0"
            || count_value == "1"
            || count_value.contains("? 1 : 0")
            || count_value.contains("? 0 : 1")
            || count_value.starts_with("var.enable_")
            || count_value.starts_with("var.create_")
            || count_value.starts_with("local.enable_")
            || count_value.starts_with("local.create_");

        if !is_toggle {
            warnings.push(CountUsageWarning {
                resource_name: format!("{resource_type}.{resource_name}"),
                resource_type: resource_type.to_string(),
                suggestion: format!(
                    "Consider using for_each instead of count in {resource_name} (file: {filename}). \
                     for_each provides stable resource addresses when items are added/removed."
                ),
            });
        }
    }
}

/// Check for hardcoded secrets in content
fn check_secrets(content: &str, filename: &str, detections: &mut Vec<SecretDetection>) {
    for (line_num, line) in content.lines().enumerate() {
        // Skip comments
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        for (pattern_name, regex) in SECRET_PATTERNS.iter() {
            if regex.is_match(line) {
                detections.push(SecretDetection {
                    file: filename.to_string(),
                    line: line_num + 1,
                    pattern: pattern_name.to_string(),
                    severity: if *pattern_name == "Private Key" || pattern_name.contains("Secret") {
                        "critical".to_string()
                    } else {
                        "high".to_string()
                    },
                });
            }
        }
    }
}

/// Calculate compliance score based on guideline violations
fn calculate_compliance_score(result: &GuidelineCheckResult, analysis: &TerraformAnalysis) -> u8 {
    let mut score: i32 = 100;

    // Variables missing type: -3 points each, max -15
    let type_penalty = (result.variables_missing_type.len() as i32 * 3).min(15);
    score -= type_penalty;

    // Variables missing description: -2 points each, max -10
    let var_desc_penalty = (result.variables_missing_description.len() as i32 * 2).min(10);
    score -= var_desc_penalty;

    // Outputs missing description: -2 points each, max -10
    let out_desc_penalty = (result.outputs_missing_description.len() as i32 * 2).min(10);
    score -= out_desc_penalty;

    // count instead of for_each: -5 points each, max -15
    let count_penalty = (result.count_instead_of_foreach.len() as i32 * 5).min(15);
    score -= count_penalty;

    // any type usage: -5 points each, max -10
    let any_penalty = (result.any_type_usage.len() as i32 * 5).min(10);
    score -= any_penalty;

    // Providers missing version: -5 points each, max -10
    let version_penalty = (result.providers_missing_version.len() as i32 * 5).min(10);
    score -= version_penalty;

    // Missing default_tags for AWS: -10 points
    if result.missing_default_tags && !analysis.providers.is_empty() {
        score -= 10;
    }

    // Hardcoded secrets: -20 points each (critical issue)
    let secret_penalty = (result.hardcoded_secrets.len() as i32 * 20).min(40);
    score -= secret_penalty;

    // Missing lifecycle protection: -5 points each, max -15
    let lifecycle_penalty = (result.missing_lifecycle_protection.len() as i32 * 5).min(15);
    score -= lifecycle_penalty;

    score.clamp(0, 100) as u8
}
