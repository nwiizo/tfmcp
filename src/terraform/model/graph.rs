use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceDependencyGraph {
    pub nodes: Vec<ResourceNode>,
    pub edges: Vec<ResourceEdge>,
    pub module_boundaries: Vec<ModuleBoundary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: String,
    pub resource_type: String,
    pub resource_name: String,
    pub module_path: String,
    pub file: String,
    pub provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceEdge {
    pub source: String,
    pub target: String,
    pub dependency_type: DependencyType,
    pub attribute: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Explicit,
    Implicit,
    DataSource,
    ModuleOutput,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleBoundary {
    pub module_path: String,
    pub resource_ids: Vec<String>,
}
