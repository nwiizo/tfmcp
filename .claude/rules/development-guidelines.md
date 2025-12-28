# Development Guidelines

## Security and Code Quality Standards

### CRITICAL SECURITY RULES

1. **NEVER use mock frameworks or mock code:**
   - FORBIDDEN: `mockall`, `mock` libraries, or any mock implementations
   - FORBIDDEN: Mock structs, mock functions, or fake implementations
   - ALLOWED: Real integration tests with temporary files/directories
   - ALLOWED: Testing with actual data structures and real implementations

   **Reason**: Mock code can mask security vulnerabilities and create false confidence in tests.

2. **Remove ALL unused code immediately:**
   - FORBIDDEN: Dead code, unused functions, unused structs, unused imports
   - FORBIDDEN: Commented-out code blocks
   - FORBIDDEN: `#[allow(dead_code)]` except for very specific cases
   - REQUIRED: Clean, minimal codebase with only actively used code
   - REQUIRED: Remove unused dependencies from Cargo.toml

3. **Code quality enforcement:**
   - FORBIDDEN: Any warnings in CI (`RUSTFLAGS="-Dwarnings"`)
   - FORBIDDEN: Unused variables, unused imports, unused functions
   - REQUIRED: Prefix test variables with `_` if intentionally unused
   - REQUIRED: Use `#[allow(dead_code)]` only for legitimate infrastructure code

## Code Style

- Follow `rustfmt` formatting (run `cargo fmt --all` before commits)
- Maximum line length: 100 characters
- Use `Result`/`Option` types appropriately
- Document public APIs with rustdoc comments
- Prefer immutable variables when possible

## Error Handling

- Use `anyhow` for application-level error propagation
- Use `thiserror` for custom error types
- Propagate errors with `?` operator
- Avoid `.unwrap()` in production code

## Testing Strategy

- Unit tests for individual modules
- Integration tests for Terraform service operations
- Registry API integration tests with real API calls (limited by timeouts)
- Performance tests for batch operations and caching
- Use `tempfile` for file system tests
- **NO MOCK FRAMEWORKS** - Use real implementations only
