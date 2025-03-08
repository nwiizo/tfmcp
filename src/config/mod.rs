use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    ConfigFileNotFound(String),

    #[error("Failed to parse config file: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub terraform: TerraformConfig,
    pub mcp: McpConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerraformConfig {
    pub executable_path: Option<String>,
    pub project_directory: Option<String>,
    pub auto_init: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpConfig {
    pub tools: Vec<String>,
}

pub fn init_default() -> anyhow::Result<Config> {
    // Check if config exists in the default location
    let config_paths = [
        format!(
            "{}/.config/tfmcp/config.json",
            std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
        ),
        "./tfmcp.json".to_string(),
    ];

    for path in config_paths {
        if Path::new(&path).exists() {
            return init_from_path(&path);
        }
    }

    // Return default config if no config file found
    Ok(Config {
        terraform: TerraformConfig {
            executable_path: None,
            project_directory: None,
            auto_init: Some(true),
        },
        mcp: McpConfig {
            tools: vec![
                "list_terraform_resources".to_string(),
                "analyze_terraform".to_string(),
                "get_terraform_plan".to_string(),
                "apply_terraform".to_string(),
            ],
        },
    })
}

pub fn init_from_path(path: &str) -> anyhow::Result<Config> {
    let path = Path::new(path);

    if !path.exists() {
        return Err(ConfigError::ConfigFileNotFound(path.to_string_lossy().to_string()).into());
    }

    let content = fs::read_to_string(path)?;

    match serde_json::from_str(&content) {
        Ok(config) => Ok(config),
        Err(e) => Err(ConfigError::ParseError(e.to_string()).into()),
    }
}
