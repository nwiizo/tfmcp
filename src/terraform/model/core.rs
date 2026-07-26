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
#[allow(dead_code)]
pub struct TerraformPlan {
    pub changes: TerraformChanges,
    pub raw_output: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TerraformChanges {
    pub add: usize,
    pub change: usize,
    pub destroy: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TerraformState {
    pub resources: Vec<TerraformStateResource>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TerraformStateResource {
    pub name: String,
    pub type_: String,
    pub provider: String,
    pub instances: Vec<TerraformResourceInstance>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TerraformResourceInstance {
    pub id: String,
    pub attributes: serde_json::Value,
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
