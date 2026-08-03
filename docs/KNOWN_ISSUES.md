# Known issues

## E2E suite is non-blocking (tauri-driver capability mismatch)

**Status:** the `E2E` CI job is marked `continue-on-error` — it runs but does
not fail the pipeline.

**Symptom:** on both Linux and Windows runners the WebdriverIO session fails
immediately:

```
session not created: WebDriverError: Failed to match capabilities
  when running "http://127.0.0.1:4444/session"
```

The tauri-driver process starts, but the native driver it proxies to
(WebKitWebDriver on Linux, msedgedriver on Windows) rejects the forwarded
capabilities. The suite has never passed in CI — earlier runs failed at the
dependency-install step, which masked this.

**Why it is not just fixed:** tauri-driver only runs on Linux/Windows, so the
harness cannot be reproduced or verified on the macOS dev machine. Fixing it
means iterating through CI and likely aligning tauri-driver with the installed
WebKitWebDriver/Edge driver versions.

**What still gates `main`:** every other check — unit + integration + property
tests on all three OSes, engine and frontend coverage, clippy `-D warnings`,
cargo-deny, the four channel compiles, and the release frontend build. Only the
UI end-to-end run is advisory until the harness is stabilized.

**macOS:** e2e is manual regardless — see [MANUAL_TESTING.md](MANUAL_TESTING.md).

**To resolve:** get the suite green on a runner, then delete the
`continue-on-error` line from the `e2e` job in `.github/workflows/ci.yml`.
