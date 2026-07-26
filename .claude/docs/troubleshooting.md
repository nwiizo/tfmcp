# Troubleshooting

## Reproduce with evidence

1. Set `TFMCP_LOG_LEVEL=debug`.
2. Reproduce with the smallest relevant command or focused test.
3. Check Claude Desktop logs at
   `~/Library/Logs/Claude/mcp-server-tfmcp.log` and audit logs at
   `~/.tfmcp/audit.log`.
4. Run the full release gate after the focused failure is fixed.

## Common checks

```bash
terraform version
cargo test --locked --all-features --test mcp_integration
cargo test --locked --all-features --test e2e_mcp_test
cargo audit
```

- Connection failures: verify the executable/project paths and start with the
  default stdio transport.
- Streamable HTTP 403: verify request `Origin` and `Host` against
  `MCP_ALLOWED_ORIGINS` and `MCP_ALLOWED_HOSTS`.
- Disabled HCP/TFE write: confirm the operation is intentional before setting
  `ENABLE_TF_OPERATIONS=true`.
- Terraform command mismatch: reproduce with Terraform 1.15.8, the CI/Docker
  baseline.

Do not disable Origin/Host checks, safety gates, TLS verification, Clippy, or
audit checks merely to make a failing path pass.
