#!/usr/bin/env bash
#
# Local mirror of the CI checks that can run off a single machine, so
# cross-platform, supply-chain, and packaging failures surface before a push
# instead of on the runner. Run it before pushing:  pnpm verify:ci
#
# It is a superset of `pnpm verify`. The extra checks are the ones that have
# bitten us despite a green `pnpm verify`:
#   - cargo-deny        : advisory / license / wildcard policy (Linux dep tree)
#   - cross clippy      : platform-only lint, e.g. an import used only on unix
#   - interface tests   : the real bundle in Chromium + WebKit
#
# Known gap: the src-tauri crate pulls in tauri/wry/webview2 sys crates that
# cannot be cross-checked from macOS, so its per-OS code still relies on the
# CI Test matrix. Everything in crates/* is covered here on all three targets.
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n\033[1m==> %s\033[0m\n' "$1"; }

step "pnpm verify (lint, typecheck, versions, build, tests, fmt, clippy, cargo test)"
pnpm verify

step "cargo-deny (advisories, licenses, bans, sources)"
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "cargo-deny not installed. Run: cargo install cargo-deny --locked" >&2
  exit 1
fi
cargo deny check

step "cross-platform clippy (crates/*, Windows + Linux targets)"
PURE_CRATES=(-p nameshift-engine -p nameshift-store -p nameshift-watcher -p nameshift-metadata)
for target in x86_64-pc-windows-msvc x86_64-unknown-linux-gnu; do
  rustup target add "$target" >/dev/null 2>&1 || true
  echo "    clippy --target $target"
  cargo clippy "${PURE_CRATES[@]}" --tests --target "$target" -- -D warnings
done

step "interface tests (Playwright: Chromium + WebKit)"
pnpm exec playwright install chromium webkit >/dev/null 2>&1 || true
pnpm test:ui

printf '\n\033[1;32mAll CI-equivalent checks passed.\033[0m\n'
