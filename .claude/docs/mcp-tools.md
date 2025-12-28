# MCP Tools Reference

## Core Terraform Tools (8)

| Tool | Description |
|------|-------------|
| `list_terraform_resources` | List all resources defined in the Terraform project |
| `get_terraform_plan` | Execute 'terraform plan' and return the output |
| `apply_terraform` | Apply Terraform configuration (requires TFMCP_ALLOW_DANGEROUS_OPS) |
| `destroy_terraform` | Destroy all Terraform resources (requires TFMCP_ALLOW_DANGEROUS_OPS) |
| `init_terraform` | Initialize a Terraform project |
| `validate_terraform` | Validate Terraform configuration files |
| `validate_terraform_detailed` | Perform detailed validation with Future Architect guideline checks |
| `get_terraform_state` | Get the current Terraform state |

## Configuration Tools (5)

| Tool | Description |
|------|-------------|
| `set_terraform_directory` | Change the current Terraform project directory |
| `analyze_terraform` | Analyze Terraform configuration with provider version checks |
| `get_security_status` | Get security status with secret detection and compliance score |
| `analyze_module_health` | Analyze module health with variable quality checks |
| `get_resource_dependency_graph` | Get the resource dependency graph |

## Future Architect Guideline Checks

Integrated into existing tools (validate_terraform_detailed, get_security_status, analyze_terraform, analyze_module_health):

- **Type/Description Checks**: Detects variables/outputs missing type or description
- **Provider Version Checks**: Identifies providers without version constraints
- **count vs for_each**: Warns when count should be for_each
- **any Type Usage**: Detects discouraged 'any' type in variables
- **Secret Detection**: Scans for hardcoded AWS keys, API tokens, private keys
- **Lifecycle Protection**: Checks critical resources for prevent_destroy
- **default_tags**: Warns if AWS provider lacks default_tags

## Module Health Analysis Tools

- **`analyze_module_health`**: Health score (0-100), cohesion/coupling, variable quality
- **`get_resource_dependency_graph`**: Resource nodes, dependency edges
- **`suggest_module_refactoring`**: SplitModule, WrapPublicModule, AddDescriptions, FlattenHierarchy

## Registry Tools (7)

| Tool | Description |
|------|-------------|
| `search_terraform_providers` | Search for Terraform providers in the official registry |
| `get_provider_info` | Get detailed information about a specific provider |
| `get_provider_docs` | Get documentation for a specific provider resource |
| `search_terraform_modules` | Search for Terraform modules in the registry |
| `get_module_details` | Get detailed information about a specific module |
| `get_latest_module_version` | Get the latest version of a module |
| `get_latest_provider_version` | Get the latest version of a provider |

## Resources (3)

| URI | Description |
|-----|-------------|
| `terraform://style-guide` | Best practices for HCL formatting and code style |
| `terraform://module-development` | Guide for developing reusable Terraform modules |
| `terraform://best-practices` | Security and operational best practices |
