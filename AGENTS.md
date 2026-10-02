# AGENTS.md

tfmcp is a Rust-based MCP server for Terraform operations using the RMCP SDK.

## Quick Reference

```bash
# Quality checks (run before commits)
rtk cargo fmt --all -- --check
rtk proxy env RUSTFLAGS=-Dwarnings cargo clippy --all-targets --all-features
rtk cargo test --locked --all-features
```

## Project Structure

| Module | Purpose |
|--------|---------|
| `src/core/` | Main application logic |
| `src/mcp/` | RMCP server, tool routing, protocol handling, and transports |
| `src/terraform/` | Terraform CLI integration |
| `src/registry/` | Terraform Registry API client |

## Key Rules

- **No mocks**: Use real implementations only
- **No dead code**: Remove unused code immediately
- **No warnings**: `RUSTFLAGS="-Dwarnings"` in CI
- **No `.unwrap()`**: Use proper error handling

## Documentation

The `.agents/rules`, `.agents/skills`, and `.agents/docs` directories are symlinks
to the shared files under `.claude/`. Keep these links when updating the content.
Apply the development guidelines to implementation work and the signal integrity
rules to structural analysis and review.

| File | Contents |
|------|----------|
| [rules/quality-commands.md](.agents/rules/quality-commands.md) | Build, test, CI commands |
| [rules/development-guidelines.md](.agents/rules/development-guidelines.md) | Code style, security rules |
| [rules/grading-integrity.md](.agents/rules/grading-integrity.md) | Comparable diagnostics and evidence-based interpretation |
| [docs/architecture.md](.agents/docs/architecture.md) | Module structure, features |
| [docs/configuration.md](.agents/docs/configuration.md) | Environment variables, Docker |
| [docs/mcp-tools.md](.agents/docs/mcp-tools.md) | Tool and resource reference |
| [docs/troubleshooting.md](.agents/docs/troubleshooting.md) | Known issues, debugging |
| [skills/release/SKILL.md](.agents/skills/release/SKILL.md) | Release process |

## Development Skills

| Skill | Use |
|-------|-----|
| [analyze](.agents/skills/analyze/SKILL.md) | Coupling reports, health, hotspots, and comparisons |
| [review](.agents/skills/review/SKILL.md) | Module boundaries and shared-knowledge review |
| [refactor](.agents/skills/refactor/SKILL.md) | Behavior-preserving structural changes |
| [similarity](.agents/skills/similarity/SKILL.md) | Duplicate-code assessment |
| [e2e-test](.agents/skills/e2e-test/SKILL.md) | Isolated MCP and Terraform verification |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `TERRAFORM_DIR` | Project directory |
| `TFMCP_ALLOW_DANGEROUS_OPS` | Enable apply/destroy |
| `TFMCP_LOG_LEVEL` | Logging verbosity |
