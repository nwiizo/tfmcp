use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleInfo {
    pub id: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub owner: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleDetails {
    pub id: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub root: Option<ModuleRoot>,
    #[serde(default)]
    pub submodules: Vec<ModuleSubmodule>,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleRoot {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub readme: String,
    #[serde(default)]
    pub empty: bool,
    #[serde(default)]
    pub inputs: Vec<ModuleInput>,
    #[serde(default)]
    pub outputs: Vec<ModuleOutput>,
    #[serde(default)]
    pub dependencies: Vec<ModuleDependency>,
    #[serde(default)]
    pub provider_dependencies: Vec<ModuleProviderDependency>,
    #[serde(default)]
    pub resources: Vec<ModuleResource>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleSubmodule {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub readme: String,
    #[serde(default)]
    pub empty: bool,
    #[serde(default)]
    pub inputs: Vec<ModuleInput>,
    #[serde(default)]
    pub outputs: Vec<ModuleOutput>,
    #[serde(default)]
    pub dependencies: Vec<ModuleDependency>,
    #[serde(default)]
    pub resources: Vec<ModuleResource>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "type")]
    pub input_type: String,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub required: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleOutput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleDependency {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub version: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleProviderDependency {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub version: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleResource {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub resource_type: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSearchResponse {
    #[serde(default)]
    pub modules: Vec<ModuleInfo>,
    #[serde(default)]
    pub meta: ModuleSearchMeta,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleSearchMeta {
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub current_offset: u32,
    #[serde(default)]
    pub next_offset: Option<u32>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
