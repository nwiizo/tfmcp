use crate::terraform::model::{
    DetailedValidationResult, TerraformAnalysis, TerraformValidateOutput,
};
use crate::terraform::parser::TerraformParser;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum TerraformError {
    #[error("Terraform command failed: {0}")]
    CommandError(String),

    #[error("Terraform binary not found at path: {0}")]
    BinaryNotFound(String),

    #[error("Invalid JSON output: {0}")]
    JsonParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Terraform init required")]
    InitRequired,
}

pub struct TerraformService {
    terraform_path: PathBuf,
    project_directory: PathBuf,
}

impl TerraformService {
    pub fn new(terraform_path: PathBuf, project_directory: PathBuf) -> Self {
        eprintln!(
            "[DEBUG] TerraformService initialized with terraform path: {} and project directory: {}",
            terraform_path.display(),
            project_directory.display()
        );
        Self {
            terraform_path,
            project_directory,
        }
    }

    #[allow(dead_code)]
    pub fn set_project_directory(&mut self, directory: PathBuf) {
        eprintln!(
            "[DEBUG] Setting project directory to: {}",
            directory.display()
        );
        self.project_directory = directory;
    }

    pub fn change_project_directory(&mut self, directory: PathBuf) -> anyhow::Result<()> {
        eprintln!(
            "[DEBUG] Changing project directory to: {}",
            directory.display()
        );
        self.project_directory = directory;
        Ok(())
    }

    pub fn get_project_directory(&self) -> &PathBuf {
        &self.project_directory
    }

    pub async fn get_version(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("version")
            .arg("-json")
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            if let Some(version) = json.get("terraform_version") {
                return Ok(version.to_string().trim_matches('"').to_string());
            }
        }

        // Fallback to non-JSON output
        let output = Command::new(&self.terraform_path).arg("version").output()?;

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version_line = version_output
            .lines()
            .find(|line| line.starts_with("Terraform") || line.starts_with("OpenTofu"))
            .unwrap_or("Unknown version");

        Ok(version_line.to_string())
    }

    pub async fn init(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("init")
            .current_dir(&self.project_directory)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Terraform init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn get_plan(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("plan")
            .arg("-json")
            .current_dir(&self.project_directory)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("terraform init") {
                Err(anyhow::anyhow!(
                    "Terraform initialization required. Please run 'terraform init' first."
                ))
            } else {
                Err(anyhow::anyhow!("Terraform plan failed: {}", stderr))
            }
        }
    }

    pub async fn apply(&self, auto_approve: bool) -> anyhow::Result<String> {
        let mut cmd = Command::new(&self.terraform_path);
        cmd.arg("apply");

        if auto_approve {
            cmd.arg("-auto-approve");
        }

        let output = cmd.current_dir(&self.project_directory).output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Terraform apply failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub async fn get_state(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("state")
            .arg("list")
            .current_dir(&self.project_directory)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Failed to get Terraform state: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[allow(dead_code)]
    pub async fn refresh(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.terraform_path)
            .arg("refresh")
            .current_dir(&self.project_directory)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Terraform refresh failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[allow(dead_code)]
    pub async fn create_terraform_configuration(&self, content: &str) -> anyhow::Result<String> {
        // Write the content to a main.tf file in the project directory
        let file_path = self.project_directory.join("main.tf");
        std::fs::write(&file_path, content)?;

        Ok(format!(
            "Terraform configuration created at: {}",
            file_path.display()
        ))
    }

    #[allow(dead_code)]
    pub async fn read_terraform_file(&self, filename: &str) -> anyhow::Result<String> {
        let file_path = self.project_directory.join(filename);
        match std::fs::read_to_string(&file_path) {
            Ok(content) => Ok(content),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to read file {}: {}",
                file_path.display(),
                e
            )),
        }
    }

    pub async fn list_resources(&self) -> anyhow::Result<Vec<String>> {
        let output = Command::new(&self.terraform_path)
            .arg("state")
            .arg("list")
            .current_dir(&self.project_directory)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to list resources: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
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
                if var.description.is_none()
                    || var
                        .description
                        .as_ref()
                        .map(|d| d.is_empty())
                        .unwrap_or(false)
                {
                    suggestions.push(format!("Variable '{}' is missing a description", var.name));
                }
            }

            // Check for hardcoded values that should be variables
            for resource in &analysis.resources {
                if resource.provider == "aws" && resource.resource_type.contains("instance") {
                    suggestions.push(format!(
                        "Consider using variables for AWS instance configurations in resource '{}'",
                        resource.name
                    ));
                }
            }

            // Check for missing output descriptions
            for output in &analysis.outputs {
                if output.description.is_none()
                    || output
                        .description
                        .as_ref()
                        .map(|d| d.is_empty())
                        .unwrap_or(false)
                {
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
        let parser = TerraformParser::new(content);

        // Parse resources
        eprintln!("[DEBUG] Parsing resources in {}", file_path.display());
        let resources = parser.parse_resources(&file_name);
        for resource in &resources {
            eprintln!(
                "[DEBUG] Found resource: {} ({})",
                resource.name, resource.resource_type
            );
        }
        analysis.resources.extend(resources);

        // Parse variables
        eprintln!("[DEBUG] Parsing variables in {}", file_path.display());
        let variables = parser.parse_variables();
        for variable in &variables {
            eprintln!("[DEBUG] Found variable: {}", variable.name);
        }
        analysis.variables.extend(variables);

        // Parse outputs
        eprintln!("[DEBUG] Parsing outputs in {}", file_path.display());
        let outputs = parser.parse_outputs();
        for output in &outputs {
            eprintln!("[DEBUG] Found output: {}", output.name);
        }
        analysis.outputs.extend(outputs);

        // Parse providers
        eprintln!("[DEBUG] Parsing providers in {}", file_path.display());
        let providers = parser.parse_providers();
        for provider in providers {
            // Check if provider already exists
            if !analysis.providers.iter().any(|p| p.name == provider.name) {
                eprintln!("[DEBUG] Found provider: {}", provider.name);
                analysis.providers.push(provider);
            }
        }

        eprintln!("[DEBUG] Completed analysis of {}", file_path.display());
        Ok(())
    }
}
