# Configuration

README is the user-facing environment-variable reference. Keep this file
focused on agent-relevant defaults and trust boundaries.

| Area | Variables | Default |
| --- | --- | --- |
| Project | `TERRAFORM_DIR`, `TERRAFORM_BINARY_NAME` | current/configured project, `terraform` |
| Local safety | `TFMCP_ALLOW_DANGEROUS_OPS`, `TFMCP_ALLOW_AUTO_APPROVE` | `false`, `false` |
| HCP/TFE | `TFE_ADDRESS`, `TFE_TOKEN`, `TFE_MAX_RESPONSE_BYTES` | HCP Terraform, unset, `65536` |
| HCP/TFE writes | `ENABLE_TF_OPERATIONS` | `false` |
| Transport | `TRANSPORT_MODE`, `TRANSPORT_HOST`, `TRANSPORT_PORT` | `stdio`, `127.0.0.1`, `8080` |
| Browser/HTTP | `MCP_ALLOWED_ORIGINS`, `MCP_ALLOWED_HOSTS` | loopback-only |
| Deployment | `MCP_ORGANIZATION_ALLOWLIST`, rate-limit and TLS variables | unset |

`MCP_CORS_MODE` controls response CORS headers. MCP request Origin validation
remains enabled in every mode. Do not add request-scoped `TFE_TOKEN`,
`Authorization`, or `TFE_ADDRESS` passthrough. With an organization allowlist,
reject account-wide and ID-only TFE requests whose owner cannot be verified.

Terraform 1.15.8 is the CI and Docker baseline.

## Claude Desktop

```json
{
  "mcpServers": {
    "tfmcp": {
      "command": "/absolute/path/to/tfmcp",
      "args": ["mcp"],
      "env": {
        "TERRAFORM_DIR": "/absolute/path/to/terraform/project"
      }
    }
  }
}
```

Use absolute paths. Keep secrets in the server process environment rather than
MCP request arguments or committed configuration.
