# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

tfmcp is a Rust-based Model Context Protocol (MCP) server that enables AI assistants to interact with Terraform configurations. It bridges the gap between AI and Infrastructure as Code (IaC) by providing secure, controlled access to Terraform operations.

## Essential Commands

### Development Workflow
```bash
# Format code (always run before commits)
cargo fmt --all

# Lint with warnings as errors (same as CI)
RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --all-features

# Run tests with locked dependencies like CI
cargo test --locked --all-features --verbose

# Build the project
cargo build --release --locked --all-features --verbose

# Install from source
cargo install --path .

# Install from crates.io
cargo install tfmcp
```

### Pre-Commit Quality Checks

**MANDATORY: Run these commands before every commit:**

```bash
# 1. Format code
cargo fmt --all

# 2. Check for clippy warnings (with CI-level strictness)
RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --all-features

# 3. Run all tests
cargo test --locked --all-features

# 4. Verify formatting is correct
cargo fmt --all -- --check
```

**If any of these fail, DO NOT COMMIT until fixed.**

### CI/CD Quality Standards

Our CI pipeline enforces strict quality standards:
- `RUSTFLAGS="-Dwarnings"` - All warnings are treated as errors
- `cargo fmt --all -- --check` - Code formatting must be perfect
- `cargo clippy --all-targets --all-features -- -D warnings` - No clippy warnings allowed
- `cargo test --locked --all-features --verbose` - All tests must pass
- Security audit with `cargo audit`
- Cross-platform testing (Ubuntu, Windows, macOS)

## Architecture Overview

### Core Components

1. **Core Module** (`src/core/`): Central tfmcp functionality
   - `tfmcp.rs`: Main application logic and Terraform service orchestration

2. **MCP Module** (`src/mcp/`): Model Context Protocol implementation
   - `handler.rs`: MCP message handling and tool implementations
   - `stdio.rs`: Standard I/O transport layer for Claude Desktop integration

3. **Terraform Module** (`src/terraform/`): Terraform integration layer
   - `service.rs`: Terraform CLI operations (init, plan, apply, destroy, etc.)
   - `model.rs`: Data structures for Terraform responses and analysis
   - `parser.rs`: Parsing Terraform output and configurations

4. **Registry Module** (`src/registry/`): Terraform Registry API integration
   - `client.rs`: HTTP client for Terraform Registry API
   - `provider.rs`: Provider resolution and information retrieval
   - `fallback.rs`: Intelligent namespace fallback (hashicorp→terraform-providers→community)
   - `batch.rs`: High-performance parallel processing
   - `cache.rs`: TTL-based intelligent caching system

5. **Prompts Module** (`src/prompts/`): Enhanced prompt system
   - `builder.rs`: Structured tool descriptions with usage guides
   - `descriptions.rs`: Comprehensive tool documentation and examples

6. **Formatters Module** (`src/formatters/`): Structured output formatting
   - `output.rs`: HashiCorp-style structured results and error messages

7. **Config Module** (`src/config/`): Configuration management
   - Handles project directory resolution, executable paths, and security settings

8. **Shared Module** (`src/shared/`): Common utilities
   - `logging.rs`: Application logging
   - `security.rs`: Security controls and validation
   - `utils/`: Helper functions for path handling

### Key Features

- **Async-First Design**: Uses `tokio` runtime for all I/O operations
- **Intelligent Caching**: TTL-based cache system with 60%+ hit rates
- **Parallel Processing**: Concurrent API calls with controlled concurrency (up to 8 parallel)
- **Fallback Strategy**: Multi-namespace provider resolution with automatic retries
- **Security by Default**: Operations like apply/destroy are disabled unless explicitly enabled
- **Auto-Bootstrap**: Creates sample Terraform projects when none exist

## Configuration

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

### Claude Desktop Integration

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

### Security and Code Quality Standards

**CRITICAL SECURITY RULES:**

1. **NEVER use mock frameworks or mock code:**
   - ❌ **FORBIDDEN**: `mockall`, `mock` libraries, or any mock implementations
   - ❌ **FORBIDDEN**: Mock structs, mock functions, or fake implementations
   - ✅ **ALLOWED**: Real integration tests with temporary files/directories
   - ✅ **ALLOWED**: Testing with actual data structures and real implementations
   
   **Reason**: Mock code can mask security vulnerabilities and create false confidence in tests.

2. **Remove ALL unused code immediately:**
   - ❌ **FORBIDDEN**: Dead code, unused functions, unused structs, unused imports
   - ❌ **FORBIDDEN**: Commented-out code blocks
   - ❌ **FORBIDDEN**: `#[allow(dead_code)]` except for very specific cases
   - ✅ **REQUIRED**: Clean, minimal codebase with only actively used code
   - ✅ **REQUIRED**: Remove unused dependencies from Cargo.toml

3. **Code quality enforcement:**
   - ❌ **FORBIDDEN**: Any warnings in CI (`RUSTFLAGS="-Dwarnings"`)
   - ❌ **FORBIDDEN**: Unused variables, unused imports, unused functions
   - ✅ **REQUIRED**: Prefix test variables with `_` if intentionally unused
   - ✅ **REQUIRED**: Use `#[allow(dead_code)]` only for legitimate infrastructure code

### Code Style
- Follow `rustfmt` formatting (run `cargo fmt --all` before commits)
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
- Registry API integration tests with real API calls (limited by timeouts)
- Performance tests for batch operations and caching
- Use `tempfile` for file system tests
- **NO MOCK FRAMEWORKS** - Use real implementations only

## Docker Operations
```bash
# Build Docker image
docker build -t tfmcp .

# Run in container
docker run -it tfmcp

# Run with mounted Terraform project
docker run -it -v /path/to/terraform:/app/terraform tfmcp --dir /app/terraform
```

## Directory Resolution Priority
1. Command line `--dir` argument
2. `TERRAFORM_DIR` environment variable  
3. Configuration file setting
4. Current working directory
5. Fallback to `~/terraform` (with auto-creation)

## Release Process

1. Update version in `Cargo.toml`
2. Run tests: `cargo test --locked --all-features`
3. Build and test: `cargo build --release --locked --all-features`
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