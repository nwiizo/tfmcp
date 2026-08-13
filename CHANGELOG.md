# Changelog

All notable changes to tfmcp are documented in this file.

## [Unreleased]

### Changed

- Keep routine Rust CI on the shortest useful path by removing duplicate Linux
  builds, non-blocking coverage upload, publish dry-runs, and heuristic
  architecture scans from every push.
- Compile-check Windows and macOS while retaining the complete test suite on
  Linux; release packaging, audit, coupling, and duplicate checks remain in the
  local release gate.
- Run RustSec audits on dependency changes and weekly schedules, and publish
  multi-platform OCI images in a tag-only workflow on native AMD64 and ARM64
  runners.
- Pin third-party GitHub Actions to immutable commit SHAs, minimize token
  permissions, cancel stale CI runs, and enable grouped Dependabot updates.
- Add a repository-managed pre-commit hook for quick, change-aware local checks.

## [0.2.2] - 2026-08-13

v0.2.2 updates tfmcp to RMCP 3.0.1 and aligns its advertised MCP surface with
the stable 2026-07-28 protocol while preserving older client compatibility.

### Added

- MCP 2026-07-28 server discovery and per-request protocol negotiation through
  RMCP's modern lifecycle.
- Structured JSON content for successful JSON-returning tools while retaining
  the existing text content for clients that consume it.
- Five-minute public cache hints on tool, resource, resource-template, and
  resource-read results.

### Changed

- Updated RMCP from 1.8 to 3.0.1 and migrated handler response, resource, and
  Streamable HTTP configuration types to the RMCP 3.x API.
- Serve MCP 2026-07-28 Streamable HTTP requests without protocol-level
  sessions, including when legacy session mode is configured.
- Stop advertising prompt support because tfmcp does not implement prompt
  retrieval; tools and resources remain the supported guidance surfaces.
- Validate MCP 2026-07-28 discovery, result discriminators, cache hints,
  structured tool results, capability metadata, and sessionless HTTP behavior
  in end-to-end tests.
- Reduce the crates.io package by excluding repository-only agent and CI
  assets, and remove duplicate package verification from release automation.

## [0.2.1] - 2026-07-26

v0.2.1 consolidates the completed v0.2.x work into one stable release while
preserving tfmcp's local-first Terraform workflows.

### Added

- Public and private Terraform Registry tools, including the
  `registry-private` toolset and HashiCorp-compatible provider, module, and
  policy aliases.
- Local project inspection, Terraform entrypoint detection, plan review,
  provider lockfile checks, module health analysis, and state safety checks.
- HCP Terraform and Terraform Enterprise read APIs for organizations,
  projects, workspaces, runs, plans, applies, variables, policy sets, variable
  sets, tags, stacks, and private Registry content.
- Explicitly gated HCP/TFE operations, including
  `attach_variable_set_to_workspaces`; operations remain disabled until
  `ENABLE_TF_OPERATIONS=true`.
- Streamable HTTP transport with loopback defaults, health and metrics
  endpoints, stateful/stateless sessions, Host and Origin validation, rate
  limits, TLS file wiring, organization allowlists, and heartbeat control.
- MCP resources including
  `/terraform/providers/{namespace}/name/{name}/version/{version}`.

### Changed

- Updated the tested and containerized Terraform baseline to 1.15.8.
- Updated the declared RMCP SDK baseline to 1.8.
- Set and continuously test the Rust 1.88 MSRV; Docker builds use the same
  locked dependency graph instead of an unpinned nightly toolchain.
- Split MCP protocol/analysis concerns and TFE client operations, configuration,
  encoding, and response handling into narrower modules.
- Consolidated Terraform formatting and refresh wrappers around explicit mode
  enums to remove duplicated control flow.
- Added `cargo coupling` and `similarity-rs` architecture gates to CI and the
  release process.

### Security

- Streamable HTTP validates request origins in every CORS mode and uses
  loopback-only origins by default.
- Removed request-scoped `TFE_TOKEN`, `Authorization`, and `TFE_ADDRESS`
  passthrough. Downstream HCP/TFE calls use only server-configured credentials,
  preventing confused-deputy and token-passthrough behavior.
- HCP/TFE writes, local dangerous operations, and auto-approve remain separate,
  fail-closed gates.
- Organization allowlists reject account-wide and ID-scoped requests whose
  ownership cannot be verified.
- TFE response bodies, HTTP metric routes, and rate-limit session state are
  bounded; sensitive credentials are not returned by status APIs.

## HashiCorp v1.0.x Compatibility

The compatible surface includes `get_plan_json_output`, `get_apply_logs`,
`list_variable_sets`, `attach_variable_set_to_workspaces`,
`read_workspace_tags`, and `list_stacks`. tfmcp also retains capabilities
beyond API mirroring: local Terraform CLI workflows, entrypoint detection,
module health analysis, state safety checks, and local dangerous-operation gates.

[0.2.2]: https://github.com/nwiizo/tfmcp/releases/tag/v0.2.2
[0.2.1]: https://github.com/nwiizo/tfmcp/releases/tag/v0.2.1
