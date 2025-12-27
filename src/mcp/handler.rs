use crate::core::tfmcp::{JsonRpcErrorCode, TfMcp};
use crate::mcp::stdio::{Message, StdioTransport, Transport};
use crate::registry::fallback::RegistryClientWithFallback;
use crate::registry::provider::ProviderResolver;
use crate::shared::logging;
use futures::StreamExt;
use serde_json::{json, Value};
use std::path::PathBuf;

// MCP Resource Contents
const TERRAFORM_STYLE_GUIDE: &str = r#"# Terraform Style Guide

## Overview
This style guide provides best practices for writing clean, maintainable Terraform configurations.

## File Structure

### Standard Files
- `main.tf` - Primary resource definitions
- `variables.tf` - Variable declarations
- `outputs.tf` - Output definitions
- `providers.tf` - Provider configurations
- `versions.tf` - Terraform and provider version constraints
- `terraform.tfvars` - Variable values (don't commit secrets)
- `locals.tf` - Local value definitions

### Directory Structure
```
project/
├── modules/
│   ├── networking/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   └── outputs.tf
│   └── compute/
│       ├── main.tf
│       ├── variables.tf
│       └── outputs.tf
├── environments/
│   ├── dev/
│   ├── staging/
│   └── prod/
├── main.tf
├── variables.tf
├── outputs.tf
└── versions.tf
```

## Naming Conventions

### Resources
- Use lowercase with underscores: `aws_instance.web_server`
- Be descriptive but concise
- Include purpose in the name: `aws_security_group.allow_https`

### Variables
- Use lowercase with underscores: `instance_type`
- Prefix with resource type for clarity: `vpc_cidr_block`
- Use descriptive names: `enable_monitoring` not `em`

### Outputs
- Use lowercase with underscores
- Prefix with resource type: `vpc_id`, `subnet_ids`
- Be consistent with variable naming

## Formatting

### Indentation
- Use 2 spaces for indentation
- Align `=` signs within blocks when it improves readability

### Blocks
```hcl
resource "aws_instance" "example" {
  ami           = var.ami_id
  instance_type = var.instance_type

  tags = {
    Name        = "example-instance"
    Environment = var.environment
  }
}
```

### Meta-Arguments Order
1. `count` or `for_each`
2. Resource-specific arguments
3. `depends_on`
4. `lifecycle`

## Variables

### Always Include
- `description` - What the variable is for
- `type` - Variable type constraint
- `default` - When a sensible default exists

### Example
```hcl
variable "instance_type" {
  description = "EC2 instance type for the web server"
  type        = string
  default     = "t3.micro"

  validation {
    condition     = can(regex("^t[23]\\.", var.instance_type))
    error_message = "Instance type must be a t2 or t3 instance."
  }
}
```

## Outputs

### Always Include
- `description` - What the output provides
- `value` - The actual output value

### Example
```hcl
output "instance_public_ip" {
  description = "Public IP address of the web server"
  value       = aws_instance.web_server.public_ip
}
```

## Comments

### When to Comment
- Complex logic or calculations
- Non-obvious dependencies
- Temporary workarounds

### Format
```hcl
# Single line comment for brief explanations

/*
 * Multi-line comment for longer
 * explanations or documentation
 */
```

## Best Practices

### State Management
- Use remote state (S3, GCS, Azure Blob)
- Enable state locking
- Don't commit state files

### Security
- Never hardcode secrets
- Use variables or data sources for sensitive values
- Enable encryption for state files
- Use least-privilege IAM policies

### Version Constraints
```hcl
terraform {
  required_version = ">= 1.0.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}
```

### Resource Dependencies
- Use implicit dependencies when possible
- Use `depends_on` only for hidden dependencies
- Document explicit dependencies with comments
"#;

const TERRAFORM_MODULE_DEVELOPMENT: &str = r#"# Terraform Module Development Guide

## Overview
This guide covers best practices for developing reusable Terraform modules.

## Module Structure

### Basic Structure
```
module/
├── main.tf          # Primary resource definitions
├── variables.tf     # Input variables
├── outputs.tf       # Output values
├── versions.tf      # Version constraints
├── README.md        # Documentation
├── examples/        # Usage examples
│   └── basic/
│       └── main.tf
└── tests/          # Module tests
    └── basic_test.go
```

### Advanced Structure
```
module/
├── main.tf
├── variables.tf
├── outputs.tf
├── versions.tf
├── locals.tf        # Local values
├── data.tf          # Data sources
├── README.md
├── CHANGELOG.md
├── LICENSE
├── .terraform-docs.yml
├── examples/
│   ├── basic/
│   ├── complete/
│   └── with-existing-resources/
├── modules/         # Submodules
│   └── submodule/
└── tests/
```

## Input Variables

### Design Principles
1. Provide sensible defaults when possible
2. Use validation blocks for constraints
3. Mark sensitive variables appropriately
4. Group related variables logically

### Example
```hcl
variable "name" {
  description = "Name prefix for all resources"
  type        = string

  validation {
    condition     = length(var.name) <= 32
    error_message = "Name must be 32 characters or less."
  }
}

variable "tags" {
  description = "Tags to apply to all resources"
  type        = map(string)
  default     = {}
}

variable "database_password" {
  description = "Password for the database"
  type        = string
  sensitive   = true
}
```

## Output Values

### Design Principles
1. Output all resource attributes users might need
2. Use clear, descriptive names
3. Include descriptions for all outputs
4. Consider output structure for complex resources

### Example
```hcl
output "id" {
  description = "The ID of the created resource"
  value       = aws_instance.this.id
}

output "arn" {
  description = "The ARN of the created resource"
  value       = aws_instance.this.arn
}

output "instance" {
  description = "All attributes of the instance"
  value       = aws_instance.this
}
```

## Versioning

### Semantic Versioning
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes (backward compatible)

### Version Constraints
```hcl
terraform {
  required_version = ">= 1.3.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = ">= 4.0.0, < 6.0.0"
    }
  }
}
```

## Documentation

### README Contents
1. Module description and purpose
2. Usage examples
3. Requirements (Terraform version, providers)
4. Input variables table
5. Output values table
6. License information

### terraform-docs
Use terraform-docs to auto-generate documentation:
```bash
terraform-docs markdown table . > README.md
```

## Testing

### Manual Testing
```bash
cd examples/basic
terraform init
terraform plan
terraform apply
terraform destroy
```

### Automated Testing with Terratest
```go
package test

import (
    "testing"
    "github.com/gruntwork-io/terratest/modules/terraform"
)

func TestBasicExample(t *testing.T) {
    terraformOptions := &terraform.Options{
        TerraformDir: "../examples/basic",
    }
    defer terraform.Destroy(t, terraformOptions)
    terraform.InitAndApply(t, terraformOptions)
}
```

## Publishing

### Terraform Registry
1. Host module on GitHub
2. Use semantic version tags (v1.0.0)
3. Follow naming convention: terraform-<PROVIDER>-<NAME>
4. Include required documentation

### Private Registry
- Use Terraform Cloud/Enterprise private registry
- Or host your own with module sources

## Best Practices

### Composition Over Inheritance
- Create small, focused modules
- Compose larger configurations from smaller modules
- Avoid deeply nested module hierarchies

### Flexibility vs. Simplicity
- Provide escape hatches for advanced users
- Keep simple use cases simple
- Use object variables for complex configurations

### Conditional Resources
```hcl
variable "create_resource" {
  description = "Whether to create the resource"
  type        = bool
  default     = true
}

resource "aws_instance" "this" {
  count = var.create_resource ? 1 : 0
  # ...
}
```

### Dynamic Blocks
```hcl
dynamic "ingress" {
  for_each = var.ingress_rules
  content {
    from_port   = ingress.value.from_port
    to_port     = ingress.value.to_port
    protocol    = ingress.value.protocol
    cidr_blocks = ingress.value.cidr_blocks
  }
}
```
"#;

const TERRAFORM_BEST_PRACTICES: &str = r#"# Terraform Best Practices

## Security Best Practices

### Secrets Management
1. Never commit secrets to version control
2. Use environment variables or secret management tools
3. Enable encryption for state files
4. Use data sources for dynamic secret retrieval

```hcl
# Good: Use data sources for secrets
data "aws_secretsmanager_secret_version" "db_password" {
  secret_id = "prod/db/password"
}

resource "aws_db_instance" "main" {
  password = data.aws_secretsmanager_secret_version.db_password.secret_string
}
```

### IAM and Access Control
- Apply least privilege principle
- Use IAM roles instead of access keys
- Enable MFA for sensitive operations
- Regularly audit permissions

### State Security
```hcl
terraform {
  backend "s3" {
    bucket         = "terraform-state"
    key            = "prod/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-locks"
  }
}
```

## Performance Best Practices

### State Management
- Use workspaces for environment separation
- Split large configurations into smaller states
- Use remote state data sources for cross-project references

### Resource Targeting
```bash
# Plan specific resources for faster feedback
terraform plan -target=aws_instance.web_server

# Apply specific changes
terraform apply -target=module.networking
```

### Parallelism
```bash
# Increase parallelism for faster operations
terraform apply -parallelism=20
```

## Operational Best Practices

### Version Control
- Use separate branches for environments
- Review all changes before applying
- Use pull requests for infrastructure changes
- Tag releases with semantic versions

### CI/CD Integration
```yaml
# Example GitHub Actions workflow
name: Terraform
on: [push, pull_request]

jobs:
  terraform:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: hashicorp/setup-terraform@v2
      - run: terraform fmt -check
      - run: terraform init
      - run: terraform validate
      - run: terraform plan
```

### Environment Management
- Use workspaces or separate directories
- Keep environment configurations DRY
- Use variable files per environment

```bash
terraform workspace select prod
terraform apply -var-file="prod.tfvars"
```

## Code Quality

### Validation
```hcl
variable "environment" {
  type = string
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}
```

### Pre-commit Hooks
```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/antonbabenko/pre-commit-terraform
    rev: v1.77.0
    hooks:
      - id: terraform_fmt
      - id: terraform_validate
      - id: terraform_tflint
      - id: terraform_docs
```

### Linting with tflint
```bash
tflint --init
tflint
```

## Disaster Recovery

### State Backup
- Enable versioning on state storage
- Regularly backup state files
- Test state recovery procedures

### Drift Detection
```bash
# Detect configuration drift
terraform plan -detailed-exitcode
```

### Import Existing Resources
```bash
terraform import aws_instance.example i-1234567890abcdef0
```

## Cost Management

### Cost Estimation
```bash
# Use Infracost for cost estimation
infracost breakdown --path .
```

### Resource Tagging
```hcl
locals {
  common_tags = {
    Project     = var.project_name
    Environment = var.environment
    ManagedBy   = "terraform"
    CostCenter  = var.cost_center
  }
}

resource "aws_instance" "example" {
  # ...
  tags = merge(local.common_tags, {
    Name = "example-instance"
  })
}
```

## Collaboration

### Code Review Checklist
- [ ] Variables have descriptions and types
- [ ] Outputs are documented
- [ ] Resources follow naming conventions
- [ ] Security best practices followed
- [ ] Tests pass
- [ ] Documentation updated

### Remote State Locking
- Always enable state locking
- Use DynamoDB for S3 backend
- Handle lock conflicts appropriately

```hcl
terraform {
  backend "s3" {
    bucket         = "terraform-state"
    key            = "terraform.tfstate"
    region         = "us-east-1"
    dynamodb_table = "terraform-locks"
  }
}
```
"#;

const TOOLS_JSON: &str = r#"{
  "tools": [
    {
      "name": "list_terraform_resources",
      "description": "List all resources defined in the Terraform project",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "resources": {
            "type": "array",
            "items": {
              "type": "string"
            },
            "description": "List of resource identifiers"
          }
        },
        "required": ["resources"]
      },
      "annotations": {
        "title": "List Terraform Resources",
        "readOnlyHint": true
      }
    },
    {
      "name": "destroy_terraform",
      "description": "Destroy all resources defined in the Terraform project (requires TFMCP_DELETE_ENABLED=true)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "auto_approve": {
            "type": "boolean",
            "description": "Whether to automatically approve the destroy operation without confirmation"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "output": {
            "type": "string",
            "description": "Output from the Terraform destroy command"
          }
        },
        "required": ["output"]
      },
      "annotations": {
        "title": "Destroy Terraform",
        "destructiveHint": true
      }
    },
    {
      "name": "analyze_terraform",
      "description": "Analyze Terraform configuration files and provide detailed information",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": {
            "type": "string",
            "description": "Path to the Terraform configuration directory (optional)"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "analysis": {
            "type": "object",
            "properties": {
              "resources": {
                "type": "array",
                "items": {
                  "type": "object",
                  "properties": {
                    "type": {
                      "type": "string",
                      "description": "Terraform resource type"
                    },
                    "name": {
                      "type": "string",
                      "description": "Resource name"
                    },
                    "file": {
                      "type": "string",
                      "description": "File containing the resource definition"
                    }
                  }
                }
              }
            }
          }
        },
        "required": ["analysis"]
      },
      "annotations": {
        "title": "Analyze Terraform",
        "readOnlyHint": true
      }
    },
    {
      "name": "get_terraform_plan",
      "description": "Execute 'terraform plan' and return the output",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "plan": {
            "type": "string",
            "description": "Terraform plan output"
          }
        },
        "required": ["plan"]
      },
      "annotations": {
        "title": "Get Terraform Plan",
        "readOnlyHint": true
      }
    },
    {
      "name": "apply_terraform",
      "description": "Apply Terraform configuration (WARNING: This will make actual changes to your infrastructure)",
      "inputSchema": {
        "type": "object",
        "properties": {
          "auto_approve": {
            "type": "boolean",
            "description": "Whether to auto-approve changes without confirmation"
          }
        }
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "output": {
            "type": "string",
            "description": "Terraform apply output"
          }
        },
        "required": ["output"]
      },
      "annotations": {
        "title": "Apply Terraform",
        "destructiveHint": true
      }
    },
    {
      "name": "validate_terraform",
      "description": "Validate Terraform configuration files",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "valid": {
            "type": "boolean",
            "description": "Whether the configuration is valid"
          },
          "message": {
            "type": "string",
            "description": "Validation message"
          }
        },
        "required": ["valid", "message"]
      },
      "annotations": {
        "title": "Validate Terraform",
        "readOnlyHint": true
      }
    },
    {
      "name": "validate_terraform_detailed",
      "description": "Perform detailed validation of Terraform configuration files with best practice checks",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "valid": {
            "type": "boolean",
            "description": "Whether the configuration is valid"
          },
          "error_count": {
            "type": "integer",
            "description": "Number of validation errors"
          },
          "warning_count": {
            "type": "integer",
            "description": "Number of warnings including best practice violations"
          },
          "diagnostics": {
            "type": "array",
            "description": "List of validation diagnostics from Terraform",
            "items": {
              "type": "object",
              "properties": {
                "severity": {
                  "type": "string",
                  "description": "Severity level (error, warning)"
                },
                "summary": {
                  "type": "string",
                  "description": "Summary of the diagnostic"
                },
                "detail": {
                  "type": "string",
                  "description": "Detailed description"
                },
                "range": {
                  "type": "object",
                  "description": "Location of the issue in the file",
                  "properties": {
                    "filename": {
                      "type": "string"
                    },
                    "start": {
                      "type": "object",
                      "properties": {
                        "line": { "type": "integer" },
                        "column": { "type": "integer" }
                      }
                    },
                    "end": {
                      "type": "object",
                      "properties": {
                        "line": { "type": "integer" },
                        "column": { "type": "integer" }
                      }
                    }
                  }
                }
              }
            }
          },
          "additional_warnings": {
            "type": "array",
            "description": "Additional warnings from best practice analysis",
            "items": {
              "type": "string"
            }
          },
          "suggestions": {
            "type": "array",
            "description": "Suggestions for improving the configuration",
            "items": {
              "type": "string"
            }
          },
          "checked_files": {
            "type": "integer",
            "description": "Number of Terraform files checked"
          }
        },
        "required": ["valid", "error_count", "warning_count", "diagnostics", "additional_warnings", "suggestions", "checked_files"]
      },
      "annotations": {
        "title": "Validate Terraform Detailed",
        "readOnlyHint": true
      }
    },
    {
      "name": "get_terraform_state",
      "description": "Get the current Terraform state",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "state": {
            "type": "string",
            "description": "Terraform state output"
          }
        },
        "required": ["state"]
      },
      "annotations": {
        "title": "Get Terraform State",
        "readOnlyHint": true
      }
    },
    {
      "name": "init_terraform",
      "description": "Initialize a Terraform project",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "output": {
            "type": "string",
            "description": "Terraform init output"
          }
        },
        "required": ["output"]
      },
      "annotations": {
        "title": "Initialize Terraform",
        "openWorldHint": true,
        "idempotentHint": true
      }
    },
    {
      "name": "get_security_status",
      "description": "Get current security policy and status",
      "inputSchema": {
        "type": "object",
        "properties": {}
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "policy": {
            "type": "object",
            "description": "Current security policy configuration"
          },
          "permissions": {
            "type": "object",
            "description": "Current operation permissions"
          },
          "audit_enabled": {
            "type": "boolean",
            "description": "Whether audit logging is enabled"
          }
        },
        "required": ["policy", "permissions", "audit_enabled"]
      },
      "annotations": {
        "title": "Get Security Status",
        "readOnlyHint": true
      }
    },
    {
      "name": "search_terraform_providers",
      "description": "Search for Terraform providers in the official registry",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": {
            "type": "string",
            "description": "Search query for provider names"
          }
        },
        "required": ["query"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "providers": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string" },
                "namespace": { "type": "string" },
                "version": { "type": "string" },
                "description": { "type": "string" }
              }
            }
          }
        },
        "required": ["providers"]
      },
      "annotations": {
        "title": "Search Terraform Providers",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "get_provider_info",
      "description": "Get detailed information about a specific Terraform provider",
      "inputSchema": {
        "type": "object",
        "properties": {
          "provider_name": {
            "type": "string",
            "description": "Name of the provider"
          },
          "namespace": {
            "type": "string",
            "description": "Provider namespace (optional, will try common namespaces)"
          }
        },
        "required": ["provider_name"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "provider": {
            "type": "object",
            "description": "Provider information including versions and documentation"
          }
        },
        "required": ["provider"]
      },
      "annotations": {
        "title": "Get Provider Info",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "get_provider_docs",
      "description": "Get documentation for specific provider resources",
      "inputSchema": {
        "type": "object",
        "properties": {
          "provider_name": {
            "type": "string",
            "description": "Name of the provider"
          },
          "namespace": {
            "type": "string",
            "description": "Provider namespace (optional)"
          },
          "service_slug": {
            "type": "string",
            "description": "Service or resource name to search for"
          },
          "data_type": {
            "type": "string",
            "description": "Type of documentation (resources, data-sources)",
            "enum": ["resources", "data-sources"]
          }
        },
        "required": ["provider_name", "service_slug"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "documentation": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "content": { "type": "string" }
              }
            }
          }
        },
        "required": ["documentation"]
      },
      "annotations": {
        "title": "Get Provider Docs",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "set_terraform_directory",
      "description": "Change the current Terraform project directory",
      "inputSchema": {
        "type": "object",
        "properties": {
          "directory": {
            "type": "string",
            "description": "Path to the new Terraform project directory"
          }
        },
        "required": ["directory"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "success": {
            "type": "boolean",
            "description": "Whether the directory change was successful"
          },
          "directory": {
            "type": "string",
            "description": "The new Terraform project directory path"
          },
          "message": {
            "type": "string",
            "description": "Status message"
          }
        },
        "required": ["success", "directory", "message"]
      },
      "annotations": {
        "title": "Set Terraform Directory",
        "idempotentHint": true
      }
    },
    {
      "name": "search_terraform_modules",
      "description": "Search for Terraform modules in the official registry",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": {
            "type": "string",
            "description": "Search query for module names (e.g., 'vpc', 'eks', 's3')"
          }
        },
        "required": ["query"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "modules": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "id": { "type": "string" },
                "namespace": { "type": "string" },
                "name": { "type": "string" },
                "provider": { "type": "string" },
                "version": { "type": "string" },
                "description": { "type": "string" },
                "downloads": { "type": "integer" },
                "verified": { "type": "boolean" }
              }
            }
          }
        },
        "required": ["modules"]
      },
      "annotations": {
        "title": "Search Terraform Modules",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "get_module_details",
      "description": "Get detailed information about a specific Terraform module including inputs, outputs, and dependencies",
      "inputSchema": {
        "type": "object",
        "properties": {
          "namespace": {
            "type": "string",
            "description": "Module namespace (e.g., 'terraform-aws-modules')"
          },
          "name": {
            "type": "string",
            "description": "Module name (e.g., 'vpc')"
          },
          "provider": {
            "type": "string",
            "description": "Provider name (e.g., 'aws')"
          },
          "version": {
            "type": "string",
            "description": "Specific version to retrieve (optional, defaults to latest)"
          }
        },
        "required": ["namespace", "name", "provider"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "module": {
            "type": "object",
            "description": "Detailed module information including inputs, outputs, resources, and submodules"
          }
        },
        "required": ["module"]
      },
      "annotations": {
        "title": "Get Module Details",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "get_latest_module_version",
      "description": "Get the latest version of a Terraform module",
      "inputSchema": {
        "type": "object",
        "properties": {
          "namespace": {
            "type": "string",
            "description": "Module namespace (e.g., 'terraform-aws-modules')"
          },
          "name": {
            "type": "string",
            "description": "Module name (e.g., 'vpc')"
          },
          "provider": {
            "type": "string",
            "description": "Provider name (e.g., 'aws')"
          }
        },
        "required": ["namespace", "name", "provider"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "version": {
            "type": "string",
            "description": "Latest version of the module"
          },
          "module_id": {
            "type": "string",
            "description": "Full module identifier"
          }
        },
        "required": ["version", "module_id"]
      },
      "annotations": {
        "title": "Get Latest Module Version",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "get_latest_provider_version",
      "description": "Get the latest version of a Terraform provider",
      "inputSchema": {
        "type": "object",
        "properties": {
          "provider_name": {
            "type": "string",
            "description": "Name of the provider (e.g., 'aws', 'google', 'kubernetes')"
          },
          "namespace": {
            "type": "string",
            "description": "Provider namespace (optional, will try common namespaces like 'hashicorp')"
          }
        },
        "required": ["provider_name"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "version": {
            "type": "string",
            "description": "Latest version of the provider"
          },
          "namespace": {
            "type": "string",
            "description": "Namespace where the provider was found"
          },
          "provider_id": {
            "type": "string",
            "description": "Full provider identifier"
          }
        },
        "required": ["version", "namespace", "provider_id"]
      },
      "annotations": {
        "title": "Get Latest Provider Version",
        "readOnlyHint": true,
        "openWorldHint": true
      }
    },
    {
      "name": "analyze_module_health",
      "description": "Analyze module health based on whitebox principles. Detects issues related to cohesion (logical vs functional), coupling (control vs data), variable exposure, hierarchy depth, and documentation quality. Returns health score, issues, and recommendations following infrastructure-as-code best practices.",
      "inputSchema": {
        "type": "object",
        "properties": {},
        "required": []
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "module_path": { "type": "string" },
          "health_score": { "type": "integer", "description": "0-100, higher is better" },
          "metrics": {
            "type": "object",
            "description": "Quantitative metrics including variable_count, resource_count, resource_type_count, hierarchy_depth"
          },
          "issues": {
            "type": "array",
            "description": "List of detected issues with severity, category, and message"
          },
          "recommendations": {
            "type": "array",
            "description": "Actionable recommendations for improvement"
          },
          "cohesion_analysis": {
            "type": "object",
            "description": "Cohesion type (Functional, Logical, etc.) and score"
          },
          "coupling_analysis": {
            "type": "object",
            "description": "Coupling type (Data, Control, etc.) and dependencies"
          }
        }
      },
      "annotations": {
        "title": "Analyze Module Health",
        "readOnlyHint": true
      }
    },
    {
      "name": "get_resource_dependency_graph",
      "description": "Build a resource dependency graph for visualization. Shows nodes (resources), edges (dependencies), and module boundaries. Useful for understanding resource relationships and identifying hidden dependencies.",
      "inputSchema": {
        "type": "object",
        "properties": {},
        "required": []
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "nodes": {
            "type": "array",
            "description": "Resource nodes with id, type, name, module_path, file, provider"
          },
          "edges": {
            "type": "array",
            "description": "Dependency edges with source, target, dependency_type (Explicit, Implicit, DataSource)"
          },
          "module_boundaries": {
            "type": "array",
            "description": "Module groupings for visualization"
          }
        }
      },
      "annotations": {
        "title": "Get Resource Dependency Graph",
        "readOnlyHint": true
      }
    },
    {
      "name": "suggest_module_refactoring",
      "description": "Generate refactoring suggestions based on module health analysis. Suggests actions like splitting modules, wrapping public modules, adding documentation, and flattening hierarchies. Each suggestion includes migration steps.",
      "inputSchema": {
        "type": "object",
        "properties": {},
        "required": []
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "suggestions": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "suggestion_type": { "type": "string", "description": "Type of refactoring (SplitModule, WrapPublicModule, etc.)" },
                "priority": { "type": "string", "description": "Critical, Warning, or Info" },
                "description": { "type": "string" },
                "affected_resources": { "type": "array" },
                "migration_steps": { "type": "array" }
              }
            }
          }
        }
      },
      "annotations": {
        "title": "Suggest Module Refactoring",
        "readOnlyHint": true
      }
    }
  ]
}"#;

pub struct McpHandler<'a> {
    tfmcp: &'a mut TfMcp,
    initialized: bool,
    registry_client: RegistryClientWithFallback,
    provider_resolver: ProviderResolver,
}

impl<'a> McpHandler<'a> {
    pub fn new(tfmcp: &'a mut TfMcp) -> Self {
        Self {
            tfmcp,
            initialized: false,
            registry_client: RegistryClientWithFallback::new(),
            provider_resolver: ProviderResolver::new(),
        }
    }

    pub async fn launch_mcp(&mut self, transport: &StdioTransport) -> anyhow::Result<()> {
        logging::info("MCP stdio transport server started. Waiting for JSON messages on stdin...");

        // Create the stream receiver first, before sending any log messages
        let mut stream = transport.receive();

        // Add a small delay to ensure the receiver is ready
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        logging::send_log_message(
            transport,
            logging::LogLevel::Info,
            "tfmcp server initialized and ready",
        )
        .await?;

        // Add debug logging for stream creation
        logging::debug("Created message stream, starting to listen for messages...");

        while let Some(msg_result) = stream.next().await {
            logging::debug("Stream received a message, processing...");

            // Debug: Log what type of message we received
            match &msg_result {
                Ok(msg) => {
                    logging::debug(&format!(
                        "Received message type: {:?}",
                        std::any::type_name_of_val(msg)
                    ));
                }
                Err(e) => {
                    logging::debug(&format!("Received error: {:?}", e));
                }
            }

            match msg_result {
                Ok(Message::Request {
                    id, method, params, ..
                }) => {
                    logging::log_both(
                        transport,
                        logging::LogLevel::Debug,
                        &format!(
                            "Got Request: id={}, method={}, params={:?}",
                            id, method, params
                        ),
                    )
                    .await?;

                    // Handle initialization request first
                    if method == "initialize" {
                        if let Err(err) = self.handle_initialize(transport, id).await {
                            logging::error(&format!("Error handling initialize request: {}", err));
                            self.send_error_response(
                                transport,
                                id,
                                JsonRpcErrorCode::InternalError,
                                format!("Failed to initialize: {}", err),
                            )
                            .await?;
                        } else {
                            self.initialized = true;
                            logging::info("MCP server successfully initialized");
                        }
                        continue;
                    }

                    // For all other requests, ensure we're initialized
                    if !self.initialized {
                        self.send_error_response(
                            transport,
                            id,
                            JsonRpcErrorCode::InvalidRequest,
                            "Server not initialized. Send 'initialize' request first.".to_string(),
                        )
                        .await?;
                        continue;
                    }

                    // Skip calling handle_request for methods already handled above
                    if method != "initialize" {
                        if let Err(err) = self.handle_request(transport, id, method, params).await {
                            logging::error(&format!("Error handling request: {:?}", err));
                            self.send_error_response(
                                transport,
                                id,
                                JsonRpcErrorCode::InternalError,
                                format!("Failed to handle request: {}", err),
                            )
                            .await?;
                        }
                    }
                }
                Ok(Message::Notification { method, params, .. }) => {
                    logging::log_both(
                        transport,
                        logging::LogLevel::Debug,
                        &format!("Got Notification: method={}, params={:?}", method, params),
                    )
                    .await?;
                }
                Ok(Message::Response {
                    id, result, error, ..
                }) => {
                    logging::log_both(
                        transport,
                        logging::LogLevel::Debug,
                        &format!(
                            "Got Response: id={}, result={:?}, error={:?}",
                            id, result, error
                        ),
                    )
                    .await?;
                }
                Err(e) => {
                    logging::error(&format!("Error receiving message: {:?}", e));
                }
            }
        }

        Ok(())
    }

    async fn handle_request(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        method: String,
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        match &*method {
            "tools/list" => self.handle_tools_list(transport, id).await?,
            "tools/call" => {
                if let Some(params_val) = params {
                    self.handle_tools_call(transport, id, params_val).await?;
                }
            }
            "resources/list" => self.handle_resources_list(transport, id).await?,
            "resources/read" => {
                if let Some(params_val) = params {
                    self.handle_resources_read(transport, id, params_val)
                        .await?;
                }
            }
            "prompts/list" => self.handle_prompts_list(transport, id).await?,
            _ => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::MethodNotFound,
                    format!("Method not found: {}", method),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn handle_initialize(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        logging::info("Handling initialize request");

        // Create a properly structured capabilities response
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "capabilities": {
                    "experimental": {},
                    "prompts": { "listChanged": false },
                    "resources": { "listChanged": false, "subscribe": false },
                    "tools": { "listChanged": false }
                },
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "tfmcp",
                    "version": "0.1.0"
                }
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending initialize response: {}", json_str));
        }

        // Send the response
        match transport.send(response).await {
            Ok(_) => {
                logging::info("Initialize response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send initialize response: {}", e));
                Err(anyhow::anyhow!("Failed to send initialize response: {}", e))
            }
        }
    }

    async fn handle_tools_list(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        let tools_value: serde_json::Value =
            serde_json::from_str(TOOLS_JSON).expect("tools.json must be valid JSON");

        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(tools_value),
            error: None,
        };

        transport.send(response).await?;
        Ok(())
    }

    async fn handle_tools_call(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        params_val: serde_json::Value,
    ) -> anyhow::Result<()> {
        let name = params_val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        logging::info(&format!("Handling tools/call for tool: {}", name));

        match name {
            "list_terraform_resources" => {
                self.handle_list_terraform_resources(transport, id).await?;
            }
            "analyze_terraform" => {
                self.handle_analyze_terraform(transport, id, &params_val)
                    .await?;
            }
            "get_terraform_plan" => {
                self.handle_get_terraform_plan(transport, id).await?;
            }
            "apply_terraform" => {
                self.handle_apply_terraform(transport, id, &params_val)
                    .await?;
            }
            "destroy_terraform" => {
                self.handle_destroy_terraform(transport, id, &params_val)
                    .await?;
            }
            "validate_terraform" => {
                self.handle_validate_terraform(transport, id).await?;
            }
            "validate_terraform_detailed" => {
                self.handle_validate_terraform_detailed(transport, id)
                    .await?;
            }
            "get_terraform_state" => {
                self.handle_get_terraform_state(transport, id).await?;
            }
            "init_terraform" => {
                self.handle_init_terraform(transport, id).await?;
            }
            "set_terraform_directory" => {
                self.handle_set_terraform_directory(transport, id, &params_val)
                    .await?;
            }
            "get_security_status" => {
                self.handle_get_security_status(transport, id).await?;
            }
            "search_terraform_providers" => {
                self.handle_search_terraform_providers(transport, id, &params_val)
                    .await?;
            }
            "get_provider_info" => {
                self.handle_get_provider_info(transport, id, &params_val)
                    .await?;
            }
            "get_provider_docs" => {
                self.handle_get_provider_docs(transport, id, &params_val)
                    .await?;
            }
            "search_terraform_modules" => {
                self.handle_search_terraform_modules(transport, id, &params_val)
                    .await?;
            }
            "get_module_details" => {
                self.handle_get_module_details(transport, id, &params_val)
                    .await?;
            }
            "get_latest_module_version" => {
                self.handle_get_latest_module_version(transport, id, &params_val)
                    .await?;
            }
            "get_latest_provider_version" => {
                self.handle_get_latest_provider_version(transport, id, &params_val)
                    .await?;
            }
            "analyze_module_health" => {
                self.handle_analyze_module_health(transport, id).await?;
            }
            "get_resource_dependency_graph" => {
                self.handle_get_resource_dependency_graph(transport, id)
                    .await?;
            }
            "suggest_module_refactoring" => {
                self.handle_suggest_module_refactoring(transport, id)
                    .await?;
            }
            _ => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::MethodNotFound,
                    format!("Tool not found: {}", name),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_list_terraform_resources(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.list_resources().await {
            Ok(resources) => {
                let result_json = json!({ "resources": resources });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to list Terraform resources: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_analyze_terraform(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        // Get optional path parameter
        let _path = params_val
            .pointer("/arguments/path")
            .and_then(Value::as_str)
            .map(PathBuf::from);

        // Analyze Terraform configurations
        match self.tfmcp.analyze_terraform().await {
            Ok(analysis) => {
                let result_json = json!({ "analysis": analysis });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to analyze Terraform configuration: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_terraform_plan(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.get_terraform_plan().await {
            Ok(plan) => {
                let result_json = json!({ "plan": plan });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get Terraform plan: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_apply_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let auto_approve = params_val
            .pointer("/arguments/auto_approve")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        match self.tfmcp.apply_terraform(auto_approve).await {
            Ok(result) => {
                // Use "output" field to match outputSchema definition
                let result_json = json!({ "output": result });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to apply Terraform configuration: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_validate_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.validate_configuration().await {
            Ok(result) => {
                // If validation succeeded, result will contain a success message
                let valid = !result.contains("Error:");
                let result_json = json!({
                    "valid": valid,
                    "message": result
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to validate Terraform configuration: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_validate_terraform_detailed(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.validate_configuration_detailed().await {
            Ok(result) => {
                let result_json = json!({
                    "valid": result.valid,
                    "error_count": result.error_count,
                    "warning_count": result.warning_count,
                    "diagnostics": result.diagnostics,
                    "additional_warnings": result.additional_warnings,
                    "suggestions": result.suggestions,
                    "checked_files": result.checked_files
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to perform detailed validation: {}", err),
                )
                .await?
            }
        }

        Ok(())
    }

    async fn handle_get_terraform_state(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.get_state().await {
            Ok(state) => {
                let result_json = json!({ "state": state });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get Terraform state: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_init_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        match self.tfmcp.init_terraform().await {
            Ok(result) => {
                // Use "output" field to match outputSchema definition
                let result_json = json!({ "output": result });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to initialize Terraform: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_resources_list(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        logging::info("Handling resources/list request");

        // Create a response with Terraform resources
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "resources": [
                    {
                        "uri": "terraform://style-guide",
                        "name": "Terraform Style Guide",
                        "description": "Official Terraform style guide with best practices for HCL formatting, naming conventions, and code organization",
                        "mimeType": "text/markdown"
                    },
                    {
                        "uri": "terraform://module-development",
                        "name": "Terraform Module Development Guide",
                        "description": "Comprehensive guide on module composition, structure, providers, publishing, and refactoring",
                        "mimeType": "text/markdown"
                    },
                    {
                        "uri": "terraform://best-practices",
                        "name": "Terraform Best Practices",
                        "description": "Security, performance, and operational best practices for Terraform configurations",
                        "mimeType": "text/markdown"
                    }
                ]
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending resources/list response: {}", json_str));
        }

        // Send the response
        match transport.send(response).await {
            Ok(_) => {
                logging::info("Resources list response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send resources/list response: {}", e));
                Err(e.into())
            }
        }
    }

    async fn handle_resources_read(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: serde_json::Value,
    ) -> anyhow::Result<()> {
        logging::info("Handling resources/read request");

        let uri = match params_val.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: uri".to_string(),
                    )
                    .await;
            }
        };

        logging::debug(&format!("Reading resource: {}", uri));

        let (content, mime_type) = match uri {
            "terraform://style-guide" => (TERRAFORM_STYLE_GUIDE, "text/markdown"),
            "terraform://module-development" => (TERRAFORM_MODULE_DEVELOPMENT, "text/markdown"),
            "terraform://best-practices" => (TERRAFORM_BEST_PRACTICES, "text/markdown"),
            _ => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        format!("Unknown resource URI: {}", uri),
                    )
                    .await;
            }
        };

        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": mime_type,
                    "text": content
                }]
            })),
            error: None,
        };

        match transport.send(response).await {
            Ok(_) => {
                logging::info(&format!("Resource {} read successfully", uri));
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send resources/read response: {}", e));
                Err(e.into())
            }
        }
    }

    async fn handle_prompts_list(&self, transport: &StdioTransport, id: u64) -> anyhow::Result<()> {
        logging::info("Handling prompts/list request");

        // Create a response with an empty prompts list
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "prompts": []
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending prompts/list response: {}", json_str));
        }

        // Send the response
        match transport.send(response).await {
            Ok(_) => {
                logging::info("Prompts list response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send prompts/list response: {}", e));
                Err(e.into())
            }
        }
    }

    async fn send_text_response(
        &self,
        transport: &StdioTransport,
        id: u64,
        text: &str,
    ) -> anyhow::Result<()> {
        logging::info(&format!("Sending text response for id {}", id));

        // Create a properly structured text response
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": text
                }]
            })),
            error: None,
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending text response: {}", json_str));
        }

        // Send the response
        match transport.send(response).await {
            Ok(_) => {
                logging::info("Text response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send text response: {}", e));
                Err(anyhow::anyhow!("Failed to send text response: {}", e))
            }
        }
    }

    async fn send_error_response(
        &self,
        transport: &StdioTransport,
        id: u64,
        code: JsonRpcErrorCode,
        message: String,
    ) -> anyhow::Result<()> {
        logging::warn(&format!(
            "Sending error response for id {}: {}",
            id, message
        ));

        // Create a properly structured error response
        let response = Message::Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({
                "code": code as i32,
                "message": message
            })),
        };

        // Log the response for debugging
        if let Ok(json_str) = serde_json::to_string_pretty(&response) {
            logging::debug(&format!("Sending error response: {}", json_str));
        }

        // Send the response
        match transport.send(response).await {
            Ok(_) => {
                logging::info("Error response sent successfully");
                Ok(())
            }
            Err(e) => {
                logging::error(&format!("Failed to send error response: {}", e));
                Err(anyhow::anyhow!("Failed to send error response: {}", e))
            }
        }
    }

    // 新しいハンドラー: Terraformディレクトリを変更する
    async fn handle_set_terraform_directory(
        &mut self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        logging::info("Handling set_terraform_directory request");

        // パラメータから新しいディレクトリパスを取得
        let directory = match params_val
            .pointer("/arguments/directory")
            .and_then(|v| v.as_str())
        {
            Some(dir) => dir.to_string(),
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: directory".to_string(),
                    )
                    .await;
            }
        };

        // ディレクトリを変更
        match self.tfmcp.change_project_directory(directory) {
            Ok(()) => {
                // 現在のディレクトリを取得して応答
                let current_dir = self.tfmcp.get_project_directory();
                let current_dir_str = current_dir.to_string_lossy().to_string();

                let response = Message::Response {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "success": true,
                        "directory": current_dir_str,
                        "message": format!("Successfully changed to Terraform project directory: {}", current_dir_str)
                    })),
                    error: None,
                };

                // レスポンスをログに記録
                if let Ok(json_str) = serde_json::to_string_pretty(&response) {
                    logging::debug(&format!(
                        "Sending set_terraform_directory response: {}",
                        json_str
                    ));
                }

                // レスポンスを送信
                match transport.send(response).await {
                    Ok(_) => {
                        logging::info("Set terraform directory response sent successfully");
                        Ok(())
                    }
                    Err(e) => {
                        logging::error(&format!(
                            "Failed to send set_terraform_directory response: {}",
                            e
                        ));
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to change Terraform directory: {}", e),
                )
                .await
            }
        }
    }

    async fn handle_destroy_terraform(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        // Check for auto_approve parameter
        let auto_approve = params_val
            .pointer("/arguments/auto_approve")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Execute destroy operation
        match self.tfmcp.destroy_terraform(auto_approve).await {
            Ok(result) => {
                let result_json = json!({ "output": result });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                logging::error(&format!("Failed to destroy Terraform resources: {}", err));
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to destroy Terraform resources: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_security_status(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        // Get security policy from TerraformService
        // Since TfMcp doesn't expose the service directly, we need to add a method
        let security_info = json!({
            "policy": {
                "allow_dangerous_operations": std::env::var("TFMCP_ALLOW_DANGEROUS_OPS").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "allow_auto_approve": std::env::var("TFMCP_ALLOW_AUTO_APPROVE").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "max_resource_limit": std::env::var("TFMCP_MAX_RESOURCES").ok().and_then(|v| v.parse::<usize>().ok()),
                "audit_enabled": std::env::var("TFMCP_AUDIT_ENABLED").map(|v| v.to_lowercase() == "true").unwrap_or(true),
                "blocked_patterns": ["**/prod*/**", "**/production*/**", "**/*prod*.tf", "**/*production*.tf", "**/*secret*"]
            },
            "permissions": {
                "apply": std::env::var("TFMCP_ALLOW_DANGEROUS_OPS").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "destroy": std::env::var("TFMCP_ALLOW_DANGEROUS_OPS").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                "plan": true,
                "validate": true,
                "init": true,
                "state": true
            },
            "audit_enabled": std::env::var("TFMCP_AUDIT_ENABLED").map(|v| v.to_lowercase() == "true").unwrap_or(true),
            "current_directory": self.tfmcp.get_project_directory().to_string_lossy(),
            "security_notes": [
                "Set TFMCP_ALLOW_DANGEROUS_OPS=true to enable apply/destroy operations",
                "Set TFMCP_ALLOW_AUTO_APPROVE=true to enable auto-approve for dangerous operations",
                "Set TFMCP_MAX_RESOURCES=N to limit maximum resource count",
                "Audit logs are stored in ~/.tfmcp/audit.log by default"
            ]
        });

        let obj_as_str = serde_json::to_string(&security_info)?;
        self.send_text_response(transport, id, &obj_as_str).await?;
        Ok(())
    }

    // Registry-related handlers
    async fn handle_search_terraform_providers(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let query = match params_val
            .pointer("/arguments/query")
            .and_then(|v| v.as_str())
        {
            Some(q) => q,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: query".to_string(),
                    )
                    .await;
            }
        };

        match self.provider_resolver.search_providers(query).await {
            Ok(providers) => {
                let result_json = json!({ "providers": providers });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to search providers: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_provider_info(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let provider_name = match params_val
            .pointer("/arguments/provider_name")
            .and_then(|v| v.as_str())
        {
            Some(name) => name,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider_name".to_string(),
                    )
                    .await;
            }
        };

        let namespace = params_val
            .pointer("/arguments/namespace")
            .and_then(|v| v.as_str());

        match self
            .registry_client
            .get_provider_info(provider_name, namespace)
            .await
        {
            Ok(provider_info) => {
                // Also get versions for comprehensive information
                match self
                    .registry_client
                    .get_provider_version(provider_name, namespace)
                    .await
                {
                    Ok((version, used_namespace)) => {
                        let result_json = json!({
                            "provider": {
                                "name": provider_info.name,
                                "namespace": used_namespace,
                                "latest_version": version,
                                "description": provider_info.description,
                                "downloads": provider_info.downloads,
                                "published_at": provider_info.published_at,
                                "info": provider_info
                            }
                        });
                        let obj_as_str = serde_json::to_string(&result_json)?;
                        self.send_text_response(transport, id, &obj_as_str).await?;
                    }
                    Err(err) => {
                        // Return basic info even if version lookup fails
                        let result_json = json!({
                            "provider": {
                                "info": provider_info,
                                "version_error": err.to_string()
                            }
                        });
                        let obj_as_str = serde_json::to_string(&result_json)?;
                        self.send_text_response(transport, id, &obj_as_str).await?;
                    }
                }
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get provider info: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_provider_docs(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let provider_name = match params_val
            .pointer("/arguments/provider_name")
            .and_then(|v| v.as_str())
        {
            Some(name) => name,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider_name".to_string(),
                    )
                    .await;
            }
        };

        let service_slug = match params_val
            .pointer("/arguments/service_slug")
            .and_then(|v| v.as_str())
        {
            Some(slug) => slug,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: service_slug".to_string(),
                    )
                    .await;
            }
        };

        let namespace = params_val
            .pointer("/arguments/namespace")
            .and_then(|v| v.as_str());
        let data_type = params_val
            .pointer("/arguments/data_type")
            .and_then(|v| v.as_str())
            .unwrap_or("resources");

        match self
            .registry_client
            .search_docs_with_fallback(provider_name, namespace, service_slug, data_type)
            .await
        {
            Ok((doc_ids, used_namespace)) => {
                // Fetch content for each documentation ID
                let mut documentation = Vec::new();

                for doc_id in doc_ids {
                    match self.provider_resolver.get_provider_docs(&doc_id.id).await {
                        Ok(content) => {
                            documentation.push(json!({
                                "id": doc_id.id,
                                "title": doc_id.title,
                                "description": doc_id.description,
                                "category": doc_id.category,
                                "content": content
                            }));
                        }
                        Err(err) => {
                            // Include the doc entry even if content fetch fails
                            documentation.push(json!({
                                "id": doc_id.id,
                                "title": doc_id.title,
                                "description": doc_id.description,
                                "category": doc_id.category,
                                "content_error": err.to_string()
                            }));
                        }
                    }
                }

                let result_json = json!({
                    "documentation": documentation,
                    "namespace": used_namespace,
                    "provider": provider_name,
                    "service": service_slug,
                    "type": data_type
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get provider documentation: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    // Module-related handlers
    async fn handle_search_terraform_modules(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let query = match params_val
            .pointer("/arguments/query")
            .and_then(|v| v.as_str())
        {
            Some(q) => q,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: query".to_string(),
                    )
                    .await;
            }
        };

        match self.registry_client.primary.search_modules(query).await {
            Ok(modules) => {
                let result_json = json!({ "modules": modules });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to search modules: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_module_details(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let namespace = match params_val
            .pointer("/arguments/namespace")
            .and_then(|v| v.as_str())
        {
            Some(ns) => ns,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: namespace".to_string(),
                    )
                    .await;
            }
        };

        let name = match params_val
            .pointer("/arguments/name")
            .and_then(|v| v.as_str())
        {
            Some(n) => n,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: name".to_string(),
                    )
                    .await;
            }
        };

        let provider = match params_val
            .pointer("/arguments/provider")
            .and_then(|v| v.as_str())
        {
            Some(p) => p,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider".to_string(),
                    )
                    .await;
            }
        };

        let version = params_val
            .pointer("/arguments/version")
            .and_then(|v| v.as_str());

        match self
            .registry_client
            .primary
            .get_module_details(namespace, name, provider, version)
            .await
        {
            Ok(module_details) => {
                let result_json = json!({ "module": module_details });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get module details: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_latest_module_version(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let namespace = match params_val
            .pointer("/arguments/namespace")
            .and_then(|v| v.as_str())
        {
            Some(ns) => ns,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: namespace".to_string(),
                    )
                    .await;
            }
        };

        let name = match params_val
            .pointer("/arguments/name")
            .and_then(|v| v.as_str())
        {
            Some(n) => n,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: name".to_string(),
                    )
                    .await;
            }
        };

        let provider = match params_val
            .pointer("/arguments/provider")
            .and_then(|v| v.as_str())
        {
            Some(p) => p,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider".to_string(),
                    )
                    .await;
            }
        };

        match self
            .registry_client
            .primary
            .get_latest_module_version(namespace, name, provider)
            .await
        {
            Ok(version) => {
                let result_json = json!({
                    "version": version,
                    "module_id": format!("{}/{}/{}", namespace, name, provider)
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get latest module version: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_latest_provider_version(
        &self,
        transport: &StdioTransport,
        id: u64,
        params_val: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let provider_name = match params_val
            .pointer("/arguments/provider_name")
            .and_then(|v| v.as_str())
        {
            Some(name) => name,
            None => {
                return self
                    .send_error_response(
                        transport,
                        id,
                        JsonRpcErrorCode::InvalidParams,
                        "Missing required parameter: provider_name".to_string(),
                    )
                    .await;
            }
        };

        let namespace = params_val
            .pointer("/arguments/namespace")
            .and_then(|v| v.as_str());

        match self
            .registry_client
            .get_provider_version(provider_name, namespace)
            .await
        {
            Ok((version, used_namespace)) => {
                let result_json = json!({
                    "version": version,
                    "namespace": used_namespace,
                    "provider_id": format!("{}/{}", used_namespace, provider_name)
                });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to get latest provider version: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    // Module health analysis handlers
    async fn handle_analyze_module_health(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        logging::info("Handling analyze_module_health request");

        match self.tfmcp.analyze_module_health().await {
            Ok(health) => {
                let result_json = serde_json::to_value(&health)?;
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to analyze module health: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_get_resource_dependency_graph(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        logging::info("Handling get_resource_dependency_graph request");

        match self.tfmcp.get_dependency_graph().await {
            Ok(graph) => {
                let result_json = serde_json::to_value(&graph)?;
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to build dependency graph: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn handle_suggest_module_refactoring(
        &self,
        transport: &StdioTransport,
        id: u64,
    ) -> anyhow::Result<()> {
        logging::info("Handling suggest_module_refactoring request");

        match self.tfmcp.suggest_refactoring().await {
            Ok(suggestions) => {
                let result_json = json!({ "suggestions": suggestions });
                let obj_as_str = serde_json::to_string(&result_json)?;
                self.send_text_response(transport, id, &obj_as_str).await?;
            }
            Err(err) => {
                self.send_error_response(
                    transport,
                    id,
                    JsonRpcErrorCode::InternalError,
                    format!("Failed to generate refactoring suggestions: {}", err),
                )
                .await?;
            }
        }

        Ok(())
    }
}
