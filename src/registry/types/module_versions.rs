use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleVersionsResponse {
    #[serde(default)]
    pub modules: Vec<ModuleVersionInfo>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleVersionInfo {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub versions: Vec<ModuleVersionDetail>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleVersionDetail {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub root: Option<ModuleVersionRoot>,
    #[serde(default)]
    pub submodules: Vec<ModuleVersionSubmodule>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleVersionRoot {
    #[serde(default)]
    pub providers: Vec<ModuleVersionProvider>,
    #[serde(default)]
    pub dependencies: Vec<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleVersionSubmodule {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub providers: Vec<ModuleVersionProvider>,
    #[serde(default)]
    pub dependencies: Vec<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleVersionProvider {
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
