#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings

if ! command -v dbus-run-session >/dev/null 2>&1; then
    echo "dbus-run-session is required for the isolated MPRIS integration test." >&2
    exit 1
fi

dbus-run-session -- cargo test --all-targets --locked
cargo build --release --locked
python3 -m py_compile tools/render_readme.py
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git diff --check
fi

echo "All checks passed."
