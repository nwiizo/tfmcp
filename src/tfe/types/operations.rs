#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreate {
    pub organization: String,
    pub name: String,
    pub description: Option<String>,
    pub terraform_version: Option<String>,
    pub execution_mode: Option<String>,
    pub auto_apply: Option<bool>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceUpdate {
    pub workspace_id: Option<String>,
    pub organization: Option<String>,
    pub workspace_name: Option<String>,
    pub new_name: Option<String>,
    pub description: Option<String>,
    pub terraform_version: Option<String>,
    pub execution_mode: Option<String>,
    pub auto_apply: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRef {
    pub workspace_id: Option<String>,
    pub organization: Option<String>,
    pub workspace_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCreate {
    pub workspace_id: String,
    pub message: Option<String>,
    pub operation: Option<String>,
    pub is_destroy: Option<bool>,
    pub refresh: Option<bool>,
    pub plan_only: Option<bool>,
    pub auto_apply: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableCreate {
    pub workspace_id: String,
    pub variable: VariableAttributes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceVariableUpdate {
    pub workspace_id: String,
    pub variable_id: String,
    pub variable: VariableAttributes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableAttributes {
    pub key: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub hcl: Option<bool>,
    pub sensitive: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySetWorkspaceAttach {
    pub policy_set_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSetCreate {
    pub organization: String,
    pub name: String,
    pub description: Option<String>,
    pub global: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSetVariableCreate {
    pub variable_set_id: String,
    pub variable: VariableAttributes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSetVariableDelete {
    pub variable_set_id: String,
    pub variable_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSetWorkspaces {
    pub variable_set_id: String,
    pub workspace_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTags {
    pub workspace_id: String,
    pub tags: Vec<String>,
}
