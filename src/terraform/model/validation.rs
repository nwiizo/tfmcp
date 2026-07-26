use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerraformValidateOutput {
    pub valid: bool,
    pub error_count: i32,
    pub warning_count: i32,
    pub diagnostics: Vec<TerraformDiagnostic>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerraformDiagnostic {
    pub severity: String,
    pub summary: String,
    pub detail: Option<String>,
    pub range: Option<DiagnosticRange>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiagnosticRange {
    pub filename: String,
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    pub line: i32,
    pub column: i32,
    pub byte: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedValidationResult {
    pub valid: bool,
    pub error_count: i32,
    pub warning_count: i32,
    pub diagnostics: Vec<TerraformDiagnostic>,
    pub additional_warnings: Vec<String>,
    pub suggestions: Vec<String>,
    pub checked_files: usize,
    pub guideline_checks: Option<GuidelineCheckResult>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GuidelineCheckResult {
    pub compliance_score: u8,
    pub variables_missing_type: Vec<String>,
    pub variables_missing_description: Vec<String>,
    pub outputs_missing_description: Vec<String>,
    pub count_instead_of_foreach: Vec<CountUsageWarning>,
    pub any_type_usage: Vec<String>,
    pub providers_missing_version: Vec<String>,
    pub missing_default_tags: bool,
    pub hardcoded_secrets: Vec<SecretDetection>,
    pub missing_lifecycle_protection: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountUsageWarning {
    pub resource_name: String,
    pub resource_type: String,
    pub suggestion: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretDetection {
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub severity: String,
}
