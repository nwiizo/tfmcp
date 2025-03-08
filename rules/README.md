# tfmcp Project Rules

This directory contains the source files for tfmcp's Cursor Project Rules.

## Structure

The rules are organized into the following categories:

- **general/**: General project guidelines and development workflow
- **rust/**: Rust coding standards and architecture guidelines
- **terraform/**: Terraform standards and best practices
- **mcp/**: MCP protocol implementation guidelines

Each category contains multiple numbered MD files (e.g., `000_init.md`, `001_architecture.md`) that are combined to generate the final MDC files.

## How to Use

1. **Adding new rules**: 
   - Create or modify MD files in the appropriate category directory
   - Follow the numbering convention (e.g., `003_new_rule.md`)
   - Use clear Markdown formatting

2. **Building MDC files**:
   - Run `npm install` (first time only)
   - Run `npm run build:mdc`
   - This will generate the `.cursor/rules/*.mdc` files

3. **Using in Cursor**:
   - Cursor will automatically load the rules based on file type
   - General rules are always applied
   - Other rules are applied based on file globs

## Updating Rules

When you need to update or add new rules:

1. Modify or add MD files in the relevant category
2. Run `npm run build:mdc` to regenerate MDC files
3. Commit both the source MD files and generated MDC files

## Best Practices

- Keep rule files focused and specific
- Use clear, concise language
- Organize rules logically with proper headers
- Ensure the numbering sequence is maintained
- Update rules as the project evolves

## Note

Do not edit the `.cursor/rules/*.mdc` files directly. Always update the source MD files in this directory. 