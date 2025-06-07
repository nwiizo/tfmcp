use thiserror::Error;

/// Enhanced error handling for MCP operations
#[derive(Error, Debug)]
pub enum McpError {
    #[error("JSON parsing error: {0}")]
    JsonParseError(String),
    
    #[error("Transport error: {0}")]
    TransportError(String),
    
    #[error("Invalid tool configuration: {0}")]
    InvalidToolConfig(String),
    
    #[error("Terraform operation failed: {0}")]
    TerraformError(String),
    
    #[error("Registry operation failed: {0}")]
    RegistryError(String),
    
    #[error("Security policy violation: {0}")]
    SecurityError(String),
    
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<serde_json::Error> for McpError {
    fn from(error: serde_json::Error) -> Self {
        McpError::JsonParseError(error.to_string())
    }
}

impl From<anyhow::Error> for McpError {
    fn from(error: anyhow::Error) -> Self {
        McpError::InternalError(error.to_string())
    }
}

/// Safe JSON parsing with proper error handling
pub fn safe_json_parse<T>(json_str: &str) -> Result<T, McpError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(json_str).map_err(|e| {
        McpError::JsonParseError(format!("Failed to parse JSON: {} - Content: {}", e, 
            if json_str.len() > 100 { 
                format!("{}...", &json_str[..100]) 
            } else { 
                json_str.to_string() 
            }
        ))
    })
}

/// Safe JSON value parsing with proper error handling
pub fn safe_json_value_parse<T>(value: serde_json::Value) -> Result<T, McpError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value).map_err(|e| {
        McpError::JsonParseError(format!("Failed to parse JSON value: {}", e))
    })
}

/// Safe regex compilation with proper error handling
pub fn safe_regex_compile(pattern: &str) -> Result<regex::Regex, McpError> {
    regex::Regex::new(pattern).map_err(|e| {
        McpError::InvalidToolConfig(format!("Invalid regex pattern '{}': {}", pattern, e))
    })
}

/// Safe HTTP status text extraction
pub fn safe_status_text(status: &reqwest::StatusCode) -> String {
    status.canonical_reason()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_safe_json_parse() {
        #[derive(serde::Deserialize)]
        struct TestStruct {
            name: String,
        }

        let valid_json = r#"{"name": "test"}"#;
        let result: Result<TestStruct, McpError> = safe_json_parse(valid_json);
        assert!(result.is_ok());

        let invalid_json = r#"{"name": test"}"#;
        let result: Result<TestStruct, McpError> = safe_json_parse(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_json_value_parse() {
        #[derive(serde::Deserialize)]
        struct TestStruct {
            name: String,
        }

        let valid_value = json!({"name": "test"});
        let result: Result<TestStruct, McpError> = safe_json_value_parse(valid_value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_regex_compile() {
        let valid_pattern = r"test.*";
        let result = safe_regex_compile(valid_pattern);
        assert!(result.is_ok());

        let invalid_pattern = r"[";
        let result = safe_regex_compile(invalid_pattern);
        assert!(result.is_err());
    }
}