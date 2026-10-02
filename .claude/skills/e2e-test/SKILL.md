---
name: e2e-test
description: Verify tfmcp MCP lifecycle, transports, and local Terraform operations end to end with real implementations and isolated fixtures.
---

# Verify tfmcp End to End

Select coverage for the requested behavior using the existing tests:

| Test target | Primary coverage |
| --- | --- |
| `e2e_mcp_test` | RMCP lifecycle, discovery, tools/resources, raw protocol, and Streamable HTTP |
| `local_execution` | Real stdio subprocess and local Terraform saved-plan operations |
| `mcp_integration` | Supporting MCP schema and integration checks; inspect conditional shortcuts |

Inspect the selected test and setup before running it. Confirm the Terraform
binary is available and compatible with the
[CI baseline](../../rules/quality-commands.md). Some tests return early when
Terraform is missing or use CI shortcuts; a successful exit alone does not prove
the requested behavior ran.

Before the first run, isolate test processes from the user's home and credentials.
Use `tempfile` or `mktemp -d` for fixture directories; set subprocess home,
Terraform CLI configuration, audit paths, and state paths inside them. Remove
inherited HCP/TFE credentials, `TF_TOKEN_*`, and Terraform CLI argument overrides
from the test process. Inspect in-process fixtures too; a temporary project alone
does not isolate home-directory access. Keep the shell's own home unchanged.

Prefer local `terraform_data` fixtures with no remote backend or cloud access.
Enable apply/destroy gates only for the owned disposable fixture when that
behavior is under test. Real-account verification requires a separate explicit
request. Use real RMCP clients, Terraform, and files; do not introduce mocks.

Once isolation is established, run the relevant target and optional test filter:

```bash
rtk cargo test --locked --all-features --test e2e_mcp_test
rtk cargo test --locked --all-features --test local_execution
```

Assert observable protocol responses, error mapping, redaction, saved-plan/state
effects, and applicable operation gates. Use loopback and disposable ports for
HTTP checks. Stop only processes started by the test and remove only its own
fixtures; retain artifacts when requested.

Report exercised scenarios, skips, failures, and retained paths. Fix failures
caused by an implementation in scope and rerun affected checks. Follow the
repository's required checks for implementation work; a focused smoke test does
not establish full-suite coverage.
