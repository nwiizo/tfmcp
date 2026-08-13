#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  ./Release.sh v0.2.1 [--publish]

Default mode runs the full local release gate and crates.io dry-run only.
Use --publish only after the worktree is clean, CI has passed, and the release
changelog has been reviewed.
USAGE
}

VERSION="${1:-}"
MODE="${2:---dry-run}"

if [[ -z "$VERSION" || "$VERSION" == "-h" || "$VERSION" == "--help" ]]; then
    usage
    exit 1
fi

if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: version must look like v0.2.1"
    exit 1
fi

if [[ "$MODE" != "--dry-run" && "$MODE" != "--publish" ]]; then
    echo "error: mode must be --dry-run or --publish"
    usage
    exit 1
fi

SEMVER="${VERSION#v}"
CHANGELOG="CHANGELOG.md"

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "error: missing required command '$1'"
        exit 1
    fi
}

run_step() {
    echo
    echo "==> $*"
    "$@"
}

require_clean_worktree() {
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "error: worktree must be clean for --publish"
        git status --short
        exit 1
    fi
}

metadata_version() {
    jq -r '.version' server.json
}

metadata_image() {
    jq -r '.packages[0].identifier' server.json
}

dockerfile_version() {
    awk -F'"' '/^ARG TFMCP_VERSION=/ { print $2; exit }' Dockerfile
}

check_release_metadata() {
    local cargo_version
    local lock_version
    local server_version
    local docker_version
    local image

    cargo_version="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "tfmcp") | .version')"
    lock_version="$(awk '
        $0 == "name = \"tfmcp\"" { in_pkg = 1; next }
        in_pkg && /^version = / {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' Cargo.lock)"
    server_version="$(metadata_version)"
    docker_version="$(dockerfile_version)"
    image="$(metadata_image)"

    if [[ "$cargo_version" != "$SEMVER" ]]; then
        echo "error: Cargo.toml version is $cargo_version, expected $SEMVER"
        exit 1
    fi
    if [[ "$lock_version" != "$SEMVER" ]]; then
        echo "error: Cargo.lock tfmcp version is $lock_version, expected $SEMVER"
        exit 1
    fi
    if [[ "$server_version" != "$SEMVER" ]]; then
        echo "error: server.json version is $server_version, expected $SEMVER"
        exit 1
    fi
    if [[ "$docker_version" != "$SEMVER" ]]; then
        echo "error: Dockerfile TFMCP_VERSION is $docker_version, expected $SEMVER"
        exit 1
    fi
    if [[ "$image" != "ghcr.io/nwiizo/tfmcp:${SEMVER}" ]]; then
        echo "error: server.json OCI image is $image, expected ghcr.io/nwiizo/tfmcp:${SEMVER}"
        exit 1
    fi
    if [[ ! -f "$CHANGELOG" ]]; then
        echo "error: missing changelog: $CHANGELOG"
        exit 1
    fi
    if ! grep -Fq "## [${SEMVER}]" "$CHANGELOG"; then
        echo "error: $CHANGELOG does not contain a ${SEMVER} release section"
        exit 1
    fi
}

check_main_branch_for_publish() {
    local branch
    branch="$(git branch --show-current)"
    if [[ "$branch" != "main" ]]; then
        echo "error: --publish must run from main, current branch is '$branch'"
        exit 1
    fi
}

verify_published_main_ci() {
    local head
    local origin_head
    local successful

    run_step git fetch origin main
    head="$(git rev-parse HEAD)"
    origin_head="$(git rev-parse origin/main)"
    if [[ "$head" != "$origin_head" ]]; then
        echo "error: HEAD $head does not match origin/main $origin_head"
        exit 1
    fi

    successful="$(
        gh run list \
            --workflow rust.yml \
            --branch main \
            --commit "$head" \
            --event push \
            --limit 20 \
            --json conclusion \
            --jq 'any(.conclusion == "success")'
    )"
    if [[ "$successful" != "true" ]]; then
        echo "error: no successful Rust CI push run found for $head on main"
        exit 1
    fi
}

ensure_tag_absent() {
    if git show-ref --verify --quiet "refs/tags/$VERSION"; then
        echo "error: local tag $VERSION already exists"
        exit 1
    fi
    if git ls-remote --exit-code --tags origin "refs/tags/$VERSION" >/dev/null 2>&1; then
        echo "error: remote tag $VERSION already exists"
        exit 1
    fi
}

create_release_tag() {
    git tag -a "$VERSION" -m "Release $VERSION"
}

create_github_release() {
    gh release create "$VERSION" \
        --title "tfmcp $VERSION" \
        --notes-file "$CHANGELOG" \
        --verify-tag
}

need_cmd cargo
need_cmd git
need_cmd jq
need_cmd cargo-audit
need_cmd cargo-coupling
need_cmd similarity-rs

echo "Release gate for tfmcp ${VERSION} (${MODE})"

check_release_metadata
if [[ "$MODE" == "--publish" ]]; then
    need_cmd gh
    require_clean_worktree
    check_main_branch_for_publish
    verify_published_main_ci
    ensure_tag_absent
fi

run_step cargo fmt --all -- --check
run_step env RUSTFLAGS=-Dwarnings cargo clippy --all-targets --all-features
run_step cargo test --locked --all-features
run_step cargo audit
run_step cargo coupling --check --min-grade B --max-critical 0 --max-circular 0 --fail-on high --max-deps 40
run_step similarity-rs src --skip-test --threshold 0.90 --min-lines 8 --fail-on-duplicates
run_step jq . server.json
run_step git diff --check
run_step cargo build --release --locked --all-features
run_step cargo package --list --locked --allow-dirty
run_step cargo publish --dry-run --locked --allow-dirty

if [[ "$MODE" == "--publish" ]]; then
    create_release_tag
    run_step git push origin "$VERSION"
    # The dry-run immediately above verifies this exact clean worktree. Avoid
    # compiling the generated package a second time before uploading it.
    run_step cargo publish --locked --no-verify
    create_github_release
    echo
    echo "Release $VERSION published to crates.io and GitHub."
    echo "The tag-triggered OCI workflow publishes the signed multi-platform GHCR image."
else
    echo
    echo "Dry-run release gate passed for $VERSION."
    echo "Run './Release.sh $VERSION --publish' from a clean main checkout to publish."
fi
