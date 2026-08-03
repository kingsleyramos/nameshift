# Manual testing checklist — macOS

tauri-driver has no macOS support, so this checklist mirrors the E2E specs
(`e2e/specs/`) at the UI level, plus macOS-only behaviors. The Rust
integration suite already exercises the whole engine on macOS CI — the scope
here is UI-level only.

Run against a real build (`pnpm tauri build`, open the bundled app). Use a
scratch folder (e.g. `~/Desktop/ns-test`) with a handful of files.

## 1. Import & preview

- [ ] 1.1 Drag three files onto the window → they appear, all checked, action bar reads `3 of 3 included`.
- [ ] 1.2 Press ⌘O → the open dialog appears with the message `Choose files to rename, or folders to watch`; cancel.
- [ ] 1.3 Add a **Add Prefix** rule with `x-` → every row shows a green underlined `x-` added span; nothing changes on disk.
- [ ] 1.4 The rule card header reads `affects 3 of 3`.
- [ ] 1.5 Clear the prefix text → the card shows `No input yet — rule is skipped`.

## 2. Apply & revert

- [ ] 2.1 With the `x-` prefix, press ⌘↩ → files rename on disk; the banner reads `Renamed 3 files` with a `Revert…` button.
- [ ] 2.2 Keep-rules default: the rules stay and the renamed files are unchecked.
- [ ] 2.3 History tab shows one version: `Prefix “x-” · 3 renames`.
- [ ] 2.4 Click the version → the revert preview lists `x-… → …` rows, each `Will be restored`; the action-bar primary is the orange `Revert…` (never blue).
- [ ] 2.5 Delete one renamed file in Finder → its row flips to `Not found (nothing to restore)`.
- [ ] 2.6 Revert → confirmation shows real counts; confirm → the surviving files restore; the deleted one is skipped.

## 3. Naming conflicts

- [ ] 3.1 Add **New Name from Template** with `same` → every row warns; the action bar shows `⚠ N naming conflicts`; Rename is disabled with `Fix or skip the naming conflicts to rename`.
- [ ] 3.2 `Skip Conflicted` unchecks them all.
- [ ] 3.3 Turn on `Auto-resolve: On` and re-check them → names become `same`, `same 2`, `same 3`; Rename enables.
- [ ] 3.4 Rename a file to an existing on-disk name via double-click edit → `A different file with this name already exists in the folder.`
- [ ] 3.5 Case-only rename (`readme → README`) applies cleanly.
- [ ] 3.6 Swap two names via manual edits (a↔b) → no conflict; Apply performs the swap.

## 4. Watched folders

- [ ] 4.1 Drag a folder in → a `Watching:` chip appears; its files populate the list.
- [ ] 4.2 Add a file to the folder in Finder → it appears in the list within ~1 s.
- [ ] 4.3 Delete it in Finder → it leaves the list.
- [ ] 4.4 The chip's menu switches Follow Global / Include Subfolders / This Folder Only; subfolder contents follow.
- [ ] 4.5 A `.app` bundle inside is listed as one folder; its contents never appear.
- [ ] 4.6 Chip ✕ with > 10 files → the `Stop watching “…”?` guard with real counts; confirm removes the folder's items. ⌘Z restores them **and** the watcher (add a file in Finder → it appears).

## 5. Manual edits & CSV

- [ ] 5.1 Double-click a new name, type `custom.txt`, press Return → pencil badge; `1 edited` chip.
- [ ] 5.2 Edit again and retype exactly what the rules produce → the pencil clears (override not pinned).
- [ ] 5.3 Options ⋯ → Rename by CSV… → download the template; edit one New Name in Numbers; drop it back → `1 of N names will change.`; Apply → one ⌘Z removes the batch.
- [ ] 5.4 Drop a non-CSV file on the modal → the zone shakes, no error text.

## 6. Folders mode

- [ ] 6.1 The `Folders · N` tab switches scope; the primary reads `Rename N Folders`.
- [ ] 6.2 Rename a folder containing tracked files → the files' paths update live (no missing rows).
- [ ] 6.3 Change Extension cards badge `Skipped for folders`.

## 7. Session & windows

- [ ] 7.1 Quit and relaunch → files, rules, toggles, watched folders restore.
- [ ] 7.2 Remove a tracked file in Finder while quit → relaunch shows `Some files were missing` with the count.
- [ ] 7.3 ⌘, opens Settings; toggles mirror the Options menu one-for-one.
- [ ] 7.4 ⌘? opens Help; the regex `Syntax reference` link deep-links to Rename Rules.
- [ ] 7.5 Close the main window with Help open → ⌘0 reopens the main window; quitting works cleanly.

## 8. macOS-only

- [ ] 8.1 File Info on an image shows Spotlight attributes (`kMDItemPixelHeight`, …); ⧉ copies `{md:kMDItemPixelHeight}`.
- [ ] 8.2 A template with `{md:kMDItemPixelHeight}` resolves in the preview for an indexed image.
- [ ] 8.3 Finder → right-click → Open With → Name Shift imports the selection (running and not running).
- [ ] 8.4 Dock-icon drop imports.
- [ ] 8.5 Second launch of the app focuses the existing window.
- [ ] 8.6 A Locked (Get Info → Locked) file renames successfully and stays Locked afterward.
- [ ] 8.7 (MAS build only, `--features channel-mas`) Importing loose files prompts `To rename these files, allow access to their folder.`; declining flags rows with the permission badge.

## 9. Cancellation

- [ ] 9.1 Apply a slow batch (1k+ files) → the overlay appears after ~350 ms with `X of N`; Esc cancels; already-renamed files stay renamed and revertible; no `.fne-tmp-*` files remain anywhere.
