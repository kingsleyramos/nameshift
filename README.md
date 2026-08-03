<div align="center">

<img src="docs/screenshots/icon.png" alt="Name Shift icon" width="128" height="128">

# Name Shift

**Bulk-rename files and folders with live preview, safe two-phase renames, and full undo history.**

[![CI](https://github.com/kingsleyramos/nameshift/actions/workflows/ci.yml/badge.svg)](https://github.com/kingsleyramos/nameshift/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/kingsleyramos/nameshift?label=download)](https://github.com/kingsleyramos/nameshift/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platforms](https://img.shields.io/badge/platforms-macOS%20·%20Windows%20·%20Linux-8A2BE2)

Stack up rename rules on the left, watch a live **before → after** diff of every
name on the right, and press **Rename**. Every batch is saved as a version you
can preview and revert — your files are never one bad regex away from chaos.

<img src="docs/screenshots/main-light.png" alt="Name Shift main window" width="800">

</div>

---

## Highlights

- 🪶 **Lightweight & native-feeling** — a small desktop app (built with [Tauri](https://tauri.app)), not a bundled browser. Fast on 10,000-file batches.
- 🔍 **Live preview with character-level diffs** — removed spans struck through in red, added spans in green, before you touch the disk.
- 🛟 **Safe by design** — renames run in two phases, so name swaps (`a↔b`) and case-only renames (`readme → README`) can never collide. Naming conflicts are detected and block renaming until fixed.
- ⏪ **Version history** — every rename batch is a snapshot. Preview exactly what a revert will restore, then roll back one version or ten.
- 📁 **Files *and* folders** — a dedicated Folders mode renames whole directories; every tracked path underneath updates automatically.
- 👀 **Watched folders** — import a folder and it stays live: files added or removed in your file manager appear in the list automatically.

## Features

### Rules

Rules run top to bottom on every included file, and each rule card shows how many files it affects. Reorder, toggle, duplicate, or delete them freely — ⌘Z / Ctrl+Z restores the stack.

| Rule | What it does |
|---|---|
| **Remove Text** | Deletes every occurrence of the text |
| **Find & Replace** | Plain-text replace, with optional Match case |
| **Regex Replace** | Full regular expressions with `$1` group references |
| **Add Prefix / Add Suffix** | Added before or after the name — suffixes go before the extension |
| **Change Case** | lowercase, UPPERCASE, or Title Case (acronyms like NASA are kept) |
| **Change Extension** | e.g. `jpeg → jpg` |
| **Fix Unsafe Characters** | Makes names Windows/NAS/cloud-safe: replaces `< > : " / \ \| ? *`, fixes Windows-reserved names like `CON.txt`, trims trailing dots/spaces, optionally strips accents and emoji |
| **Number Sequentially** | Counter that follows the current sort order — sort by date, number in shoot order; can restart in each folder |
| **New Name from Template** | Rebuild the whole name from tokens, like `{created} {name} {n:3}` |

By default rules never touch the extension (each rule has an *Include extension* option), and a global **Trim spaces from names** pass cleans up leading/trailing spaces after all rules run.

### Tokens

Tokens expand per file inside templates, prefixes, suffixes, replacement fields, and number separators. Unknown tokens stay visible so typos are easy to spot.

| Token | Expands to |
|---|---|
| `{n}` / `{n:3}` | Sequence number, optionally zero-padded (`001, 002…`) — deselected files don't consume numbers |
| `{name}` | Current name without extension |
| `{ext}` | Extension without the dot |
| `{folder}` | Name of the containing folder |
| `{created}` `{modified}` `{date}` | Created / modified / today — default format `2026-07-14`, custom formats like `{created:yyyy-MM-dd HH.mm}` |
| `{size}` | Human-readable file size, e.g. `1.2 MB` |
| `{kind}` | File kind, e.g. `JPEG image` |
| `{md:…}` | Any file-metadata attribute — see [platform notes](#platform-notes) |

Open the **{ } Tokens** reference in the rules panel, or use the `{ }` button on any token-capable field to insert one at the cursor. The **File Info** drawer shows every metadata attribute a file has, each with a one-click copy of its `{md:…}` token.

### Selection, sorting & filtering

- Every file has a checkbox (all included by default); skipped files aren't renamed and don't consume sequence numbers. Shift-click a checkbox to include or skip a whole range; clicking a row selects it for inspection and keyboard shortcuts.
- Sort by name, extension, folder, or date with an ↑/↓ direction toggle — numbering follows the sort, so a descending date sort numbers newest-first.
- Filter by search text, *will change*, or *naming conflicts* (the status-bar counts are clickable filters). **Select Only These** turns a filter into an exact selection.

### Manual edits & spreadsheet renaming

- Double-click any file's new name to type an exact name for that one file — manual edits win over the rules (marked with a pencil).
- Rename from a spreadsheet: **Rename by CSV** — download a template CSV of your current names, edit the *New Name* column in any spreadsheet app, then drop the file back. You'll see how many names will change (and any rows that didn't match) before anything is applied.

### Naming conflicts

A warning marks anything that can't be renamed safely — duplicate targets, collisions with existing files, empty or invalid names (checked against *this* operating system's rules, including Windows reserved names). Naming conflicts block renaming; resolve them, click **Skip Conflicted**, or enable **Auto-Resolve** to append ` 2`, ` 3`… to colliding names.

### History & revert

Every rename batch is a persisted snapshot (the 50 most recent are kept). Click a version to preview exactly what a revert will restore — files moved or deleted since are flagged and skipped; a file whose original name is now taken is kept as is. Reverting an older version also undoes the newer ones above it, and the confirmation tells you so.

### Presets & sessions

Save a rule stack as a named preset and reapply it anytime; share presets between machines as portable `.json` files. Your whole workspace — rules, toggles, file list, watched folders — is restored on the next launch.

## Install

### macOS (11+)

Download the `.dmg` from the [latest release](https://github.com/kingsleyramos/nameshift/releases/latest), open it, and drag **Name Shift** to Applications. Universal binary (Apple Silicon + Intel).
<!-- mac-app-store-badge: add link when live -->

### Windows (10 1809+)

Download the installer (`.exe`) from the [latest release](https://github.com/kingsleyramos/nameshift/releases/latest) — WebView2 is set up automatically.
<!-- ms-store-badge: add link when live -->

### Linux

Download the `.AppImage`, `.deb`, or `.rpm` from the [latest release](https://github.com/kingsleyramos/nameshift/releases/latest). Requires `webkit2gtk-4.1` (present on Ubuntu 22.04+, Fedora 36+, and current Arch/openSUSE).
<!-- flathub-badge: add link when live -->

Direct downloads update themselves through the built-in updater; store installs update through their store.

## Quick start

1. **Add files** — drag files or folders onto the window, or press ⌘O / Ctrl+O. Folders are watched live.
2. **Stack rules** — add a rule (say, *Rename to Template* with `{created} {name} {n:3}`) and watch the preview update as you type.
3. **Check the preview** — green additions, red strikethrough removals, warnings on any naming conflict.
4. **Rename** — ⌘↩ / Ctrl+Enter renames on disk. A banner confirms the count with a one-click **Revert…**.

## Platform notes

| | macOS | Windows | Linux |
|---|---|---|---|
| `{md:…}` metadata source | Spotlight (`{md:kMDItemPixelHeight}`) | Windows Property System (`{md:System.Photo.CameraModel}`) | EXIF + extended attributes (`{md:exif:Model}`) |
| `{created}` | ✅ | ✅ | ✅ where the filesystem records birth time |
| Name validation | `/` and `:` | Reserved names, `< > : " / \ \| ? *`, no trailing dot/space | `/` |
| Reveal in… | Finder | File Explorer | Your file manager |

Metadata attribute names are platform-specific by design — the **File Info** drawer always shows exactly what's available for a file on *your* machine, with copyable tokens.

## Keyboard shortcuts

| | macOS | Windows / Linux |
|---|---|---|
| Add files or folders | ⌘O | Ctrl+O |
| Rename | ⌘↩ | Ctrl+Enter |
| File Info drawer | ⌘I | Ctrl+I |
| Find / filter | ⌘F | Ctrl+F |
| Undo rule change | ⌘Z | Ctrl+Z |
| Settings | ⌘, | Ctrl+, |
| Help | ⌘? | F1 |

## Building from source

Prerequisites: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+, [pnpm](https://pnpm.io), and the [Tauri platform dependencies](https://tauri.app/start/prerequisites/) for your OS.

```sh
git clone https://github.com/kingsleyramos/nameshift.git
cd nameshift
pnpm install
pnpm tauri dev      # run in development
pnpm tauri build    # build a release bundle for your OS
```

### Running the tests

```sh
cargo test --workspace   # engine, persistence, watcher, metadata — unit + on-disk integration + property tests
pnpm test                # frontend component tests (Vitest)
pnpm test:ui             # interface tests (every OS; see docs/TESTING.md)
```

The suite includes real-filesystem apply/revert tests, fuzzers, and a round-trip property test (*apply then revert restores the original tree*) — please keep it green.

### Project layout

```
src/            React + TypeScript UI
src-tauri/      Tauri shell — commands, events, menus (thin)
crates/engine   The rename engine: rules, tokens, preview, two-phase apply/revert
crates/store    Session, history, and preset persistence
crates/watcher  Live folder watching
crates/metadata Per-OS file metadata providers
```

## Contributing

Issues and pull requests are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for setup, test gates, and conventions. Behavior changes should update the in-app Help and this README in the same PR.

## License

[MIT](LICENSE) © Kingsley Ramos
