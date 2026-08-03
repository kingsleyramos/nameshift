# Contributing to Name Shift

Issues and pull requests are welcome. The build spec (`docs/SPEC.md`) is the
source of truth for behavior; `CLAUDE.md` lists the invariants that must not
break.

## Toolchain setup

Everything: [Rust](https://rustup.rs) stable, [Node.js](https://nodejs.org)
20+, [pnpm](https://pnpm.io).

- **macOS** — Xcode Command Line Tools (`xcode-select --install`).
- **Windows** — Visual Studio Build Tools (C++ workload) + the Evergreen
  WebView2 runtime (preinstalled on Win 11).
- **Linux** — webkit deps:
  ```sh
  sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev
  ```

## Dev loop

```sh
pnpm install
pnpm tauri dev        # the app with hot reload
pnpm test:watch       # frontend tests in watch mode
cargo test --workspace
```

## Before you open a PR

```sh
pnpm verify           # lint + typecheck + vitest + fmt + clippy + cargo test
```

Gates that must stay green: clippy `-D warnings`, rustfmt, ESLint strict,
tsc, engine coverage ≥ 85% lines (`cargo llvm-cov -p nameshift-engine`),
frontend coverage ≥ 80% lines (`pnpm vitest run --coverage`), property tests
(256 cases), cargo-deny.

Checklist:

- [ ] Tests for the change (every bugfix lands with a regression test named
      after the behavior).
- [ ] In-app Help topics and README updated in the same PR when behavior
      changes.
- [ ] User-facing strings live in `crates/engine/src/copy.rs` /
      `src/lib/strings.ts` — typographic quotes, and “naming conflict”,
      never bare “conflict”.
- [ ] New persisted fields decode with defaults (see `CLAUDE.md`).

## Commit style

Conventional-commit-ish prefixes by area: `engine:`, `store:`, `watcher:`,
`metadata:`, `shell:`, `ui:`, `e2e:`, `ci:`, `docs:`, `packaging:` — small,
single-concern commits with imperative messages.

## Code of conduct

Be kind and constructive; the
[Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/)
applies.
