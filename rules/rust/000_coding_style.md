# Rust Coding Style Guide

## Code Style
- Follow the Rust style guide as enforced by `rustfmt` and `cargo fmt`.
- Use 4 spaces for indentation, not tabs.
- Maximum line length is 100 characters.
- Always run `cargo fmt` before committing code.

## Error Handling
- Use `Result` and `Option` types appropriately.
- Propagate errors with the `?` operator where appropriate.
- Create custom error types in modules with complex error handling.
- Use `thiserror` for defining error types.
- Use `anyhow` for error propagation in application code.

## Documentation
- Document all public functions, methods, and types with rustdoc comments.
- Include examples in documentation when useful.
- Document complex or non-obvious code sections.

## Best Practices
- Prefer immutable variables (`let` instead of `let mut`) when possible.
- Use strong typing rather than type aliases for clarity.
- Leverage Rust's ownership system properly.
- Avoid `unsafe` code unless absolutely necessary.
- Use `clippy` to catch common mistakes. 