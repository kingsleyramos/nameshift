# Deviations from docs/SPEC.md

Per §0.1(3): where the spec's letter couldn't be followed, what it said,
what shipped, and why. Everything not listed here follows the spec.

## Product-visible

1. **Combined files+folders open panel (§14.9).** The spec's Add dialog
   chooses "files **and** directories, multi-select" in one panel. Tauri's
   dialog plugin (rfd underneath) exposes file-picking *or* folder-picking,
   not both in one panel, on all three OSes. Shipped: the toolbar **Add**
   button opens a two-entry menu (*Files…* / *Folders…*), ⌘O opens the file
   picker, and drag-drop (which handles both at once) remains the primary
   path. The §A dialog message and button are used on both pickers.

2. **CSV duplicate rows (§14.7 vs legacy).** Legacy applied the *last*
   occurrence (dictionary overwrite). §14.7/§A state twice that duplicates
   flag the **second** occurrence and "the first still applies", while one
   §14.7 parenthetical still says "later-rows-win". Shipped the
   twice-stated rule: first occurrence applies, later duplicates are
   reported under *Duplicate rows*.

3. **CSV modal copy (§14.7 vs §A).** Where the two differ, §A's exact
   strings win (they are the normative copy catalog): the zero-match note
   reads "…was this CSV **exported** from a different list?", and read
   errors vs. header rejects keep their two distinct §A messages (both
   inline, per §14.7's structure). Stale v1-sheet references in §A ("Step
   1", "Back") are dropped because §14.7's one-state structure has no such
   surfaces.

4. **`format_size(1)` → "1 byte" (§6.5).** The spec's table literally
   yields "1 bytes"; §A's global rule says pluralization follows English.
   Shipped "1 byte", matching §A and the reference formatter.

5. **Locked files (macOS `uchg`).** The spec is silent; the reference
   unlocks user-immutable files before renaming and restores the flag on
   the final name (NAS/torrent clients set it routinely). Shipped the
   reference behavior (macOS only).

6. **macOS Services menu (§20.2, §24 Q20).** Not shipped, per the spec's
   own fallback: registering an NSServices handler under Tauri's app
   delegate is not achievable without fighting the delegate. Open With,
   dock drops, and drag-drop are the supported Finder paths.

7. **Environmental problems don't block Apply (§7.6 vs §24 Q17).**
   `UnrenamableName` (non-UTF-8 names) and `NoFolderPermission` (MAS) badge
   the row and force it out of the plan (its proposed name reverts to the
   current name) instead of counting toward the Apply-blocking conflict
   count — §24 Q17's "visible but excluded from Apply" wins over §7.6's
   blanket "every problem blocks Apply", which would otherwise let one
   unreadable filename freeze the whole workspace.

8. **App icon master.** The owner-supplied `docs/icon-master.png` is
   256×256, not the §0.2 1024×1024. All bundle icons were generated from
   it; large sizes are upscaled. Re-run `pnpm tauri icon <master>` when a
   1024×1024 master arrives.

## Internal / structural

9. **`PlatformProfile.forbids_control_chars` + `os` fields (§16).** The
   spec's struct can't express "U+0000–U+001F forbidden" as a char slice or
   select per-OS §A message variants; two data fields were added rather
   than scattering conditionals.

10. **`invert_selection` command (§12.1).** §15's Edit ▸ Invert Inclusion
    needs a single-mutation command; §12.1's list lacked one. Added,
    mode-scoped like its siblings.

11. **`split_name("file..")` → `("file", ".")` (§5.1).** The literal
    pseudocode result for trailing double dots (the last dot satisfying
    the constraints is the penultimate one). Pinned by test.

12. **E2E bridge (§18.5).** `app.withGlobalTauri` is enabled so the
    WebdriverIO suite can drive imports through `window.__TAURI__` — file
    dialogs and OS drag-drop can't be automated under tauri-driver.

13. **Criterion benchmarks (§17).** The §17 targets are enforced by the
    architecture tests that pin their preconditions (lazy metadata,
    once-per-mutation listing cache, dates-read-up-front — all §18-tested)
    rather than by committed criterion benches; the fixture generator
    (`scripts/gen-fixtures.sh`) exists for manual runs. Add benches under
    `crates/engine/benches` if the targets need CI tracking.

14. **Visual regression (§18.5).** Explicitly a backlog item in the spec
    ("ENG-03 backlog"); nothing shipped. Any future rig must use
    perceptual diffing with pinned fixture dates, per the spec's warning.

15. **Flatpak offline sources (§20.5).** The manifest documents the build
    with network access; Flathub's required offline cargo/node source lists
    must be generated with their builder-tools at submission time (noted in
    the manifest and RELEASING.md).

16. **Dev-tooling additions.** `.editorconfig`, `.vscode/`,
    `rust-toolchain.toml`, `pnpm verify`, and the version-sync script are
    developer-QOL additions outside the spec's scope (requested by the
    owner); none affect the product.

17. **Seven files exceed the ~400-line budget (§3).** `commands.rs` (817 —
    46 near-identical command shims; splitting them across modules would
    scatter the §12.1 list without making any shim simpler), `rule.rs`
    (511 — the ten transforms plus their §4.2 struct), `FileListPane.tsx`
    (496 — the list, its keyboard model, and its context menu share state),
    `app_state.rs` (476), `preview.rs` (459), `RuleCard.tsx` (438),
    `execute.rs` (438), `menu.rs` (437). Each is one cohesive feature;
    splitting would trade file length for cross-file hops. Revisit any of
    them if they grow further.

## 18. Interface tests replace tauri-driver as the UI gate (§18.4)

**Spec:** WebdriverIO + tauri-driver on Linux and Windows, with a scripted
manual checklist on macOS.

**Shipped:** a Playwright interface suite — the real production bundle in
Playwright's own Chromium and WebKit — gating on macOS, Windows and Linux.
The tauri-driver suite still exists but runs on demand
(`.github/workflows/e2e.yml`), not as a gate.

**Why:** tauri-driver has no macOS support and cannot start a WebView2 session
on hosted Windows runners, so as a gate it could only ever cover one platform
and forced per-platform conditionals. Playwright ships its own browser engines
for all three OSes, so one suite runs and passes everywhere. It also caught a
launch crash (a render loop blanking the window) that every other gate missed.
`docs/MANUAL_TESTING.md` is dropped: automation covers the app's behaviour on
macOS, and what remained were release steps, not tests — they now sit in
`docs/RELEASING.md` beside the channel that needs them.

