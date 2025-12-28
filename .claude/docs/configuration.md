# Configuration

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TERRAFORM_DIR` | Override default project directory | - |
| `TFMCP_ALLOW_DANGEROUS_OPS` | Enable apply/destroy operations | `false` |
| `TFMCP_ALLOW_AUTO_APPROVE` | Enable auto-approve for dangerous operations | `false` |
| `TFMCP_LOG_LEVEL` | Control logging verbosity | `info` |
| `TERRAFORM_BINARY_NAME` | Custom Terraform binary name | `terraform` |

## Security Features

- Built-in protection against production file patterns (`prod*`, `production*`, `secret*`)
- Audit logging to `~/.tfmcp/audit.log`
- Resource count limits and access controls
- Dangerous operations disabled by default

## Claude Desktop Integration

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "tfmcp": {
      "command": "/path/to/tfmcp",
      "args": ["mcp"],
      "env": {
        "TERRAFORM_DIR": "/path/to/terraform/project"
      }
    }
  }
}
```

## Directory Resolution Priority

1. Command line `--dir` argument
2. `TERRAFORM_DIR` environment variable
3. Configuration file setting
4. Current working directory
5. Fallback to `~/terraform` (with auto-creation)

## Docker Operations

```bash
# Build Docker image
docker build -t tfmcp .

# Run in container
docker run -it tfmcp

# Run with mounted Terraform project
docker run -it -v /path/to/terraform:/app/terraform tfmcp --dir /app/terraform
```
