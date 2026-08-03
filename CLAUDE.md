# Name Shift — AI maintainer guide

Name Shift bulk-renames files and folders with a live before → after preview,
two-phase on-disk renames, and full revert history. Tauri v2 + React/TS
frontend + pure Rust engine crates. The authoritative build spec is
`docs/SPEC.md`; deliberate divergences live in `docs/DEVIATIONS.md`.

## Commands

```sh
pnpm install              # once
pnpm tauri dev            # run the app in development
pnpm dev                  # frontend only (Vite)
cargo test --workspace    # engine/store/watcher/metadata: unit + on-disk + property tests
pnpm test                 # frontend component tests (Vitest); add --coverage for the gate
pnpm lint && pnpm typecheck
pnpm verify               # the full §0.1(4) self-verification loop
pnpm verify:ci            # verify + cargo-deny + cross-OS clippy + e2e install (mirror CI before pushing)
pnpm tauri build          # release bundle for the host OS
pnpm tauri build --debug  # bundle smoke
pnpm test:ui              # interface tests (Playwright: Chromium + WebKit, every OS)
pnpm e2e                  # full-stack tauri-driver pass (on demand; Linux/Windows only)
cargo test -p nameshift --test export_bindings   # regenerate src/ipc/gen after TS-derive changes
node scripts/check-versions.mjs                  # version sync (package.json is the source)
# channels:
pnpm tauri build --config src-tauri/tauri.mas.conf.json --no-default-features --features channel-mas
pnpm tauri build --no-default-features --features channel-msstore   # then packaging/msix/make-msix.ps1
pnpm tauri build --no-default-features --features channel-flathub   # manifest in packaging/flatpak/
```

## Architecture map

- `crates/engine` — THE core, no Tauri/UI: rules+tokens (`rule.rs`, `tokens.rs`),
  preview pipeline (`preview.rs`, `sort.rs`), validation as data (`platform.rs`,
  `validate.rs`, `diffkey.rs`), two-phase mover (`execute.rs`), plan/finish
  (`plan.rs`), revert + simulation (`revert.rs`), CSV (`csv.rs`), user-facing
  copy (`copy.rs`).
- `crates/store` — session/history/presets JSON, atomic writes, legacy-tolerant
  decode (`session.rs`, `paths.rs`); §B fixtures under `tests/fixtures/legacy`.
- `crates/watcher` — debounced folder watching + scan/reconcile.
- `crates/metadata` — per-OS `{md:…}`/inspector providers + the lazy stat cache.
- `src-tauri` — thin shell: the mutate funnel (`app_state.rs`), command shims
  (`commands.rs`), workers (`worker.rs`), watchers/import glue
  (`watch_glue.rs`), session glue, menus (`menu.rs`), channel modules
  (`updater.rs`, `mas.rs`), `FileAccess` seam (`access.rs`).
- `src` — presentation only: Zustand mirror (`state/`), typed IPC (`ipc/`,
  generated types in `ipc/gen` — never hand-edit), components per feature
  (`components/`), §A copy in `lib/strings.ts`, §14.6 diff in `lib/nameDiff.ts`.
- `e2e` — WebdriverIO + tauri-driver. `packaging/` — per-store lanes.

## Invariants (do not break)

- Every `CoreState` mutation goes through `AppState::mutate` (version bump =
  cache invalidation = event). Never mutate outside it; never add a second
  preview cache.
- Renames are two-phase (temp → final) so swaps and case-only renames never
  collide; batches with nested folders run parents-first on apply,
  children-first on revert; recorded history entries always match the final
  disk layout. Do not "optimize" the two phases away for small batches.
- `rewrite_live_paths` prefix-rewrites tracked paths after directory moves
  **without existence guards** — batch ordering makes such guards wrong.
- Persisted structs decode with per-field defaults (`#[serde(default)]`) and
  ignore unknown fields, so old presets/sessions always import; new fields
  must follow suit. UUIDs emit uppercase-hyphenated; snapshot dates emit
  whole-second UTC.
- The temp prefix is `.fne-tmp-` forever — orphan recovery also rescues temps
  stranded by earlier releases. Scans and rescans always ignore it; orphan
  recovery never runs while a batch is processing.
- Rule cards and file rows are keyed by stable ids, never array position
  (positional identity crashed the app when Apply cleared the array
  mid-interaction; regression-tested).
- Every persisted path goes through `FileAccess::persist_token` (MAS
  bookmarks depend on it), and any new fs call goes through
  `platform::win_long_path`.
- Watcher handles tear down on Drop (test-asserted); watchers re-arm by
  folder id when roots move or list-undo restores them.
- The preview never stats per keystroke: the directory-listing cache and the
  metadata stat cache invalidate on list mutations only; dates for sorting
  are read once up front, never in a comparator.
- User-facing copy says "naming conflict", never bare "conflict"; "Skipped"
  belongs to unchecked list rows only; *selection* (rows) and *inclusion*
  (checkboxes) are never conflated. Strings live in `crates/engine/src/copy.rs`
  and `src/lib/strings.ts` only.
- Row selection is frontend-only ui state; it never crosses IPC and is never
  persisted.
- Each keyboard shortcut is bound in exactly one place — the menu item.
- Update the in-app Help topics and README whenever behavior changes.
- Run the full test suite before every commit; the on-disk integration tests
  catch what unit tests miss.

## Gates (CI-enforced)

Clippy `-D warnings`, rustfmt, ESLint, tsc; `crates/engine` ≥ 85% line
coverage (cargo-llvm-cov); frontend ≥ 80% lines (vitest); property tests at
256 cases; cargo-deny advisories/licenses.

The release-mode frontend build (`pnpm build`) and version sync
(`pnpm check:versions`) run in both `pnpm verify` and CI. The debug bundle
smoke sets `minify:false`, so only `pnpm build` exercises the production
minifier — keep it in the gate. Each non-default channel
(`channel-mas`/`-msstore`/`-flathub`) is compile-checked on its target OS,
because its code is gated behind `cfg(feature)` and no other job builds it.

`pnpm verify:ci` (`scripts/verify-ci.sh`) is the local mirror of the
CI-only checks — cargo-deny, cross-OS clippy for the pure-Rust crates,
and the Playwright interface suite. Run it before pushing; it needs
`cargo install cargo-deny` once.

Every CI job runs on all three OSes and must pass on all three — no
`continue-on-error`, no platform conditionals, no skips. The interface
layer uses Playwright's own Chromium/WebKit so it never depends on the
host webview. The tauri-driver full-stack pass is deliberately OUT of the
gate (`.github/workflows/e2e.yml`, on demand) because it cannot run on
macOS. See docs/TESTING.md.
