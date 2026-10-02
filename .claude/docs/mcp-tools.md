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

Successful JSON-producing tools return both text JSON for legacy clients and
the same value in `structuredContent`. Tool and resource metadata carries a
five-minute public cache hint.

## Local execution

`prepare_terraform_change` reports native validation, provider requirements,
workspace/backend identity, and absent/unreadable state. `ready` covers these
prerequisites only; required input values are resolved by the actual plan.

`get_terraform_plan` saves a plan and returns an opaque `plan_id`. Use the same ID
for `analyze_plan`, `review_terraform_plan`, `summarize_plan_for_pr`, and
`apply_terraform`. The plan tool also retrieves an existing ID's result/status.
Planning options include `var_files`, `replace`, `refresh_only`, and `destroy`.
Destroy plans cannot combine `refresh_only` or `replace`. Review the saved ID,
then pass it to `destroy_terraform` or `apply_terraform`; both enforce
`TFMCP_DELETE_ENABLED=true` in addition to the apply permissions.

Apply requires the ID, `auto_approve=true`, and both existing local write gates.
It does not replan, refuses changed targets and previously attempted plans, and
returns structured failures with `isError=true`. `failed` records a nonzero exit;
`outcome_unknown` records an unconfirmed exit, including timeout or cancellation.
Both require state inspection and a new reviewed plan rather than retrying the
same ID. Results include `recovery` guidance and are retained in `apply_result`;
cancellation before result capture leaves it null. State verification checks
resource addresses, not attribute values or outputs.

Plans are process-local, with private temporary files and a 64-plan retention
limit. `list_terraform_plans` returns metadata in creation order;
`discard_terraform_plan` deletes one plan and its files to free capacity. Neither
tool changes infrastructure. Discard does not cancel or roll back operations.
Both are available in the default and Terraform toolsets. There is no live
progress or restart recovery.

`terraform_output` redacts sensitive values for both all-output and named-output
queries. State/backend access failures return tool errors instead of an empty
successful output list.

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
