# Rust Architecture Guidelines

## Project Structure
- Follow the modular structure in `src/`:
  - `core/`: Core tfmcp functionality and abstractions
  - `mcp/`: Model Context Protocol implementation
  - `terraform/`: Terraform integration services
  - `config/`: Configuration handling
  - `shared/`: Shared utilities

## Module Organization
- Each module should have a clear, single responsibility.
- Public APIs should be exposed through the module's `mod.rs` or `lib.rs`.
- Keep implementation details private whenever possible.
- Use feature flags for optional functionality.

## Dependencies
- Be conservative with external dependencies.
- Evaluate new dependencies carefully:
  - Is it actively maintained?
  - Is it widely used/trusted?
  - Would it be better to implement the functionality ourselves?
- Pin dependency versions in Cargo.toml for reproducible builds.

## Asynchronous Programming
- Use `async/await` for asynchronous code.
- Use `tokio` for async runtime.
- Be careful with blocking operations in async contexts.
- Consider using channels for communication between components. 