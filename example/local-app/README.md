# Local application verification

This example uses the real `hashicorp/local` provider to generate a deployment
page. The Rust `local_app` example serves that page and a health endpoint on
loopback. No cloud account or remote backend is required.

The opt-in MCP test initializes and validates the configuration, reviews and
applies v1, then reviews v2. It changes the input to v3 before applying the v2
saved plan and verifies that the HTTP page and Terraform output still report v2.
Finally it applies a reviewed destroy plan and verifies a 404 response and empty
state. The app process is stopped after the check.

## Run the complete check

Use Terraform 1.15.8 on PATH, matching the repository's tested baseline. The
provider must be downloadable from the public registry. Run from the repository
root; use a disposable home for all test processes:

```bash
rtk proxy terraform version
tfmcp_demo_home=$(rtk proxy mktemp -d)
tfmcp_cargo_home="${CARGO_HOME:-$HOME/.cargo}"
tfmcp_rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"
tfmcp_app_binary="$PWD/target/debug/examples/local_app"

rtk proxy env -i HOME="$tfmcp_demo_home" CARGO_HOME="$tfmcp_cargo_home" RUSTUP_HOME="$tfmcp_rustup_home" PATH="$PATH" cargo build --locked --example local_app
rtk proxy env -i HOME="$tfmcp_demo_home" CARGO_HOME="$tfmcp_cargo_home" RUSTUP_HOME="$tfmcp_rustup_home" PATH="$PATH" TFMCP_LOCAL_APP_BINARY="$tfmcp_app_binary" cargo test --locked --all-features --test local_execution local_web_app_is_deployed_updated_and_destroyed -- --ignored --nocapture
```

The test prints the retained artifact directory. It contains:

- `report.json`: reviewed plans, results, HTTP checks, and matching audit entries.
- `terraform.log`: Terraform's actual CLI arguments and provider execution.
- `server.log` and `audit.log`: MCP activity and apply/destroy records.
- `preview/index.html`: the verified v2 page, copied before teardown.
- `.terraform.lock.hcl` and `terraform.tfstate`: selected provider version and
  the final empty state.

Audit `command` entries identify the operation and opaque saved plan ID; they
are not shell commands. The report also retains the corresponding actual
Terraform apply arguments. Terraform debug logging is enabled only for this
owned fixture with no secrets; it is not a redacted logging facility.

## View the verified page

After the check, substitute its printed artifact directory:

```bash
rtk proxy cargo run --locked --example local_app -- --directory /path/to/artifacts/preview
```

Open the loopback URL printed by the app. The generated Terraform-managed page
has already been destroyed; this serves the retained preview. Stop the server
with Ctrl-C.
