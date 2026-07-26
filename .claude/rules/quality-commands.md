# Quality Commands

## Fast development loop

```bash
cargo fmt --all
RUSTFLAGS="-Dwarnings" cargo check --all-targets --all-features
cargo test --locked --all-features
```

## Before commit

```bash
cargo fmt --all -- --check
RUSTFLAGS="-Dwarnings" cargo clippy --all-targets --all-features
cargo test --locked --all-features
git diff --check
```

Do not commit while a required check fails.

## Architecture diagnostics

```bash
cargo coupling --ai --git-months 6 --exclude-tests
cargo coupling --hotspots 10 --deps
similarity-rs src --skip-test --threshold 0.90 --min-lines 8
```

Use these as design signals. Prefer moving behavior to its domain owner,
splitting router/protocol/configuration responsibilities, or introducing one
named mode over creating pass-through abstractions.

## Release gate

Run `./Release.sh vX.Y.Z`. The script additionally enforces:

- `cargo audit`
- coupling grade B or better, no Critical/High/cycles, and at most 40
  dependencies per checked module
- no duplicate functions at 0.90 similarity with at least 8 lines
- valid MCP Registry metadata and synchronized versions
- locked release build, package, and publish dry run

CI tests Terraform 1.15.8 on Linux, macOS, and Windows.
