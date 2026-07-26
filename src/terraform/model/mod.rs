pub mod core;
pub mod graph;
pub mod health;
pub mod refactoring;
pub mod validation;

pub use core::{
    TerraformAnalysis, TerraformChanges, TerraformOutput, TerraformPlan, TerraformProvider,
    TerraformResource, TerraformResourceInstance, TerraformState, TerraformStateResource,
    TerraformVariable,
};
pub use graph::{
    DependencyType, ModuleBoundary, ResourceDependencyGraph, ResourceEdge, ResourceNode,
};
pub use health::{
    CohesionAnalysis, CohesionType, CouplingAnalysis, CouplingType, IssueCategory, IssueSeverity,
    ModuleDependency, ModuleHealthAnalysis, ModuleIssue, ModuleMetrics, ResourceTypeGroup,
};
pub use refactoring::{ProposedModuleStructure, RefactoringSuggestion, RefactoringType};
pub use validation::{
    CountUsageWarning, DetailedValidationResult, DiagnosticRange, GuidelineCheckResult, Position,
    SecretDetection, TerraformDiagnostic, TerraformValidateOutput,
};
