use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformAnalysis {
    pub project_directory: String,
    pub file_count: usize,
    pub resources: Vec<TerraformResource>,
    pub variables: Vec<TerraformVariable>,
    pub outputs: Vec<TerraformOutput>,
    pub providers: Vec<TerraformProvider>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformResource {
    pub resource_type: String,
    pub name: String,
    pub file: String,
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformPlan {
    pub changes: TerraformChanges,
    pub raw_output: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformChanges {
    pub add: usize,
    pub change: usize,
    pub destroy: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformState {
    pub resources: Vec<TerraformStateResource>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformStateResource {
    pub name: String,
    pub type_: String,
    pub provider: String,
    pub instances: Vec<TerraformResourceInstance>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformResourceInstance {
    pub id: String,
    pub attributes: serde_json::Value,
}

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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformVariable {
    pub name: String,
    pub description: Option<String>,
    pub type_: Option<String>,
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformOutput {
    pub name: String,
    pub description: Option<String>,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformProvider {
    pub name: String,
    pub version: Option<String>,
}

// ==================== Module Health Analysis Models ====================
// Based on the whitebox approach to Infrastructure as Code
// Reference: "インフラコードはホワイトボックス的利用が必要"

/// Module health analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleHealthAnalysis {
    pub module_path: String,
    pub metrics: ModuleMetrics,
    pub health_score: u8, // 0-100
    pub issues: Vec<ModuleIssue>,
    pub recommendations: Vec<String>,
    pub cohesion_analysis: CohesionAnalysis,
    pub coupling_analysis: CouplingAnalysis,
}

/// Quantitative metrics for module analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleMetrics {
    pub variable_count: usize,
    pub output_count: usize,
    pub resource_count: usize,
    pub resource_type_count: usize, // Number of distinct resource types
    pub provider_count: usize,
    pub data_source_count: usize,
    pub local_count: usize,
    pub module_call_count: usize, // Number of module blocks
    pub file_count: usize,
    pub lines_of_code: usize,
    pub hierarchy_depth: usize, // Depth of nested modules
    pub variables_with_defaults: usize,
    pub variables_without_description: usize,
}

/// Issue severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

/// Module issue detected during analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleIssue {
    pub severity: IssueSeverity,
    pub category: IssueCategory,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

/// Categories of module issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueCategory {
    LogicalCohesion,      // Too many unrelated resource types
    ExcessiveVariables,   // Too many variables exposed
    DeepHierarchy,        // Too many nested module levels
    MissingDocumentation, // Variables/outputs without descriptions
    ControlCoupling,      // Excessive conditional logic
    ModelCoupling,        // Internal model exposed through variables
    NamingConvention,     // Poor file/resource naming
    PublicModuleRisk,     // Using public registry modules without wrappers
}

/// Cohesion type analysis (based on software engineering principles)
#[derive(Debug, Serialize, Deserialize)]
pub struct CohesionAnalysis {
    pub cohesion_type: CohesionType,
    pub score: u8, // 0-100, higher is better
    pub resource_type_groups: Vec<ResourceTypeGroup>,
    pub explanation: String,
}

/// Types of module cohesion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CohesionType {
    Functional,      // Best: Single, well-defined purpose
    Sequential,      // Good: Output of one feeds into another
    Communicational, // OK: Operates on same data
    Procedural,      // Weak: Steps that must happen in order
    Temporal,        // Weak: Things that happen at the same time
    Logical,         // Poor: Only related by category (e.g., "network things")
    Coincidental,    // Worst: No meaningful relationship
}

/// Group of related resource types
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceTypeGroup {
    pub name: String,
    pub resource_types: Vec<String>,
    pub resource_count: usize,
}

/// Coupling analysis between modules
#[derive(Debug, Serialize, Deserialize)]
pub struct CouplingAnalysis {
    pub coupling_type: CouplingType,
    pub score: u8, // 0-100, lower coupling is better
    pub dependencies: Vec<ModuleDependency>,
    pub explanation: String,
}

/// Types of module coupling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CouplingType {
    Data,    // Best: Only data passed between modules
    Stamp,   // OK: Structured data passed
    Control, // Poor: Flags/conditionals control behavior
    Common,  // Poor: Shared global data
    Content, // Worst: One module modifies another's internal data
}

/// Dependency information between modules
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleDependency {
    pub source_module: String,
    pub target_module: String,
    pub dependency_type: String,
    pub variables_passed: Vec<String>,
}

/// Resource dependency graph for visualization
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceDependencyGraph {
    pub nodes: Vec<ResourceNode>,
    pub edges: Vec<ResourceEdge>,
    pub module_boundaries: Vec<ModuleBoundary>,
}

/// A node in the resource dependency graph
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: String,
    pub resource_type: String,
    pub resource_name: String,
    pub module_path: String,
    pub file: String,
    pub provider: String,
}

/// An edge in the resource dependency graph
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceEdge {
    pub source: String,
    pub target: String,
    pub dependency_type: DependencyType,
    pub attribute: Option<String>,
}

/// Types of resource dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Explicit,     // depends_on
    Implicit,     // Reference to another resource
    DataSource,   // Data source reference
    ModuleOutput, // Reference to module output
}

/// Module boundary for visualization
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleBoundary {
    pub module_path: String,
    pub resource_ids: Vec<String>,
}

/// Refactoring suggestion
#[derive(Debug, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    pub suggestion_type: RefactoringType,
    pub priority: IssueSeverity,
    pub description: String,
    pub affected_resources: Vec<String>,
    pub proposed_structure: Option<ProposedModuleStructure>,
    pub migration_steps: Vec<String>,
}

/// Types of refactoring suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    SplitModule,           // Extract resources to new module
    MergeModules,          // Combine small modules
    ExtractSubmodule,      // Create submodule for related resources
    FlattenHierarchy,      // Reduce nesting depth
    WrapPublicModule,      // Create wrapper for public module
    RemoveUnusedVariables, // Clean up unused inputs
    AddDescriptions,       // Document variables/outputs
}

/// Proposed new module structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ProposedModuleStructure {
    pub module_name: String,
    pub resources: Vec<String>,
    pub variables: Vec<String>,
    pub outputs: Vec<String>,
}
