use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformAnalysis {
    pub project_directory: String,
    pub file_count: usize,
    pub resources: Vec<TerraformResource>,
    pub variables: Vec<String>,
    pub outputs: Vec<String>,
    pub providers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraformResource {
    pub resource_type: String,
    pub name: String,
    pub file: String,
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
