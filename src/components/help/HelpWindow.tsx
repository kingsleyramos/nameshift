// The Help window (§14.12): topic sidebar + article pane, deep-linkable.
// Content per §22.5 — it must match the app's behaviors exactly.

import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { isMac } from '../../lib/platform';

interface Topic {
  id: string;
  title: string;
  body: React.ReactNode;
}

const mod = isMac ? '⌘' : 'Ctrl+';

function topics(): Topic[] {
  return [
    {
      id: 'getting-started',
      title: 'Getting Started',
      body: (
        <>
          <p>
            Add files by dragging them onto the window or pressing {mod}O. Importing a folder
            watches it: files added or removed in your file manager appear in the list
            automatically, and the folder shows as a chip above the list.
          </p>
          <p>
            Build rules on the left and watch the live before → after preview. When it looks
            right, press <strong>Rename</strong> (or {isMac ? '⌘↩' : 'Ctrl+Enter'}). A banner
            confirms how many files were renamed, with one-click <em>Revert…</em>.
          </p>
          <p>
            Your whole workspace — rules, toggles, file list, watched folders — is restored the
            next time you open the app. Files that disappeared in the meantime are counted and
            removed with a notice; nothing on disk is changed.
          </p>
          <p>
            A determinate progress overlay appears for long batches; Esc cancels safely — files
            already renamed stay renamed (and revertible), everything else is untouched.
          </p>
        </>
      ),
    },
    {
      id: 'rename-rules',
      title: 'Rename Rules',
      body: (
        <>
          <p>
            Rules run top to bottom on every included file. Reorder them by dragging (or the
            ↑/↓ buttons); the switch on each card disables it without losing its settings; ✕
            removes it ({mod}Z restores).
          </p>
          <ul>
            <li>
              <strong>Remove Text / Find &amp; Replace</strong> — plain text, with optional Match
              case.
            </li>
            <li>
              <strong>Regex Replace</strong> — full regular expressions with <code>$1</code>{' '}
              group references. Patterns use Rust regex syntax with lookaround support. An
              invalid pattern turns the field red and the rule is skipped. Tokens in the
              replacement expand first, so a token that expands to a literal <code>$1</code> is
              treated as a group reference.
            </li>
            <li>
              <strong>Add Prefix / Add Suffix</strong> — suffixes go before the extension.
            </li>
            <li>
              <strong>Change Case</strong> — lowercase, UPPERCASE, or Title Case. Title Case
              keeps all-caps words of two or more letters (NASA, HDR); mixed-case words
              normalize (iPhone → Iphone), and an apostrophe splits words (it’s → It’S).
            </li>
            <li>
              <strong>Change Extension</strong> — files only; a file without an extension gains
              one.
            </li>
            <li>
              <strong>Fix Unsafe Characters</strong> — makes names Windows/NAS/cloud-safe:
              replaces <code>&lt; &gt; : " / \ | ? *</code> and control characters, optionally
              strips accents and emoji, trims trailing dots and spaces, and guards
              Windows-reserved names (CON.txt → CON_.txt) on every OS.
            </li>
            <li>
              <strong>Number Sequentially</strong> — a counter that follows the current sort
              order; skipped files don’t consume numbers. It can restart in each folder.
            </li>
            <li>
              <strong>New Name from Template</strong> — rebuild the whole name from tokens.
            </li>
          </ul>
          <p>
            By default rules never touch the extension — each rule has an{' '}
            <em>Include extension</em> option. The pinned <em>Trim spaces from names</em> pass
            runs after all rules.
          </p>
        </>
      ),
    },
    {
      id: 'tokens',
      title: 'Tokens',
      body: (
        <>
          <p>
            Tokens are <code>{'{key}'}</code> or <code>{'{key:argument}'}</code> placeholders
            expanded per file. They work in templates, prefixes, suffixes, replacement fields,
            the number separator, and Change Extension — never in search text or regex
            patterns. Unknown tokens stay visible in the preview so typos are easy to spot.
          </p>
          <ul>
            <li>
              <code>{'{name}'}</code> the name at this rule’s stage · <code>{'{ext}'}</code> the
              extension · <code>{'{folder}'}</code> the containing folder
            </li>
            <li>
              <code>{'{n}'}</code> / <code>{'{n:3}'}</code> sequence number, zero-padded
            </li>
            <li>
              <code>{'{created}'}</code> / <code>{'{modified}'}</code> /{' '}
              <code>{'{date}'}</code> — default format <code>yyyy-MM-dd</code>; custom formats
              like <code>{'{created:yyyy-MM-dd HH.mm}'}</code>. Dates always render with
              English names and the Gregorian calendar so filenames stay stable. On Linux
              filesystems without birth time, <code>{'{created}'}</code> stays visible as an
              unknown token rather than guessing.
            </li>
            <li>
              <code>{'{size}'}</code> e.g. 1.2 MB · <code>{'{kind}'}</code> e.g. JPEG image
            </li>
            <li>
              <code>{'{md:…}'}</code> any metadata attribute. Attribute names are
              platform-specific by design — macOS Spotlight names
              (<code>{'{md:kMDItemPixelHeight}'}</code>), Windows Property System names
              (<code>{'{md:System.Photo.CameraModel}'}</code>), Linux EXIF/xattr names
              (<code>{'{md:exif:Model}'}</code>). The File Info drawer lists exactly what’s
              available on this machine, each with a copy button.
            </li>
          </ul>
        </>
      ),
    },
    {
      id: 'selecting',
      title: 'Selecting, Sorting & Filtering',
      body: (
        <>
          <p>
            Two different things have two different names. <strong>Inclusion</strong> is the
            checkbox: only checked files are renamed, and unchecked rows read{' '}
            <em>Skipped</em>. <strong>Selection</strong> is the highlighted row: it drives the
            File Info drawer, keyboard operations, and the context menu — it never affects
            renaming.
          </p>
          <p>
            Shift-click a checkbox to include or skip a whole range. Click selects a row;{' '}
            {isMac ? '⌘' : 'Ctrl'}-click toggles; ⇧-click extends; {mod}A selects all rows. On
            the selection: Space toggles inclusion, Return edits the new name, Delete removes
            from the list (undoable).
          </p>
          <p>
            Sort by name, extension, folder, or date — numbering follows the sort. The
            status-bar counts are clickable filters (will change, naming conflicts, edited);
            filtering changes what you see, never what Rename does. While a filter is active,
            the <em>matching</em> menu offers Select Only These and Remove These from List.
          </p>
        </>
      ),
    },
    {
      id: 'folders',
      title: 'Renaming Folders',
      body: (
        <>
          <p>
            The Folders tab switches the whole app to renaming directories. Folder names are
            treated as one whole string — there’s no extension, and Change Extension is
            skipped.
          </p>
          <p>
            When a folder is renamed, every tracked path underneath it updates automatically —
            files inside keep working in the list, and watched folders keep watching.
          </p>
          <p>
            Reverting works newest-first: reverting an older version also undoes the newer
            ones above it, so nested renames restore cleanly.
          </p>
        </>
      ),
    },
    {
      id: 'manual-edits',
      title: 'Manual Edits & Rename by CSV',
      body: (
        <>
          <p>
            Double-click a file’s new name (or press Return) to type an exact name for that
            one file. Manual edits win over the rules and are marked with a pencil. Typing a
            name identical to what the rules already produce clears the edit instead of
            pinning it.
          </p>
          <p>
            <strong>Rename by CSV</strong> (Options ⋯): download a template CSV of your current
            names, edit the New Name column in any spreadsheet app, and drop the file back.
            You’ll see how many names will change — and any rows that didn’t match, grouped by
            reason — before anything is applied. Applying creates manual edits; one {mod}Z
            removes the whole batch. Rows you leave unchanged do nothing, and a new name typed
            without an extension keeps the file’s current one.
          </p>
        </>
      ),
    },
    {
      id: 'conflicts',
      title: 'Renaming & Naming Conflicts',
      body: (
        <>
          <p>
            A warning marks anything that can’t be renamed safely: duplicate targets, a name
            that already exists in the folder, empty or invalid names, names too long, and
            Windows-reserved names. Naming conflicts block renaming until you fix them, click{' '}
            <em>Skip Conflicted</em>, or turn on <em>Auto-resolve</em>, which appends 2, 3… to
            colliding names (files keeping their current name always win).
          </p>
          <p>
            Renames run in two phases — every file moves to a hidden temporary name, then to
            its final name — so name swaps (a ↔ b) and case-only renames (readme → README)
            can never collide. Renaming a file to a name another file is giving up in the same
            batch is safe for the same reason.
          </p>
        </>
      ),
    },
    {
      id: 'history',
      title: 'History & Revert',
      body: (
        <>
          <p>
            Every rename batch is a version in the History tab (the 50 most recent are kept).
            Click a version to preview exactly what a revert will restore: files moved or
            deleted since are flagged and skipped, and a file whose original name is now taken
            by a different file is kept as is.
          </p>
          <p>
            Reverting an older version also undoes the newer versions above it — the
            confirmation says so with real counts. Reverted versions leave History; a revert
            you cancel keeps the partially-reverted version so you can finish later.
          </p>
        </>
      ),
    },
    {
      id: 'presets',
      title: 'Rule Presets',
      body: (
        <>
          <p>
            Save the current rule stack (plus the Trim spaces toggle) as a named preset from
            the Presets menu, and load it back anytime — loading is undoable. Presets export
            and import as portable JSON files you can share between machines; imports never
            overwrite, they add “(imported)” on name collisions.
          </p>
        </>
      ),
    },
    {
      id: 'recipes',
      title: 'Recipes',
      body: (
        <>
          <p>Three worked examples to copy:</p>
          <p>
            <strong>Date-stamp a photo shoot.</strong> One rule — New Name from Template with{' '}
            <code>{'{created} {name} {n:3}'}</code>. <br />
            <code>IMG_0421.jpg → 2026-07-27 IMG_0421 001.jpg</code>
          </p>
          <p>
            <strong>Clean up download names.</strong> Find &amp; Replace <code>_</code> with a
            space, then Change Case → Title Case, then Fix Unsafe Characters. <br />
            <code>project_final(2).PDF → Project Final(2).PDF</code>
          </p>
          <p>
            <strong>Number in shoot order.</strong> Sort by Date Created, then Number
            Sequentially (before name, separator <code>-</code>, 3 digits). <br />
            <code>beach.jpg → 001-beach.jpg</code> — deselected files don’t consume numbers.
          </p>
        </>
      ),
    },
    {
      id: 'file-info',
      title: 'File Info & Metadata',
      body: (
        <>
          <p>
            The File Info drawer ({mod}I, or click a row) shows Kind, Size, Created, Modified,
            and Folder — each with a copy button for the matching token — plus every metadata
            attribute the file has on this machine, with one-click copies of its{' '}
            <code>{'{md:…}'}</code> token.
          </p>
        </>
      ),
    },
    {
      id: 'shortcuts',
      title: 'Keyboard Shortcuts',
      body: (
        <>
          <ul>
            <li>Add files or folders — {mod}O</li>
            <li>Rename — {isMac ? '⌘↩' : 'Ctrl+Enter'}</li>
            <li>File Info drawer — {mod}I</li>
            <li>Find / filter — {mod}F</li>
            <li>Undo / Redo — {mod}Z / {isMac ? '⇧⌘Z' : 'Ctrl+Shift+Z'}</li>
            <li>Settings — {isMac ? '⌘,' : 'Ctrl+,'}</li>
            <li>Help — {isMac ? '⌘?' : 'F1'}</li>
            <li>Esc — cancel processing, close a name edit, or clear the filter</li>
          </ul>
        </>
      ),
    },
  ];
}

export default function HelpWindow() {
  const allTopics = useMemo(topics, []);
  const initial = new URLSearchParams(window.location.search).get('topic');
  const [active, setActive] = useState(
    allTopics.find((topic) => topic.id === initial)?.id ?? 'getting-started',
  );

  useEffect(() => {
    const unlisten = listen<string>('help-topic', (event) => {
      setActive(event.payload);
    });
    return () => {
      void unlisten.then((dispose) => {
        dispose();
      });
    };
  }, []);

  const current = allTopics.find((topic) => topic.id === active) ?? allTopics[0];

  return (
    <div className="flex h-full" style={{ background: 'var(--pane-bg)' }}>
      <nav
        className="w-52 shrink-0 overflow-auto py-2"
        style={{ borderRight: '1px solid var(--separator)', background: 'var(--window-bg)' }}
        aria-label="Help topics"
      >
        {allTopics.map((topic) => (
          <button
            key={topic.id}
            onClick={() => {
              setActive(topic.id);
            }}
            aria-current={active === topic.id}
            className="block w-full px-3 py-1.5 text-left text-[13px]"
            style={{
              background: active === topic.id ? 'var(--wash-selected)' : undefined,
              color: 'var(--text-primary)',
            }}
          >
            {topic.title}
          </button>
        ))}
      </nav>
      <article className="ns-help min-w-0 flex-1 overflow-auto px-6 py-4">
        <h1 className="text-[17px] font-semibold" style={{ color: 'var(--text-primary)' }}>
          {current?.title}
        </h1>
        <div className="mt-2 space-y-2 text-[13px] leading-relaxed" style={{ color: 'var(--text-primary)' }}>
          {current?.body}
        </div>
      </article>
    </div>
  );
}
