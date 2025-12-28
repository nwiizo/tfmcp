# Quality Commands

## Development Workflow

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
```

## Pre-Commit Quality Checks

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

## CI/CD Quality Standards

Our CI pipeline enforces strict quality standards:
- `RUSTFLAGS="-Dwarnings"` - All warnings are treated as errors
- `cargo fmt --all -- --check` - Code formatting must be perfect
- `cargo clippy --all-targets --all-features -- -D warnings` - No clippy warnings allowed
- `cargo test --locked --all-features --verbose` - All tests must pass
- Security audit with `cargo audit`
- Cross-platform testing (Ubuntu, Windows, macOS)

## Standard Rust CI workflow

```bash
cargo fmt --all -- --check           # Formatting check
cargo clippy --all-targets -- -D warnings  # Linting with warnings as errors
cargo test --locked --all-features   # Testing with locked dependencies
cargo build --release --locked       # Release build verification
```
