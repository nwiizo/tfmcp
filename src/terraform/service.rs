use crate::shared::security::SecurityManager;
use crate::terraform::analyzer;
use crate::terraform::model::{
    DetailedValidationResult, GuidelineCheckResult, ModuleHealthAnalysis, RefactoringSuggestion,
    ResourceDependencyGraph, TerraformAnalysis, TerraformValidateOutput,
};
use crate::terraform::parser::TerraformParser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct TerraformService {
    terraform_path: PathBuf,
    project_directory: PathBuf,
    security_manager: SecurityManager,
}

impl TerraformService {
    pub fn new(terraform_path: PathBuf, project_directory: PathBuf) -> Self {
        eprintln!(
            "[DEBUG] TerraformService initialized with terraform path: {} and project directory: {}",
            terraform_path.display(),
            project_directory.display()
        );
        let security_manager = SecurityManager::new().unwrap_or_else(|e| {
            eprintln!("[WARN] Failed to initialize security manager: {}", e);
            // Create a default security manager with basic settings
            SecurityManager {
                policy: crate::shared::security::SecurityPolicy::default(),
                audit_log: None,
            }
        });
        Self {
            terraform_path,
            project_directory,
            security_manager,
        }
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
        // Security checks
        if !self.security_manager.is_command_allowed("apply") {
            return Err(anyhow::anyhow!(
                "Apply operation blocked by security policy. Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable."
            ));
        }

        if auto_approve && !self.security_manager.is_auto_approve_allowed("apply") {
            return Err(anyhow::anyhow!(
                "Auto-approve for apply operation blocked by security policy. Set TFMCP_ALLOW_AUTO_APPROVE=true to enable."
            ));
        }

        // Validate directory security
        self.security_manager
            .validate_directory(&self.project_directory)?;

        // Check resource limits
        if let Ok(resources) = self.list_resources().await {
            self.security_manager
                .check_resource_limit(resources.len())?;
        }

        let mut cmd = Command::new(&self.terraform_path);
        cmd.arg("apply");

        if auto_approve {
            cmd.arg("-auto-approve");
        }

        let command_args = vec!["terraform".to_string(), "apply".to_string()];
        let output = cmd.current_dir(&self.project_directory).output()?;
        let success = output.status.success();

        // Log audit entry
        let error_msg = if !success {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        } else {
            None
        };

        let resource_count = if success {
            self.list_resources().await.ok().map(|r| r.len())
        } else {
            None
        };

        let audit_entry = self.security_manager.create_audit_entry(
            "apply",
            &self.project_directory.to_string_lossy(),
            &command_args,
            success,
            error_msg.clone(),
            resource_count,
        );

        if let Err(e) = self.security_manager.log_audit_entry(audit_entry) {
            eprintln!("[WARN] Failed to log audit entry: {}", e);
        }

        if success {
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
        let mut guideline_checks = None;

        // Check for .tf files in the directory
        let tf_files = self.find_terraform_files().await?;
        if tf_files.is_empty() {
            warnings.push("No Terraform configuration files found in the directory".to_string());
        }

        // Analyze configuration for best practices
        if !tf_files.is_empty() {
            let analysis = self.analyze_configurations().await?;
            let file_contents = self.read_file_contents().await?;

            // Run Future Architect guideline checks
            let checks = analyzer::check_guidelines(&analysis, &file_contents);

            // Add suggestions based on guideline checks
            for var_name in &checks.variables_missing_type {
                suggestions.push(format!(
                    "[Guideline] Variable '{}' is missing a type definition",
                    var_name
                ));
            }

            for var_name in &checks.variables_missing_description {
                suggestions.push(format!(
                    "[Guideline] Variable '{}' is missing a description",
                    var_name
                ));
            }

            for output_name in &checks.outputs_missing_description {
                suggestions.push(format!(
                    "[Guideline] Output '{}' is missing a description",
                    output_name
                ));
            }

            for count_warning in &checks.count_instead_of_foreach {
                suggestions.push(format!(
                    "[Guideline] {}: {}",
                    count_warning.resource_name, count_warning.suggestion
                ));
            }

            for var_name in &checks.any_type_usage {
                suggestions.push(format!(
                    "[Guideline] Variable '{}' uses 'any' type - consider using a specific type",
                    var_name
                ));
            }

            for provider_name in &checks.providers_missing_version {
                warnings.push(format!(
                    "[Guideline] Provider '{}' is missing a version constraint",
                    provider_name
                ));
            }

            if checks.missing_default_tags {
                warnings.push(
                    "[Guideline] AWS provider is missing default_tags configuration".to_string(),
                );
            }

            for secret in &checks.hardcoded_secrets {
                warnings.push(format!(
                    "[SECURITY] Potential {} detected in {}:{} (severity: {})",
                    secret.pattern, secret.file, secret.line, secret.severity
                ));
            }

            for resource_id in &checks.missing_lifecycle_protection {
                suggestions.push(format!(
                    "[Guideline] Critical resource '{}' is missing lifecycle.prevent_destroy",
                    resource_id
                ));
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

            guideline_checks = Some(checks);
        }

        Ok(DetailedValidationResult {
            valid: validate_output.valid,
            error_count: validate_output.error_count,
            warning_count: validate_output.warning_count + warnings.len() as i32,
            diagnostics: validate_output.diagnostics,
            additional_warnings: warnings,
            suggestions,
            checked_files: tf_files.len(),
            guideline_checks,
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
        // Security checks
        if !self.security_manager.is_command_allowed("destroy") {
            return Err(anyhow::anyhow!(
                "Destroy operation blocked by security policy. Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable."
            ));
        }

        if auto_approve && !self.security_manager.is_auto_approve_allowed("destroy") {
            return Err(anyhow::anyhow!(
                "Auto-approve for destroy operation blocked by security policy. Set TFMCP_ALLOW_AUTO_APPROVE=true to enable."
            ));
        }

        // Validate directory security
        self.security_manager
            .validate_directory(&self.project_directory)?;

        let mut cmd = Command::new(&self.terraform_path);
        cmd.arg("destroy");

        if auto_approve {
            cmd.arg("-auto-approve");
        }

        let command_args = vec!["terraform".to_string(), "destroy".to_string()];
        let output = cmd.current_dir(&self.project_directory).output()?;
        let success = output.status.success();

        // Log audit entry
        let error_msg = if !success {
            Some(String::from_utf8_lossy(&output.stderr).to_string())
        } else {
            None
        };

        let audit_entry = self.security_manager.create_audit_entry(
            "destroy",
            &self.project_directory.to_string_lossy(),
            &command_args,
            success,
            error_msg.clone(),
            None, // Resource count not applicable for destroy
        );

        if let Err(e) = self.security_manager.log_audit_entry(audit_entry) {
            eprintln!("[WARN] Failed to log audit entry: {}", e);
        }

        if success {
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

    /// Get current security policy for debugging/reporting
    #[allow(dead_code)]
    pub fn get_security_policy(&self) -> &crate::shared::security::SecurityPolicy {
        self.security_manager.get_policy()
    }

    /// Check if a specific operation is allowed by security policy
    #[allow(dead_code)]
    pub fn is_operation_allowed(&self, operation: &str) -> bool {
        self.security_manager.is_command_allowed(operation)
    }

    // ==================== Module Health Analysis Methods ====================

    /// Read all Terraform file contents from the project directory
    async fn read_file_contents(&self) -> anyhow::Result<HashMap<String, String>> {
        let mut file_contents = HashMap::new();
        let entries = std::fs::read_dir(&self.project_directory)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "tf") {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy().to_string();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        file_contents.insert(filename_str, content);
                    }
                }
            }
        }

        // Also check for nested modules
        let modules_dir = self.project_directory.join("modules");
        if modules_dir.exists() && modules_dir.is_dir() {
            Self::read_nested_modules(&modules_dir, "modules", &mut file_contents)?;
        }

        Ok(file_contents)
    }

    /// Recursively read nested module contents
    fn read_nested_modules(
        dir: &Path,
        prefix: &str,
        file_contents: &mut HashMap<String, String>,
    ) -> anyhow::Result<()> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let submodule_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let new_prefix = format!("{}/{}", prefix, submodule_name);
                    Self::read_nested_modules(&path, &new_prefix, file_contents)?;
                } else if path.is_file() && path.extension().is_some_and(|ext| ext == "tf") {
                    if let Some(filename) = path.file_name() {
                        let key = format!("{}/{}", prefix, filename.to_string_lossy());
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            file_contents.insert(key, content);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Analyze module health based on whitebox principles
    /// Detects issues related to cohesion, coupling, and module structure
    pub async fn analyze_module_health(&self) -> anyhow::Result<ModuleHealthAnalysis> {
        eprintln!(
            "[DEBUG] Analyzing module health in {}",
            self.project_directory.display()
        );

        let analysis = self.analyze_configurations().await?;
        let file_contents = self.read_file_contents().await?;

        let health = analyzer::analyze_module_health(&analysis, &file_contents);

        eprintln!(
            "[INFO] Module health analysis complete: score={}, issues={}",
            health.health_score,
            health.issues.len()
        );

        Ok(health)
    }

    /// Build resource dependency graph for visualization
    pub async fn get_dependency_graph(&self) -> anyhow::Result<ResourceDependencyGraph> {
        eprintln!(
            "[DEBUG] Building dependency graph for {}",
            self.project_directory.display()
        );

        let analysis = self.analyze_configurations().await?;
        let file_contents = self.read_file_contents().await?;

        let graph = analyzer::build_dependency_graph(&analysis, &file_contents);

        eprintln!(
            "[INFO] Dependency graph built: {} nodes, {} edges",
            graph.nodes.len(),
            graph.edges.len()
        );

        Ok(graph)
    }

    /// Generate refactoring suggestions based on module health analysis
    pub async fn suggest_refactoring(&self) -> anyhow::Result<Vec<RefactoringSuggestion>> {
        eprintln!(
            "[DEBUG] Generating refactoring suggestions for {}",
            self.project_directory.display()
        );

        let analysis = self.analyze_configurations().await?;
        let file_contents = self.read_file_contents().await?;
        let health = analyzer::analyze_module_health(&analysis, &file_contents);

        let suggestions = analyzer::suggest_refactoring(&analysis, &health);

        eprintln!(
            "[INFO] Generated {} refactoring suggestions",
            suggestions.len()
        );

        Ok(suggestions)
    }

    /// Run security scan using guideline checks (secret detection, etc.)
    pub async fn run_security_scan(&self) -> anyhow::Result<GuidelineCheckResult> {
        eprintln!(
            "[DEBUG] Running security scan in {}",
            self.project_directory.display()
        );

        let tf_files = self.find_terraform_files().await?;
        if tf_files.is_empty() {
            return Ok(GuidelineCheckResult::default());
        }

        let analysis = self.analyze_configurations().await?;
        let file_contents = self.read_file_contents().await?;

        let checks = analyzer::check_guidelines(&analysis, &file_contents);

        eprintln!(
            "[INFO] Security scan complete: {} secrets found, compliance score: {}",
            checks.hardcoded_secrets.len(),
            checks.compliance_score
        );

        Ok(checks)
    }
}
