use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleHealthAnalysis {
    pub module_path: String,
    pub metrics: ModuleMetrics,
    pub health_score: u8,
    pub issues: Vec<ModuleIssue>,
    pub recommendations: Vec<String>,
    pub cohesion_analysis: CohesionAnalysis,
    pub coupling_analysis: CouplingAnalysis,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleMetrics {
    pub variable_count: usize,
    pub output_count: usize,
    pub resource_count: usize,
    pub resource_type_count: usize,
    pub provider_count: usize,
    pub data_source_count: usize,
    pub local_count: usize,
    pub module_call_count: usize,
    pub file_count: usize,
    pub lines_of_code: usize,
    pub hierarchy_depth: usize,
    pub variables_with_defaults: usize,
    pub variables_without_description: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueCategory {
    LogicalCohesion,
    ExcessiveVariables,
    DeepHierarchy,
    MissingDocumentation,
    ControlCoupling,
    ModelCoupling,
    NamingConvention,
    PublicModuleRisk,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CohesionAnalysis {
    pub cohesion_type: CohesionType,
    pub score: u8,
    pub resource_type_groups: Vec<ResourceTypeGroup>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CohesionType {
    Functional,
    Sequential,
    Communicational,
    Procedural,
    Temporal,
    Logical,
    Coincidental,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceTypeGroup {
    pub name: String,
    pub resource_types: Vec<String>,
    pub resource_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CouplingAnalysis {
    pub coupling_type: CouplingType,
    pub score: u8,
    pub dependencies: Vec<ModuleDependency>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CouplingType {
    Data,
    Stamp,
    Control,
    Common,
    Content,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub source_module: String,
    pub target_module: String,
    pub dependency_type: String,
    pub variables_passed: Vec<String>,
}
