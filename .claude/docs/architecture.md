# Architecture Overview

## Core Components

1. **Core Module** (`src/core/`)
   - `tfmcp.rs`: Main application logic and Terraform service orchestration

2. **MCP Module** (`src/mcp/`)
   - `server.rs`: RMCP-based MCP server with ServerHandler implementation
   - `types.rs`: Input types with schemars for JSON Schema generation
   - `resources.rs`: Terraform guides and best practices content

3. **Terraform Module** (`src/terraform/`)
   - `service.rs`: Terraform CLI operations (init, plan, apply, destroy, etc.)
   - `model.rs`: Data structures for Terraform responses and analysis
   - `parser.rs`: Parsing Terraform output and configurations
   - `analyzer.rs`: Module health analysis with cohesion/coupling metrics

4. **Registry Module** (`src/registry/`)
   - `client.rs`: HTTP client for Terraform Registry API
   - `provider.rs`: Provider resolution and information retrieval
   - `fallback.rs`: Intelligent namespace fallback (hashicorp→terraform-providers→community)
   - `batch.rs`: High-performance parallel processing
   - `cache.rs`: TTL-based intelligent caching system

5. **Prompts Module** (`src/prompts/`)
   - `builder.rs`: Structured tool descriptions with usage guides
   - `descriptions.rs`: Comprehensive tool documentation and examples

6. **Formatters Module** (`src/formatters/`)
   - `output.rs`: HashiCorp-style structured results and error messages

7. **Config Module** (`src/config/`)
   - Handles project directory resolution, executable paths, and security settings

8. **Shared Module** (`src/shared/`)
   - `logging.rs`: Application logging
   - `security.rs`: Security controls and validation
   - `utils/`: Helper functions for path handling

## Key Features

- **Async-First Design**: Uses `tokio` runtime for all I/O operations
- **Intelligent Caching**: TTL-based cache system with 60%+ hit rates
- **Parallel Processing**: Concurrent API calls with controlled concurrency (up to 8 parallel)
- **Fallback Strategy**: Multi-namespace provider resolution with automatic retries
- **Security by Default**: Operations like apply/destroy are disabled unless explicitly enabled
- **Auto-Bootstrap**: Creates sample Terraform projects when none exist
- **Module Health Analysis**: Whitebox IaC approach with cohesion/coupling metrics
- **Resource Dependency Graph**: Visualization of resource relationships
- **RMCP SDK**: Uses official rust-sdk for MCP protocol implementation
