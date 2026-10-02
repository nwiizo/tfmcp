---
name: refactor
description: Propose or implement behavior-preserving Rust refactors in tfmcp for confirmed coupling or duplication problems.
---

# Refactor tfmcp

Start from the user's finding or a current [analysis](../analyze/SKILL.md).
Confirm the responsibility and its callers before selecting a change. Follow
[development guidelines](../../rules/development-guidelines.md) and
[signal integrity](../../rules/grading-integrity.md).

Run `cargo coupling` and `similarity-rs` before changing Rust structure, using the
[repository commands](../../rules/quality-commands.md). Reuse current results.
Prefer moving behavior to its existing domain owner, a named operation mode, or
a conversion beside its destination type. Extract shared policy only when the
responsibilities agree; do not add pass-through layers to improve a metric.

Preserve MCP tool names and schemas, protocol versus tool error distinctions,
saved-plan and state safety, operation gates, credential boundaries, and output
redaction. Verify callers of shared code, including intentional differences.

For a proposal, explain the problem, recommended change, and verification plan.
For implementation, carry out the authorized change and verify affected behavior
with real implementations. Follow [E2E guidance](../e2e-test/SKILL.md) when the
change crosses a transport or Terraform execution boundary.

Compare diagnostics with the same settings and history. Run relevant tests and
the required repository checks, including the full release gate required by the
development guidelines for refactoring. Report actual results and limitations.
Commit or push only when requested.
