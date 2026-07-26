---
name: release
description: Prepare, verify, publish, or troubleshoot a tfmcp release to crates.io and GitHub. Use for version changes, release readiness checks, changelog preparation, tagging, or publication.
---

# Release tfmcp

Treat publication as an irreversible operation. Keep the version synchronized
across `Cargo.toml`, `Cargo.lock`, `server.json`, the OCI identifier, and
`Dockerfile`.

## Prepare

1. Confirm the target version does not already exist on crates.io, as a Git
   tag, or as a GitHub release.
2. Update the synchronized version fields and add a dated `CHANGELOG.md`
   section.
3. Update the tested Terraform version in CI and both Docker stages when the
   release changes the supported baseline.
4. Run `./Release.sh vX.Y.Z`. Fix every failure; do not weaken gates to make a
   release pass.
5. Review `cargo package --list` and the generated package contents for secrets,
   temporary files, and stale release documents.

## Publish

1. Commit the verified release preparation on `main`.
2. Push `main` and confirm the Rust CI workflow succeeds for that exact commit.
3. Re-run `./Release.sh vX.Y.Z` from the clean commit.
4. Run `./Release.sh vX.Y.Z --publish`.
5. Verify the crates.io version, annotated Git tag, public GitHub release,
   multi-platform GHCR image, and GitHub Actions results.

Never publish from a dirty worktree, bypass a failing check, reuse a release
version, or move an existing release tag.

## Required gates

The release script is authoritative. It runs formatting, Clippy with warnings
denied, locked tests, audit, `cargo coupling`, `similarity-rs`, metadata
validation, diff checks, release build, package verification, and a crates.io
publish dry run.

Use `cargo coupling --ai --git-months 6 --exclude-tests` to diagnose structural
hotspots and `similarity-rs src --skip-test --threshold 0.90 --min-lines 8` to
inspect duplicate candidates. Refactor findings only when the new boundary or
abstraction has a clear owner and preserves behavior.
