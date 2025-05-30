use crate::config::{self, Config};
use crate::mcp::handler::McpHandler;
use crate::mcp::stdio::StdioTransport;
use crate::shared::logging;
use crate::terraform::model::{DetailedValidationResult, TerraformAnalysis};
use crate::terraform::service::TerraformService;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum TfMcpError {
    #[error("Terraform binary not found")]
    TerraformNotFound,

    #[error("Invalid Terraform project directory: {0}")]
    #[allow(dead_code)]
    InvalidProjectDirectory(String),

    #[error("Error running Terraform command: {0}")]
    #[allow(dead_code)]
    TerraformCommandError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum JsonRpcErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    // Custom error codes should be in the range -32000 to -32099
    TerraformNotFound = -32000,
    InvalidProjectDirectory = -32001,
}

pub struct TfMcp {
    #[allow(dead_code)]
    config: Config,
    terraform_service: TerraformService,
}

impl TfMcp {
    pub fn new(config_path: Option<String>, project_dir: Option<String>) -> anyhow::Result<Self> {
        // Check environment variable for Terraform directory first
        let env_terraform_dir = std::env::var("TERRAFORM_DIR").ok();
        if let Some(dir) = &env_terraform_dir {
            logging::info(&format!(
                "Found TERRAFORM_DIR environment variable: {}",
                dir
            ));
        }

        // Initialize config
        let config = match config_path {
            Some(path) => {
                let path_buf = PathBuf::from(&path);
                if path_buf.is_absolute() {
                    logging::info(&format!("Using absolute config path: {}", path));
                    config::init_from_path(&path)?
                } else {
                    // Convert to absolute path
                    let abs_path = std::env::current_dir()?.join(&path);
                    logging::info(&format!(
                        "Converting relative config path to absolute: {}",
                        abs_path.display()
                    ));
                    config::init_from_path(abs_path.to_str().unwrap_or(&path))?
                }
            }
            None => {
                logging::info("No config path provided, using default configuration");
                config::init_default()?
            }
        };

        // Priority for project directory:
        // 1. Command line argument
        // 2. Environment variable
        // 3. Config file
        // 4. Current directory
        let project_directory = match project_dir {
            Some(dir) => {
                let dir_buf = PathBuf::from(&dir);
                if dir_buf.is_absolute() {
                    logging::info(&format!(
                        "Using absolute project directory from CLI arg: {}",
                        dir
                    ));
                    dir_buf
                } else {
                    // Convert to absolute path
                    let abs_dir = std::env::current_dir()?.join(dir);
                    logging::info(&format!(
                        "Converting relative project directory from CLI to absolute: {}",
                        abs_dir.display()
                    ));
                    abs_dir
                }
            }
            None => {
                match env_terraform_dir {
                    Some(dir) => {
                        logging::info(&format!(
                            "Using project directory from TERRAFORM_DIR env var: {}",
                            dir
                        ));
                        PathBuf::from(dir)
                    }
                    None => {
                        match &config.terraform.project_directory {
                            Some(dir) => {
                                let dir_buf = PathBuf::from(dir);
                                if dir_buf.is_absolute() {
                                    logging::info(&format!(
                                        "Using project directory from config: {}",
                                        dir
                                    ));
                                    dir_buf
                                } else {
                                    // Convert to absolute path
                                    let abs_dir = std::env::current_dir()?.join(dir);
                                    logging::info(&format!("Converting relative project directory from config to absolute: {}", abs_dir.display()));
                                    abs_dir
                                }
                            }
                            None => {
                                // If we're in root (/) directory and it's not a valid Terraform directory,
                                // let's use HOME directory as fallback
                                let current_dir = std::env::current_dir()?;
                                if current_dir == PathBuf::from("/") {
                                    // We're likely running from Claude Desktop with undefined working dir
                                    let home_dir = dirs::home_dir().unwrap_or(current_dir.clone());
                                    let tf_dir = home_dir.join("terraform");
                                    logging::info(&format!("Working directory is root (/), falling back to home directory: {}", tf_dir.display()));
                                    tf_dir
                                } else {
                                    logging::info(&format!("No project directory specified, using current directory: {}", current_dir.display()));
                                    current_dir
                                }
                            }
                        }
                    }
                }
            }
        };

        // Check if terraform is installed
        let terraform_path = match &config.terraform.executable_path {
            Some(path) => {
                let path_buf = PathBuf::from(path);
                if path_buf.is_absolute() {
                    logging::info(&format!("Using specified Terraform binary: {}", path));
                    path_buf
                } else {
                    // Convert to absolute path
                    let abs_path = std::env::current_dir()?.join(path);
                    logging::info(&format!(
                        "Converting relative Terraform path to absolute: {}",
                        abs_path.display()
                    ));
                    abs_path
                }
            }
            None => {
                // Read TERRAFORM_BINARY_NAME env var, fallback to "terraform"
                let terraform_binary = std::env::var("TERRAFORM_BINARY_NAME")
                    .unwrap_or_else(|_| "terraform".to_string());
                match which::which(&terraform_binary) {
                    Ok(path) => {
                        logging::info(&format!(
                            "Found Terraform binary '{}' in PATH: {}",
                            terraform_binary,
                            path.display()
                        ));
                        path
                    }
                    Err(_) => {
                        logging::error(&format!(
                            "Terraform binary '{}' not found in PATH",
                            terraform_binary
                        ));
                        return Err(TfMcpError::TerraformNotFound.into());
                    }
                }
            }
        };

        // Verify Terraform binary exists
        if !terraform_path.exists() {
            logging::error(&format!(
                "Terraform binary not found at: {}",
                terraform_path.display()
            ));
            return Err(TfMcpError::TerraformNotFound.into());
        }

        // Create a sample Terraform file if the directory doesn't have .tf files
        // This ensures we can always start the MCP server even without a valid Terraform project
        let has_tf_files = std::fs::read_dir(&project_directory)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.path().extension().is_some_and(|ext| ext == "tf"))
            })
            .unwrap_or(false);

        if !has_tf_files {
            // Directory doesn't exist or has no .tf files, create a sample project
            logging::info(&format!(
                "No Terraform (.tf) files found in {}. Creating a sample project.",
                project_directory.display()
            ));

            // Create directory if it doesn't exist
            if !project_directory.exists() {
                logging::info(&format!(
                    "Creating directory: {}",
                    project_directory.display()
                ));
                std::fs::create_dir_all(&project_directory)?;
            }

            // Create a sample main.tf file
            let main_tf_path = project_directory.join("main.tf");
            logging::info(&format!(
                "Creating sample Terraform file at: {}",
                main_tf_path.display()
            ));
            let sample_tf_content = r#"# This is a sample Terraform file created by tfmcp
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
  }
}

resource "local_file" "example" {
  content  = "Hello from tfmcp!"
  filename = "${path.module}/example.txt"
}
"#;
            std::fs::write(&main_tf_path, sample_tf_content)?;
        }

        let terraform_service = match TerraformService::new(terraform_path, project_directory) {
            Ok(service) => service,
            Err(e) => {
                logging::error(&format!("Error creating TerraformService: {}", e));
                // Instead of immediately returning error, create a dummy service
                // This allows the MCP server to start but operations will fail gracefully
                return Err(e.into());
            }
        };

        logging::info("TfMcp initialized successfully");
        Ok(Self {
            config,
            terraform_service,
        })
    }

    pub async fn launch_mcp(&mut self) -> anyhow::Result<()> {
        let (transport, _sender) = StdioTransport::new();

        // Log environment information
        let cwd = std::env::current_dir()?;
        logging::info(&format!("Current working directory: {}", cwd.display()));

        // Check if TERRAFORM_DIR environment variable is set, and if not, set it to a default
        // location inside the user's home directory to avoid root directory issues
        if std::env::var("TERRAFORM_DIR").is_err() {
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let default_tf_dir = home_dir.join("terraform");

            // Create the directory if it doesn't exist
            if !default_tf_dir.exists() {
                logging::info(&format!(
                    "Creating default Terraform directory: {}",
                    default_tf_dir.display()
                ));
                std::fs::create_dir_all(&default_tf_dir)?;
            }

            // Create a sample Terraform file to ensure valid Terraform directory
            let main_tf_path = default_tf_dir.join("main.tf");
            if !main_tf_path.exists() {
                logging::info(&format!(
                    "Creating sample Terraform file at: {}",
                    main_tf_path.display()
                ));
                let sample_tf_content = r#"# This is a sample Terraform file created by tfmcp
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
  }
}

resource "local_file" "example" {
  content  = "Hello from tfmcp!"
  filename = "${path.module}/example.txt"
}
"#;
                std::fs::write(&main_tf_path, sample_tf_content)?;
            }

            // Set the environment variable for future uses in this process
            std::env::set_var(
                "TERRAFORM_DIR",
                default_tf_dir.to_string_lossy().to_string(),
            );
            logging::info(&format!(
                "Set TERRAFORM_DIR to: {}",
                default_tf_dir.display()
            ));
        }

        // Create the handler and launch MCP
        let mut handler = McpHandler::new(self);
        handler.launch_mcp(&transport).await
    }

    pub async fn analyze_terraform(&mut self) -> anyhow::Result<()> {
        let analysis = self.terraform_service.analyze_configurations().await?;
        println!("{}", serde_json::to_string_pretty(&analysis)?);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_terraform_analysis(&self) -> anyhow::Result<TerraformAnalysis> {
        self.terraform_service.analyze_configurations().await
    }

    #[allow(dead_code)]
    pub async fn get_terraform_version(&self) -> anyhow::Result<String> {
        self.terraform_service.get_version().await
    }

    pub async fn get_terraform_plan(&self) -> anyhow::Result<String> {
        self.terraform_service.get_plan().await
    }

    pub async fn apply_terraform(&self, auto_approve: bool) -> anyhow::Result<String> {
        self.terraform_service.apply(auto_approve).await
    }

    pub async fn init_terraform(&self) -> anyhow::Result<String> {
        self.terraform_service.init().await
    }

    pub async fn get_state(&self) -> anyhow::Result<String> {
        self.terraform_service.get_state().await
    }

    pub async fn list_resources(&self) -> anyhow::Result<Vec<String>> {
        self.terraform_service.list_resources().await
    }

    pub async fn validate_configuration(&self) -> anyhow::Result<String> {
        self.terraform_service.validate().await
    }

    pub async fn validate_configuration_detailed(&self) -> anyhow::Result<DetailedValidationResult> {
        self.terraform_service.validate_detailed().await
    }

    pub async fn destroy_terraform(&self, auto_approve: bool) -> anyhow::Result<String> {
        // Check if delete functionality is enabled via environment variable
        let delete_enabled = std::env::var("TFMCP_DELETE_ENABLED")
            .map(|val| val.to_lowercase() == "true")
            .unwrap_or(false);

        if !delete_enabled {
            return Err(anyhow::anyhow!(
                "Delete functionality is disabled. Set TFMCP_DELETE_ENABLED=true to enable it."
            ));
        }

        logging::info("Executing Terraform destroy operation");
        self.terraform_service.destroy(auto_approve).await
    }

    // プロジェクトディレクトリを変更するメソッド
    pub fn change_project_directory(&mut self, new_directory: String) -> anyhow::Result<()> {
        let dir_path = PathBuf::from(new_directory);
        let project_directory = if dir_path.is_absolute() {
            logging::info(&format!(
                "Changing to absolute project directory: {}",
                dir_path.display()
            ));
            dir_path
        } else {
            // 相対パスを絶対パスに変換
            let abs_dir = std::env::current_dir()?.join(dir_path);
            logging::info(&format!(
                "Converting relative project directory to absolute: {}",
                abs_dir.display()
            ));
            abs_dir
        };

        // ディレクトリが存在しない場合は作成
        if !project_directory.exists() {
            logging::info(&format!(
                "Creating directory: {}",
                project_directory.display()
            ));
            std::fs::create_dir_all(&project_directory)?;
        }

        // .tfファイルがあるか確認し、なければサンプルプロジェクトを作成
        let has_tf_files = std::fs::read_dir(&project_directory)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.path().extension().is_some_and(|ext| ext == "tf"))
            })
            .unwrap_or(false);

        if !has_tf_files {
            // .tfファイルがないのでサンプルプロジェクトを作成
            logging::info(&format!(
                "No Terraform (.tf) files found in {}. Creating a sample project.",
                project_directory.display()
            ));

            // サンプルのmain.tfファイルを作成
            let main_tf_path = project_directory.join("main.tf");
            logging::info(&format!(
                "Creating sample Terraform file at: {}",
                main_tf_path.display()
            ));
            let sample_tf_content = r#"# This is a sample Terraform file created by tfmcp
terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
  }
}

resource "local_file" "example" {
  content  = "Hello from tfmcp!"
  filename = "${path.module}/example.txt"
}
"#;
            std::fs::write(&main_tf_path, sample_tf_content)?;
        }

        // TerraformServiceのプロジェクトディレクトリを変更
        match self
            .terraform_service
            .change_project_directory(project_directory.clone())
        {
            Ok(_) => {
                // 環境変数も更新
                std::env::set_var(
                    "TERRAFORM_DIR",
                    project_directory.to_string_lossy().to_string(),
                );
                logging::info(&format!(
                    "Successfully changed project directory to: {}",
                    project_directory.display()
                ));
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to change project directory: {}", e));
                Err(e.into())
            }
        }
    }

    // 現在のプロジェクトディレクトリを取得するメソッド
    pub fn get_project_directory(&self) -> PathBuf {
        self.terraform_service.get_project_directory().clone()
    }
}
