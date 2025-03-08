use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let git_hash = match output {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .expect("Failed to convert Git commit hash to string.")
                    .trim()
                    .to_string()
            } else {
                "unknown".to_string()
            }
        }
        Err(_) => "unknown".to_string(),
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
