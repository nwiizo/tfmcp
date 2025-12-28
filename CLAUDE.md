# CLAUDE.md

tfmcp is a Rust-based MCP server for Terraform operations using the RMCP SDK.

## Quick Reference

```bash
# Quality checks (run before commits)
cargo fmt --all && RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --all-features && cargo test --locked --all-features
```

## Project Structure

| Module | Purpose |
|--------|---------|
| `src/core/` | Main application logic |
| `src/mcp/` | RMCP-based MCP server (21 tools, 3 resources) |
| `src/terraform/` | Terraform CLI integration |
| `src/registry/` | Terraform Registry API client |

## Key Rules

- **No mocks**: Use real implementations only
- **No dead code**: Remove unused code immediately
- **No warnings**: `RUSTFLAGS="-Dwarnings"` in CI
- **No `.unwrap()`**: Use proper error handling

## Documentation

| File | Contents |
|------|----------|
| [rules/quality-commands.md](.claude/rules/quality-commands.md) | Build, test, CI commands |
| [rules/development-guidelines.md](.claude/rules/development-guidelines.md) | Code style, security rules |
| [docs/architecture.md](.claude/docs/architecture.md) | Module structure, features |
| [docs/configuration.md](.claude/docs/configuration.md) | Environment variables, Docker |
| [docs/mcp-tools.md](.claude/docs/mcp-tools.md) | Tool and resource reference |
| [docs/troubleshooting.md](.claude/docs/troubleshooting.md) | Known issues, debugging |
| [skills/release/SKILL.md](.claude/skills/release/SKILL.md) | Release process |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `TERRAFORM_DIR` | Project directory |
| `TFMCP_ALLOW_DANGEROUS_OPS` | Enable apply/destroy |
| `TFMCP_LOG_LEVEL` | Logging verbosity |
