# Name Shift — Cross-Platform Build Specification

**Target:** a complete, production-quality desktop app for macOS, Windows, and Linux, built with **Tauri v2 + React (TypeScript) + a Rust engine**, distributed via **direct download, the Mac App Store, the Microsoft Store, and Flathub**.

**Audience:** an AI builder agent. This document is the single source of truth. It is written to be executed **in one continuous build** with no human in the loop. Every product behavior, data format, algorithm, test requirement, and packaging target is specified here. Where a judgment call remains, this document makes the call — follow it.

**Provenance note (do not document publicly):** the behaviors specified here are the behaviors of an existing, mature macOS app. The public README, help content, and code comments must describe the app on its own terms — never as a port, rewrite, or successor of anything.

**Spec lineage:** this is **v2** of the build spec (2026-08-01). It folds the full UX/UI engineering audit of the reference app into the product: audit fixes that already shipped in the reference implementation are baseline behavior here, and the audit's rebuild-tagged redesigns (bottom action bar, scope tabs, a real row-selection model, the guided Rename-via-Spreadsheet sheet, the rules-panel restructure, workspace-wide undo) are normative in §13–§14. Where this spec and the reference app disagree, this spec wins; §24 records every deliberate departure.

---

## Table of contents

- [§0 Builder contract](#0-builder-contract)
- [§1 Product overview & identity](#1-product-overview--identity)
- [§2 Locked technology decisions](#2-locked-technology-decisions)
- [§3 Repository layout](#3-repository-layout)
- [§4 Domain model & persistence formats](#4-domain-model--persistence-formats)
- [§5 The rename engine — rule semantics](#5-the-rename-engine--rule-semantics)
- [§6 Tokens](#6-tokens)
- [§7 The preview pipeline](#7-the-preview-pipeline)
- [§8 Apply & revert — the two-phase move engine](#8-apply--revert--the-two-phase-move-engine)
- [§9 Folder watching](#9-folder-watching)
- [§10 File access & sandbox architecture (MAS-ready)](#10-file-access--sandbox-architecture-mas-ready)
- [§11 Metadata providers](#11-metadata-providers)
- [§12 IPC contract — commands & events](#12-ipc-contract--commands--events)
- [§13 Frontend architecture & state management](#13-frontend-architecture--state-management)
- [§14 UI specification](#14-ui-specification)
- [§15 Menus & keyboard shortcuts](#15-menus--keyboard-shortcuts)
- [§16 Platform rules — validation, case, normalization, limits](#16-platform-rules--validation-case-normalization-limits)
- [§17 Performance targets](#17-performance-targets)
- [§18 Testing requirements](#18-testing-requirements)
- [§19 CI/CD](#19-cicd)
- [§20 Distribution & packaging](#20-distribution--packaging)
- [§21 Code conventions & AI-maintainability](#21-code-conventions--ai-maintainability)
- [§22 Documentation deliverables](#22-documentation-deliverables)
- [§23 Build order & definition of done](#23-build-order--definition-of-done)
- [§24 Questions asked and answered (FAQ)](#24-questions-asked-and-answered-faq)
- [§A Appendix: user-facing copy catalog](#a-appendix-user-facing-copy-catalog)
- [§B Appendix: legacy JSON fixtures](#b-appendix-legacy-json-fixtures)

---

## §0 Builder contract

### 0.1 Ground rules

1. **Build in a fresh, empty repository.** The first commit is the scaffold; this file lives at `docs/SPEC.md` in the new repo. A finished `README.next.md` accompanies this spec — install it verbatim as `README.md` (then delete `README.next.md`).
2. **No placeholder code.** Every feature in this spec ships working. If a feature must be stubbed for a platform (e.g., Spotlight metadata off-macOS), the stub is the *specified degradation*, not a TODO.
3. **Deviations file.** If an API named here no longer exists or a specified approach is impossible, implement the nearest equivalent that preserves the stated behavior and record it in `docs/DEVIATIONS.md`: what the spec said, what you did, why. Never silently diverge.
4. **Self-verification loop.** You are not done until, on the host OS: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm lint`, `pnpm typecheck`, `pnpm test`, and `pnpm tauri build` (debug bundle) all pass. Run them; fix failures; repeat. Report actual results — never claim green without running.
5. **Commit discipline.** Small, single-concern commits with imperative messages (`Engine: two-phase move executor + rollback tests`). The milestone order in §23 is the commit order skeleton.
6. **Copy is normative.** User-facing strings in §14 and §A are exact (including typographic quotes `’` `“` `”` and the term *naming conflict* — never bare "conflict"). Strings not cataloged: write them in the same voice (sentence case, direct, no exclamation marks).
7. **When this spec is silent**, prefer: platform convention > simplest correct implementation > fewest dependencies. Note the decision in `docs/DEVIATIONS.md` if it is user-visible.

### 0.2 Inputs supplied by the owner (placeholders until then)

| Input | Placeholder to use | Needed for |
|---|---|---|
| Repo slug | `kingsleyramos/nameshift` — builder must substitute the real slug everywhere (README badges, updater endpoint, links) | README, CI, updater |
| App icon master (1024×1024 PNG) | Generate a temporary geometric placeholder icon; run `pnpm tauri icon <master>` when the real one arrives | All bundles |
| Apple Developer ID cert + notary creds | CI secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` (release signing steps are conditional on their presence) | macOS direct |
| Apple Distribution cert + provisioning profile | `packaging/mas/` documents the manual steps | MAS |
| Microsoft Partner Center identity | `Publisher`, `PublisherDisplayName`, `Identity Name` placeholders in `packaging/msix/AppxManifest.xml` | Microsoft Store |
| Updater signing keypair | CI secrets `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (generate with `pnpm tauri signer generate`) | Auto-update |

Everything above gates **release/publish lanes only**. The app, tests, and unsigned local bundles must build with zero secrets.

### 0.3 The reference implementation (read-only)

A mature Swift/SwiftUI implementation of this product lives at `/Users/kingsleyramos/Developer/Github/nameshift-swift` (this repo is `/Users/kingsleyramos/Developer/Github/nameshift`). If that path is unavailable, this spec is self-contained — build from it alone. Use the reference exactly three ways:

1. **Behavior oracle** — where this spec reads ambiguous, run/read the reference (`swift test` is green there) and match it, unless §24 records a deliberate departure.
2. **Test-vector source** — its suite encodes hard-won edge cases (case-only swaps, mid-batch cancellation, orphan temps, revert chains, numbering-skips-deselected). Port the *assertions*, never the code.
3. **Copy source** — its Help topics and alert strings are post-audit and current; §A remains the normative subset.

Never import Swift/SwiftUI structure into the Rust/React design, and never mention the reference app in public-facing docs (provenance note above).

---

## §1 Product overview & identity

**Name Shift** renames many files (and folders) at once. The user builds a stack of rename rules, watches a live before → after preview of every name, and presses **Apply** to rename on disk. Every Apply is recorded as a version that can be previewed and reverted later. Safety is the brand: two-phase renames (swaps and case-only renames never collide), conflict detection that blocks Apply, and full revert history.

### 1.1 Identity constants

| Constant | Value |
|---|---|
| Product name (user-facing, everywhere) | `Name Shift` |
| App identifier (Tauri `identifier`, macOS bundle id) | `com.kingsleyramos.nameshift` |
| Flatpak app id (Flathub convention, GitHub-verifiable) | `io.github.kingsleyramos.NameShift` |
| Binary/product slug | `nameshift` |
| Version | `2.0.0` (semver; the app has shipped a 1.x before — never version below 2) |
| License | MIT (`LICENSE` file, copyright Kingsley Ramos) |
| Category | Utilities (`public.app-category.utilities` on macOS) |
| Data directory name (all OSes) | `NameShift` |

### 1.2 Supported platforms (floors)

| OS | Floor | Rationale |
|---|---|---|
| macOS | 11.0 Big Sur, universal (arm64 + x86_64) | Tauri v2 floor is lower, but 11+ keeps WKWebView behavior uniform |
| Windows | Windows 10 1809+ | MSIX baseline; WebView2 Evergreen bootstrapped by the installer |
| Linux | Distros with `webkit2gtk-4.1` (Ubuntu 22.04+, Fedora 36+, Arch, openSUSE) | wry requirement |

### 1.3 Non-goals (v2.0)

- No file *moving* between directories — Name Shift renames items in place; the directory component never changes.
- No file content editing, tagging, or conversion.
- No telemetry, analytics, or network calls of any kind, except the updater (direct-download channel only) checking a GitHub Releases endpoint.
- No localization in v2.0 (all strings centralized to make it cheap later — §21.4).
- No cloud sync of presets/history.

---

## §2 Locked technology decisions

These are decided. Do not re-litigate; record impossibilities in `DEVIATIONS.md`.

| Layer | Choice | Notes |
|---|---|---|
| Shell | **Tauri v2** (latest stable 2.x) | |
| Frontend | **React** (latest stable) + **TypeScript** (`strict: true`) + **Vite** | |
| Package manager | **pnpm** | committed `pnpm-lock.yaml` |
| Frontend state | **Zustand** | one store per domain slice (§13) |
| Styling | **Tailwind CSS v4** + CSS custom properties for theme tokens | light/dark via `prefers-color-scheme`, overridable |
| List virtualization | **@tanstack/react-virtual** | file list must handle 50k rows |
| Icons | **lucide-react** | the single icon language; §14.13 maps every glyph — no per-OS symbol fonts |
| Drag & drop | **@dnd-kit/core** + **@dnd-kit/sortable** | rule-card reorder; ↑/↓ buttons stay for keyboard parity |
| Engine | **Rust** (stable toolchain, edition 2021+), pure crates with **zero Tauri dependencies** | §3 workspace |
| Serialization | **serde / serde_json** | field-compatible with legacy JSON (§4) |
| Regex | **fancy-regex** | closest to ICU semantics (lookaround, backrefs); divergences documented (§24 Q14) |
| Unicode | **unicode-normalization** (NFC/NFD), **icu_properties** (emoji sets), **icu_collator** (natural sort, numeric mode) | |
| Dates | **chrono** + a custom LDML-pattern formatter (§6.4) | user-facing date patterns are `yyyy-MM-dd` style, not strftime |
| FS watching | **notify** + **notify-debouncer-full** | debounce ≈ 400 ms |
| Parallelism | **rayon** for metadata/stat fan-out; **std::thread** + channels for the apply worker | |
| Errors | **thiserror** in crates; one `AppError` type at the Tauri boundary | |
| Type sharing | **ts-rs**: derive `TS` on every IPC-crossing type; generated files land in `src/ipc/gen/` via a `cargo test` export hook; a thin hand-written typed `invoke` wrapper in `src/ipc/commands.ts` | do not hand-maintain duplicate types |
| Tauri plugins | `dialog`, `clipboard-manager`, `opener` (reveal in file manager), `single-instance`, `window-state`, `updater` (direct channel only) | nothing else without a DEVIATIONS entry |
| Frontend tests | **Vitest** + React Testing Library + `@tauri-apps/api/mocks` (`mockIPC`) | |
| E2E | **WebdriverIO + tauri-driver** on Linux & Windows; scripted manual checklist on macOS (tauri-driver has no macOS support) | §18.4 |
| Rust coverage | **cargo-llvm-cov**, gate ≥ 85% lines on `crates/engine` | §18.6 |
| Lint | `cargo clippy -D warnings`, `cargo fmt`; ESLint (typescript-eslint strict) + Prettier | |
| Supply chain | **cargo-deny** (advisories + licenses) in CI | |

**Why a Rust engine and not TS?** Chosen deliberately: the engine is CPU-and-filesystem work (10k-file previews, two-phase batch moves), it must be testable with plain `cargo test` on all three CI OSes without a webview, and the strongest typing lives where the invariants are. React is presentation only.

---

## §3 Repository layout

```
nameshift/
├── README.md                     # installed from README.next.md (§22.1)
├── LICENSE                       # MIT
├── CONTRIBUTING.md               # §22.4
├── CLAUDE.md                     # AI-maintainer guide, §22.3 (authoritative invariants)
├── AGENTS.md                     # one line: "See CLAUDE.md." (tool-agnostic pointer)
├── package.json  pnpm-lock.yaml  vite.config.ts  tsconfig.json
├── eslint.config.js  .prettierrc  tailwind.config (v4 CSS-first: styles/theme.css)
├── Cargo.toml                    # [workspace] members: src-tauri, crates/*
├── deny.toml                     # cargo-deny config
├── index.html
├── src/                          # ── React frontend (presentation only) ──
│   ├── main.tsx  App.tsx
│   ├── ipc/
│   │   ├── gen/                  # ts-rs output (generated; committed)
│   │   ├── commands.ts           # typed invoke wrappers, one per §12 command
│   │   └── events.ts             # typed listen wrappers, one per §12 event
│   ├── state/                    # Zustand slices: appState.ts, uiState.ts
│   ├── components/
│   │   ├── shell/                # Window chrome, SplitLayout, Toolbar, StatusBar,
│   │   │                         #   DropOverlay, ProcessingOverlay, Banners, Dialogs
│   │   ├── rules/                # RulesPanel, RuleCard (one file per rule kind's fields),
│   │   │                         #   TokenInsertMenu, TokenReference, PresetsMenu
│   │   ├── files/                # FileListPane, FileRow, NameDiff, FilterBar,
│   │   │                         #   WatchedFolderChips, ModeSwitch, SortMenu
│   │   ├── history/              # HistoryPanel, RevertPreviewPane
│   │   ├── inspector/            # InspectorView
│   │   ├── help/                 # HelpWindow (topics from §22.5)
│   │   └── settings/             # SettingsWindow
│   ├── lib/                      # pure TS: nameDiff.ts, format.ts, platform.ts, keys.ts
│   └── styles/                   # theme.css (tokens), base.css
├── src-tauri/                    # ── Tauri shell (thin!) ──
│   ├── Cargo.toml                # features: channel-direct (default), channel-mas,
│   │                             #   channel-msstore, channel-flathub
│   ├── tauri.conf.json           # base config (productName "Name Shift", identifier)
│   ├── tauri.direct.conf.json    # + updater artifacts/endpoints
│   ├── tauri.mas.conf.json       # MAS overrides (§20.3)
│   ├── capabilities/default.json # minimal v2 ACL for the plugins in §2
│   ├── Info.plist                # macOS extras: usage descriptions, doc types (§20.2)
│   ├── icons/                    # tauri icon output
│   └── src/
│       ├── main.rs  lib.rs
│       ├── commands.rs           # #[tauri::command] fns — argument marshaling ONLY;
│       │                         #   every body is one call into crates/*
│       ├── app_state.rs          # Mutex<CoreState> + version bump + event emission
│       ├── events.rs             # typed event names + payload structs
│       ├── menu.rs               # native menus (§15)
│       └── platform/             # single_instance, opened-files (macOS), services stub
├── crates/
│   ├── engine/                   # nameshift-engine: THE core. No Tauri, no UI.
│   │   └── src/
│   │       ├── lib.rs  item.rs  rule.rs  tokens.rs  preview.rs  sort.rs
│   │       ├── plan.rs           # ApplyPlan construction
│   │       ├── execute.rs        # two-phase + hierarchical mover, cancel, recovery
│   │       ├── revert.rs         # revert simulation + execution
│   │       ├── validate.rs       # per-platform name validation (§16)
│   │       ├── platform.rs       # PlatformProfile (injectable for tests)
│   │       ├── csv.rs            # name-mapping CSV build/parse
│   │       └── diffkey.rs        # collision keys (fold/normalize per profile)
│   ├── store/                    # nameshift-store: session/history/presets JSON,
│   │   └── src/                  #   atomic writes, legacy decode tolerance, StorePaths
│   ├── watcher/                  # nameshift-watcher: notify wrapper, debounce, rescan
│   └── metadata/                 # nameshift-metadata: MetadataProvider trait + impls
│       └── src/                  #   macos.rs (Spotlight FFI), windows.rs (Property System),
│                                 #   linux.rs (stat+EXIF+xattr), common.rs
├── e2e/                          # WebdriverIO project (Linux/Windows)
│   ├── wdio.conf.ts  fixtures/  specs/
├── packaging/
│   ├── msix/                     # AppxManifest.xml, assets/, make-msix.ps1
│   ├── mas/                      # entitlements.plist, README (manual signing steps)
│   ├── flatpak/                  # io.github.kingsleyramos.NameShift.yml, metainfo.xml
│   └── linux/                    # .desktop file, shared AppStream metainfo
├── scripts/                      # gen-fixtures.(rs|sh) — 10k-file benchmark tree
├── docs/
│   ├── SPEC.md                   # ← this document
│   ├── DEVIATIONS.md             # builder-maintained
│   ├── MANUAL_TESTING.md         # macOS E2E checklist (§18.5)
│   ├── RELEASING.md              # per-channel release runbook (§20.6)
│   └── screenshots/              # README images
└── .github/
    ├── workflows/ci.yml  release.yml
    └── ISSUE_TEMPLATE/  PULL_REQUEST_TEMPLATE.md
```

Layout rules:

- **Dependency direction:** `src-tauri` → `crates/*`; crates never depend on `src-tauri`; `engine` depends on no other workspace crate except (optionally) nothing — `store`, `watcher`, `metadata` may depend on `engine`'s types, never the reverse.
- **File size discipline:** no source file over ~400 lines without a `DEVIATIONS.md` note; split by feature, not by type.
- **One component per file**, colocated `*.test.tsx`.

---

## §4 Domain model & persistence formats

### 4.1 Core types (Rust, `crates/engine` unless noted)

All IPC-crossing types derive `Serialize, Deserialize, TS, Clone, Debug, PartialEq`.

```rust
pub struct FileItem {
    pub id: Uuid,
    pub path: PathBuf,            // absolute, standardized (no trailing sep, no ./..)
    pub folder_id: Option<Uuid>,  // Some(_) when discovered by a watched folder
    pub is_selected: bool,        // default true — only selected items are renamed
    pub is_directory: bool,       // Folders-mode targets
    pub override_name: Option<String>, // manual edit; wins over rules
}
// Derived: name() = final path component; directory() = parent path.

pub struct WatchedFolder {
    pub id: Uuid,
    pub path: PathBuf,
    pub include_subfolders: Option<bool>,  // None = follow the global toggle; per-folder override (§9)
}  // watcher handle lives in crates/watcher

pub enum ListMode { Files, Folders }          // serialized as "Files" / "Folders" (legacy raw values)

pub enum FileSortKey { OrderAdded, Name, FileExtension, Folder, DateCreated, DateModified }
// serialized as the legacy raw values:
// "Order Added" | "Name" | "Extension" | "Folder" | "Date Created" | "Date Modified"

pub enum FilterMode { All, WillChange, Conflicts }  // UI filter; never affects the preview computation

pub struct CoreState {                        // src-tauri/app_state.rs owns Mutex<CoreState>
    pub files: Vec<FileItem>,
    pub rules: Vec<RenameRule>,
    pub history: Vec<Snapshot>,               // newest first, capped at 50
    pub presets: Vec<RulePreset>,
    pub watched_folders: Vec<WatchedFolder>,
    pub excluded_paths: HashSet<PathBuf>,     // user-removed; rescans skip these
    pub trims_whitespace: bool,
    pub auto_resolves_conflicts: bool,
    pub keep_rules_after_apply: bool,
    pub include_subfolders: bool,
    pub sort_key: FileSortKey, pub sort_ascending: bool,
    pub list_mode: ListMode,
    pub selected_snapshot_id: Option<Uuid>,   // Some → revert-preview mode
    pub rules_cleared_by_apply: bool,         // drives the rules panel post-apply empty state
    pub version: u64,                         // §13.1 — bumped by EVERY mutation
}
```

### 4.2 `RenameRule` — fields and defaults

One struct for all rule kinds (exactly the legacy shape — presets are portable files users may already have):

| Field | Type | Default | Used by |
|---|---|---|---|
| `id` | Uuid | new | all |
| `kind` | enum | — (required) | all |
| `isEnabled` | bool | `true` | all |
| `includesExtension` | bool | `false` | all except `changeExtension` |
| `caseSensitive` | bool | `true` | removeText, replaceText, regexReplace |
| `text` | String | `""` | primary input (pattern/prefix/suffix/template/new-ext/number-separator/sanitize-replacement) |
| `replacement` | String | `""` | replaceText, regexReplace |
| `caseStyle` | enum | `lowercase` | changeCase |
| `numberPosition` | enum | `after` | numberSequentially |
| `numberStart` | i64 | `1` | numberSequentially |
| `numberPadding` | i64 | `3` | numberSequentially |
| `stripsDiacritics` | bool | `false` | sanitize |
| `restartPerFolder` | bool | `false` | numberSequentially — restart the counter in each folder (§7.3) |
| `removesEmoji` | bool | `false` | sanitize |

Serialized enum values (exact strings, camelCase — legacy compatibility is a hard requirement):

- `kind`: `removeText` `replaceText` `regexReplace` `addPrefix` `addSuffix` `changeCase` `changeExtension` `sanitize` `numberSequentially` `template`
- `caseStyle`: `lowercase` `uppercase` `titleCase`
- `numberPosition`: `before` `after` `replaceName`

**Serde policy (normative, applies to every persisted struct):** every field except `kind` carries `#[serde(default = ...)]` with the defaults above; unknown fields are ignored (serde's default); JSON field names are exactly the camelCase names in the table. This reproduces the legacy "decode with defaults so old presets keep importing" behavior — **keep this policy when adding fields in the future** (it is a CLAUDE.md invariant, §22.3). UUIDs: accept any case on read, emit uppercase-hyphenated on write.

`is_effective(rule)` — a rule with insufficient input is skipped entirely by the pipeline:

- `removeText | replaceText | addPrefix | addSuffix | template | changeExtension` → `!text.is_empty()`
- `regexReplace` → `!text.is_empty() && pattern_compiles(text)`
- `changeCase | numberSequentially | sanitize` → always effective

`summary(rule)` (used in snapshot summaries and rule-card subtitles; `“ ”` are typographic quotes):

| kind | summary |
|---|---|
| removeText | `Remove “{text}”` |
| replaceText | `Replace “{text}” with “{replacement}”` |
| regexReplace | `Regex “{text}” → “{replacement}”` |
| addPrefix | `Prefix “{text}”` |
| addSuffix | `Suffix “{text}”` |
| changeCase | `Case → {lowercase\|UPPERCASE\|Title Case}` |
| changeExtension | `Extension → “{text}”` |
| sanitize | `Fix unsafe characters` |
| numberSequentially | `Number {before name\|after name\|replace name} from {numberStart}`; append ` per folder` when `restartPerFolder` |
| template | `Template “{text}”` |

### 4.3 Persisted files

Location: the per-OS app-data directory + `/NameShift`:

| OS | Directory |
|---|---|
| macOS | `~/Library/Application Support/NameShift` (identical to the legacy install — history/presets/session carry over automatically; also migrate a `File Name Editor` sibling directory to `NameShift` on first access if `NameShift` doesn't exist) |
| Windows | `%APPDATA%\NameShift` (Roaming) |
| Linux | `$XDG_DATA_HOME/NameShift` (default `~/.local/share/NameShift`) |

All writes are **atomic**: write `<name>.json.tmp` in the same directory, then rename over. All reads tolerate a missing or corrupt file by falling back to empty defaults (never crash on bad JSON; log via the tracing facility).

**`history.json`** — array of snapshots, newest first, `maxHistoryCount = 50`:

```jsonc
[{ "id": "UUID", "date": "2026-07-27T18:04:11Z",   // ISO-8601/RFC-3339 UTC seconds; accept fractional on read
   "summary": "Prefix “photo-” · Trim spaces",
   "entries": [{ "from": "/abs/old/path", "to": "/abs/new/path" }] }]
```

**`presets.json`** — array of `{ "id": UUID, "name": String, "rules": [RenameRule], "trimsWhitespace": bool }`.

**`session.json`** — the restorable workspace:

```jsonc
{ "rules": [RenameRule], "trimsWhitespace": false, "autoResolvesConflicts": false,
  "keepRulesAfterApply": false, "includeSubfolders": false,
  "sortKeyRaw": "Order Added", "sortAscending": true, "listModeRaw": "Files",
  "files": [{ "path": "/abs", "isSelected": true, "overrideName": "opt — omit when null", "isFromFolder": false,
              "bookmark": "opt base64 — MAS builds only (§10)" }],
  "watchedFolderPaths": ["/abs"],
  "watchedFolderBookmarks": ["opt base64 — MAS builds only, parallel array (§10)"],
  "excludedPaths": ["/abs"] }
```

Legacy tolerance: also accept the pre-split sort field `sortOrderRaw` and migrate: `"Name (A–Z)"→(Name, asc)`, `"Name (Z–A)"→(Name, desc)`, `"Extension"/"Folder"/"Date Created"/"Date Modified"`→(that key, asc), anything else→(OrderAdded, asc). The `bookmark`/`watchedFolderBookmarks` fields are additive — absent in direct-channel output, ignored by it on read.

**Fresh-install default (deliberate departure, §24 Q26):** when no `session.json` exists, `keepRulesAfterApply` defaults to **true**. A session file *lacking* the key decodes to `false` (it was written under the old default; preserve what that user experienced). All other toggles default `false`.

**Per-folder watch depth (additive):** an optional `watchedFolderSubfolders` array parallels `watchedFolderPaths` (`true` / `false` / `null` = follow global); absent in legacy files, ignored by older readers.

**Session save triggers:** after import, clear-all, apply, revert, live-path rewrite; debounced (1 s) after any other `CoreState` mutation; and on app exit. Tests must cover a save/load round-trip of every field.

**Session restore** (on launch): apply scalars; re-add each direct file whose path still exists (count the missing, then show the info alert `Some files were missing` — copy in §A); re-arm a watcher for each `watchedFolderPaths` entry that still exists and is a directory; after rescans, re-apply `isSelected`/`overrideName` to folder-discovered files by path match (first occurrence wins on duplicate paths). Direct files (`isFromFolder: false`) are re-imported straight from the entry list.

---

## §5 The rename engine — rule semantics

The heart of the app. Implement in `crates/engine/rule.rs` as a pure function:

```rust
pub fn apply_rule(rule: &RenameRule, name: &str, ctx: Option<&TokenContext>, is_directory: bool) -> String
```

### 5.1 Name splitting (normative)

```
split(name) -> (stem, ext):
  let i = index of the LAST '.' in name such that i > 0 (not the first char)
                                         and i < len-1 (at least one char after it)
  if such i exists: stem = name[..i], ext = name[i+1..]
  else:             stem = name,      ext = ""
```

Vectors: `photo.jpg → (photo, jpg)` · `archive.tar.gz → (archive.tar, gz)` · `.gitignore → (.gitignore, "")` · `README → (README, "")` · `file. → (file., "")` · `.config.json → (.config, json)`. Extensions are treated as opaque; no MIME logic.

### 5.2 Scope: what the rule transforms

- **Directories:** the whole name, always (no extension concept). `changeExtension` is a **no-op** for directories.
- **Files:** `changeExtension` has its own path (§5.3-g). Otherwise: if `includesExtension == true` **or** the file has no extension, transform the whole name; else transform the stem and re-attach `"." + ext` unchanged.

`{token}` expansion (§6) inside rule inputs uses `baseName = the string being transformed at this rule's stage` (i.e., the output of the previous rule) and `fileExtension = ext` from the split above.

### 5.3 Per-kind transforms (on the in-scope string `s`)

a. **removeText** — remove every occurrence of `text` (no token expansion in the needle). Case-insensitive when `caseSensitive == false`: match by simple Unicode case folding of both needle and haystack, char-aligned (§16.4). Overlapping occurrences: scan left-to-right, non-overlapping (standard replace semantics).

b. **replaceText** — same matching as (a); the replacement is `expand_tokens(replacement)`. The needle is plain text, never token-expanded.

c. **regexReplace** — compile `text` with fancy-regex (case-insensitive flag `(?i)` prepended when `caseSensitive == false`). Replace **all** matches with the template `expand_tokens(replacement)` — token expansion happens **first**, then the result is used as the regex replacement template, so `$1`/`${name}` group references work (and yes, a token that expands to a literal `$1` will be treated as a group reference — legacy-faithful; note in help). An invalid pattern makes the rule ineffective (skipped) — the UI shows the invalid state (§14.3), the engine never errors on it.

d. **addPrefix** — `expand_tokens(text) + s`. **addSuffix** — `s + expand_tokens(text)` (with default scope this lands before the extension — document as "suffixes go before the extension").

e. **changeCase** — `lowercase`: Unicode lowercase. `uppercase`: Unicode uppercase. `titleCase` (normative): a *word* is a maximal run of alphabetic chars; everything else passes through and separates words. A word that is **entirely uppercase and ≥ 2 letters keeps its capitalization** (acronym guard: `NASA`, `HDR`, `II`). Every other word: first letter uppercased, the rest lowercased. Vectors: `hello world → Hello World` · `hello-world → Hello-World` · `NASA report → NASA Report` · `4K HDR clip → 4K HDR Clip` · `v2 draft → V2 Draft` · `iPhone photo → Iphone Photo` (mixed-case words normalize — deliberate; help notes it) · `it’s → It’S` (the apostrophe splits words; legacy-faithful quirk) · `abc3def → Abc3Def` · `ΣΊΣΥΦΟΣ → ΣΊΣΥΦΟΣ` (the guard is script-agnostic). Departure from legacy 1.x title-casing: §24 Q15.

f. **sanitize** — makes names Windows/NAS/cloud-safe. In order: (1) if `stripsDiacritics`, decompose NFD and drop nonspacing marks (`Mn`), recompose NFC; (2) per grapheme cluster: if it is one of `< > : " / \ | ? *` **or** every scalar in it is a control character → emit `text` (the replacement string; default `""` = delete); else if `removesEmoji` and any scalar has `Emoji_Presentation`, or has `Emoji` with codepoint ≥ U+1F300 → drop it; else keep it; (3) strip trailing `.` and space characters (repeat until neither); (4) **reserved-name guard**: if the result's segment before the first `.` case-insensitively equals a Windows reserved device name (`CON PRN AUX NUL COM1–COM9 LPT1–LPT9`), append the replacement string (or `_` when it is empty) to that segment — `CON.txt → CON_.txt`, `nul → nul_`. The guard runs on every OS; the rule's promise is portability. Runs on the in-scope string per §5.2 (extension untouched by default). Display title: **Fix Unsafe Characters** (§14.3); the serialized `kind` stays `sanitize`.

g. **changeExtension** (files only, `text` non-empty) — `new_ext = expand_tokens(text)` trimmed of surrounding whitespace, then strip **all** leading dots. `base` = whole name if no extension, else stem. Result: `base` if `new_ext` is empty, else `base + "." + new_ext`. A file without an extension **gains** one.

h. **numberSequentially** — `start = clamp(numberStart, 0, 99_999)`; `n = idx + start − 1` where `idx = restartPerFolder ? ctx.folder_index : ctx.index` (both 1-based, §7.3; when ctx is absent use 1); `width = clamp(numberPadding, 1, 10)`; `num = zero-pad(n, width)` (numbers wider than `width` are not truncated). `sep = expand_tokens(text)`. Position `before` → `num + sep + s`; `after` → `s + sep + num`; `replaceName` → `num` alone. The clamps are load-bearing: hand-edited preset files must not overflow or allocate huge strings.

i. **template** — `expand_tokens(text)` replaces `s` entirely. With default scope the extension is preserved automatically.

### 5.4 Whitespace trim (global post-pass)

When `trimsWhitespace` is on, after all rules run: directories → trim leading/trailing whitespace of the whole name. Files → trim the whole name, then re-split and trim the stem's trailing/leading whitespace, rejoin (`"  draft .txt  "` → `draft.txt`). Implement as `trimmed_name(name)` and test both branches.

---

## §6 Tokens

Tokens are `{key}` or `{key:argument}` placeholders expanded per file. They work in: templates, prefixes, suffixes, the Find & Replace / Regex **replacement** fields, the number **separator**, and the Change Extension field — i.e., every field routed through `expand_tokens` in §5.3. They are **not** expanded in search needles or regex patterns.

### 6.1 Grammar & expansion (normative)

- Token syntax: `\{([^{}]+)\}` — braces cannot nest. Split the inner text on the **first** `:`; left of it is the key, right of it (may itself contain `:`) is the argument. An all-colon degenerate token like `{:}` has key `""` → unknown.
- **Single pass, no re-expansion:** expanded values are never rescanned for tokens (a file literally named `{n}.txt` cannot inject).
- **Unknown tokens are left in place verbatim** — so typos stay visible in the preview. This is a feature; never "helpfully" strip them.

### 6.2 Token table

`ctx` = `TokenContext { index: u32 /* 1-based, §7.3 */, path, metadata }`. When a value is unavailable (no ctx, metadata missing), the token resolves to **unknown** (left in place) — with the exceptions noted.

| Token | Value |
|---|---|
| `{name}` | The stem being transformed at this rule's stage (post-prior-rules). |
| `{ext}` | The extension (no dot); empty string if none (expands to empty, not left in place). |
| `{folder}` | Name of the containing directory. |
| `{n}` / `{n:W}` | Sequence number, zero-padded to `clamp(W, 1, 10)` digits (default 1; non-numeric W → 1). Without ctx: 1. |
| `{created}` / `{created:FMT}` | File creation time formatted per §6.4 (default pattern `yyyy-MM-dd`). Unknown if the filesystem lacks birth time (some Linux FS — §24 Q16). |
| `{modified}` / `{modified:FMT}` | Modification time, same formatting. |
| `{date}` / `{date:FMT}` | Now (local time). Always resolves. |
| `{size}` | Human-readable file size per §6.5. |
| `{kind}` | Human-readable file kind from the metadata provider (§11). |
| `{md:Attribute}` | Provider-namespaced metadata attribute (§11.3); unknown if absent. |

### 6.3 Sequence numbering contract

`index` is the file's 1-based position **among selected items in the current view order** (§7). Deselected items don't consume numbers. Manually-overridden items **do** consume a number (their index is assigned before the override short-circuits). This exact behavior is tested.

`folder_index` is the same counter keyed by parent directory (each directory's selected items count 1, 2, 3… in view order). Only Number Sequentially's `restartPerFolder` option reads it; the `{n}` token always uses the global `index`. Per-folder restart pairs naturally with Folder sort but does not require it.

### 6.4 Date formatting — LDML patterns

User-facing date patterns are Unicode LDML style (the app's docs and existing user presets use `yyyy-MM-dd HH.mm`). Implement `format_date(dt: DateTime<Local>, pattern: &str) -> String` supporting this subset, longest-match-first tokenization:

`yyyy` `yy` `MM` `M` `dd` `d` `HH` `H` `hh` `h` `mm` `m` `ss` `s` `a` (AM/PM) `EEEE` `EEE` `MMMM` `MMM`. Characters inside `'…'` are literal (with `''` = one quote); any other character passes through unchanged; unrecognized pattern letters pass through as literals. Rendering is **locale-independent**: English month/day names, Gregorian calendar, host-local timezone. Default pattern everywhere: `yyyy-MM-dd`. Vectors: `2026-07-27 18:04` with `yyyy-MM-dd HH.mm` → `2026-07-27 18.04`; `d/M/yy` → `27/7/26`; `EEE MMM d` → `Mon Jul 27`.

### 6.5 Size formatting

1000-based decimal units like macOS: `0 bytes` → `Zero bytes`; `1..999` → `N bytes`; then KB, MB, GB, TB with at most one decimal, trailing `.0` dropped (`1.2 MB`, `12 MB`, `3 GB`). Locale-independent (`.` decimal separator).

### 6.6 Metadata laziness (invariant)

Constructing a `TokenContext` performs **no IO**. Stat/metadata reads happen only when a token (or the inspector) actually asks — a preview over 10k files with no date/metadata tokens must not touch the disk per-file. Enforce with a test that runs a preview against paths that don't exist and asserts no error and no stat (mock/count via the provider trait).

---

## §7 The preview pipeline

`compute_preview(state) -> Vec<PreviewEntry>` in `crates/engine/preview.rs`. Pure given its inputs + an injected `DirectoryLister` (for on-disk name sets) + `PlatformProfile` (§16).

```rust
pub struct PreviewEntry {
    pub id: Uuid,                 // == FileItem.id
    pub current_name: String,
    pub new_name: String,
    pub is_selected: bool,
    pub has_override: bool,
    pub is_changed: bool,         // new_name != current_name
    pub problem: Option<Problem>, // §7.6
}
pub enum Problem { EmptyName, InvalidCharacters, NameTooLong, ReservedName, EndsWithDotOrSpace,
                   DuplicateTarget, ExistingFileCollision, UnrenamableName /* §24 Q17 */ }
```

### 7.1 Scope

Only `active_items` — items whose `is_directory` matches the current `list_mode` (Files ↔ files, Folders ↔ directories). Everything user-facing (preview, counts, selection ops, Apply, CSV) is scoped to the active mode. The other mode's items, selection, and overrides are untouched.

### 7.2 Ordering

Sort `active_items` by `sort_key`, then reverse the whole result if `!sort_ascending` (direction applies to every key):

- `OrderAdded` — insertion order (stable).
- `Name` — natural/Finder-style compare: case-insensitive, numeric-aware (`file2 < file10`), via `icu_collator` with numeric collation on.
- `FileExtension` — lowercased extension (natural compare), ties by Name. Not offered in Folders mode; switching to Folders while it's active resets `sort_key` to OrderAdded.
- `Folder` — parent directory path (natural compare), ties by Name.
- `DateCreated` / `DateModified` — read each item's date **once up front** (never inside the comparator — that was a real performance bug), missing date sorts as distant-past; ties by Name.

### 7.3 Name computation per item (in sorted order)

```
counter = 0; folder_counter = {}                            # global + per-directory (§6.3)
for item in sorted(active_items):
    if !item.is_selected: emit (item, item.name); continue     # keeps name, consumes nothing
    counter += 1                                               # overrides DO consume numbers
    folder_counter[item.directory] += 1
    if let Some(ov) = item.override_name: emit (item, ov); continue
    ctx = TokenContext { index: counter, folder_index: folder_counter[item.directory], path, lazy metadata }
    name = item.name
    for rule in rules where rule.isEnabled && is_effective(rule):   # stack order, top→bottom
        name = apply_rule(rule, name, ctx, item.is_directory)
    if trims_whitespace: name = trimmed_name(name)              # §5.4
    emit (item, name)
```

`rule_derived_name(id)` — the same computation for one item **ignoring its override** (used by the UI to refuse to pin an override that merely restates the rules' output, §14.4). Deselected/unknown id → current name.

### 7.4 Auto-resolve pass (only when `auto_resolves_conflicts`)

Appends ` 2`, ` 3`… to colliding proposed names instead of blocking Apply. Semantics (order matters):

1. Collision keys use `diff_key(directory, name)` — §16.2 (per-platform fold/normalize).
2. Selected items **keeping their current name claim it** (`taken`); selected items changing names **vacate** their old keys (`vacated = old keys − taken`).
3. Walk items in view order. For each selected, changed, non-empty proposed name: it is *free* if its key is not in `taken` **and** (its key is in `vacated` **or** it does not exist on disk in that directory). If not free: `base`/`ext` split (directories: whole name, suffix appended at the end), try `base 2.ext`, `base 3.ext`… stopping below 1000 (still-colliding names fall through to §7.6 flagging). Claim the resolved key in `taken`.

First-come-first-served in view order; files keeping their names always win.

### 7.5 On-disk name sets (the `DirectoryLister` cache)

Collision checks need each active directory's on-disk names. List each directory **once**, store as a set of `diff_key`-folded names, and cache until the file list itself mutates (imports, rescans, apply) — **never** relist per keystroke; editing a rule must cost zero stat calls (this beachballed the app historically on network volumes). Known, accepted staleness: external changes to *unwatched* directories can make a collision verdict stale until the next list mutation; Apply's two-phase executor still fails safely on a real collision, so this is a preview-accuracy gap, not a data-loss path. Document this exact tradeoff as a comment at the cache site.

### 7.6 Problem detection (per selected item; first match wins)

Let `changed = new_name != current_name` — an unchanged item's name is valid by definition (it already exists on disk), so **intrinsic checks apply only when `changed`**; a stale `:` in an existing name must not be flagged (real false-positive fixed historically). Checks:

1. `changed` and new name is `""`, `"."`, or `".."` → **EmptyName**
2. `changed` and contains chars invalid on this platform (§16.1) → **InvalidCharacters**
3. `changed` and name exceeds the platform length limit (§16.3) → **NameTooLong**
4. `changed` and platform reserves the name (Windows `CON`, `NUL`, … — §16.1) → **ReservedName**
5. `changed` and platform forbids trailing dot/space (Windows) → **EndsWithDotOrSpace**
6. Target key count > 1 among **all selected items'** targets (unchanged items claim their current key here, so renaming *onto* a selected-but-unchanged file is caught) → **DuplicateTarget**
7. `changed`, target key not among any selected item's **current** key (a name being vacated in the same batch is fine — two-phase makes swaps safe), and the name exists on disk in that directory → **ExistingFileCollision**

Every problem blocks Apply. Human-readable messages in §A. Deselected items never carry problems.

### 7.7 Derived counts (status bar / gating)

`selected_count`, `change_count` (entries with `is_changed`), `conflict_count` (entries with a problem), `can_apply = change_count > 0 && conflict_count == 0`, and `apply_disabled_reason` (§A) when items exist but `can_apply` is false.

### 7.8 Caching (invariant)

The preview is read by several UI surfaces per frame and is expensive. Cache the computed `Vec<PreviewEntry>` keyed on `CoreState.version`; **every** state mutation goes through one funnel that bumps `version` (§13.1), so stale previews are structurally impossible. Never add a second mutation path.

### 7.9 Per-rule impact counts

While running §7.3, count for every enabled+effective rule how many items it actually changed (`name_before != name_after` at that rule's stage; deselected and overridden items never count). Expose on the preview payload: `rule_impact: Vec<RuleImpact { rule_id: Uuid, affected: u32, eligible: u32 }>` (`eligible` = selected, non-overridden active items). Cost is one string compare per rule per item — it must fit inside the §17 keystroke budget (bench it). Drives the rule-card `affects 14 of 20` header; `affects 0` renders in the card's warning slot and is the instant tell for a valid-but-matchless regex.

---

## §8 Apply & revert — the two-phase move engine

`crates/engine/{plan,execute,revert}.rs`. This is the most safety-critical code in the app; port it with total fidelity.

### 8.1 The plan

`build_plan(state, preview)` — requires `can_apply`; moves = every changed entry `(from: item.path, to: directory/new_name)`. Snapshot summary = ` · `-joined: each active rule's `summary` (§4.2), then `Trim spaces` if on, then `Manual edits` if any changed entry had an override, with `Folders` **prepended** in Folders mode.

### 8.2 Hierarchical execution

```
perform_moves_hierarchical(moves, parents_first, is_cancelled, on_progress)
    -> { succeeded: Vec<(from, to)>, errors: Vec<String> }
```

Group moves by **path depth** (component count of `from`). Apply processes shallow→deep (`parents_first = true`); revert processes deep→shallow. After each depth group completes (parents-first only), record `(old_path → new_path)` **prefix rewrites** from its successes and apply them to all deeper pending moves' `from` **and** `to` paths (exact-match or `old + separator` prefix) — renaming a parent mid-batch re-bases its descendants' pending moves, which is what makes recorded history entries always match the final on-disk layout. Within one depth group, run the two-phase mover (§8.3). Check `is_cancelled` between groups.

### 8.3 Two-phase mover (per depth group)

Temp names: `TEMP_PREFIX = ".fne-tmp-"` (keep this exact historical prefix — it lets orphan recovery also rescue temps left by earlier releases). Temp name = `.fne-tmp-<UUID>-<originalName>`, same directory; if that exceeds 255 bytes UTF-8, fall back to `.fne-tmp-<UUID>` (unrecoverable-by-name but still correct). On Windows additionally set the hidden file attribute on temps (dot-prefix doesn't hide there); clear it on the final move.

- **Phase 1 — stage:** move every `from` to its temp. Per-item failure → record error `Couldn’t rename “{name}”: {os error}`, continue. Cancel during phase 1 → move all staged temps back to their originals and return with nothing succeeded (a phase-1 cancel renames **nothing**).
- **Phase 2 — commit:** move each temp to its final `to`, recording success and throttled progress (emit every ~1% of items and on the last). Per-item failure → error `Couldn’t rename “{from}” to “{to}”: {os error}` and roll that one temp back to its original. Cancel during phase 2 → committed moves **stay** (recorded + revertible); for each uncommitted temp, first try moving back to its original; if the original name is occupied (its swap partner already committed), **complete the move forward** to its target and count it succeeded — never strand a file at a temp name; if both fail: error `Couldn’t finish renaming “{name}”; it is safe at “{temp}”.`

Two-phase is why sibling swaps (`a↔b`) and case-only renames (`readme → README` on case-insensitive filesystems) can never collide. Do not "optimize" it away for small batches.

### 8.4 Orphan temp recovery

`recover_orphaned_temp_files(dir) -> usize`: for each entry named `.fne-tmp-<36-char-UUID>-<original>` (validate the UUID), if `<original>` is non-empty and free in that directory, move it back. Never clobber an existing file. Run it on: importing loose files (once per distinct parent directory) and adding a watched folder — but **skip entirely while a batch is processing** (it would grab the live batch's phase-1 temps; historical bug). Watched-folder rescans must **ignore** `.fne-tmp-*` entries so mid-batch FSEvents don't import temps.

### 8.5 Finish-apply (single-threaded, post-move bookkeeping — order is normative)

1. For each success, update the matching tracked item's path (`from` → `to`).
2. `rewrite_live_paths(successes)`: for each move, prefix-rewrite (`==` or `prefix + sep`) every tracked path — item paths, `excluded_paths`, watched-folder roots (re-arming each affected folder's watcher on its new path, same folder id). **No "does the old path still exist" guards** — batch ordering makes existence checks wrong (hard-learned; a CLAUDE.md invariant). A file's path can never prefix another tracked path, so unconditional rewriting is safe.
3. If any success: insert `Snapshot { id, now, summary, entries: successes }` at history position 0; truncate history to 50; persist.
4. Emit apply feedback `{ count, snapshot_id, is_folders, cleared_rules, kept_rules }` — the flags are true only if rules were actually non-empty (`had_rules`), so applying pure manual edits doesn't claim rules were cleared/kept.
5. `keep_rules_after_apply` on → deselect exactly the items whose (standardized) path is a `to` of a success (prevents double-transform on re-Apply). Off → clear the rule stack **through the undoable replace-rules funnel** (§13.3, action name `Clear Rules`) and set `rules_cleared_by_apply = had_rules`.
6. Clear `override_name` on every item of the applied mode only (the other mode's pending edits survive).
7. Surface errors: at most the first 8 messages + `…and N more.` under title `Some renames couldn’t complete`.
8. Save session.

### 8.6 Concurrency & progress

Exactly **one** Apply/Revert in flight, enforced at the state layer (`processing != None` → reject; the toolbar double-click race is real). Apply runs on a worker thread; cancellation via a shared `AtomicBool`; progress events `{ title, completed, total }` with titles `Renaming files…` / `Renaming folders…` / `Reverting…`. The synchronous engine function is the tested reference path; the async wrapper adds only threading, progress, and cancel.

### 8.7 Revert

`revert_through(snapshot_id)`: for each snapshot from newest through the selected one, execute the **reversed** moves (each recorded `from→to` becomes `to→from`, entry order reversed) with `parents_first = false` (children first — nested batches undo inner renames before their parents, whose recorded paths become valid again). After each snapshot: fold successes into live state (§8.5 steps 1–2), collect errors. Fully-reverted snapshots are removed from history; a snapshot only **partially** reverted due to cancel stays in history (its already-reverted entries will simply skip as missing sources on a later re-revert). Then persist history + session, surface errors. Reverting an older snapshot always reverts every newer one first — the UI confirmation states this.

### 8.8 Revert preview (simulation)

`compute_revert_preview(history, selected_id)` simulates the revert **without touching disk**, per snapshot newest→selected:

- Maintain `completed_moves: Vec<(from, to)>` of simulated successes. Virtual existence check `exists(path)`: walk completed moves newest-first; if `path` equals or is under a move's `to`, remap to the `from` side (directory moves carry their contents); if it equals or is under a `from`, it's been moved away → false; else check the real filesystem.
- Per snapshot, over its reversed entries: target `to`, restored name `from`. Status: **missing** if `!exists(to)`; else **nameTaken** if `from ≠ to` (compare case-insensitively), `from` is not being vacated by another entry of the *same* snapshot (two-phase makes same-snapshot swaps safe), and `exists(from)`; else **ok**. Record ok moves into `completed_moves` (reversed).
- Emit `RevertEntry { id: "{snapshotId}-{offset}", snapshot_id, snapshot_date, directory_path, current_name, restored_name, status }`.

Derived counts: `revert_restorable_rename_count` = ok entries; `revert_restorable_file_count` = ok **chain heads** (ok entries whose current path is no other ok entry's restored path — N versions touching one file is 1 file, not N); `revert_name_taken_count`; `newer_snapshot_count` = index of the selected snapshot. Cache on `(state.version)` like the preview. **Property test (§18.2): simulation verdicts must match what actually executing the revert does.**

---

## §9 Folder watching

`crates/watcher`. Importing a folder = watching it; its contents populate the list live.

- One recursive watcher per watched root (`notify` recommended watcher + `notify-debouncer-full`, ~400 ms debounce). Any event under a root (including rename/removal of the root itself — watch the root's parent non-recursively if the backend can't report root-level events) triggers a **rescan** of that root.
- **Rescan semantics** (`scan_folder(root, recursive)`): list regular files, and directories (Folders-mode targets); the root itself is never a target. Non-recursive (default) lists immediate children only; `include_subfolders` walks the whole tree but **prunes macOS bundle directories** (do not descend into directories whose name ends in a known bundle extension: `.app .bundle .framework .photoslibrary .fcpbundle .imovielibrary .band .logicx` — macOS only; no-op elsewhere). Skip hidden entries (dotfiles everywhere; hidden-attribute files on Windows) and anything matching `.fne-tmp-*`. Sort results by path (natural compare).
- Reconcile: drop tracked items of this root that vanished, add new ones (`folder_id` set, selected by default), skip `excluded_paths`. Existing items keep their id/selection/override.
- **Effective depth per root** = `folder.include_subfolders.unwrap_or(global)`. Each chip's menu offers Follow Global / Include Subfolders / This Folder Only. Toggling the global rescans roots that follow it; setting a per-folder value rescans that root. Removing a watched folder drops its watcher and its discovered items (guarded + undoable per §14.9). Re-importing an already-watched root just rescans it.
- **Deallocation invariant:** dropping a `WatchedFolder`'s handle must tear down the OS watcher (Drop impl). A test asserts no watcher survives removal (e.g., events after removal don't fire; use a counter).
- If a watched root disappears or is renamed externally, the rescan naturally empties its items; keep the chip with its (stale) path — user removes it manually. The app's *own* renames of ancestors instead flow through `rewrite_live_paths`, which re-arms watchers on the new path (§8.5).

---

## §10 File access & sandbox architecture (MAS-ready)

The Mac App Store build runs sandboxed. **Design for it from day one** so MAS is a packaging flip, not a rewrite. Direct, Microsoft Store, and Flathub builds use the plain path implementation.

### 10.1 The `FileAccess` seam

```rust
pub trait FileAccess: Send + Sync {
    /// A persistable token for a path the user granted. Direct builds: the path itself.
    /// MAS builds: a security-scoped bookmark blob.
    fn persist_token(&self, path: &Path) -> Option<Vec<u8>>;
    /// Re-acquire access from a stored token at launch. Returns the (possibly moved)
    /// resolved path, having started security-scoped access where applicable.
    fn resolve_token(&self, token: &[u8]) -> Option<PathBuf>;
    /// Record a grant obtained via open dialog / drag-drop (MAS: create + start scope).
    fn note_user_granted(&self, path: &Path);
    fn stop_all(&self);   // release scoped resources on shutdown
}
```

- `DirectAccess` (default feature `channel-direct`, also msstore/flathub): tokens are UTF-8 path bytes; every method trivial.
- `ScopedAccess` (`channel-mas`, macOS only): implemented with `objc2` NSURL APIs — `bookmarkDataWithOptions: .withSecurityScope`, resolve with `.withSecurityScope`, then `startAccessingSecurityScopedResource`. Balance stop/start; hold live scopes only for watched roots and direct-imported files' parent directories (there is an OS cap on simultaneously-open scoped resources — release scopes for items removed from the list). Stale bookmark on resolve → drop the entry and count it toward the `Some files were missing` alert.

Session integration: the optional `bookmark` / `watchedFolderBookmarks` fields (§4.3) store `persist_token` output base64-encoded; restore resolves tokens **before** path-existence checks. **Invariant for future code: any newly persisted path must go through `persist_token`** (CLAUDE.md, §22.3).

### 10.2 MAS behavioral rule — renaming needs the *directory*

Renaming is a move within the parent directory; a sandbox grant on a *file* alone doesn't permit it. Therefore, in MAS builds only:

- Folder imports (the primary flow) are already directory grants — everything works.
- Loose-file imports (open panel or drag-drop): after import, if the files' parent directory isn't covered by an existing grant, immediately present a directory-grant open panel pre-navigated to that parent with message `To rename these files, allow access to their folder.` and prompt `Allow`. Declining leaves the files listed but flags their rows with problem **UnrenamableName**-style messaging: `Name Shift doesn’t have permission to rename items in this folder.` (define a distinct `Problem::NoFolderPermission` for this — MAS builds only).
- Entitlements: `com.apple.security.app-sandbox`, `com.apple.security.files.user-selected.read-write`, `com.apple.security.files.bookmarks.app-scope` (§20.3).

All of this is compiled out of non-MAS builds (`#[cfg(feature = "channel-mas")]`).

### 10.3 Flatpak

Ship with `--filesystem=host` (plus wayland/x11 sockets, dri) and a manifest comment justifying it: the app's whole purpose is renaming arbitrary user files, and portal-scoped access cannot follow live folder watching + revert across sessions. Revisit portals if Flathub review pushes back (documented fallback in `packaging/flatpak/`).

---

## §11 Metadata providers

`crates/metadata`. Powers `{created}` `{modified}` `{size}` `{kind}` `{md:…}` and the File Info inspector.

### 11.1 Trait

```rust
pub trait MetadataProvider: Send + Sync {
    fn created(&self, p: &Path) -> Option<SystemTime>;
    fn modified(&self, p: &Path) -> Option<SystemTime>;
    fn size(&self, p: &Path) -> Option<u64>;
    fn kind(&self, p: &Path) -> Option<String>;                       // "JPEG image", "Folder", "PNG image"…
    fn attribute(&self, p: &Path, key: &str) -> Option<AttrValue>;    // {md:key}
    fn all_attributes(&self, p: &Path) -> Vec<(String, String)>;      // inspector list, sorted by name, stringified
}
pub enum AttrValue { Str(String), Num(f64), Bool(bool), Date(SystemTime), List(Vec<AttrValue>) }
```

Per-file access is wrapped in a **lazy handle** — one stat batch for created/modified/size on first request, cached for the handle's life; `attribute`/`all_attributes` hit the platform store only on demand (§6.6). Stringification rules (shared, tested): booleans → `Yes`/`No` (never 0/1 — the inspector and `{md:…}` must agree); dates → §6.4 default pattern; lists → `, `-joined; numbers → shortest round-trip decimal.

### 11.2 Platform implementations

- **macOS** — Spotlight via `MDItemCreate` / `MDItemCopyAttribute` / `MDItemCopyAttributeNames` (CoreServices C FFI). `{md:…}` keys are raw Spotlight names (`kMDItemPixelHeight`, `kMDItemAcquisitionModel`, …). `kind`: `kMDItemKind`, falling back to localized type description via extension.
- **Windows** — Windows Property System: `SHGetPropertyStoreFromParsingName` (`windows` crate), keys are canonical property names (`System.Image.HorizontalSize`, `System.Photo.CameraModel`, `System.Media.Duration`, …); enumerate the store for `all_attributes`, mapping each PROPERTYKEY to its canonical name (skip unnamed). `kind`: `System.ItemTypeText`.
- **Linux** — composed sources, namespaced keys: `exif:<Tag>` via kamadak-exif for images (`exif:Model`, `exif:DateTimeOriginal`…), `xattr:<name>` for user extended attributes, plus the stat basics. `kind`: humanized from extension via a small built-in table (`jpg → JPEG image`, `png → PNG image`, `pdf → PDF document`, ~40 common entries, fallback `EXT file` uppercased / `Folder`). `created`: btime where the FS provides it (statx), else None.

### 11.3 Cross-platform posture (documented, not hidden)

`{md:…}` keys are **platform-namespaced and non-portable by design** — a preset written on macOS with `{md:kMDItemPixelHeight}` resolves as *unknown token* on Windows (visible in preview, per §6.1) rather than guessing. The inspector is the discovery surface on every OS: it lists exactly the attributes available for the selected file *on this machine*, each with a copy-token button. README/help must state this plainly with a per-OS availability table.

---

## §12 IPC contract — commands & events

All commands live in `src-tauri/src/commands.rs` as thin marshaling shims over crate functions. Every payload type derives `TS` (§2). Errors cross the boundary as `AppError { title: String, message: String }` — already user-presentable (§A copy); the frontend shows them verbatim in the error alert.

### 12.1 Commands

State-reading:

| Command | Signature | Notes |
|---|---|---|
| `get_state` | `() -> StateSnapshot` | scalars + items + watched folders + counts + `version`; called at boot and on `state-changed` |
| `get_preview` | `(known_version: u64) -> PreviewPayload` | `{ version, entries, counts, rule_impact, apply_disabled_reason }`; serves from cache when version matches (§7.8–7.9) |
| `get_revert_preview` | `() -> RevertPreviewPayload` | entries + the derived counts of §8.8 |
| `get_history` | `() -> Vec<SnapshotMeta>` | id, date, summary, entry count |
| `get_file_metadata` | `(id: Uuid) -> InspectorPayload` | general info + `all_attributes` (§11) |
| `rule_derived_name` | `(id: Uuid) -> String` | §7.3 |
| `get_platform` | `() -> PlatformInfo` | os, channel, capabilities (has_spotlight etc.) — drives per-OS UI copy |

State-mutating (each bumps `version` and emits `state-changed`; grouped by domain):

- Import/list: `import_paths(paths: Vec<String>)` (files→direct import + un-exclude + orphan recovery per §8.4; dirs→watch), `pick_and_import()` (open dialog, §14.9), `remove_items(ids)`, `remove_watched_folder(id)`, `clear_all()`, `rescan_watched_folders()`.
- Inclusion (checkboxes): `set_selected(id, bool)`, `set_selected_many(ids, bool)` (shift-click ranges — one mutation, one undo entry), `set_all_selected(bool)`, `select_only(ids)`, `deselect_conflicted()`. Row *selection* is frontend-only ui state (§13.1); it never crosses IPC.
- Rules: `set_rules(rules: Vec<RenameRule>)` (full-array idempotent set from the editing UI — **not** undo-recorded), `replace_rules_undoable(rules, trims, action_name)` (delete rule / clear stack / load preset — §13.3), `undo()`, `redo()` (one workspace stack — §13.3), `set_trims_whitespace(bool)`.
- Options: `set_auto_resolve(bool)`, `set_keep_rules(bool)`, `set_include_subfolders(bool)`, `set_watched_folder_subfolders(id, Option<bool>)`, `set_sort(key, ascending)`, `set_list_mode(mode)`.
- Overrides: `set_override(id, name: Option<String>)` (trim; empty→clear), `clear_all_overrides()` (active mode only; one undoable action `Clear All Manual Edits`).
- Apply/revert: `apply() -> ()` (spawns worker; progress + `apply-finished` events), `select_snapshot(id: Option<Uuid>)`, `revert_selected() -> ()` (worker + `revert-finished`), `cancel_processing()`, `clear_history()`.
- Presets: `save_preset(name, resolution: "replace" | "keepBoth")`, `preset_name_exists(name) -> bool`, `apply_preset(id)` (regenerates rule ids; undoable, action `Apply Preset`), `delete_preset(id)`, `import_presets()` / `export_presets()` (dialogs; §14.8 behaviors).
- Spreadsheet: `export_csv_template()` (save dialog; template per §14.7), `csv_dry_run(path: String) -> CsvMatchReport` (parse + match, **zero mutation** — powers the modal's results), `csv_apply(matches: Vec<CsvMatch { id, new_name }>)` (sets overrides + selects; one undoable action `Import Edited CSV`), `copy_preview_tsv()` (clipboard).
- Misc: `reveal_in_file_manager(id)` (opener plugin: Finder/Explorer/file manager), `open_help(topic: Option<String>)`, `save_session_now()`.

### 12.2 Events (Rust → frontend)

| Event | Payload | When |
|---|---|---|
| `state-changed` | `{ version: u64 }` | after every mutation (watchers included) — frontend refetches, debounced ~30 ms |
| `processing` | `{ title, completed, total } \| null` | worker progress; `null` = done |
| `apply-finished` | `{ count, snapshot_id, is_folders, cleared_rules, kept_rules }` | drives the feedback banner |
| `revert-finished` | `{ restored: u32 }` | |
| `alert` | `{ kind: "error" \| "info", title, message }` | engine-originated alerts (watcher errors, restore notices) |
| `open-paths` | `{ paths: Vec<String> }` | single-instance forward, macOS dock/Open-With → frontend calls `import_paths` unless processing |

Threading: watcher callbacks and workers lock the same `Mutex<CoreState>`; hold locks briefly (compute outside, commit inside). No command may block on the watcher debounce thread.

---

## §13 Frontend architecture & state management

### 13.1 Single source of truth

Rust owns all domain state. The frontend holds a **mirror** (Zustand `appState`) refreshed from `get_state`/`get_preview` on `state-changed`, plus ephemeral `uiState` (active side tab, filter text, `filter_mode`, inspector open/inspected id, search-focus flag, dialogs, drag-over, banner). Mutations are fire-and-forget command calls; the authoritative echo returns via `state-changed`. Only rule-field **typing** is optimistic: edits apply to a local draft immediately and sync via debounced (~120 ms) `set_rules`; the draft reconciles to authoritative state whenever an undo/redo/preset/apply changes rules underneath it (compare a rules-revision counter included in `StateSnapshot`).

Version discipline: responses/events carry `version`; the store ignores anything older than what it has (out-of-order guard). Preview fetches are keyed by version — a stale response is dropped, never rendered.

### 13.2 The `version` funnel (invariant)

In Rust, all mutations go through `AppState::mutate(|s| …)` which bumps `s.version`, invalidates preview/revert caches, and emits `state-changed`. **Never mutate `CoreState` outside `mutate`.** This replaces the legacy pattern of per-property cache-invalidation hooks with something structurally un-forgettable — the single most important architectural carry-over.

### 13.3 Undo/redo — one workspace stack

An in-Rust undo stack (cap 100) of tagged deltas, each with an `action_name` for the Edit menu (`Undo {action_name}`):

- **Rules delta** `{ rules, trims_whitespace }` — recorded by rule delete, clear-stack, preset load, and the post-apply clear. **Never** by field typing, reorder, or enable-toggle (text-input undo belongs to the OS text machinery).
- **List delta** `{ files, watched_folders, excluded_paths }` — recorded by remove item(s), bulk removals, clear list, stop-watching, `clear_all_overrides`, and `csv_apply` (the whole import = one entry).

Undoing a list delta restores selection and overrides byte-for-byte and **re-arms watchers** for restored roots (the §9 Drop-teardown invariant makes recreate safe — test this with a post-undo rescan). Restoring rules clears `rules_cleared_by_apply`. Apply/Revert (disk mutations) are never on this stack — History is their undo. This generalizes the legacy rules-only stack and is what makes the §14.0 destructive policy honest: every ✕ in the app is recoverable.

### 13.4 React rules

- **Rule cards are keyed by `rule.id`** — never by array index. Positional identity crashed the legacy app when Apply cleared the array mid-interaction; encode this as a lint-visible comment and a regression test (§18.3).
- File rows keyed by `item.id`; virtualized (§2); row height fixed for virtualization.
- The `NameDiff` computation (§14.6) is memoized per `(old, new)`.

---

## §14 UI specification

A two-pane split layout in a single main window (min 920×540). Match platform feel: system font stack, native-density controls, light/dark from the OS. All interactive elements keyboard-reachable and labeled for screen readers; the apply-result banner is announced (aria-live). This layout is the UX audit's north star: **decision info and the primary action share the bottom action bar; scope is a first-class tab pair; row selection and checkbox inclusion are distinct, named concepts.**

### 14.0 Style rules (normative, app-wide)

- **Button tiers.** T1 — filled capsule, exactly one per context: **Rename N Files** (accent) or **Revert…** (`--revert-action`). T2 — bordered: secondary commands (Skip Conflicted, Select Only These, { } Tokens, sheet buttons). T3 — plain glyph **with a hover background** and ≥ 22×22 px hit area: utilities (reorder, ✕, ⧉ copy). No bare-text buttons anywhere; disabled appearance comes only from the disabled state, never from a lighter default style.
- **Removal icons.** ✕ = remove from a collection, always undoable (§13.3). 🗑 = destroy stored data, always confirmed. Never both meanings for one glyph.
- **Destructive policy.** Single item → instant + undoable. Bulk (> 10 items, or any manual edit affected) → confirm with real counts (§A). No third category.
- **Text contrast.** Meaningful text ≥ 4.5:1 in both themes; muted styling is for decoration only. Interactive glyphs ≥ 3:1. State/warning text never relies on hue alone.
- **Theme tokens** (`src/styles/theme.css`, CSS custom properties — the only file where raw color values may appear; audit UI-05):

| Token | Light | Dark | Use |
|---|---|---|---|
| `--accent` | system accent (fallback #0B57D0) | same | T1 Rename, selection bar, links |
| `--link-text` | #0B57D0 (≥ 4.5:1 on the bar) | system accent | clickable counts |
| `--warning-text` | #A34A00 | systemOrange-equivalent | conflict counts, inline ⚠, revert footer |
| `--success-text` | #1E7F4F | green ≥ 4.5:1 | sheet "will change" count |
| `--revert-action` | #A34A00 fill / white text | same | the Revert T1 — one color = one verb, app-wide |
| `--wash-selected` | accent @ 0.18 | @ 0.24 | selected rows |
| `--wash-conflict` | yellow @ 0.14 | @ 0.20 | conflict rows |
| `--wash-hover` | fill @ 0.06 | @ 0.10 | hover |
| `--radius-card` / `--radius-overlay` | 10px / 14px | same | cards / overlays |

Diff tint tokens live with §14.6.

### 14.1 Window structure

```
┌ Toolbar: [＋ Add] [⋯ Options] [ⓘ File Info]                       [🔍 Filter…] ┐
├──────────────┬─────────────────────────────────────────────────────────────────┤
│ Side panel   │ [Files · 20] [Folders · 4]   Watching: Iceland ✕    [Sort ▾ ⇅]  │
│ [Rules|Hist] │ ☑ │ Current Name ↓        →   New Name        (resizable split) │
│ 320–460 px   │ …rows (FileListPane) — or — RevertPreviewPane   [Inspector ▸]   │
├──────────────┴─────────────────────────────────────────────────────────────────┤
│ 18 of 20 included · 17 will change · ⚠ 1 naming conflict  [Skip Conflicted]    │
│ [Auto-resolve: Off]                             1 edited   ▐ Rename 17 Files ▌ │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- **The action bar (bottom) owns the primary action** (§24 Q27). Left → right: live counts (each a click-toggle filter, §14.2), remedies (`Skip Conflicted` T2; `Auto-resolve: On|Off` toggle chip), then `{N} edited` chip and the T1 primary. When items exist but Apply is disabled, the §A reason renders beside the button. Collapse priority at narrow widths: disabled reason → "Showing X of Y" → edited chip → Auto-resolve chip (into the ⋯ menu); counts and remedies never collapse.
- **Toolbar**: Add, Options, File Info, filter field. **No primary action in the toolbar.**
- **Options ⋯ menu — four titled sections** (destructive last): *Importing* — Include Subfolders (global); *Renaming* — Auto-Resolve Naming Conflicts · Keep Rules After Applying · Trim Spaces from Names; *Spreadsheet* — Rename by CSV…; *List* — Clear All Manual Edits ({N}) · Remove Skipped from List · Remove Included from List · Clear File List….
- Side panel: segmented `Rules (N)` | `History (N)` (counts hidden at zero). Selecting a history version switches the detail pane to the revert preview and the side tab to History.
- Whole window is a drop target (accent inset ring). One import gate for every entry point (drop, ⌘O, second-instance, OS open-with): **processing → refuse**; idle revert preview → auto-dismiss the preview, then import.
- **Revert preview mode**: Add/Options disabled; the action-bar primary becomes **Revert…** in `--revert-action` with an undo-arrow icon. Revert must never wear Rename's blue.
- Processing overlay: appears only after 350 ms; determinate `X of N`; Cancel bound to Esc.
- Apply feedback banner (bottom-center capsule above the action bar, auto-dismiss 6 s, aria-live): `Renamed N files` + cleared/kept suffix + `Revert…` + dismiss ✕.

### 14.2 FileListPane

**Two concepts, named everywhere** (the audit's one Critical finding): **Selection** = highlighted rows — ephemeral, frontend-only; drives the inspector, keyboard operations, and context-menu scope. **Inclusion** = the checkbox — domain state, the only thing Rename reads; excluded rows read "Skipped".

- **Scope tabs** `Files · N` | `Folders · N`: first-class tabs above the list, never styled like the filters (they change what Apply *does*). Switching swaps list, counts, and the primary button's noun. Extension sort is hidden in Folders mode (§7.2).
- **Watched-folder chips** sit inline in the scope row when ≤ 2, else on their own row (chrome budget: ≤ 2 divider lines above the first row). Chip = folder name + depth menu (§9) + ✕ (guard per §14.0 policy; undoable).
- **Headers are real controls**: tri-state checkbox · `Current Name` (click sorts by Name, click again flips direction, caret shown; the Sort menu mirrors and never disagrees) · `New Name`. The split between the two name columns is **draggable** (persisted in ui state; default 50/50).
- **View ▸ Inline Diff View**: single-column density mode — one combined diff line per row (reuses §14.6 spans; middle-truncation keeps the changed span visible). For long media names where two truncated columns hide the actual change.
- **Row anatomy**: checkbox · icon · current name [· folder suffix] · → · [⚠] new name [✎] · hover ✕. **Folder suffix** (secondary, ~11.5px): only when the visible set spans > 1 directory; the name middle-truncates first, the suffix tail-truncates but keeps ≥ 12 chars.
- **Selection & keyboard** (listbox semantics, roving focus): click selects; ⌘-click toggles; ⇧-click extends (visible order); ⌘A selects all rows. On the selection: **Space** toggles inclusion, **Return** edits the focused row's new name, **Delete** removes from the list (undoable), ↑/↓ move focus (⇧ extends). Focus ring always visible. Screen readers announce name + included/skipped + conflict reason.
- **Checkbox ranges** (audit FEAT-02): ⇧-clicking a checkbox applies that checkbox's new state to the whole visible range from the last-toggled anchor. The anchor resets whenever the visible set or order changes; a filtered-out anchor is discarded, never remapped. One `set_selected_many` call = one undo entry.
- **Row visual precedence** (normative): conflict wash owns the background; selection renders as `--wash-selected` plus a 3 px leading accent bar *on top of* any wash; hover is suppressed while selected; a skipped row dims **content only** to 0.65 — never the checkbox, never the selection. `Skipped` renders as a full-opacity pill (min-width, never truncates — the name column truncates first).
- **Conflicts**: an inline ⚠ (`--warning-text`) sits immediately **before the new name** carrying the §A reason as tooltip, and **survives hover** (the hover ✕ uses the trailing slot). Conflict rows tint `--wash-conflict`.
- **Manual-edit affordance** (audit UX-02): the hover pencil is a real T3 button at secondary contrast — single click opens the editor, and its hit-test beats row selection (one click, one effect). A pencil badge marks overridden rows. Editing: Return commits; **focus loss commits** (Finder behavior); Esc cancels; committing text identical to `rule_derived_name` **clears** the override instead of pinning it.
- **Filtering**: the toolbar field matches substrings of current or new name; the action-bar counts toggle Will Change / Naming Conflicts / Manual Edits filters (active = filled chip with ✕ in the bar). While any filter is active, a trailing `{N} matching ▾` menu appears at the bar's edge (stable geometry — Sort/Show controls never shift) holding **Select Only These** and **Remove These from List** (guarded per §14.0). Esc clears the filter (§15 priority). Filtering changes visibility only — never what Rename does.
- Empty states: no items → drag/drop prompt + the Add shortcut; filter hides everything → `No files match the filter` + Clear Filter.
- Context menu (acts on the selection): Edit New Name…, Clear Manual Edit, Copy Name, Copy New Name, Copy Path, Remove from List, Reveal in Finder / Show in Explorer / Show in File Manager.

### 14.3 RulesPanel

- **The stack**, top to bottom: rule cards → a **dashed ghost card `＋ Add Rule…`** (it sits exactly where the next rule will land) → the pinned pseudo-card **`AFTER ALL RULES · ☐ Trim spaces from names`** (placement teaches execution order). Rules tab only.
- **Footer** (persistent while the stack scrolls — the constant affordance for long stacks): `＋ Add Rule` (same menu as the ghost card; deliberate redundancy) · `Presets ▾` · `{ } Tokens` (T2) · spacer · 🗑 Clear All Rules (confirmed + undoable). Adding a rule from either entry point scrolls the new card into view and focuses its first field.
- **Card header**: kind icon + title + `affects {A} of {E}` (right-aligned, secondary — from §7.9; `affects 0` renders in the warning slot instead) + enable switch (small size, ≥ 22 px) + drag grip (dnd-kit reorder; live preview during drag) + ↑/↓ buttons (keyboard parity) + ✕ (undoable). Context menu: Duplicate Rule, Move Up, Move Down, Delete.
- **Display titles** (serialized `kind` values unchanged): Remove Text · Find & Replace · Regex Replace · Add Prefix · Add Suffix · Change Case · Change Extension · **Fix Unsafe Characters** · Number Sequentially · **New Name from Template**.
- Per-kind fields: removeText `Text` (+ `Match case`); replaceText `Find` / `Replace with` (+ `Match case`); regexReplace `Pattern` / `Replacement` (+ `Match case`; invalid state = red outline + `This pattern isn’t a valid regular expression.` + rule inert + a `Syntax reference` link opening Help ▸ Rename Rules); addPrefix/addSuffix `Text`; changeCase segmented lowercase/UPPERCASE/Title Case; changeExtension `New extension` (placeholder `jpg`); sanitize `Replace unsafe characters with` + `Strip accents` + `Remove emoji`; numberSequentially Position + `Separator` + `Start`/`Digits` steppers (0–99999 / 1–10) + `☐ Restart numbering in each folder`; template `Template` (placeholder `{created} {name} {n:3}`).
- **Token-capable ⇔ { }** (the §6 list, zero exceptions — audit CON-08): every such field gets the trailing `{ }` insert-at-cursor menu with one-line descriptions; literal fields (Remove Text, Find, regex Pattern, sanitize replacement) say `exact text` in placeholder or tooltip instead.
- **No-input state**: `No input yet — rule is skipped` at secondary contrast with a hollow warning glyph — it explains why a rule does nothing; never render it in a decorative style.
- **Mode-aware cards** (audit CON-05): in Folders mode, `Include extension` is disabled with help `Folder names have no extension`, and Change Extension cards badge `Skipped for folders` in the warning slot. Switching back restores both.
- Per-rule `Include extension` checkbox (hidden for changeExtension).
- Empty states: default `No rules yet` + hint + `See Example Recipes` link (opens Help ▸ Recipes) with the ghost card beneath as the obvious first click; post-apply state per §A (reachable only when keep-rules is off). Any rule added/restored resets `rules_cleared_by_apply`.

### 14.4 Manual overrides

Pencil-marked; win over rules; consume sequence numbers (§6.3); cleared for the applied mode after a successful Apply; commit-guard via `rule_derived_name` (§14.2). Counted in the action bar's `{N} edited` chip (click = Manual Edits filter); bulk-cleared via **Clear All Manual Edits ({N})** (Edit menu + Options ⋯ List section; disabled at 0; mode-scoped; one undo entry).

### 14.5 HistoryPanel & RevertPreviewPane

- History rows: relative date (absolute tooltip), summary, `N renames`. Click → select (revert preview); click again/elsewhere → deselect. Footer: `Clear History…` (confirm; copy §A).
- RevertPreviewPane: orange banner summarizing `This will restore N files (M renames across V versions)` with the prominent **Revert…** button (styling per §14.1 — never accent blue); table rows: directory-grouped `current name → restored name` + status per §8.8/§A (`Will be restored` / `Not found (nothing to restore)` / name-taken); the summary calls out skipped and name-taken counts when non-zero.

### 14.6 Name diff highlighting (port exactly; implement in `src/lib/nameDiff.ts`)

Compute the longest common **prefix** and **suffix** (in Unicode grapheme clusters; suffix limited to `len − prefix`). Old name: middle span gets red-tinted background + strikethrough + secondary text color. New name: middle span gets green-tinted background; **underline only its non-whitespace runs** (underlining a space renders as an underscore — `chapter 2.md` must not read as `chapter_2.md`); a middle span that is *entirely* whitespace gets a stronger green (it has no letters to anchor it). Dark mode uses stronger tint opacities (≈0.30 vs 0.16 red, 0.30/0.42 vs 0.18/0.34 green). Unchanged names render plain. These four tints are theme tokens beside §14.0's table. The underline-means-added convention is reserved: never use hover underlines on row text for affordance (it would collide — use cursor + contrast changes instead).

### 14.7 Rename by CSV — the modal

One modal replaces the legacy export/import menu pair, and is the **only** CSV path (no power-user direct items). A standard sheet, ~480 px wide, **one state**; entry points: Options ⋯ and File menu `Rename by CSV…` (disabled while processing). Owner-approved v2 design (Handoff Part 3, SHEET-01…08 v2): almost no words — the controls are the explanation.

> **Design source — Handoff Part 3** (figures at real ~480 pt width, behavior spec SHEET-01…08 v2, final strings, QA, element-audit rationale).
> Use the claude_design MCP (https://api.anthropic.com/v1/design/mcp, auth via /design-login) to import this project:
> https://claude.ai/design/p/d23093d8-0903-4f16-8f54-77c8d6f92ef5?file=UX-UI+Analysis+Handoff+Part+3.dc.html
>
> Focus on these files (the whole project is readable):
> - `UX-UI Analysis Handoff Part 3.dc.html`
>
> Also read these files the selection imports:
> - `doc-page.js`
> - `support.js`
>
> Implement: `UX-UI Analysis Handoff Part 3.dc.html` (the §14.7 modal it specifies — this section is the normative summary; the handoff carries the mocks and rationale)

Layout, top to bottom:

- `Download Template CSV ({N} files|folders)…` — live count + mode noun; disabled on an empty list; existing save-dialog export; the modal stays open after export.
- Drop zone `Drop the edited CSV here` + `Choose CSV…` — **both live from open**; downloading is never a prerequisite. CSV only, everywhere: the open dialog filters to .csv; any other drop (or > 1 file) shakes the zone — no error text. Space on the focused zone = choose. After a read the zone collapses to a **passive file chip** (`{filename}`) with `Choose CSV…` kept trailing beside it; the whole modal stays a drop target and re-dropping replaces the read.
- **Results as a sentence** (after `csv_dry_run`; **nothing mutated yet**): `{A} of {N} names will change.` — body size, semibold, primary color. Zero misses: nothing else renders (no success badge). Otherwise one amber line (`--warning-text`) — `{C} rows didn't match` — behind the modal's **single disclosure**; expanding grows the sheet into a full-width scrolling list **grouped by reason** (`Not in the list` · `Duplicate rows` · `Couldn't read`; stock section-header styling; duplicates flag the *second* occurrence — the first still applies). Names middle-truncate to one line, full name on hover. **No match table**: the main-window preview after Apply is the review surface; misses are fixed in the user's own CSV and re-dropped.
- Footer, **fixed composition in every state**: `Cancel` · `Apply {A} Changes` (default button; renders as count-less disabled `Apply Changes` until a read exists; disabled at 0 matches with the zero-match note `No rows matched — was this CSV made from a different list?`). Committing calls `csv_apply`, closes the modal, and the `{N} edited` chip confirms; one ⌘Z removes the whole batch.

Malformed file / missing header: inline `Couldn't read that file.` under the drop zone — never a modal; a later good read clears it. Esc/Cancel close without side effects at any time. Keyboard order: Download → drop zone → Choose CSV → footer; Space/Enter toggles the disclosure. The screen reader announces the summary sentence and miss count when results render. CSV format, parsing, header validation, and extension inheritance are unchanged from the legacy behaviors (tab-fallback parse, quoted-CSV state machine, case-insensitive current-name match, later-rows-win → but flagged per above).

**Copy Preview to Clipboard** (unchanged): TSV `Current Name	New Name	Status` with the §A statuses.

### 14.8 Presets

Menu: saved presets (click = load, undoable), `Save Current Rules as Preset…` (name prompt; on collision offer **Replace** / **Keep Both** — Keep Both auto-suffixes ` 2`, ` 3`…), `Manage Presets…` (sheet with delete buttons; deleting confirms — presets have no undo), `Import Presets…` / `Export Presets…` (JSON dialogs; import appends, suffixing ` (imported)` on collisions; export pretty-printed sorted-key JSON). Presets store rules + `trimsWhitespace`; loading regenerates rule ids. **No presets ship with the app** (owner decision): first-run teaching lives in Help ▸ Recipes, linked from the rules panel's empty state.

### 14.9 Dialogs & alerts

Open dialog (Add): files **and** directories, multi-select, message `Choose files to rename, or folders to watch`, button `Add`. Error/info alerts: title + message + OK (§A catalog); the only modal *notice* is the mid-processing import refusal — everything else non-blocking. Confirmations per §14.0's destructive policy: Revert, Clear List, Clear History, stop-watching guard, bulk-removal guard (§A copy, real counts in every message).

### 14.10 Inspector (File Info drawer)

Right-side drawer (260–420 px), toggled by toolbar ⓘ / ⌘I / row selection. Header: icon + name + path (middle-truncated). **General**: Kind, Size, Created, Modified, Folder — each with a ⧉ button copying the matching token. **Metadata** (macOS `Spotlight Metadata`; Windows `File Properties`; Linux `Metadata`): every `all_attributes` entry, name + stringified value + ⧉ copying `{md:<name>}`. Selecting a row updates it (drawer scrolls to top on file change); empty state: `Click a file to see its info.`

### 14.11 Settings window

From the app menu (§15). Renaming group: `Trim leading and trailing spaces` · `Auto-resolve naming conflicts` · `Keep rules after applying` (**default on** — §4.3; help text: `Instead of clearing the rules after Apply, keep them for the next batch. The files you just renamed are unchecked, so re-applying only affects new files.`). Folders group: `Include subfolders when watching a folder` (global default; per-folder overrides on the chips). **Canonical-homes rule** (audit CON-03): Settings = the discoverable home; in-context mirrors (Options ⋯, action-bar chips) are allowed **only as interactive controls** — passive status text about a toggle is banned. One state underneath all surfaces.

### 14.12 Help window

Separate window (min 640×440, default 760×540): sidebar of topics + article pane, content per §22.5 (including **Recipes**). Reachable via menu/shortcut and deep-linked from the rules panel's empty state and the regex `Syntax reference` link (`open_help(topic)`).

### 14.13 Icon language (lucide-react)

One library, one mapping, committed as this table (do not improvise per-glyph; audit XP-04):

| Concept | Icon | Concept | Icon |
|---|---|---|---|
| Add files/folders | `folder-plus` | Options | `ellipsis` |
| File Info | `info` | Rules tab | `wand` |
| History tab | `history` | Tokens | `braces` |
| Reorder up/down | `arrow-up` / `arrow-down` | Drag grip | `grip-vertical` |
| Remove (✕) | `x` | Destroy (🗑) | `trash-2` |
| Naming conflict | `triangle-alert` | Manual edit | `pencil` |
| Revert | `undo-2` | Number Sequentially | `hash` |
| Fix Unsafe Characters | `shield-check` | Add Prefix / Suffix | `arrow-left-to-line` / `arrow-right-to-line` |
| Change Extension | `file-cog` | New Name from Template | `square-stack` |
| Remove Text | `eraser` | Find & Replace | `replace` |
| Regex Replace | `regex` | Change Case | `case-sensitive` |
| Copy token | `copy` | Reveal in file manager | `folder-open` |

QA gate before ship: an icon-only lineup screenshot; a person (or the owner) names each icon's function blind; ≥ 80% hit rate or the misses get relabeled/replaced.

## §15 Menus & keyboard shortcuts

Native menus via Tauri's Menu API: macOS gets the standard menu bar (App/File/Edit/View/Window/Help with system items); Windows/Linux get a window menu bar with the same custom items minus macOS-specific entries. `⌘` below means Cmd on macOS, Ctrl elsewhere.

| Menu | Item | Shortcut | Enabled when |
|---|---|---|---|
| App (macOS) | Settings… | ⌘, | always |
| File | Add Files or Folders… | ⌘O | not processing, no revert preview |
| File | Rename {N} Files / Rename {N} Folders (live label, mirrors the action bar) | ⌘↩ (Cmd/Ctrl+Enter) | `can_apply` && idle && no revert preview |
| File | Revert Previewed Renames… | — | revert preview active && restorable > 0 && idle |
| File | Skip Conflicted | — | conflict_count > 0 |
| File | Rename by CSV… | — | active items > 0 && idle |
| File | Import Presets… / Export Presets… | — | always / presets exist |
| Edit | Undo {action} / Redo {action} | ⌘Z / ⇧⌘Z | workspace undo stack (§13.3) when focus isn't in a text input (text inputs keep native undo) |
| Edit | Select All | ⌘A | rows exist — selects **rows**, never checkboxes |
| Edit | Include All / Skip All / Invert Inclusion | — | active items > 0 (mode-scoped checkbox operations) |
| Edit | Clear All Manual Edits ({N}) | — | override count > 0 (mode-scoped, undoable) |
| Edit | Find | ⌘F | focuses the filter field |
| View | Show/Hide File Info | ⌘I | always |
| View | Inline Diff View | — | toggle (§14.2); persists in ui state |
| Window | Name Shift | ⌘0 | reopens the main window (single-window app — closing it with Help open must not strand the process) |
| Help | Name Shift Help | ⌘? (F1 on Win/Linux) | always |

Additional: ⌘W closes the focused window; Esc cancels processing (via the overlay button), closes an in-progress row edit, or clears an active filter — in that priority. Settings on Windows/Linux lives under Edit → Preferences… (Ctrl+,). Single-window rule: no "New Window"; the single-instance plugin forwards second launches (§12.2 `open-paths`).

**Shortcut ownership rule (hard-learned):** each shortcut is bound in exactly one place — the menu item. Toolbar/action-bar buttons show the shortcut in tooltips but must not register their own accelerator (double-binding double-fired actions historically).

## §16 Platform rules — validation, case, normalization, limits

Centralize in `crates/engine/platform.rs` as data, not scattered conditionals:

```rust
pub struct PlatformProfile {
    pub invalid_chars: &'static [char],
    pub reserved_names: &'static [&'static str],   // compare stem-before-first-dot, ASCII-uppercased
    pub forbid_trailing_dot_space: bool,
    pub max_name_bytes: Option<usize>,             // UTF-8 bytes
    pub max_name_utf16: Option<usize>,             // UTF-16 units (Windows)
    pub case_fold_keys: bool,                      // §16.2
    pub nfc_normalize_keys: bool,
}
pub fn host_profile() -> &'static PlatformProfile;   // + named profiles for tests
```

### 16.1 Values

| | macOS | Windows | Linux |
|---|---|---|---|
| invalid chars | `/` `:` | `< > : " / \ \| ? *` + U+0000–U+001F | `/` + U+0000 |
| reserved names | — | `CON PRN AUX NUL COM1–COM9 LPT1–LPT9` (with or without extension: `CON.txt` is reserved) | — |
| trailing dot/space | allowed | **forbidden** | allowed |
| name length | 255 bytes | 255 UTF-16 units | 255 bytes |

Full-path length, Windows: convert paths to `\\?\` verbatim form for every filesystem call through one helper (`win_long_path()`), so deep trees and 255-char names work regardless of the system MAX_PATH setting. Test with a > 300-char path.

### 16.2 Collision keys (`diff_key`)

`diff_key(dir, name)`: macOS → NFC-normalize then lowercase both parts (APFS is case- and normalization-insensitive by default); Windows → lowercase, **no** normalization (NTFS is case-insensitive but normalization-sensitive); Linux → exact bytes. Used by duplicate-target detection, auto-resolve, and the on-disk name sets (§7.5 stores names pre-folded). The engine takes the profile as a parameter — **Windows rules are unit-tested on every CI OS** by injecting the Windows profile; only the on-disk integration tests are OS-gated.

### 16.3 Case-only renames

`readme → README` counts as a change, produces no self-collision (its target key equals its own vacated source key), and works on case-insensitive filesystems only because of the two-phase mover. Integration test: perform one on each OS; on case-sensitive Linux filesystems it's an ordinary rename.

### 16.4 Case-insensitive matching (rules)

Simple Unicode case folding per char for find/replace matching. Divergence from the legacy NSString behavior (which uses canonical caseless matching — e.g. `ß`≈`SS`) is accepted and documented (§24 Q14).

### 16.5 Non-UTF-8 names (Linux)

Paths whose final component isn't valid UTF-8: display lossily (U+FFFD), mark with `Problem::UnrenamableName` (`This name uses an encoding Name Shift can’t edit safely.`), exclude from Apply. Never panic on them anywhere (fuzz-tested).

---

## §17 Performance targets

Enforced by benchmarks (criterion, not CI-gating) + a generated fixture tree (`scripts/gen-fixtures` → 10k files across 100 dirs):

| Scenario | Target |
|---|---|
| Preview, 10k files, text-only rules (warm) | < 150 ms end-to-end (engine < 50 ms) |
| Preview, 10k files, `{created}` token | < 800 ms cold (stat fan-out via rayon), < 150 ms warm |
| Keystroke in a rule field → updated preview visible | < 250 ms (debounce included) |
| Apply 10k renames (local SSD) | IO-bound; progress events throttled ≤ ~100 total |
| Scroll 50k-row list | 60 fps (virtualized; no layout thrash) |
| Cold start → interactive (restored session, 1k files) | < 1.5 s |
| Idle memory, 10k files | < 250 MB |

Soft cap: importing beyond 50k active items warns (`Name Shift works best under 50,000 files; the preview may be slow.`) but proceeds.

---

## §18 Testing requirements

Coverage is a first-class deliverable. The historical suite caught real bugs unit checks missed — especially **on-disk integration tests** — so the pyramid here is deliberately integration-heavy.

### 18.1 Rust unit tests (`crates/engine` and friends)

- **Per-rule tables:** every §5.3 kind × (plain, `includesExtension`, directory, empty-input-ineffective, case-insensitive where applicable) with the §5 vectors verbatim, plus: multi-dot names, dotfiles, unicode (CJK, emoji, combining marks), clamps (numberStart 10^9, padding 99).
- **Tokens:** every §6.2 token; unknown-token passthrough; `{:}`; single-pass non-reentrancy; `{n}` width clamps; LDML vectors of §6.4; size vectors of §6.5; laziness (§6.6 counting provider).
- **Preview:** ordering per key & direction; numbering skips deselected / counts overrides; override wins; trim pass; auto-resolve (§7.4: keeper-wins, vacated-reuse, on-disk block, folder-suffix placement, 1000 cap); every §7.6 problem incl. the unchanged-name exemptions and the swap-vacancy exemption; counts.
- **Audit behaviors:** titleCase acronym vectors (§5.3-e verbatim, incl. `iPhone`); sanitize reserved-name vectors (`CON.txt`, `nul`, mixed case, near-misses `CONSOLE` / `COM10` / `connect.log`); `restartPerFolder` (two folders × 3 files sorted by Folder → `001–003` twice; deselected files skip numbers; missing field decodes `false`); `rule_impact` (hand-verified 3-rule stack; disabling a rule zeroes its count; keystroke budget benched per §7.9).
- **Validation:** every §16.1 rule on **every** OS via injected profiles (Windows reserved names + trailing-dot tested on Linux CI too); `diff_key` fold/normalization matrix.
- **CSV:** build/parse round-trip, quoting/escaping, tab fallback, header rejection (missing/wrong/case), extension inheritance, later-row-wins.
- **Store:** round-trip every field of all three files; decode-with-defaults (drop random fields → defaults); legacy fixtures of §B decode byte-exactly into expected structs; legacy `sortOrderRaw` migration table; corrupt file → defaults without panic; atomic write leaves no `.tmp` on success.
- **Undo stack:** record/undo/redo sequences, cap, action names.

### 18.2 Rust integration tests (tempdir, real filesystem — `crates/engine/tests/`)

Each creates a scratch tree, runs the real engine, asserts the resulting disk layout **and** recorded snapshot:

1. Plain batch rename; verify preview == outcome.
2. Sibling swap `a↔b` (two-phase proof).
3. Case-only rename (assert per host FS semantics).
4. Nested folder batch: rename parent + children + grandchild files in one Apply → parents-first rewriting; recorded entries match final layout; then **revert** → children-first restores the exact original tree.
5. `rewrite_live_paths`: imported files, exclusions, and a watched root inside a renamed directory all track to the new path (and the re-armed watcher fires on the new path).
6. Cancel in phase 1 → disk untouched; cancel in phase 2 mid-swap → committed entries recorded, partner completed forward, no `.fne-tmp-*` left.
7. Orphan recovery: plant stranded temps (valid, invalid-UUID, target-occupied) → only the valid+free one restored.
8. Mid-batch failures: source vanished; destination directory read-only (Unix perms) — errors surfaced, remainder proceeds, rollback correct. Windows-gated: destination file open with no sharing → error path.
9. Revert with missing file (skipped) and name-taken (kept) — and assert §8.8's simulation predicted exactly that.
10. History: cap at 50; revert-through removes the right snapshots; partial-cancel keeps the partial snapshot.
11. Watcher: create/delete/rename files under a watched root → debounced rescan reconciles (ids/selection/overrides preserved); `include_subfolders` toggle; bundle-pruning (macOS-gated); removal tears down (§9 deallocation test); `.fne-tmp-*` ignored.
12. Session: full save→restore in a fresh state (watchers re-armed, selection/overrides re-applied to folder files, missing files counted).
13. Long-path (Windows-gated, > 300 chars) and unicode-name (NFC/NFD pair on macOS) renames.
14. Workspace undo: each list mutation (remove, bulk remove, clear list, stop-watching, CSV apply, clear-all-edits) → undo → state byte-identical (selection, overrides, watcher **re-armed** — drop a file into the restored root and assert the rescan fires).

### 18.3 Property & fuzz tests (proptest)

- **Round-trip law:** ∀ generated tree + rule stack: if preview shows no problems → apply succeeds fully; then revert restores a byte-identical tree (names). Shrinkable; ≥ 256 cases in CI.
- **No-temp law:** after any (apply | cancel-at-random-point | revert) sequence, no `.fne-tmp-*` remains except the deliberately-stranded case.
- **Simulation law:** revert-preview statuses == actual revert outcomes, ∀ generated histories with injected missing/taken conditions.
- **Fuzz:** `apply_rule` + `expand_tokens` + CSV parse + name validation over arbitrary unicode strings (incl. non-UTF-8 `OsString` at the fs boundary) — no panics, ever.

### 18.4 Frontend tests (Vitest + RTL + `mockIPC`)

Rules panel (add/edit/reorder/toggle/delete/duplicate; **regression: clearing the rules array from underneath an open card must not crash — id-keyed rendering**; regex invalid state; token insert-at-cursor); file list (diff spans incl. whitespace-underline rules, badges, filters + Select Only These, header tri-state, inline override edit + `rule_derived_name` guard); status-bar gating & `apply_disabled_reason`; banner variants + auto-dismiss; history/revert pane counts & copy; inspector token-copy; the Rename-via-Spreadsheet sheet (both states, partition rendering, zero-match, drop-zone rejection); the row-selection model (click/⌘/⇧ ranges, Space/Return/Delete, conflict-wash vs selection precedence); action-bar gating; store logic (version guard drops stale payloads; optimistic rules reconcile). Component coverage ≥ 80% lines.

### 18.5 E2E & manual

- **WebdriverIO + tauri-driver** (Linux & Windows CI): import fixture dir → build rule stack → assert preview → Apply → assert **real files on disk** → revert → assert restored; conflict blocking + Skip Conflicted; the spreadsheet round-trip (export template → edit rows → dry-run review → apply as manual edits); Folders mode apply/revert; session restore across relaunch.
- **macOS** (no tauri-driver support): `docs/MANUAL_TESTING.md` — numbered checklist mirroring the E2E specs plus macOS-only items (Spotlight tokens/inspector, Services/Open-With, MAS folder-grant flow when built with `channel-mas`). Each step lists action + expected result + a checkbox. The Rust integration suite already exercises the whole engine on macOS CI, so manual scope is UI-level only.
- **Visual regression (ENG-03 backlog item).** Lesson from the Swift app (its NEW-02 finding): an offscreen-rendered PNG catalog is *not byte-stable* across re-renders even on unchanged code (~48 of 110 files differed per run — sample-data dates plus renderer nondeterminism). Naive commit-the-PNGs + `git diff` baselining does not work. Any visual-regression rig here must use perceptual/tolerance diffing (or content hashing after masking dynamic regions), and fixture data must use pinned dates — never "now".

### 18.6 Gates

CI fails unless: all suites green on all three OSes; `crates/engine` line coverage ≥ 85% (cargo-llvm-cov); frontend coverage ≥ 80%; clippy/fmt/ESLint/tsc clean; cargo-deny clean.

---

## §19 CI/CD

### 19.1 `ci.yml` (push + PR)

Matrix `[ubuntu-latest, windows-latest, macos-latest]`: checkout → pnpm + Rust toolchain (cached) → Linux deps (`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`) → `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, coverage job (ubuntu) with the §18.6 gate → `pnpm lint`, `pnpm typecheck`, `pnpm test -- --coverage` → `pnpm tauri build --debug` (bundle smoke) → E2E job (ubuntu: `webkit2gtk-driver`; windows: matching `msedgedriver`) running §18.5. Separate job: `cargo deny check`.

### 19.2 `release.yml` (tag `v*`)

Per-OS builds: macOS `--target universal-apple-darwin` (dmg + updater archive; sign + notarize **only if** the §0.2 Apple secrets exist, else unsigned artifact clearly labeled), Windows NSIS + MSI, Linux AppImage + .deb + .rpm. Generate the updater `latest.json` (signed when the updater key secrets exist) and attach everything to a **draft** GitHub Release. Store lanes are deliberately manual: `packaging/msix/make-msix.ps1` (MSIX for Partner Center), `packaging/mas/` runbook (MAS pkg), `packaging/flatpak/` (Flathub PR) — each documented in `docs/RELEASING.md`.

---

## §20 Distribution & packaging

Four channels; Cargo features select behavior, config overlays select bundling. Store channels never self-update; only `channel-direct` compiles the updater.

| Channel | Feature | Config | Artifacts | Updates via |
|---|---|---|---|---|
| Direct download | `channel-direct` (default) | `tauri.direct.conf.json` | dmg (universal), NSIS + MSI, AppImage/deb/rpm | tauri-plugin-updater → GitHub Releases `latest.json` |
| Mac App Store | `channel-mas` | `tauri.mas.conf.json` | signed `.pkg` | App Store |
| Microsoft Store | `channel-msstore` | base | MSIX (`packaging/msix/`) | Store |
| Flathub | `channel-flathub` | base | Flatpak (`packaging/flatpak/`) | Flathub |

### 20.1 In-app channel awareness

`get_platform` reports the channel; the Help/About surface shows it; the updater UI (menu `Check for Updates…`, background daily check, standard consent flow) exists only on `channel-direct`.

### 20.2 macOS specifics (all macOS channels)

`src-tauri/Info.plist` additions: the folder/removable/network-volume usage-description strings (§A verbatim — required for non-sandboxed folder access prompts and good citizenship in both), `LSApplicationCategoryType public.app-category.utilities`, `CFBundleDocumentTypes` viewer role for `public.item` + `public.folder` (enables Open With / dock drops → `open-paths`). `ITSAppUsesNonExemptEncryption = NO`. macOS Services menu entry (`Rename with Name Shift`): implement via NSServices in Info.plist + an `objc2` service handler registered in setup **if** achievable without fighting Tauri's app delegate; otherwise document Open-With/dock-drop as the supported Finder paths and record in DEVIATIONS (§24 Q20).

### 20.3 Mac App Store lane

`packaging/mas/entitlements.plist`: `com.apple.security.app-sandbox`, `…files.user-selected.read-write`, `…files.bookmarks.app-scope`. Build `pnpm tauri build --config src-tauri/tauri.mas.conf.json --features channel-mas` with signing identity `Apple Distribution`, embedded provisioning profile, then `productbuild` (installer identity `3rd Party Mac Developer Installer`) → upload with Transporter. The runbook documents every step + required portal setup. Behavior deltas in MAS builds are exactly §10.2 — nothing else may differ.

### 20.4 Microsoft Store lane

`packaging/msix/AppxManifest.xml` (Identity placeholders per §0.2, `runFullTrust` desktop app packaging the NSIS-installed layout), assets, and `make-msix.ps1` (makeappx + optional signtool with a self-signed test cert for local install). Store submission signs with Microsoft's cert. Windows 10 1809+ target.

### 20.5 Flathub lane

`packaging/flatpak/io.github.kingsleyramos.NameShift.yml`: `org.gnome.Platform` runtime (current stable at build time), cargo/node offline-sources generated per Flathub's builder-tools, finish-args `--socket=wayland --socket=fallback-x11 --share=ipc --device=dri --filesystem=host` (+ the §10.3 justification comment). AppStream `metainfo.xml` + `.desktop` shared with deb/rpm from `packaging/linux/`.

### 20.6 `docs/RELEASING.md`

Step-by-step per channel: version bump locations (single source: root `package.json` → synced to `tauri.conf.json` + `Cargo.toml` by a check script), tag, CI draft release, store submissions, Flathub PR, updater-key handling, and a "first-time setup" section per store account.

---

## §21 Code conventions & AI-maintainability

The maintainers of this codebase will primarily be AI agents. Optimize for **legibility of intent**:

1. **Comments explain *why*, and especially *why not*** — every invariant in this spec marked "hard-learned" gets an inline comment at its code site explaining the failure mode it prevents (the legacy codebase's greatest asset was exactly these comments; §22.3 lists them).
2. **Doc comments on every public item** in crates (`#![warn(missing_docs)]` on `engine`).
3. Naming: full words, no abbreviations (`compute_revert_preview`, not `cmpRevPrev`); TS mirrors Rust names across IPC.
4. Module = feature; ≤ ~400 lines/file; no `utils.rs` grab-bags.
5. No `unsafe` outside `crates/metadata` FFI and `ScopedAccess` (each block commented with its safety argument).
6. All user-facing strings in one module per side (`crates/engine/src/copy.rs` for engine-originated, `src/lib/strings.ts` for UI) — greppable, future-l10n-ready.
7. Every bugfix lands with a regression test named after the behavior.
8. Conventional-commit-ish messages (`engine:`, `ui:`, `store:`, `ci:`, `docs:` prefixes).

---

## §22 Documentation deliverables

### 22.1 `README.md`

Install the provided `README.next.md` **verbatim** as `README.md` (substituting the real repo slug per §0.2, and capturing real screenshots into `docs/screenshots/` — a main-window shot in light and dark — once the UI runs). Keep it current with behavior forever (CLAUDE.md rule).

### 22.2 `docs/DEVIATIONS.md`, `docs/MANUAL_TESTING.md`, `docs/RELEASING.md`

Per §0.1, §18.5, §20.6.

### 22.3 `CLAUDE.md` (the new repo's agent guide — author it with this skeleton)

Project one-liner; commands (`pnpm dev`, `pnpm tauri dev`, `cargo test --workspace`, `pnpm test`, `pnpm tauri build`, per-channel builds); architecture map (one line per crate/dir); then **Invariants (do not break)** — carry each of these forward verbatim in spirit:

- Every `CoreState` mutation goes through `AppState::mutate` (version bump = cache invalidation = event). Never mutate outside it; never add a second preview cache.
- Renames are two-phase (temp → final) so swaps and case-only renames never collide; batches with nested folders run parents-first on apply, children-first on revert; recorded history entries always match the final disk layout.
- `rewrite_live_paths` prefix-rewrites tracked paths after directory moves **without existence guards** — batch ordering makes such guards wrong.
- Persisted structs decode with per-field defaults (`#[serde(default)]`) and ignore unknown fields, so old presets/sessions always import; new fields must follow suit.
- Rule cards and file rows are keyed by stable ids, never array position.
- Every persisted path goes through `FileAccess::persist_token` (MAS bookmarks depend on it).
- Watcher handles tear down on Drop (test-asserted).
- User-facing copy says "naming conflict", never "conflict"; "Skipped" belongs to unchecked rows only; *selection* (rows) and *inclusion* (checkboxes) are never conflated. Strings live in the strings modules.
- Row selection is frontend-only ui state; it never crosses IPC and is never persisted (§24 Q28).
- Update Help topics and README whenever behavior changes.
- Run the full test suite before every commit; on-disk integration tests catch what unit tests miss.

### 22.4 `CONTRIBUTING.md`

Toolchain setup per OS (incl. Linux webkit deps), dev loop, test commands, coverage gates, commit style, PR checklist (tests + help/README updates), code-of-conduct pointer.

### 22.5 In-app Help content

Port these topics, rewritten platform-neutrally (shortcut labels adapt per OS; macOS-only notes marked): Getting Started (import, watched folders, session restore, banner/progress) · Rename Rules (all 10 kinds, options, undo) · Tokens (full reference incl. §11.3 per-OS `{md:…}` availability) · Selecting, Sorting & Filtering · Renaming Folders (whole-name semantics, paths-update-automatically, revert-newest-first caveat) · Manual Edits & Rename by CSV · Renaming & Naming Conflicts (reasons, Skip Conflicted, Auto-Resolve, two-phase safety) · History & Revert (50-cap, older-reverts-newer) · Rule Presets · Recipes (three worked examples — exact rules + before → after, each pinned by an engine test so the docs can't drift) · File Info & Metadata · Keyboard Shortcuts. Content must match this spec's behaviors exactly.

---

## §23 Build order & definition of done

### 23.1 Milestones (commit skeleton)

1. Scaffold: workspace + Tauri + React + toolchain configs + CI skeleton green.
2. `engine`: types, rule engine + tokens (§5–6) + unit tables.
3. `engine`: preview pipeline + validation profiles (§7, §16) + tests.
4. `engine`: plan/execute/revert (§8) + integration + property tests.
5. `store` (§4.3) + legacy fixtures; `watcher` (§9) + tests.
6. `metadata` (§11) per-OS + tests (host-OS gated where needed).
7. Tauri shell: `AppState::mutate`, commands, events, ts-rs generation (§12–13).
8. UI: shell/layout + action bar → file list (selection model, sortable headers, chips) → rules panel (ghost card, impact counts) → Rename by CSV modal → history/revert → inspector → dialogs/banners → settings/help (§14) with component tests as you go.
9. Menus/shortcuts/single-instance/open-paths (§15).
10. E2E suites + `MANUAL_TESTING.md` (§18.5).
11. Packaging: channels, Info.plist, MAS/MSIX/Flatpak lanes, updater, `release.yml` (§20).
12. Docs sweep (§22) + `README.md` install + screenshots + final full-suite run.

### 23.2 Definition of done — every box checked, honestly

- [ ] §0.1(4) commands all green on the host OS; CI green on all three OSes.
- [ ] Coverage gates met (engine ≥ 85%, frontend ≥ 80%).
- [ ] §14.0 style rules hold: no bare-text buttons; Revert never wears the accent; contrast spot-checks (warning text, skipped pill, link counts) pass in both themes.
- [ ] Workspace-undo matrix (§18.2.14) green: every list mutation undoes byte-identically.
- [ ] Property tests (§18.3) pass at ≥ 256 cases.
- [ ] `pnpm tauri build` produces a working bundle on the host OS; app launches, imports, previews, applies, reverts on real files.
- [ ] Session/history/preset files written by the app on macOS are readable by the legacy fixtures' schema and vice versa (§B round-trip test).
- [ ] All four channel configs build (`--features channel-mas` compiles on macOS even unsigned; MSIX script runs; Flatpak manifest validates with `flatpak-builder --show-manifest` where available).
- [ ] Every §14/§A string present verbatim; "conflict" never appears without "naming".
- [ ] `CLAUDE.md`, `CONTRIBUTING.md`, `RELEASING.md`, `MANUAL_TESTING.md`, `DEVIATIONS.md` exist and are accurate; README installed and true.
- [ ] No `TODO`/`FIXME`/`unimplemented!` anywhere; `DEVIATIONS.md` covers every divergence.

---

## §24 Questions asked and answered (FAQ)

Decisions a careful reader would question, answered so the builder never has to guess:

**Q1. Why does the engine live behind a version-bump funnel instead of per-field cache invalidation?** The legacy design invalidated caches in per-property observers — correct but fragile (every new input had to remember to invalidate). One mutation funnel makes staleness structurally impossible. Same behavior, safer shape.

**Q2. Do manually-overridden files consume sequence numbers?** Yes (§6.3/§7.3). The counter increments before the override short-circuits. Legacy-faithful; tested.

**Q3. What happens when rules produce a name identical to the current one?** It's not a change: not counted, not applied, and intrinsic problems aren't checked (§7.6). It can still be a *duplicate target* if something else renames onto it.

**Q4. Can Apply run with zero rules?** Yes — manual edits (and CSV mappings) alone can drive an Apply. The feedback banner then reports neither "rules cleared" nor "rules kept" (§8.5.4).

**Q5. What if the user renames a file to a name being vacated in the same batch?** Allowed (§7.6 rule 7) — the two-phase mover guarantees it works. That's what makes swap workflows possible.

**Q6. Why is the on-disk collision set allowed to go stale?** Re-listing directories per keystroke costs one stat per file and beachballs on network volumes. Staleness is preview-only; Apply still fails safe (§7.5). Deliberate trade.

**Q7. Hidden files?** Watched-folder scans skip them (dotfiles everywhere, hidden attribute on Windows). Directly imported hidden files are allowed and renameable. `.fne-tmp-*` is always ignored by scans.

**Q8. Symlinks?** Renaming a symlink renames the link itself (std `rename` semantics); never followed, never resolved. Watched scans classify by the link target's type as the OS reports it; broken links are skipped. Document, don't special-case.

**Q9. What does `{created}` do on Linux filesystems without birth time?** Resolves unknown → token stays visible in the preview (§6.2). Help notes it. Never silently substitute mtime.

**Q10. Are `{md:…}` presets portable across OSes?** No, by design — keys are platform-namespaced (§11.3). Visible unknown-token output beats wrong guesses.

**Q11. Why keep the exact legacy JSON schemas?** Users have exported preset files and, on macOS, existing history/session data in `~/Library/Application Support/NameShift` this app reads natively. Compatibility costs nothing (the schemas are clean) and buys seamless continuity.

**Q12. History migration risk: old history entries were written by the previous app — can this app revert them?** Yes — snapshots are plain from/to path pairs (§4.3) and the revert engine only needs those. The revert *preview* handles missing/taken files regardless of who wrote the snapshot.

**Q13. Why `.fne-tmp-` and not a new prefix?** Orphan recovery (§8.4) then also rescues temps stranded by earlier releases on the same machine. A cosmetic rename would orphan them forever.

**Q14. Regex/case-folding parity with the legacy ICU behavior?** fancy-regex covers lookaround and backrefs (the advertised features). Known divergences — ICU canonical caseless matching (`ß`≈`SS`), possessive quantifiers, `\p{...}` dialect details — are accepted; document in help ("patterns use Rust regex syntax with lookaround support"). Tests pin the *documented* behaviors, not ICU quirks.

**Q15. Title Case produces `It’S` — really?** Yes — the apostrophe splits words and single letters capitalize (§5.3-e). v2 *does* depart from legacy 1.x in one way: all-caps words of ≥ 2 letters are preserved (`NASA report → NASA Report`, not `Nasa Report`), matching the current reference implementation. `iPhone → Iphone` stays (mixed case normalizes). Both are pinned by tests and noted in help.

**Q16. Windows: file open/locked during Apply?** The move fails for that item with the OS message; the batch continues; the item rolls back to its original name (§8.3). Integration-tested (§18.2.8). No retry loop in v2.0.

**Q17. What is `UnrenamableName`?** Non-UTF-8 names on Linux (§16.5) and, in MAS builds, `NoFolderPermission` (§10.2) — items visible but excluded from Apply with an explanatory badge. Never silently skipped.

**Q18. Multi-window?** No. One main window + Help + Settings (§15 `⌘0` rule). Multiple workspaces are out of scope for 2.0.

**Q19. Why is the updater absent from store builds?** Store policies (all three) require store-managed updates; shipping updater code there risks rejection. Feature-gated out (§20).

**Q20. macOS Services menu — required?** Best-effort (§20.2). If the NSServices handler can't register cleanly under Tauri's delegate, ship without it: Open With, dock drops, and drag-drop cover the workflow; DEVIATIONS records it. Do not destabilize the app for this.

**Q21. Second instance / CLI args?** `single-instance` forwards argv paths to the running instance (`open-paths` → import), matching dock/Open-With behavior on macOS. A bare second launch just focuses the window.

**Q22. Time zones for `{date}`/`{created}`?** Host-local wall time, Gregorian, English names, exactly as specified in §6.4 — filenames must be stable and locale-independent (a non-Gregorian system calendar once baked year 2569 into filenames; hence the fixed calendar).

**Q23. What blocks a 10k-file preview from stat-ing 10k times?** §6.6 laziness + §7.5 listing cache + dates-read-up-front sorting (§7.2). All three are tested; regressions here are the app's historical performance bugs.

**Q24. Where do "Trim spaces" and other toggles live — Settings or Options menu?** Both (§14.11): the Options toolbar menu is the fast path, Settings is the discoverable home. One state underneath.

**Q25. Accessibility bar?** Full keyboard reachability, visible focus, labeled controls, announced apply results, tooltips duplicated in accessible descriptions, contrast ≥ WCAG AA in both themes, reduced-motion honored (no essential information conveyed by color alone — diff spans also carry strikethrough/underline, §14.6).

**Q26. Why does `keepRulesAfterApply` default to true when legacy defaulted false?** Owner decision from the UX audit: clearing the stack reads as data loss, and every comparable tool keeps the configuration. Legacy sessions keep whatever they had (§4.3); only fresh installs get the new default.

**Q27. Why is the primary button in a bottom action bar instead of the toolbar?** The audit's highest-traffic finding: everything the user checks before committing (counts, conflicts, remedies) lives at the bottom edge; putting **Rename** beside them makes the decision one left-to-right scan. It is also the platform-neutral choice — Windows/Linux users have no toolbar-primary reflex. ⌘↩ and the File-menu item are unchanged.

**Q28. Is row selection persisted or shared with the engine?** No. Selection (highlighted rows) is ephemeral frontend state for inspection, keyboard ops, and range gestures; *inclusion* (checkboxes) is domain state and the only thing Rename reads. The two are named distinctly everywhere (§14.2); conflating them was the reference app's single worst UX defect.

**Q29. Per-volume filesystem semantics (a Samba mount on Linux behaving like Windows)?** Out of scope for v2.0: `diff_key` and validation use the host profile (§16). The on-disk listing cache reflects what the volume actually reports, and the two-phase mover fails safe with the OS error on a true collision — the gap is preview accuracy on exotic mounts, not data loss. The `PlatformProfile` seam makes a per-volume upgrade non-breaking; revisit if reports arrive.

**Q30. Why do CSV imports go through a dry-run review instead of applying directly?** The legacy silent-apply created mystery manual edits. `csv_dry_run` is pure; the user sees matched / unchanged / unmatched with per-row reasons before `csv_apply` commits — and the commit is one undo entry.

---

## §A Appendix: user-facing copy catalog

Verbatim strings (typographic punctuation included). `{…}` are runtime substitutions; `N`-pluralization follows English rules (`1 file` / `2 files`).

**Problems (§7.6)** — `The new name would be empty.` · `The new name contains “/” or “:”, which aren’t allowed.` (macOS; Windows variant: `The new name contains characters Windows doesn’t allow: < > : " / \ | ? *` ; Linux variant: `The new name contains “/”, which isn’t allowed.`) · `The new name is longer than {macOS|this system|Windows} allows (255 {bytes|characters}).` · `This name is reserved by Windows and can’t be used.` · `Windows names can’t end with a dot or a space.` · `Two or more files would end up with the same name.` · `A different file with this name already exists in the folder.` · `This name uses an encoding Name Shift can’t edit safely.` · MAS only: `Name Shift doesn’t have permission to rename items in this folder.`

**Revert statuses (§8.8)** — ok: `Will be restored` · missing: `Not found (nothing to restore)` with tooltip `It may have been moved or deleted since this version was applied. This rename will be skipped.` · nameTaken: `The original name is taken by a different file — kept as is.` ("Skipped" belongs to unchecked list rows only — glossary rule.)

**Primary action & gating (§7.7)** — button label: `Rename {N} {Files|Folders}` (live count of included-and-changing items; at 0: `Rename Files`, disabled). Disabled reasons: conflicts → `Fix or skip the naming conflict{s} to rename` · no changes → `Add a rule that changes at least one included {file|folder}`. Tooltips — enabled: `Rename {N} {file|folder}{s} on disk (⌘↩)`; conflicts: `Fix or skip the {N} naming conflict{s} — see the warnings in the list`; empty: `Nothing to rename yet — add a rule that changes at least one included {file|folder}`.

**Banner (§14.1)** — `Renamed {N} {file|folder}{s}` · `· Rules cleared (⌘Z restores them)` · `· Kept rules; renamed files deselected` · button `Revert…`.

**Action bar & chips (§14.1–14.3)** — counts: `{N} of {M} included` · `{N} will change` · `{N} naming conflict{s}` (click-toggle filters; active = filled chip with ✕) · `{N} edited` (pencil chip → Manual Edits filter) · `Auto-resolve: {On|Off}` (toggle chip — passive status text is banned, §14.0). Buttons: `Skip Conflicted` (tooltip `Uncheck the conflicted files so the rest can be renamed`). Rule cards: `affects {A} of {E}` · `No input yet — rule is skipped` · Folders-mode badge `Skipped for folders`. Skipped-row pill: `Skipped`.

**Confirmations** — Revert title `Revert these renames?`, message: `{N} file{s} will be renamed back on disk.` [+ ` ({R} renames across {V} versions.)` when R≠N] [+ ` This also undoes the {K} newer version{s}.` when K>0], button `Revert`. Clear list title `Clear the file list?`, message `This empties the list and stops watching any folders. Files on disk aren’t changed.`, button `Clear List`. Stop-watching guard (only when the folder contributed > 10 items or any manual edit): title `Stop watching “{folder}”?`, message `Its {N} file{s} leave the list{, and {M} manual edit{s} are discarded}. Files on disk aren’t changed.`, button `Stop Watching`. Bulk-removal guard (> 10 items or any manual edit affected): title `Remove {N} {files|folders} from the list?`, message `{M} manual edit{s} will be discarded. Files on disk aren’t changed.` (first sentence only when M > 0), button `Remove`. Anything smaller is instant + undoable (§13.3).

**Errors/info** — Generic error title `Couldn’t complete that`. Batch errors title `Some renames couldn’t complete` (first 8 + `…and {N} more.`). Per-move errors: `Couldn’t rename “{name}”: {reason}` · `Couldn’t rename “{from}” to “{to}”: {reason}` · `Couldn’t finish renaming “{name}”; it is safe at “{temp}”.` Session restore: title `Some files were missing`, `{N} file{s} from your last session {was|were} missing and removed from the list. Files on disk aren’t changed.` Spreadsheet sheet (§14.7) — title `Rename by CSV`; subtitle `Edit new names in Excel, Numbers, or Google Sheets, then bring the file back here.`; Step 1 `Get the spreadsheet` / `A CSV with two columns — Current Name and New Name — prefilled with your {N} {files|folders} as they are now.` / button `Download CSV of {N} {Files|Folders}…` (disabled at 0, hint `Add files to the list first.`); Step 2 `Bring it back` / drop zone `Drop the edited CSV here` / `Choose File…`; footer `Rows you leave unchanged do nothing.`; multi/wrong-type drop: `One CSV file, please`; read error (inline, never modal): `Couldn’t read that file — export a fresh template from Step 1.`; header reject (inline): `Its first row must be the exported header “Current Name, New Name”.`; review header `Review changes from “{filename}”` + `{A} name{s} will change · {B} row{s} unchanged · {C} row{s} didn’t match`; unmatched reasons: `no file with this name` · `duplicate row` · `couldn’t read this line` (duplicates flag the *second* occurrence; the first still applies); zero-match note `No rows matched — was this CSV exported from a different list?`; buttons `Back` · `Cancel` · `Apply {A} as Manual Edits` (default). The sheet only creates manual edits — the main **Rename** button stays the sole commit-to-disk. Presets imported: title `Presets imported`, `Imported {N} preset{s}.` Info alert default title: `Name Shift`.

**macOS usage descriptions (Info.plist)** — Desktop/Documents/Downloads/Removable/Network, pattern: `Name Shift renames files and folders you choose, including items {on your Desktop|in your Documents folder|in your Downloads folder|on external drives|on network drives}.`

**Rules panel empty states** — default: `No rules yet` + `Add a rule to start renaming, or load a preset.` + link `See Example Recipes` (opens Help ▸ Recipes); the ghost Add-Rule card renders beneath (§14.3) · post-apply (reachable only when keep-rules is off): `Rules applied and cleared` + `The renames are saved in History. Press ⌘Z to bring the rules back, or turn on “Keep rules after applying” in Settings.`

**Dialogs** — Add: message `Choose files to rename, or folders to watch`, button `Add`. CSV open: `Choose a CSV with two columns: current name, new name`. CSV save default name `Rename Template.csv`, message `Edit the New Name column, then bring it back with Import Edited CSV`. Presets open: `Choose a presets file exported from Name Shift`; save default `Name Shift Presets.json`. MAS folder grant: `To rename these files, allow access to their folder.`, button `Allow`.

**TSV/CSV headers** — clipboard: `Current Name	New Name	Status`; statuses `Naming conflict: {reason}` `Skipped (deselected)` `Will change (manual edit)` `Will change` `No change`; CSV header `Current Name,New Name`.

---

## §B Appendix: legacy JSON fixtures

Commit these under `crates/store/tests/fixtures/legacy/` and test that they decode into the expected structs (and that re-encoding preserves semantics). They are byte-faithful to the legacy encoders (sorted keys; pretty where noted).

**`presets.json`** (pretty):
```json
[
  {
    "id": "7B4C2A10-53E5-4D2A-9C6F-2E8B1F0A9D11",
    "name": "Photo import",
    "rules": [
      {
        "caseSensitive": true, "caseStyle": "lowercase",
        "id": "0A1B2C3D-4E5F-6071-8293-A4B5C6D7E8F9",
        "includesExtension": false, "isEnabled": true,
        "kind": "template", "numberPadding": 3, "numberPosition": "after",
        "numberStart": 1, "removesEmoji": false, "replacement": "",
        "stripsDiacritics": false, "text": "{created} {name} {n:3}"
      }
    ],
    "trimsWhitespace": true
  }
]
```

**`history.json`** (pretty): one snapshot, `"date": "2026-05-11T09:30:00Z"`, two entries with absolute from/to paths (make one a folder move whose second entry lives beneath it, to exercise §8.7 ordering on revert).

**`session.json`** (compact, sorted keys): every field of §4.3 populated, including one file entry with `overrideName`, one with `isFromFolder: true`, a `watchedFolderPaths` entry, an `excludedPaths` entry — plus a **second** legacy variant using `sortOrderRaw: "Name (Z–A)"` (no `sortKeyRaw`/`sortAscending`) to pin the migration.

Also fixture a rule with **missing optional fields** (only `kind` + `text`) asserting all defaults land (incl. `restartPerFolder: false`), one with an unknown extra field asserting it's ignored, and a legacy session lacking `keepRulesAfterApply` + `watchedFolderSubfolders` asserting `false` / all-`null` (§4.3).

---

*End of specification. Build well — and when in doubt, protect the user's files first.*
