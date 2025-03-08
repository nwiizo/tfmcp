# Terraform Standards

## Version Support
- Target Terraform 1.11.1 as the primary supported version.
- Ensure backward compatibility with Terraform 1.0+ when possible.
- Test with multiple Terraform versions for compatibility.

## Terraform Parsing
- Use the Terraform provided HCL parser when available.
- Implement proper escaping and handling of HCL syntax.
- Handle comments and formatting appropriately.

## Configuration Analysis
- Analyze Terraform configurations thoroughly:
  - Check for resource dependencies
  - Validate provider configurations
  - Identify potential issues or optimizations
- Support various Terraform resources and providers.

## Best Practices
- Follow HCL formatting conventions.
- Respect Terraform state management principles.
- Handle terraform.tfstate files with care.
- Implement proper error handling for Terraform CLI operations.

## Demo Environment
- Maintain example/ directory with good Terraform examples.
- Ensure the demo environment is self-contained and works out of the box.
- Keep example/ code up to date with best practices. 