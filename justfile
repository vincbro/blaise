set shell := ["bash", "-uc"]

# rustfmt's import options (code-standards/rust/STYLE.md S-IMPORTS) are nightly-only, so formatting
# runs on this pinned nightly. Bump it like the toolchain: in its own change.
fmt_toolchain := "nightly-2026-10-13"

# List available recipes
default:
    @just --list

# Build the crate
build:
    cargo build

# Format every Rust and TOML file, and sort Cargo.toml dependency tables
fmt:
    cargo +{{ fmt_toolchain }} fmt --all
    cargo sort --no-format
    taplo fmt

# Apply clippy's automatic fixes, then format
fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets
    cargo +{{ fmt_toolchain }} fmt --all

# The CI gate: run it before you push
check: fmt-check clippy doc test deps

# The CI gate with every property test run for 40000 cases instead of 256, which takes
# a few minutes: run it before a release and after touching a stateful type
check-full: fmt-check clippy doc test-full deps

# Fail on any file `just fmt` would change
fmt-check:
    cargo +{{ fmt_toolchain }} fmt --all -- --check
    cargo sort --no-format --check
    taplo fmt --check

# Fail on any lint
clippy:
    cargo clippy --all-targets --all-features --locked -- -D warnings

# Build the docs; any rustdoc warning fails
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked

# Run every test
test:
    cargo test --locked

# Run every test, the property tests for 40000 cases each
test-full:
    PROPTEST_CASES=40000 cargo test --locked

# Time a route query forward and backward on the network in bench.gtfs; criterion compares with
# the last run. Extra args go to criterion: just bench [args]
bench *args:
    cargo bench --bench route -- {{ args }}

# Fail on unused dependencies, or on any cargo-deny check in deny.toml:
# advisories, licenses, sources
deps:
    cargo machete
    cargo deny --locked check

# Install the pinned toolchain from rust-toolchain.toml and the nightly rustfmt
# `just fmt` uses, and off CI the cargo tools other recipes call (CI installs
# those prebuilt) and the git hooks
toolchain:
    rustup toolchain install
    rustup toolchain install {{ fmt_toolchain }} --profile minimal --component rustfmt
    @[ -n "${CI:-}" ] || just _tools hooks

# Use the hooks in .githooks: commit-msg checks code-standards/COMMITS.md S-COMMITS
hooks:
    git config core.hooksPath .githooks

# Fail on a commit message that breaks code-standards/COMMITS.md S-COMMITS: just commit-msg <file>
commit-msg file:
    #!/usr/bin/env bash
    set -euo pipefail
    # Comments and the diff `git commit -v` appends aren't part of the message
    message=$(sed '/^# ------------------------ >8 ------------------------$/,$d' {{ quote(file) }} | grep -v '^#' || true)
    subject=$(printf '%s\n' "$message" | head -n 1)
    fail() { printf 'commit message: %s\n  %s\nSee code-standards/COMMITS.md S-COMMITS.\n' "$1" "$subject" >&2; exit 1; }
    # Merges and fixups are rewritten or generated, not written
    [[ "$subject" =~ ^(Merge|fixup!|squash!|amend!) ]] && exit 0
    [[ "$subject" =~ ^[a-z0-9][a-z0-9_/-]*(,\ [a-z0-9][a-z0-9_/-]*)*:\ [^\ ] ]] || fail "first line must be 'area: summary', area lowercase"
    summary=${subject#*: }
    [[ "$summary" =~ ^[a-z0-9\`] ]] || fail "summary must start lowercase (or with \`code\`)"
    [[ "$subject" != *. ]] || fail "first line must not end with a period"
    (( ${#subject} <= 72 )) || fail "first line is ${#subject} characters, at most 72"
    second=$(printf '%s\n' "$message" | sed -n 2p)
    [[ -z "$second" ]] || fail "second line must be blank"

# Check every commit since `base` against code-standards/COMMITS.md S-COMMITS: just commits [base]
commits base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    status=0
    for commit in $(git rev-list --no-merges {{ quote(base) }}..HEAD); do
        git log -1 --format=%B "$commit" > "${TMPDIR:-/tmp}/blaise-commit-msg"
        just commit-msg "${TMPDIR:-/tmp}/blaise-commit-msg" || { echo "  in $(git log -1 --format=%h "$commit")" >&2; status=1; }
    done
    exit $status

# Each tool only if it's missing: command:crate
_tools:
    for tool in cargo-machete:cargo-machete cargo-deny:cargo-deny cargo-sort:cargo-sort taplo:taplo-cli; do command -v "${tool%%:*}" >/dev/null || cargo install --locked "${tool#*:}"; done
