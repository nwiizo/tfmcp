# tfmcp Project Rules

This directory contains the Cursor AI rules for the tfmcp project.

## Structure

- **project.json**: Global settings and rules for the entire project
- **rust.json**: Rules specific to Rust code files
- **terraform.json**: Rules specific to Terraform configuration files

## Usage

These rules are automatically applied by Cursor IDE when working with relevant files.
No manual action is required to enable them.

## Moving from .cursorrules

This project is migrating from the legacy `.cursorrules` file to the new Project Rules system.
The `.cursorrules` file is still maintained for backward compatibility but will eventually be removed.

## Rules Philosophy

1. **Consistency**: Maintain consistent coding style across the project
2. **Clarity**: Promote clear, readable code with proper documentation
3. **Efficiency**: Optimize workflows with helpful commands and automation
4. **Security**: Follow best practices for secure coding and infrastructure

## Custom Commands

Use the Command Palette (Cmd+Shift+P) and type any of the registered commands:

- `tfmcp:release` - Release a new version
- `tfmcp:build` - Build the project
- `tfmcp:test` - Run tests
- `tfmcp:lint` - Run linter
- `tfmcp:run` - Run the MCP server
- `tfmcp:analyze` - Analyze Terraform code 