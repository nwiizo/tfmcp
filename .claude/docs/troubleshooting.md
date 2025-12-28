# Troubleshooting

## Logs and Debugging

- Application logs via `shared/logging.rs` module
- Claude Desktop MCP logs: `~/Library/Logs/Claude/mcp-server-tfmcp.log`
- Security audit logs: `~/.tfmcp/audit.log`
- Use `TFMCP_LOG_LEVEL=debug` for detailed debugging output

## Known Issues and Solutions

### RMCP Server Initialization (Fixed - Dec 2024)

**Problem**: MCP server exiting immediately after initialization.

**Root Cause**: Missing `.waiting()` call after `server.serve(transport)`.

**Solution**: Added `service.waiting().await` after serve() to keep server alive.

**Testing**:
```bash
# Test MCP server manually
echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"protocolVersion":"2025-03-26","capabilities":{}}}' | ./target/release/tfmcp mcp
```

### MCP Connection Issues (Fixed - June 2025)

**Problem**: MCP server initialization hanging during Claude Desktop connection.

**Root Cause**: Race condition in broadcast channel implementation.

**Solution**: Replaced `broadcast::channel` with `mpsc::unbounded_channel`.

**Testing**:
```bash
pkill -f "tfmcp mcp" && pkill -f "Claude" && sleep 3 && open -a Claude
tail -f ~/Library/Logs/Claude/mcp-server-tfmcp.log
```

## CI/CD Pipeline Issues

### GitHub Actions Environment Detection (Fixed - June 2025)

**Problem**: MCP integration tests failing in CI.

**Solution**: Enhanced CI environment detection with multiple GitHub Actions variables.

### Security Audit Compatibility (June 2025)

**Problem**: `cargo-audit` installation failing due to edition2024 requirement.

**Temporary Solution**: Disabled automatic security audit in CI.

**Manual Audit**:
```bash
cargo audit
cargo tree
cargo outdated
```

## CI/CD Best Practices

- Always implement CI environment detection for tests requiring external dependencies
- Provide fallback behavior for missing tools in CI
- Pin versions of CI tools to avoid breaking changes
- Use `--locked` flag for reproducible builds
- Cross-platform testing (Ubuntu, Windows, macOS)
