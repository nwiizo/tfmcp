use crate::terraform::model::{TerraformOutput, TerraformProvider, TerraformResource, TerraformVariable};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

// Lazy-initialized regex patterns for better performance
static RESOURCE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"resource\s+"([^"]+)"\s+"([^"]+)""#).expect("Invalid resource regex")
});

static VARIABLE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"variable\s+"([^"]+)""#).expect("Invalid variable regex")
});

static OUTPUT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"output\s+"([^"]+)""#).expect("Invalid output regex")
});

static PROVIDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"provider\s+"([^"]+)""#).expect("Invalid provider regex")
});

/// Parser for Terraform HCL files
pub struct TerraformParser {
    content: String,
}

impl TerraformParser {
    pub fn new(content: String) -> Self {
        Self { content }
    }

    /// Parse all resources from the content
    pub fn parse_resources(&self, file_name: &str) -> Vec<TerraformResource> {
        RESOURCE_REGEX
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if captures.len() >= 3 {
                    let resource_type = captures[1].to_string();
                    let resource_name = captures[2].to_string();
                    let provider = resource_type
                        .split('_')
                        .next()
                        .unwrap_or("unknown")
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
            .collect()
    }

    /// Parse all variables from the content
    pub fn parse_variables(&self) -> Vec<TerraformVariable> {
        VARIABLE_REGEX
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if captures.len() >= 2 {
                    let name = captures[1].to_string();
                    Some(TerraformVariable {
                        name: name.clone(),
                        description: self.extract_field_value(&name, "variable", "description"),
                        type_: self.extract_field_type(&name),
                        default: self.extract_field_json(&name, "variable", "default"),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parse all outputs from the content
    pub fn parse_outputs(&self) -> Vec<TerraformOutput> {
        OUTPUT_REGEX
            .captures_iter(&self.content)
            .filter_map(|captures| {
                if captures.len() >= 2 {
                    let name = captures[1].to_string();
                    Some(TerraformOutput {
                        name: name.clone(),
                        description: self.extract_field_value(&name, "output", "description"),
                        value: None,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parse all providers from the content
    pub fn parse_providers(&self) -> Vec<TerraformProvider> {
        let mut providers = HashMap::new();

        // Parse provider blocks
        for captures in PROVIDER_REGEX.captures_iter(&self.content) {
            if captures.len() >= 2 {
                let name = captures[1].to_string();
                let version = self.extract_provider_version(&name);
                providers.insert(name.clone(), TerraformProvider { name, version });
            }
        }

        // Also check required_providers block
        if let Some(required_providers) = self.extract_required_providers() {
            for (name, version) in required_providers {
                providers
                    .entry(name.clone())
                    .or_insert(TerraformProvider { name, version });
            }
        }

        providers.into_values().collect()
    }

    /// Extract type field specifically (handles unquoted values like 'string', 'number', etc.)
    fn extract_field_type(&self, name: &str) -> Option<String> {
        let pattern = format!(
            r#"variable\s+"{}"\s*\{{[^}}]*type\s*=\s*([^\n\s}}]+)"#,
            regex::escape(name)
        );
        
        Regex::new(&pattern)
            .ok()?
            .captures(&self.content)?
            .get(1)
            .map(|m| m.as_str().trim().to_string())
    }

    /// Extract a field value from a block
    fn extract_field_value(&self, name: &str, block_type: &str, field: &str) -> Option<String> {
        let pattern = format!(
            r#"{}\s+"{}"\s*\{{[^}}]*{}\s*=\s*"([^"]+)""#,
            block_type,
            regex::escape(name),
            field
        );
        
        Regex::new(&pattern)
            .ok()?
            .captures(&self.content)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    /// Extract a JSON field value from a block
    fn extract_field_json(&self, name: &str, block_type: &str, field: &str) -> Option<Value> {
        let pattern = format!(
            r#"{}\s+"{}"\s*\{{[^}}]*{}\s*=\s*([^\n]+)"#,
            block_type,
            regex::escape(name),
            field
        );
        
        let regex = Regex::new(&pattern).ok()?;
        let captures = regex.captures(&self.content)?;
        let value_str = captures.get(1)?.as_str().trim();
        
        // Try to parse as JSON
        if let Ok(value) = serde_json::from_str(value_str) {
            Some(value)
        } else {
            // If not valid JSON, return as string
            Some(Value::String(value_str.to_string()))
        }
    }

    /// Extract provider version from various locations
    fn extract_provider_version(&self, provider_name: &str) -> Option<String> {
        // Check provider block
        if let Some(version) = self.extract_field_value(provider_name, "provider", "version") {
            return Some(version);
        }

        // Check required_providers block
        let pattern = format!(
            r#"required_providers\s*\{{[^}}]*{}\s*=\s*\{{[^}}]*version\s*=\s*"([^"]+)""#,
            regex::escape(provider_name)
        );
        
        Regex::new(&pattern)
            .ok()?
            .captures(&self.content)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    /// Extract all required providers
    fn extract_required_providers(&self) -> Option<HashMap<String, Option<String>>> {
        let pattern = r#"required_providers\s*\{([^}]+)\}"#;
        let regex = Regex::new(pattern).ok()?;
        let captures = regex.captures(&self.content)?;
        let block_content = captures.get(1)?.as_str();
        
        let mut providers = HashMap::new();
        let provider_pattern = r#"(\w+)\s*=\s*\{[^}]*version\s*=\s*"([^"]+)""#;
        let provider_regex = Regex::new(provider_pattern).ok()?;
        
        for captures in provider_regex.captures_iter(block_content) {
            if let (Some(name), Some(version)) = (captures.get(1), captures.get(2)) {
                providers.insert(
                    name.as_str().to_string(),
                    Some(version.as_str().to_string()),
                );
            }
        }
        
        Some(providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resources() {
        let content = r#"
resource "aws_instance" "example" {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = "t2.micro"
}

resource "aws_s3_bucket" "data" {
  bucket = "my-data-bucket"
}
"#;
        let parser = TerraformParser::new(content.to_string());
        let resources = parser.parse_resources("test.tf");
        
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].resource_type, "aws_instance");
        assert_eq!(resources[0].name, "example");
        assert_eq!(resources[0].provider, "aws");
        assert_eq!(resources[1].resource_type, "aws_s3_bucket");
        assert_eq!(resources[1].name, "data");
    }

    #[test]
    fn test_parse_variables() {
        let content = r#"
variable "region" {
  description = "AWS region"
  type        = string
  default     = "us-west-2"
}

variable "instance_count" {
  description = "Number of instances"
  type        = number
  default     = 2
}

variable "tags" {
  type = map(string)
}
"#;
        let parser = TerraformParser::new(content.to_string());
        let variables = parser.parse_variables();
        
        assert_eq!(variables.len(), 3);
        assert_eq!(variables[0].name, "region");
        assert_eq!(variables[0].description, Some("AWS region".to_string()));
        assert_eq!(variables[0].type_, Some("string".to_string()));
        assert_eq!(variables[1].name, "instance_count");
        assert_eq!(variables[2].name, "tags");
        assert_eq!(variables[2].description, None);
    }

    #[test]
    fn test_parse_outputs() {
        let content = r#"
output "instance_ip" {
  description = "Public IP of the instance"
  value       = aws_instance.example.public_ip
}

output "bucket_arn" {
  value = aws_s3_bucket.data.arn
}
"#;
        let parser = TerraformParser::new(content.to_string());
        let outputs = parser.parse_outputs();
        
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].name, "instance_ip");
        assert_eq!(outputs[0].description, Some("Public IP of the instance".to_string()));
        assert_eq!(outputs[1].name, "bucket_arn");
        assert_eq!(outputs[1].description, None);
    }

    #[test]
    fn test_parse_providers() {
        let content = r#"
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "3.1.0"
    }
  }
}

provider "aws" {
  region = var.region
}

provider "google" {
  project = "my-project"
  version = "~> 4.0"
}
"#;
        let parser = TerraformParser::new(content.to_string());
        let providers = parser.parse_providers();
        
        assert!(providers.len() >= 2);
        
        let aws_provider = providers.iter().find(|p| p.name == "aws");
        assert!(aws_provider.is_some());
        assert_eq!(aws_provider.unwrap().version, Some("~> 4.0".to_string()));
        
        let google_provider = providers.iter().find(|p| p.name == "google");
        assert!(google_provider.is_some());
    }
}