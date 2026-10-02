---
name: similarity
description: Inspect tfmcp Rust duplicate-code candidates with similarity-rs and judge whether sharing their responsibilities would improve maintenance.
---

# Inspect Rust Similarity

Use the requested scope, defaulting to `src`, and start with the repository's
configured diagnostic settings:

```bash
rtk proxy similarity-rs src --skip-test --threshold 0.90 --min-lines 8
```

Honor explicit analysis options and check `similarity-rs --help` for other modes.
Use `--print` when source excerpts help; avoid threshold sweeps without a specific
unresolved question. Exploratory thresholds do not change the release gate.

Inspect candidate functions and callers. Share code when it represents the same
policy or control flow with the same owner. Preserve thin compatibility aliases,
protocol-specific behavior, and distinct security checks even when syntax looks
similar. A high percentage alone does not justify traits, generics, or extraction.

For a structural change, inspect [coupling](../analyze/SKILL.md) as well and follow
[refactor](../refactor/SKILL.md). Report locations, the shared responsibility if
any, and a recommendation, including intentional duplication. Implement changes
only within the requested scope and verify the affected behavior.
