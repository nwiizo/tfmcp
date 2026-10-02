---
name: analyze
description: Run and interpret cargo-coupling analysis for tfmcp, including coupling health, hotspots, and baseline comparisons.
---

# Analyze tfmcp Coupling

Run the installed `cargo coupling` from the tfmcp root; `cargo run -- coupling`
would run tfmcp itself. Default to `src` and honor the requested scope.

```bash
rtk proxy cargo coupling --ai --git-months 6 --exclude-tests src
```

Reuse a current report when it answers the question. For a short health check use
`--summary`; for ranked candidates use `--hotspots=10 --deps`. Use `--baseline`
or `--history` only when comparison or trends matter. Check local `--help` for
available options rather than forwarding skill names as CLI flags.

Read [signal integrity](../../rules/grading-integrity.md) before interpreting
grades. Inspect the analysis manifest, discovered configuration, and source
behind significant findings. State missing history, unanalysed areas, and any
scope expansion performed by the tool. Do not invent subdomains when no
`.coupling.toml` exists.

Explain the reported grade, supported findings with source locations, and
material limitations. Distinguish tool rankings from recommendations verified in
code. Use the existing [quality commands](../../rules/quality-commands.md) and
`Release.sh` for gate thresholds; do not introduce a separate standard here.
Analysis alone does not authorize refactoring, commits, or publication.
