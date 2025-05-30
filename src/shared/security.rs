use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Whether dangerous operations (apply, destroy) are allowed
    pub allow_dangerous_operations: bool,
    /// Whether auto-approve is allowed for apply/destroy operations
    pub allow_auto_approve: bool,
    /// List of allowed Terraform commands
    pub allowed_commands: Vec<String>,
    /// List of blocked file patterns (e.g., production configs)
    pub blocked_file_patterns: Vec<String>,
    /// Maximum number of resources that can be managed
    pub max_resource_limit: Option<usize>,
    /// Required approval patterns for certain operations
    pub approval_patterns: HashMap<String, String>,
    /// Audit logging configuration
    pub audit_logging: AuditConfig,
}
/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    pub enabled: bool,
    /// Path to audit log file
    pub log_file: Option<PathBuf>,
    /// Whether to log sensitive information (state files, etc.)
    pub log_sensitive: bool,
}
/// Audit log entry
#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub operation: String,
    pub directory: String,
    pub command: Vec<String>,
    pub success: bool,
    pub error: Option<String>,
    pub resource_count: Option<usize>,
}
/// Security manager for tfmcp operations
pub struct SecurityManager {
    pub policy: SecurityPolicy,
    pub audit_log: Option<PathBuf>,
}
impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_dangerous_operations: false,
            allow_auto_approve: false,
            allowed_commands: vec![
                "version".to_string(),
                "init".to_string(),
                "validate".to_string(),
                "plan".to_string(),
                "show".to_string(),
                "state".to_string(),
            ],
            blocked_file_patterns: vec![
                "**/prod*/**".to_string(),
                "**/production*/**".to_string(),
                "**/*prod*.tf".to_string(),
                "**/*production*.tf".to_string(),
                "**/*secret*".to_string(),
            ],
            max_resource_limit: Some(50),
            approval_patterns: HashMap::new(),
            audit_logging: AuditConfig {
                enabled: true,
                log_file: None,
                log_sensitive: false,
            },
        }
    }
}
impl SecurityManager {
    pub fn new() -> Result<Self> {
        let policy = Self::load_security_policy()?;
        let audit_log = if policy.audit_logging.enabled {
            policy
                .audit_logging
                .log_file
                .clone()
                .or_else(|| dirs::home_dir().map(|d| d.join(".tfmcp").join("audit.log")))
        } else {
            None
        };
        Ok(Self { policy, audit_log })
    }
    /// Load security policy from environment variables and config files
    fn load_security_policy() -> Result<SecurityPolicy> {
        let mut policy = SecurityPolicy::default();
        // Check environment variables for security settings
        if let Ok(val) = env::var("TFMCP_ALLOW_DANGEROUS_OPS") {
            policy.allow_dangerous_operations = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("TFMCP_ALLOW_AUTO_APPROVE") {
            policy.allow_auto_approve = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("TFMCP_MAX_RESOURCES") {
            if let Ok(limit) = val.parse::<usize>() {
                policy.max_resource_limit = Some(limit);
            }
        }
        if let Ok(val) = env::var("TFMCP_AUDIT_ENABLED") {
            policy.audit_logging.enabled = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("TFMCP_AUDIT_LOG_SENSITIVE") {
            policy.audit_logging.log_sensitive = val.to_lowercase() == "true";
        }
        if let Ok(path) = env::var("TFMCP_AUDIT_LOG_FILE") {
            policy.audit_logging.log_file = Some(PathBuf::from(path));
        }
        // Load additional security policy from config file if exists
        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".tfmcp").join("security.json");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(file_policy) = serde_json::from_str::<SecurityPolicy>(&content) {
                        // Merge with environment-based policy
                        policy = file_policy;
                        // Re-apply environment overrides
                        if let Ok(val) = env::var("TFMCP_ALLOW_DANGEROUS_OPS") {
                            policy.allow_dangerous_operations = val.to_lowercase() == "true";
                        }
                    }
                }
            }
        }
        Ok(policy)
    }
    /// Check if a Terraform command is allowed
    pub fn is_command_allowed(&self, command: &str) -> bool {
        // Special handling for dangerous operations
        match command {
            "apply" | "destroy" => self.policy.allow_dangerous_operations,
            _ => self.policy.allowed_commands.contains(&command.to_string()),
        }
    }
    /// Check if auto-approve is allowed for the given command
    pub fn is_auto_approve_allowed(&self, command: &str) -> bool {
        match command {
            "apply" | "destroy" => {
                self.policy.allow_dangerous_operations && self.policy.allow_auto_approve
            }
            _ => true, // Auto-approve is always allowed for safe commands
        }
    }
    /// Check if a file path is blocked by security policy
    pub fn is_file_blocked(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy().to_lowercase();
        for pattern in &self.policy.blocked_file_patterns {
            let pattern_lower = pattern.to_lowercase();
            // Handle different glob patterns
            if pattern_lower.contains("**") {
                // Pattern like "**/prod*/**" or "**/*prod*.tf"
                let pattern_parts: Vec<&str> = pattern_lower.split("**").collect();
                if pattern_parts.len() == 3 {
                    // Pattern: **/xxx/**
                    let middle = pattern_parts[1];
                    // For patterns like "**/prod*/**", the middle part is "/prod*/"
                    // We need to handle this properly
                    if middle.starts_with('/') && middle.ends_with('/') {
                        let inner = &middle[1..middle.len() - 1]; // Remove leading and trailing '/'
                        if inner.contains('*') {
                            // Handle wildcards in the middle part
                            let inner_parts: Vec<&str> = inner.split('*').collect();
                            if inner_parts.len() == 2 {
                                let prefix = inner_parts[0];
                                let suffix = inner_parts[1];
                                // Look for /prefix*suffix/ pattern in path
                                for segment in path_str.split('/') {
                                    if segment.starts_with(prefix) && segment.ends_with(suffix) {
                                        return true;
                                    }
                                }
                            }
                        } else {
                            // Exact match for middle directory
                            if path_str.contains(&format!("/{}/", inner)) {
                                return true;
                            }
                        }
                    } else if path_str.contains(middle) {
                        return true;
                    }
                } else if pattern_parts.len() == 2 {
                    // Pattern: **/xxx or xxx/**
                    let prefix = pattern_parts[0];
                    let suffix = pattern_parts[1];
                    if prefix.is_empty() && path_str.ends_with(suffix) {
                        // Pattern: **/xxx
                        return true;
                    } else if suffix.is_empty() && path_str.contains(prefix) {
                        // Pattern: xxx/**
                        return true;
                    } else if path_str.contains(prefix) && path_str.ends_with(suffix) {
                        // Pattern: prefix**suffix
                        return true;
                    }
                }
            } else if pattern_lower.contains('*') {
                // Simple wildcard matching
                let parts: Vec<&str> = pattern_lower.split('*').collect();
                let mut pos = 0;
                let mut matched = true;
                for (i, part) in parts.iter().enumerate() {
                    if i == 0 && !part.is_empty() {
                        // First part must match from the beginning
                        if !path_str.starts_with(part) {
                            matched = false;
                            break;
                        }
                        pos = part.len();
                    } else if i == parts.len() - 1 && !part.is_empty() {
                        // Last part must match at the end
                        if !path_str.ends_with(part) {
                            matched = false;
                            break;
                        }
                    } else if !part.is_empty() {
                        // Middle parts must be found in order
                        if let Some(found_pos) = path_str[pos..].find(part) {
                            pos += found_pos + part.len();
                        } else {
                            matched = false;
                            break;
                        }
                    }
                }
                if matched {
                    return true;
                }
            } else if path_str.contains(&pattern_lower) {
                // Exact substring matching
                return true;
            }
        }
        false
    }
    /// Check if the number of resources exceeds the limit
    pub fn check_resource_limit(&self, resource_count: usize) -> Result<()> {
        if let Some(limit) = self.policy.max_resource_limit {
            if resource_count > limit {
                return Err(anyhow::anyhow!(
                    "Operation blocked: Resource count ({}) exceeds security limit ({})",
                    resource_count,
                    limit
                ));
            }
        }
        Ok(())
    }
    /// Log an audit entry
    pub fn log_audit_entry(&self, entry: AuditLogEntry) -> Result<()> {
        if !self.policy.audit_logging.enabled {
            return Ok(());
        }
        if let Some(log_file) = &self.audit_log {
            // Ensure the directory exists
            if let Some(parent) = log_file.parent() {
                fs::create_dir_all(parent)?;
            }
            let log_line = serde_json::to_string(&entry)?;
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)?;
            file.write_all(format!("{}\n", log_line).as_bytes())?;
        }
        Ok(())
    }
    /// Create an audit log entry for a Terraform operation
    pub fn create_audit_entry(
        &self,
        operation: &str,
        directory: &str,
        command: &[String],
        success: bool,
        error: Option<String>,
        resource_count: Option<usize>,
    ) -> AuditLogEntry {
        AuditLogEntry {
            timestamp: Utc::now(),
            user: env::var("USER")
                .or_else(|_| env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string()),
            operation: operation.to_string(),
            directory: directory.to_string(),
            command: command.to_vec(),
            success,
            error,
            resource_count,
        }
    }
    /// Get current security policy (for reporting/debugging)
    #[allow(dead_code)]
    pub fn get_policy(&self) -> &SecurityPolicy {
        &self.policy
    }
    /// Validate a directory for security compliance
    pub fn validate_directory(&self, directory: &Path) -> Result<()> {
        if self.is_file_blocked(directory) {
            return Err(anyhow::anyhow!(
                "Directory access blocked by security policy: {}",
                directory.display()
            ));
        }
        // Check for sensitive files in the directory
        if directory.exists() && directory.is_dir() {
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                let path = entry.path();
                if self.is_file_blocked(&path) {
                    return Err(anyhow::anyhow!(
                        "Directory contains blocked files: {}",
                        path.display()
                    ));
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_security_policy() {
        let policy = SecurityPolicy::default();
        assert!(!policy.allow_dangerous_operations);
        assert!(!policy.allow_auto_approve);
        assert!(policy.allowed_commands.contains(&"init".to_string()));
        assert!(!policy.allowed_commands.contains(&"apply".to_string()));
    }
    #[test]
    fn test_command_security() {
        let manager = SecurityManager {
            policy: SecurityPolicy::default(),
            audit_log: None,
        };
        assert!(manager.is_command_allowed("init"));
        assert!(manager.is_command_allowed("plan"));
        assert!(!manager.is_command_allowed("apply"));
        assert!(!manager.is_command_allowed("destroy"));
    }
    #[test]
    fn test_file_blocking() {
        let manager = SecurityManager {
            policy: SecurityPolicy::default(),
            audit_log: None,
        };
        let prod_file = PathBuf::from("/some/path/prod/main.tf");
        let production_file = PathBuf::from("/some/path/production.tf");
        let safe_file = PathBuf::from("/some/path/dev/main.tf");
        assert!(manager.is_file_blocked(&prod_file));
        assert!(manager.is_file_blocked(&production_file));
        assert!(!manager.is_file_blocked(&safe_file));
    }
    #[test]
    fn test_resource_limit() {
        let manager = SecurityManager {
            policy: SecurityPolicy {
                max_resource_limit: Some(10),
                ..SecurityPolicy::default()
            },
            audit_log: None,
        };
        assert!(manager.check_resource_limit(5).is_ok());
        assert!(manager.check_resource_limit(15).is_err());
    }
    #[test]
    fn test_audit_entry_creation() {
        let manager = SecurityManager {
            policy: SecurityPolicy::default(),
            audit_log: None,
        };
        let entry = manager.create_audit_entry(
            "plan",
            "/test/dir",
            &["terraform".to_string(), "plan".to_string()],
            true,
            None,
            Some(5),
        );
        assert_eq!(entry.operation, "plan");
        assert_eq!(entry.directory, "/test/dir");
        assert!(entry.success);
        assert_eq!(entry.resource_count, Some(5));
    }
}
