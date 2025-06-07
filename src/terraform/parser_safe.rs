use crate::terraform::model::{
    TerraformOutput, TerraformProvider, TerraformResource, TerraformVariable,
};
use crate::mcp::error_handling::{safe_regex_compile, McpError};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

/// Safe regex patterns with proper error handling
fn get_resource_regex() -> Result<&'static Regex, McpError> {
    static RESOURCE_REGEX: Lazy<Result<Regex, McpError>> = Lazy::new(|| {
        safe_regex_compile(r#"resource\s+"([^"]+)"\s+"([^"]+)""#)
    });
    RESOURCE_REGEX.as_ref().map_err(|e| McpError::InvalidToolConfig(e.to_string()))
}

fn get_variable_regex() -> Result<&'static Regex, McpError> {
    static VARIABLE_REGEX: Lazy<Result<Regex, McpError>> = Lazy::new(|| {
        safe_regex_compile(r#"variable\s+"([^"]+)""#)
    });
    VARIABLE_REGEX.as_ref().map_err(|e| McpError::InvalidToolConfig(e.to_string()))
}

fn get_output_regex() -> Result<&'static Regex, McpError> {
    static OUTPUT_REGEX: Lazy<Result<Regex, McpError>> = Lazy::new(|| {
        safe_regex_compile(r#"output\s+"([^"]+)""#)
    });
    OUTPUT_REGEX.as_ref().map_err(|e| McpError::InvalidToolConfig(e.to_string()))
}

fn get_provider_regex() -> Result<&'static Regex, McpError> {
    static PROVIDER_REGEX: Lazy<Result<Regex, McpError>> = Lazy::new(|| {
        safe_regex_compile(r#"provider\s+"([^"]+)""#)
    });
    PROVIDER_REGEX.as_ref().map_err(|e| McpError::InvalidToolConfig(e.to_string()))
}

/// Safe Terraform parser with proper error handling
pub struct TerraformParserSafe {
    content: String,
}

impl TerraformParserSafe {
    pub fn new(content: String) -> Self {
        Self { content }
    }

    /// Parse all resources from the content with safe error handling
    pub fn parse_resources(&self, file_name: &str) -> Result<Vec<TerraformResource>, McpError> {
        let regex = get_resource_regex()?;
        
        let resources = regex
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if captures.len() >= 3 {
                    let resource_type = captures[1].to_string();
                    let resource_name = captures[2].to_string();
                    let provider = resource_type
                        .split('_')
                        .next()
                        .unwrap_or("unknown")  // This unwrap_or is safe
                        .to_string();

                    Some(TerraformResource {
                        resource_type,
                        name: resource_name,
                        file: file_name.to_string(),
                        provider,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(resources)
    }

    /// Parse all variables from the content with safe error handling
    pub fn parse_variables(&self) -> Result<Vec<TerraformVariable>, McpError> {
        let regex = get_variable_regex()?;
        
        let variables = regex
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let description = self.extract_description_after(&name, "variable");
                    let var_type = self.extract_type_after(&name, "variable");
                    let default_value = self.extract_default_after(&name, "variable");

                    Some(TerraformVariable {
                        name,
                        description,
                        type_: var_type,
                        default: default_value,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(variables)
    }

    /// Parse all outputs from the content with safe error handling
    pub fn parse_outputs(&self) -> Result<Vec<TerraformOutput>, McpError> {
        let regex = get_output_regex()?;
        
        let outputs = regex
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let description = self.extract_description_after(&name, "output");
                    let value_str = self.extract_value_after(&name, "output");
                    let value = value_str.and_then(|s| serde_json::from_str(&s).ok());

                    Some(TerraformOutput {
                        name,
                        description,
                        value,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(outputs)
    }

    /// Parse all providers from the content with safe error handling
    pub fn parse_providers(&self) -> Result<Vec<TerraformProvider>, McpError> {
        let regex = get_provider_regex()?;
        
        let providers = regex
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let version = self.extract_version_after(&name, "provider");

                    Some(TerraformProvider {
                        name,
                        version,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(providers)
    }

    /// Extract description field from block (helper method)
    fn extract_description_after(&self, name: &str, block_type: &str) -> Option<String> {
        // Look for description field within the block
        let pattern = format!(r#"{}\s+"{}"\s*\{{[^}}]*description\s*=\s*"([^"]+)""#, block_type, regex::escape(name));
        if let Ok(desc_regex) = safe_regex_compile(&pattern) {
            if let Some(captures) = desc_regex.captures(&self.content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    /// Extract type field from variable block
    fn extract_type_after(&self, name: &str, block_type: &str) -> Option<String> {
        let pattern = format!(r#"{}\s+"{}"\s*\{{[^}}]*type\s*=\s*([^,\n}}]+)"#, block_type, regex::escape(name));
        if let Ok(type_regex) = safe_regex_compile(&pattern) {
            if let Some(captures) = type_regex.captures(&self.content) {
                return captures.get(1).map(|m| m.as_str().trim().to_string());
            }
        }
        None
    }

    /// Extract default value from variable block
    fn extract_default_after(&self, name: &str, block_type: &str) -> Option<Value> {
        let pattern = format!(r#"{}\s+"{}"\s*\{{[^}}]*default\s*=\s*([^,\n}}]+)"#, block_type, regex::escape(name));
        if let Ok(default_regex) = safe_regex_compile(&pattern) {
            if let Some(captures) = default_regex.captures(&self.content) {
                let default_str = captures.get(1)?.as_str().trim();
                // Try to parse as JSON value, fallback to string
                return serde_json::from_str(default_str)
                    .or_else(|_| serde_json::to_value(default_str))
                    .ok();
            }
        }
        None
    }

    /// Extract value field from output block
    fn extract_value_after(&self, name: &str, block_type: &str) -> Option<String> {
        let pattern = format!(r#"{}\s+"{}"\s*\{{[^}}]*value\s*=\s*([^,\n}}]+)"#, block_type, regex::escape(name));
        if let Ok(value_regex) = safe_regex_compile(&pattern) {
            if let Some(captures) = value_regex.captures(&self.content) {
                return captures.get(1).map(|m| m.as_str().trim().to_string());
            }
        }
        None
    }

    /// Extract version from provider block
    fn extract_version_after(&self, name: &str, block_type: &str) -> Option<String> {
        let pattern = format!(r#"{}\s+"{}"\s*\{{[^}}]*version\s*=\s*"([^"]+)""#, block_type, regex::escape(name));
        if let Ok(version_regex) = safe_regex_compile(&pattern) {
            if let Some(captures) = version_regex.captures(&self.content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    /// Extract source from provider block
    fn extract_source_after(&self, name: &str, block_type: &str) -> Option<String> {
        let pattern = format!(r#"{}\s+"{}"\s*\{{[^}}]*source\s*=\s*"([^"]+)""#, block_type, regex::escape(name));
        if let Ok(source_regex) = safe_regex_compile(&pattern) {
            if let Some(captures) = source_regex.captures(&self.content) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_regex_creation() {
        let result = get_resource_regex();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_resources_safe() {
        let content = r#"
        resource "aws_instance" "example" {
          ami = "ami-123456"
        }
        "#;
        
        let parser = TerraformParserSafe::new(content.to_string());
        let resources = parser.parse_resources("test.tf");
        
        assert!(resources.is_ok());
        let resources = resources.unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].resource_type, "aws_instance");
        assert_eq!(resources[0].name, "example");
    }

    #[test]
    fn test_parse_variables_safe() {
        let content = r#"
        variable "instance_type" {
          description = "Type of instance"
          type = string
          default = "t2.micro"
        }
        "#;
        
        let parser = TerraformParserSafe::new(content.to_string());
        let variables = parser.parse_variables();
        
        assert!(variables.is_ok());
        let variables = variables.unwrap();
        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].name, "instance_type");
    }
}