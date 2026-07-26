# MCP surface

Do not maintain a hand-counted tool list here. The RMCP tool router and
`tests/e2e_mcp_test.rs` are the source of truth.

## Toolsets

| Toolset | Purpose |
| --- | --- |
| `default` | Safe local analysis and read-oriented Registry/Terraform tools |
| `terraform` | Terraform CLI and project workflows |
| `analysis` | Configuration, plan, module-health, and state-safety analysis |
| `registry` | Public Terraform Registry |
| `registry-private` | HCP/TFE private Registry |
| `tfe` | HCP Terraform/TFE reads |
| `operations` | Explicitly gated HCP/TFE writes |
| `all` | Every registered tool, still subject to runtime safety gates |

Use `tfmcp mcp --toolsets ...` for categories and `--tools ...` for an explicit
allowlist. Unknown toolsets fail closed.

## MCP resources

- `terraform://style-guide` and `/terraform/style-guide`
- `terraform://module-development` and `/terraform/module-development`
- `terraform://best-practices`
- `terraform://providers/{namespace}/{name}/{version}/docs`
- `/terraform/providers/{namespace}/name/{name}/version/{version}`

When adding or renaming a tool:

1. Define a typed `schemars` input.
2. Set accurate read-only/destructive/idempotent annotations.
3. Return input/domain failures as tool execution errors.
4. Add tool-filter and end-to-end protocol coverage.
5. Update README only when the user-facing capability changes.
