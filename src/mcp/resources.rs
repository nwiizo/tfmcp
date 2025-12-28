//! MCP Resource content for Terraform guides and best practices.

pub const TERRAFORM_STYLE_GUIDE: &str = r#"# Terraform Style Guide

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

pub const TERRAFORM_MODULE_DEVELOPMENT: &str = r#"# Terraform Module Development Guide

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

pub const TERRAFORM_BEST_PRACTICES: &str = r#"# Terraform Best Practices

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
