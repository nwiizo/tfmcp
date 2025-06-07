# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

tfmcp is a Rust-based Model Context Protocol (MCP) server that enables AI assistants to interact with Terraform configurations. It bridges the gap between AI and Infrastructure as Code (IaC) by providing secure, controlled access to Terraform operations.

## Essential Commands

### Development Workflow
```bash
# Format code (always run before commits)
cargo fmt

# Lint with warnings as errors
cargo clippy -- -D warnings

# Run tests
cargo test

# Build the project
cargo build --release

# Install from source
cargo install --path .

# Install from crates.io
cargo install tfmcp
```

### Docker Operations
```bash
# Build Docker image
docker build -t tfmcp .

# Run in container
docker run -it tfmcp

# Run with mounted Terraform project
docker run -it -v /path/to/terraform:/app/terraform tfmcp --dir /app/terraform
```

### Project-Specific Scripts
```bash
# Build Cursor IDE rules (when editing documentation)
npm install  # first time only
npm run build:mdc
```

## Architecture Overview

### Core Components

1. **Core Module** (`src/core/`): Central tfmcp functionality
   - `tfmcp.rs`: Main application logic, project initialization, and Terraform service orchestration

2. **MCP Module** (`src/mcp/`): Model Context Protocol implementation
   - `handler.rs`: MCP message handling and tool implementations
   - `stdio.rs`: Standard I/O transport layer for Claude Desktop integration

3. **Terraform Module** (`src/terraform/`): Terraform integration layer
   - `service.rs`: Terraform CLI operations (init, plan, apply, destroy, etc.)
   - `model.rs`: Data structures for Terraform responses and analysis
   - `parser.rs`: Parsing Terraform output and configurations

4. **Registry Module** (`src/registry/`): **NEW** - Terraform Registry API integration
   - `client.rs`: HTTP client for Terraform Registry API
   - `provider.rs`: Staged information retrieval and provider resolution
   - `fallback.rs`: Intelligent namespace fallback (hashicorp→terraform-providers→community)
   - `batch.rs`: High-performance parallel processing
   - `cache.rs`: TTL-based intelligent caching system

5. **Prompts Module** (`src/prompts/`): **NEW** - Enhanced prompt system
   - `builder.rs`: Structured tool descriptions with usage guides
   - `descriptions.rs`: Comprehensive tool documentation and examples

6. **Formatters Module** (`src/formatters/`): **NEW** - Structured output formatting
   - `output.rs`: HashiCorp-style structured results and error messages

7. **Config Module** (`src/config/`): Configuration management
   - Handles project directory resolution, executable paths, and security settings

8. **Shared Module** (`src/shared/`): Common utilities
   - `logging.rs`: Application logging
   - `security.rs`: Security controls and validation
   - `utils/`: Helper functions for path handling

### Key Architectural Patterns

- **Async-First Design**: Uses `tokio` runtime for all I/O operations
- **Staged Information Retrieval**: HashiCorp-style ID resolution → detailed fetching
- **Intelligent Caching**: TTL-based cache system with 60%+ hit rates
- **Parallel Processing**: Concurrent API calls with controlled concurrency (up to 8 parallel)
- **Fallback Strategy**: Multi-namespace provider resolution with automatic retries
- **Error Propagation**: Comprehensive error handling with `anyhow` and `thiserror`
- **Security by Default**: Operations like apply/destroy are disabled unless explicitly enabled
- **Auto-Bootstrap**: Creates sample Terraform projects when none exist

### Performance Features

- **Registry API Integration**: Direct integration with Terraform Registry for provider information
- **Batch Operations**: High-performance parallel fetching of multiple providers/versions
- **Smart Caching**: Individual caches for providers (10min), documentation (30min), versions (5min)
- **Namespace Fallback**: Automatic search across hashicorp → terraform-providers → community
- **Structured Output**: HashiCorp-quality formatted results with usage examples

## Important Configuration

### Environment Variables
- `TERRAFORM_DIR`: Override default project directory
- `TFMCP_ALLOW_DANGEROUS_OPS`: Enable apply/destroy operations (default: false)
- `TFMCP_ALLOW_AUTO_APPROVE`: Enable auto-approve for dangerous operations (default: false)
- `TFMCP_LOG_LEVEL`: Control logging verbosity (debug, info, warn, error)
- `TERRAFORM_BINARY_NAME`: Custom Terraform binary name (default: "terraform")

### Security Features
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

## Development Guidelines

### Code Style (from rules/rust/)
- Follow `rustfmt` formatting (run `cargo fmt` before commits)
- Maximum line length: 100 characters
- Use `Result`/`Option` types appropriately
- Document public APIs with rustdoc comments
- Prefer immutable variables when possible

### Error Handling
- Use `anyhow` for application-level error propagation
- Use `thiserror` for custom error types
- Propagate errors with `?` operator
- Avoid `.unwrap()` in production code

### Testing Strategy
- Unit tests for individual modules
- Integration tests for Terraform service operations
- Registry API integration tests with mock responses
- Performance tests for batch operations and caching
- Use `tempfile` for file system tests
- Use `mockall` for mocking external dependencies

### New Module Guidelines

**Registry Module Development:**
- Always implement caching for external API calls
- Use batch operations for multiple related requests
- Implement intelligent fallback for provider resolution
- Structure responses using `OutputFormatter` for consistency

**Prompt System Development:**
- Use `ToolDescription` builder for all new tools
- Include usage guides, constraints, and security notes
- Provide practical examples with expected outputs
- Follow HashiCorp documentation standards

**Performance Considerations:**
- Leverage `BatchFetcher` for multiple API calls
- Configure appropriate cache TTLs based on data volatility
- Use structured logging for performance monitoring
- Implement rate limiting respect for external APIs

## Project Structure Logic

### Directory Resolution Priority
1. Command line `--dir` argument
2. `TERRAFORM_DIR` environment variable  
3. Configuration file setting
4. Current working directory
5. Fallback to `~/terraform` (with auto-creation)

### Automatic Project Bootstrap
When no `.tf` files are found, tfmcp automatically creates a sample project with:
- `main.tf`: Basic local provider configuration
- `example.txt`: Sample resource output

This ensures the MCP server can always start and provide a working environment for AI assistants.

## Release Process

1. Update version in `Cargo.toml`
2. Run tests: `cargo test`
3. Build and test: `cargo build --release`
4. Create release with `./Release.sh`
5. Publish to crates.io: `cargo publish`

## Logs and Debugging

- Application logs via `shared/logging.rs` module
- Claude Desktop MCP logs: `~/Library/Logs/Claude/mcp-server-tfmcp.log`
- Security audit logs: `~/.tfmcp/audit.log`

Use `TFMCP_LOG_LEVEL=debug` for detailed debugging output.

## Known Issues and Solutions

### MCP Connection Issues (Fixed - June 7, 2025)

**Problem**: MCP server initialization was hanging during the Claude Desktop connection process.

**Root Cause**: Race condition in the broadcast channel implementation where the initialize message was sent before the stream receiver was created, causing message loss.

**Solution**: Replaced `broadcast::channel` with `mpsc::unbounded_channel` in `src/mcp/stdio.rs` to ensure message buffering and reliable delivery.

**Files Modified**:
- `src/mcp/stdio.rs`: Switched from broadcast to mpsc channel, added debug logging
- `src/mcp/handler.rs`: Added timing adjustments for receiver initialization
- `src/core/tfmcp.rs`: Updated sender type annotations

**Testing Commands**:
```bash
# Kill existing processes and restart Claude Desktop
pkill -f "tfmcp mcp" && pkill -f "Claude" && sleep 3 && open -a Claude

# Monitor MCP logs in real-time
tail -f "/Users/nwiizo/Library/Logs/Claude/mcp-server-tfmcp.log"

# Test MCP server manually
echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"capabilities":{}}}' | ./target/release/tfmcp mcp
```

**Debug Output Sequence** (when working correctly):
1. `[DEBUG] Received JSON: {"method":"initialize"...`
2. `[DEBUG] Successfully sent message to channel`
3. `[DEBUG] Stream received message successfully`
4. `[debug] Stream received a message, processing...`
5. `[info] Handling initialize request`
6. `[info] Initialize response sent successfully`

**Prevention**: Added comprehensive debug logging and tests to catch similar channel issues in the future.