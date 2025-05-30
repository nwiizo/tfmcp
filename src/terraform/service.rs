use crate::terraform::model::{
    DetailedValidationResult, TerraformAnalysis, TerraformOutput,
    TerraformProvider, TerraformResource, TerraformValidateOutput, TerraformVariable,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerraformError {
    #[error("Terraform command failed: {0}")]
    CommandFailed(String),

    #[error("Terraform binary not found at: {0}")]
    ExecutableNotFound(String),

    #[error("Invalid Terraform project directory: {0}")]
    InvalidProjectDirectory(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse Terraform output: {0}")]
    #[allow(dead_code)]
    ParseError(String),
}

pub struct TerraformService {
    terraform_path: PathBuf,
    project_directory: PathBuf,
}

impl TerraformService {
    pub fn new(
        terraform_path: PathBuf,
        project_directory: PathBuf,
    ) -> Result<Self, TerraformError> {
        // Validate terraform path
        if !terraform_path.exists() {
            return Err(TerraformError::ExecutableNotFound(
                terraform_path.to_string_lossy().to_string(),
            ));
        }

        // Validate project directory
        if !project_directory.exists() || !project_directory.is_dir() {
            return Err(TerraformError::InvalidProjectDirectory(
                project_directory.to_string_lossy().to_string(),
            ));
        }

        // Check if the directory contains terraform files
        let has_tf_files = std::fs::read_dir(&project_directory)?
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().is_some_and(|ext| ext == "tf"));

        if !has_tf_files {
            return Err(TerraformError::InvalidProjectDirectory(format!(
                "No Terraform (.tf) files found in {}",
                project_directory.display()
            )));
        }

        Ok(Self {
            terraform_path,
            project_directory,
        })
    }

    pub fn change_project_directory(
        &mut self,
        new_directory: PathBuf,
    ) -> Result<(), TerraformError> {
        // Validate new project directory
        if !new_directory.exists() || !new_directory.is_dir() {
            return Err(TerraformError::InvalidProjectDirectory(
                new_directory.to_string_lossy().to_string(),
            ));
        }

        // Check if the directory contains terraform files
        let has_tf_files = std::fs::read_dir(&new_directory)?
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().is_some_and(|ext| ext == "tf"));

        if !has_tf_files {
            return Err(TerraformError::InvalidProjectDirectory(format!(
                "No Terraform (.tf) files found in {}",
                new_directory.display()
            )));
        }

        // 新しいディレクトリに変更
        self.project_directory = new_directory;

        Ok(())
    }

    pub fn get_project_directory(&self) -> &PathBuf {
        &self.project_directory
    }

    #[allow(dead_code)]
    pub async fn get_version(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("version")
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(TerraformError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn init(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .args(["init", "-no-color"])
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(TerraformError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn get_plan(&self) -> anyhow::Result<String> {
        // Run terraform plan and capture output
        let output = Command::new(&self.terraform_path)
            .args(["plan", "-no-color"])
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(TerraformError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn apply(&self, auto_approve: bool) -> anyhow::Result<String> {
        let mut args = vec!["apply", "-no-color"];
        if auto_approve {
            args.push("-auto-approve");
        }

        let output = Command::new(&self.terraform_path)
            .args(&args)
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(TerraformError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn get_state(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .args(["show", "-no-color"])
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(TerraformError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn list_resources(&self) -> anyhow::Result<Vec<String>> {
        let output = Command::new(&self.terraform_path)
            .args(["state", "list"])
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(TerraformError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }

        let resources = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(resources)
    }

    pub async fn validate(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("validate")
            .arg("-json")
            .current_dir(&self.project_directory)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Terraform validate failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn validate_detailed(&self) -> anyhow::Result<DetailedValidationResult> {
        // Run terraform validate with JSON output
        let validate_json = self.validate().await?;
        let validate_output: TerraformValidateOutput = serde_json::from_str(&validate_json)?;
        
        // Additional validation checks
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();
        
        // Check for .tf files in the directory
        let tf_files = self.find_terraform_files().await?;
        if tf_files.is_empty() {
            warnings.push("No Terraform configuration files found in the directory".to_string());
        }
        
        // Analyze configuration for best practices
        if !tf_files.is_empty() {
            let analysis = self.analyze_configurations().await?;
            
            // Check for missing descriptions
            for var in &analysis.variables {
                if var.description.is_none() || var.description.as_ref().map(|d| d.is_empty()).unwrap_or(false) {
                    suggestions.push(format!("Variable '{}' is missing a description", var.name));
                }
            }
            
            // Check for hardcoded values that should be variables
            for resource in &analysis.resources {
                if resource.provider == "aws" && resource.resource_type.contains("instance") {
                    suggestions.push(format!("Consider using variables for AWS instance configurations in resource '{}'", resource.name));
                }
            }
            
            // Check for missing output descriptions
            for output in &analysis.outputs {
                if output.description.is_none() || output.description.as_ref().map(|d| d.is_empty()).unwrap_or(false) {
                    suggestions.push(format!("Output '{}' is missing a description", output.name));
                }
            }
            
            // Check for provider version constraints
            if analysis.providers.iter().any(|p| p.version.is_none()) {
                warnings.push("Some providers are missing version constraints".to_string());
            }
        }
        
        Ok(DetailedValidationResult {
            valid: validate_output.valid,
            error_count: validate_output.error_count,
            warning_count: validate_output.warning_count + warnings.len() as i32,
            diagnostics: validate_output.diagnostics,
            additional_warnings: warnings,
            suggestions,
            checked_files: tf_files.len(),
        })
    }

    async fn find_terraform_files(&self) -> anyhow::Result<Vec<String>> {
        let mut tf_files = Vec::new();
        let entries = std::fs::read_dir(&self.project_directory)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "tf" || ext == "tf.json" {
                        if let Some(name) = path.file_name() {
                            tf_files.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        
        Ok(tf_files)
    }

    pub async fn destroy(&self, auto_approve: bool) -> anyhow::Result<String> {
        let mut cmd = Command::new(&self.terraform_path);
        cmd.arg("destroy");

        if auto_approve {
            cmd.arg("-auto-approve");
        }

        let output = cmd.current_dir(&self.project_directory).output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Terraform destroy failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn analyze_configurations(&self) -> anyhow::Result<TerraformAnalysis> {
        eprintln!(
            "[DEBUG] Analyzing Terraform configurations in {}",
            self.project_directory.display()
        );
        // Check if the directory exists
        if !self.project_directory.exists() {
            return Err(anyhow::anyhow!(
                "Project directory does not exist: {}",
                self.project_directory.display()
            ));
        }

        // Find all .tf files in the project directory
        let entries = std::fs::read_dir(&self.project_directory)?;
        let mut tf_files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "tf") {
                eprintln!("[DEBUG] Found Terraform file: {}", path.display());
                tf_files.push(path);
            }
        }

        if tf_files.is_empty() {
            eprintln!(
                "[WARN] No Terraform (.tf) files found in {}",
                self.project_directory.display()
            );
            return Err(anyhow::anyhow!(
                "No Terraform (.tf) files found in {}",
                self.project_directory.display()
            ));
        }

        let mut analysis = TerraformAnalysis {
            project_directory: self.project_directory.to_string_lossy().to_string(),
            file_count: tf_files.len(),
            resources: Vec::new(),
            variables: Vec::new(),
            outputs: Vec::new(),
            providers: Vec::new(),
        };

        // Parse each file to identify resources, variables, outputs
        for file_path in tf_files {
            eprintln!("[DEBUG] Analyzing file: {}", file_path.display());
            match self.analyze_file(&file_path, &mut analysis) {
                Ok(_) => eprintln!("[DEBUG] Successfully analyzed {}", file_path.display()),
                Err(e) => eprintln!("[ERROR] Failed to analyze {}: {}", file_path.display(), e),
            }
        }

        eprintln!("[INFO] Terraform analysis complete: found {} resources, {} variables, {} outputs, {} providers",
                 analysis.resources.len(), analysis.variables.len(), analysis.outputs.len(), analysis.providers.len());

        Ok(analysis)
    }

    fn analyze_file(
        &self,
        file_path: &Path,
        analysis: &mut TerraformAnalysis,
    ) -> anyhow::Result<()> {
        eprintln!("[DEBUG] Reading file: {}", file_path.display());
        let content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("[ERROR] Failed to read file {}: {}", file_path.display(), e);
                return Err(anyhow::anyhow!("Failed to read file: {}", e));
            }
        };

        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();

        // Very basic parsing for demonstration purposes
        // In a real implementation, you would want to use a proper HCL parser

        eprintln!("[DEBUG] Parsing resources in {}", file_path.display());
        // Find resources
        let resource_regex = regex::Regex::new(r#"resource\s+"([^"]+)"\s+"([^"]+)"#).unwrap();
        for captures in resource_regex.captures_iter(&content) {
            if captures.len() >= 3 {
                let resource_type = captures[1].to_string();
                let resource_name = captures[2].to_string();

                eprintln!(
                    "[DEBUG] Found resource: {} ({})",
                    resource_name, resource_type
                );
                let provider = resource_type.split('_').next().unwrap_or("unknown").to_string();
                analysis.resources.push(TerraformResource {
                    resource_type,
                    name: resource_name,
                    file: file_name.to_string(),
                    provider,
                });
            }
        }

        eprintln!("[DEBUG] Parsing variables in {}", file_path.display());
        // Find variables
        let variable_regex = regex::Regex::new(r#"variable\s+"([^"]+)"#).unwrap();
        for captures in variable_regex.captures_iter(&content) {
            if captures.len() >= 2 {
                let variable_name = captures[1].to_string();
                eprintln!("[DEBUG] Found variable: {}", variable_name);
                // Extract variable details from the content
                let var_description = self.extract_variable_description(&content, &variable_name);
                let var_type = self.extract_variable_type(&content, &variable_name);
                let var_default = self.extract_variable_default(&content, &variable_name);
                
                analysis.variables.push(TerraformVariable {
                    name: variable_name,
                    description: var_description,
                    type_: var_type,
                    default: var_default,
                });
            }
        }

        eprintln!("[DEBUG] Parsing outputs in {}", file_path.display());
        // Find outputs
        let output_regex = regex::Regex::new(r#"output\s+"([^"]+)"#).unwrap();
        for captures in output_regex.captures_iter(&content) {
            if captures.len() >= 2 {
                let output_name = captures[1].to_string();
                eprintln!("[DEBUG] Found output: {}", output_name);
                // Extract output details from the content
                let output_description = self.extract_output_description(&content, &output_name);
                
                analysis.outputs.push(TerraformOutput {
                    name: output_name,
                    description: output_description,
                    value: None,
                });
            }
        }

        eprintln!("[DEBUG] Parsing providers in {}", file_path.display());
        // Find providers
        let provider_regex = regex::Regex::new(r#"provider\s+"([^"]+)"#).unwrap();
        for captures in provider_regex.captures_iter(&content) {
            if captures.len() >= 2 {
                let provider_name = captures[1].to_string();
                // Check if provider already exists
                if !analysis.providers.iter().any(|p| p.name == provider_name) {
                    eprintln!("[DEBUG] Found provider: {}", provider_name);
                    let provider_version = self.extract_provider_version(&content, &provider_name);
                    analysis.providers.push(TerraformProvider {
                        name: provider_name,
                        version: provider_version,
                    });
                }
            }
        }

        eprintln!("[DEBUG] Completed analysis of {}", file_path.display());
        Ok(())
    }

    fn extract_variable_description(&self, content: &str, var_name: &str) -> Option<String> {
        // Simple extraction - looks for description field within variable block
        let pattern = format!(r#"variable\s+"{}"\s*\{{[^}}]*description\s*=\s*"([^"]+)""#, regex::escape(var_name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    fn extract_variable_type(&self, content: &str, var_name: &str) -> Option<String> {
        // Simple extraction - looks for type field within variable block
        let pattern = format!(r#"variable\s+"{}"\s*\{{[^}}]*type\s*=\s*([^\n]+)"#, regex::escape(var_name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(content) {
                return captures.get(1).map(|m| m.as_str().trim().to_string());
            }
        }
        None
    }

    fn extract_variable_default(&self, content: &str, var_name: &str) -> Option<serde_json::Value> {
        // Simple extraction - looks for default field within variable block
        let pattern = format!(r#"variable\s+"{}"\s*\{{[^}}]*default\s*=\s*([^\n]+)"#, regex::escape(var_name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(content) {
                if let Some(default_str) = captures.get(1).map(|m| m.as_str().trim()) {
                    // Try to parse as JSON value
                    if let Ok(value) = serde_json::from_str(default_str) {
                        return Some(value);
                    }
                    // If not valid JSON, return as string
                    return Some(serde_json::Value::String(default_str.to_string()));
                }
            }
        }
        None
    }

    fn extract_output_description(&self, content: &str, output_name: &str) -> Option<String> {
        // Simple extraction - looks for description field within output block
        let pattern = format!(r#"output\s+"{}"\s*\{{[^}}]*description\s*=\s*"([^"]+)""#, regex::escape(output_name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    fn extract_provider_version(&self, content: &str, provider_name: &str) -> Option<String> {
        // Look for version constraint in provider block
        let pattern = format!(r#"provider\s+"{}"\s*\{{[^}}]*version\s*=\s*"([^"]+)""#, regex::escape(provider_name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        
        // Also check required_providers block
        let pattern = format!(r#"required_providers\s*\{{[^}}]*{}\s*=\s*\{{[^}}]*version\s*=\s*"([^"]+)""#, regex::escape(provider_name));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        
        None
    }
}
