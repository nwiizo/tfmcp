# Release Process Skill

## When to Use

Use this skill when the user asks to:
- Create a new release
- Publish to crates.io
- Update the version

## Process

**Note**: Automated CI release is disabled. Use manual release process:

1. Update version in `Cargo.toml`

2. Run quality checks:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   cargo test --all-features
   ```

3. Build release: `cargo build --release`

4. Commit and push changes

5. Create GitHub release:
   ```bash
   gh release create v0.1.x --title "v0.1.x - Title" --notes "Release notes"
   ```

6. Publish to crates.io: `cargo publish`

## Re-enabling Automated Release

To re-enable automated release in CI, remove `false &&` from `.github/workflows/rust.yml` release job's `if` condition.
