# Development Guidelines

## Security and Code Quality Standards

### Critical security rules

1. **Do not use mock frameworks or mock implementations:**
   - Use real integration tests with temporary files/directories.
   - Test with actual data structures and protocol messages.

   **Reason**: Mock code can mask security vulnerabilities and create false confidence in tests.

2. **Remove unused code immediately:**
   - Remove dead code, commented-out code, and unused dependencies.
   - Use `#[allow(dead_code)]` only when a documented infrastructure boundary
     requires it.

3. **Keep CI warning-free:**
   - Run Clippy for all targets and features with warnings denied.
   - Fix lints locally; do not add broad `allow` attributes.

4. **Keep MCP trust boundaries explicit:**
   - Validate tool inputs and return domain/input failures as tool execution
     errors; reserve protocol errors for malformed or unknown MCP requests.
   - Validate Streamable HTTP `Origin` and `Host`; bind to loopback by default.
   - Never accept a client token and pass it through to HCP/TFE. Use only
     server-configured downstream credentials.
   - Keep write and destructive tools disabled by default, scope credentials,
     bound response sizes, and sanitize tool output and logs.

## Code Style

- Follow `rustfmt` formatting (run `cargo fmt --all` before commits)
- Use `Result`/`Option` types appropriately
- Document public APIs with rustdoc comments
- Prefer immutable variables when possible
- Use typed request/response structures at MCP and TFE boundaries
- Keep configuration parsing, transport wiring, protocol dispatch, and domain
  operations in separate modules

## Error Handling

- Use `anyhow` at application boundaries
- Use `thiserror` for library/domain error types
- Propagate errors with `?` operator
- Avoid `.unwrap()` and `.expect()` in production paths
- Include actionable context without exposing tokens, paths, or sensitive
  Terraform values

## Testing Strategy

- Unit tests for individual modules
- Integration tests for Terraform service operations
- Registry API integration tests with real API calls (limited by timeouts)
- Use `tempfile` for file system tests
- Add protocol integration tests when MCP schemas, annotations, error mapping,
  or transport behavior changes
- Add Terraform 1.15.8 coverage when CLI commands or output parsing changes
- Use real implementations rather than mock frameworks

## Refactoring

- Run `cargo coupling` before and after structural work; reduce High findings
  without hiding stable shared types behind artificial traits.
- Run `similarity-rs` before extracting helpers. Extract duplicated policy or
  control flow; keep thin compatibility aliases when they preserve the public
  MCP surface.
- Prefer enums for a closed set of operation modes and `From` conversions at
  type boundaries.
- Keep changes behavior-preserving and verify with focused tests plus the full
  release gate.
