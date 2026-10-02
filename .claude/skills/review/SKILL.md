---
name: review
description: Review tfmcp Rust module boundaries, shared knowledge, and change impact using coupling reports and source evidence.
---

# Review tfmcp Modularity

Review the requested diff or modules against the
[architecture boundaries](../../docs/architecture.md). Reuse current diagnostics
or follow [analyze](../analyze/SKILL.md). For a Rust structural or simplification
review, also use [similarity](../similarity/SKILL.md).

Follow [signal integrity](../../rules/grading-integrity.md). Inspect callers and
shared rules, constants, ordering assumptions, and co-changing files; static
edges alone do not establish a design defect. High fan-out in server wiring or
high fan-in to stable domain types may be appropriate.

Trace consequential findings across configuration, MCP routing and error mapping,
Terraform execution, Registry access, and HCP/TFE operations. Check that moving
responsibilities would preserve input validation, operation gates, downstream
credential ownership, bounded responses, and redaction. Do not infer runtime or
deployment behavior from a static report.

Report supported findings with locations, shared knowledge, likely impact, and a
concrete recommendation. Include material blind spots and checks actually run;
omit unsupported numeric scores and empty report sections. A review request ends
with findings; when fixes are also requested, apply and verify the scoped changes.
