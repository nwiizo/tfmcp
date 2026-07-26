# Architecture

## Source boundaries

| Path | Responsibility |
| --- | --- |
| `src/core/` | Application orchestration |
| `src/mcp/server/` | Tool routing, domain dispatch, protocol resources, and analysis rendering |
| `src/mcp/types/` | Typed MCP inputs and generated JSON Schemas |
| `src/mcp/transport.rs` | stdio/Streamable HTTP configuration and Origin/Host controls |
| `src/mcp/http.rs` | HTTP router, health/metrics, rate limits, and TLS serving |
| `src/terraform/` | Terraform CLI operations, parsing, analysis, plan review, and state safety |
| `src/registry/` | Public/private Registry clients, fallback, policy, and provider resolution |
| `src/tfe/` | HCP Terraform/TFE domain types and bounded API client |
| `src/shared/` | Logging, metrics, and security/audit primitives |

Keep configuration parsing, transport wiring, MCP protocol behavior, and domain
operations separate. Put conversions beside the destination domain type rather
than in the tool router.

## Safety boundaries

- stdio and loopback HTTP are the defaults.
- Streamable HTTP validates `Origin` and `Host`.
- HCP/TFE uses server-configured credentials; request tokens are never
  forwarded downstream.
- HCP/TFE writes, local dangerous operations, and auto-approve have separate
  fail-closed gates.
- TFE responses and Terraform resource counts are bounded.

Use `cargo coupling` and `similarity-rs` as architecture diagnostics. The
canonical commands and thresholds live in
[`../rules/quality-commands.md`](../rules/quality-commands.md).
