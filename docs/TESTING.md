# Testing

Every gate runs on macOS, Windows, and Linux, and passes on all three. No
suite is skipped, warned past, or made conditional on the platform — if a
check can't run somewhere, that's a flaw in the setup, not something to
document around.

## The layers

| Layer | What it proves | Where it runs |
|---|---|---|
| **Engine** — `cargo test --workspace` | Renaming logic against a real disk: two-phase moves, revert, watching, metadata, 256-case property tests | all 3 OSes |
| **Components** — `pnpm test` (Vitest) | Individual components in isolation, incl. coverage gate | all 3 OSes |
| **Interface** — `pnpm test:ui` (Playwright) | The real production bundle in a real browser engine: mounting, importing, previewing, conflicts, filtering | all 3 OSes × Chromium + WebKit |
| **Packaging** — `pnpm tauri build --debug` | The app compiles and bundles | all 3 OSes |
| **Supply chain** — cargo-deny | Advisories, licences, dependency policy | Linux |

Playwright ships its own Chromium and WebKit builds for every platform, so the
Interface layer has no dependency on the host's webview or on any desktop
automation driver. That is what lets it run everywhere. WebKit is the engine
family behind the macOS webview, so it doubles as a check on Mac rendering.

The backend is stood in for during Interface tests (`ui-tests/fake-backend.ts`)
by replacing `window.__TAURI_INTERNALS__`. The frontend itself — commands,
events, store, components — runs unmodified.

## Full-stack pass (on demand)

`.github/workflows/e2e.yml` drives the packaged app through `tauri-driver`,
renaming real files. It is **not** part of the gate: `tauri-driver` has no
macOS support and cannot start a WebView2 session on hosted Windows runners,
so it could only ever run on one platform. Keeping it out of CI is what keeps
the gate uniform.

Run it from the Actions tab when a change touches the shell — commands,
events, session handling, or folder watching. Locally: `pnpm e2e` (Linux and
Windows only).

## Before pushing

```sh
pnpm verify      # lint, typecheck, versions, build, components, fmt, clippy, cargo test
pnpm verify:ci   # the above + cargo-deny, cross-OS clippy, interface tests
```

## What isn't automated

Two things, both tied to shipping rather than to the code: Gatekeeper on a
genuinely downloaded build, and Mac App Store sandbox prompts. Neither can
happen on a runner, so both are checklist items in
[RELEASING.md](RELEASING.md) beside the channel that needs them.
