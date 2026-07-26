use super::IssueSeverity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    pub suggestion_type: RefactoringType,
    pub priority: IssueSeverity,
    pub description: String,
    pub affected_resources: Vec<String>,
    pub proposed_structure: Option<ProposedModuleStructure>,
    pub migration_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    SplitModule,
    MergeModules,
    ExtractSubmodule,
    FlattenHierarchy,
    WrapPublicModule,
    RemoveUnusedVariables,
    AddDescriptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposedModuleStructure {
    pub module_name: String,
    pub resources: Vec<String>,
    pub variables: Vec<String>,
    pub outputs: Vec<String>,
}
