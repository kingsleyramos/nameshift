// One file row (§14.2): checkbox · icon · current name [· folder suffix] ·
// → · [⚠] new name [✎] · hover ✕. Selection wash + accent bar sit on top
// of the conflict wash; a skipped row dims content only.

import { memo, useEffect, useRef, useState } from 'react';
import { File, Folder, Pencil, TriangleAlert } from 'lucide-react';
import type { PreviewEntry } from '../../ipc/gen/PreviewEntry';
import type { Problem } from '../../ipc/gen/Problem';
import { NewName, OldName } from './NameDiff';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export const ROW_HEIGHT = 28;

/** §A problem copy for tooltips; the per-OS variant comes from the shell's
 * preview payload messages — the UI keys off the problem kind. */
export function problemLabel(problem: Problem, os: string): string {
  switch (problem) {
    case 'EmptyName':
      return 'The new name would be empty.';
    case 'InvalidCharacters':
      return os === 'windows'
        ? 'The new name contains characters Windows doesn’t allow: < > : " / \\ | ? *'
        : os === 'linux'
          ? 'The new name contains “/”, which isn’t allowed.'
          : 'The new name contains “/” or “:”, which aren’t allowed.';
    case 'NameTooLong':
      return os === 'windows'
        ? 'The new name is longer than Windows allows (255 characters).'
        : os === 'linux'
          ? 'The new name is longer than this system allows (255 bytes).'
          : 'The new name is longer than macOS allows (255 bytes).';
    case 'ReservedName':
      return 'This name is reserved by Windows and can’t be used.';
    case 'EndsWithDotOrSpace':
      return 'Windows names can’t end with a dot or a space.';
    case 'DuplicateTarget':
      return 'Two or more files would end up with the same name.';
    case 'ExistingFileCollision':
      return 'A different file with this name already exists in the folder.';
    case 'UnrenamableName':
      return 'This name uses an encoding Name Shift can’t edit safely.';
    case 'NoFolderPermission':
      return 'Name Shift doesn’t have permission to rename items in this folder.';
  }
}

interface FileRowProps {
  entry: PreviewEntry;
  isDirectory: boolean;
  folderSuffix: string | null;
  selected: boolean;
  focused: boolean;
  editing: boolean;
  inlineDiff: boolean;
  os: string;
  onRowClick: (event: React.MouseEvent) => void;
  onCheckboxClick: (event: React.MouseEvent) => void;
  onStartEdit: () => void;
  onEndEdit: () => void;
  onContextMenu: (event: React.MouseEvent) => void;
}

export default memo(function FileRow({
  entry,
  isDirectory,
  folderSuffix,
  selected,
  focused,
  editing,
  inlineDiff,
  os,
  onRowClick,
  onCheckboxClick,
  onStartEdit,
  onEndEdit,
  onContextMenu,
}: FileRowProps) {
  const skipped = !entry.isSelected;
  const conflictWash = entry.problem != null;
  const Icon = isDirectory ? Folder : File;

  return (
    <div
      role="option"
      aria-selected={selected}
      aria-label={`${entry.currentName}, ${skipped ? 'skipped' : 'included'}${
        entry.problem ? `, naming conflict: ${problemLabel(entry.problem, os)}` : ''
      }`}
      tabIndex={-1}
      data-row-id={entry.id}
      onClick={onRowClick}
      onContextMenu={onContextMenu}
      className="group relative flex h-full items-center gap-2 px-2 text-[13px]"
      style={{
        background: conflictWash ? 'var(--wash-conflict)' : undefined,
        outline: focused ? '2px solid var(--accent)' : undefined,
        outlineOffset: -2,
      }}
    >
      {selected && (
        <>
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0"
            style={{ background: 'var(--wash-selected)' }}
          />
          <div
            aria-hidden
            className="pointer-events-none absolute inset-y-0 left-0 w-[3px]"
            style={{ background: 'var(--accent)' }}
          />
        </>
      )}
      <input
        type="checkbox"
        aria-label={`Include ${entry.currentName}`}
        checked={entry.isSelected}
        onClick={onCheckboxClick}
        onChange={() => {
          /* handled in onClick for range logic */
        }}
        className="relative z-10 h-3.5 w-3.5 accent-[var(--accent)]"
      />
      <div
        className="relative z-10 flex min-w-0 flex-1 items-center gap-2"
        style={{ opacity: skipped ? 0.65 : 1 }}
      >
        <Icon size={14} aria-hidden style={{ color: 'var(--text-secondary)' }} className="shrink-0" />
        {inlineDiff ? (
          <span className="min-w-0 flex-1 truncate" style={{ color: 'var(--text-primary)' }}>
            <NewName oldName={entry.currentName} newName={entry.newName} />
          </span>
        ) : (
          <>
            <span className="min-w-0 flex-1 truncate" style={{ color: 'var(--text-primary)' }}>
              <OldName oldName={entry.currentName} newName={entry.newName} />
              {folderSuffix != null && (
                <span
                  className="ml-1.5 text-[11.5px]"
                  style={{ color: 'var(--text-secondary)' }}
                  title={folderSuffix}
                >
                  {folderSuffix}
                </span>
              )}
            </span>
            <span aria-hidden className="shrink-0" style={{ color: 'var(--text-secondary)' }}>
              →
            </span>
            <span className="flex min-w-0 flex-1 items-center gap-1">
              {entry.problem != null && (
                <span title={problemLabel(entry.problem, os)} className="shrink-0">
                  <TriangleAlert size={13} aria-hidden style={{ color: 'var(--warning-text)' }} />
                </span>
              )}
              {editing ? (
                <InlineEditor entry={entry} onDone={onEndEdit} />
              ) : (
                <button
                  className="min-w-0 flex-1 truncate text-left"
                  style={{ color: 'var(--text-primary)', cursor: 'default' }}
                  onDoubleClick={onStartEdit}
                  tabIndex={-1}
                >
                  <NewName oldName={entry.currentName} newName={entry.newName} />
                </button>
              )}
              {entry.hasOverride && !editing && (
                <Pencil size={12} aria-hidden className="shrink-0" style={{ color: 'var(--text-secondary)' }} />
              )}
              {!editing && (
                <button
                  aria-label={`Edit new name for ${entry.currentName}`}
                  title="Edit new name"
                  onClick={(event) => {
                    // One click, one effect: the pencil beats row selection
                    // (§14.2 UX-02).
                    event.stopPropagation();
                    onStartEdit();
                  }}
                  className="z-10 hidden h-[22px] w-[22px] shrink-0 items-center justify-center rounded hover:bg-[var(--wash-hover)] group-hover:flex"
                  style={{ color: 'var(--text-secondary)' }}
                >
                  <Pencil size={12} aria-hidden />
                </button>
              )}
            </span>
          </>
        )}
      </div>
      {skipped && (
        <span
          className="relative z-10 shrink-0 rounded-full px-2 py-0.5 text-[11px] font-medium"
          style={{ background: 'var(--wash-hover)', color: 'var(--text-primary)', minWidth: 52, textAlign: 'center' }}
        >
          {strings.skippedPill}
        </span>
      )}
      <button
        aria-label={`Remove ${entry.currentName} from the list`}
        title={strings.removeFromList}
        onClick={(event) => {
          event.stopPropagation();
          void commands.removeItems([entry.id]);
        }}
        className="relative z-10 hidden h-[22px] w-[22px] shrink-0 items-center justify-center rounded hover:bg-[var(--wash-hover)] group-hover:flex"
        style={{ color: 'var(--text-secondary)' }}
      >
        ✕
      </button>
    </div>
  );
});

function InlineEditor({ entry, onDone }: { entry: PreviewEntry; onDone: () => void }) {
  const [value, setValue] = useState(entry.newName);
  const input = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);

  const commit = async () => {
    const trimmed = value.trim();
    if (trimmed.length === 0 || trimmed === entry.newName) {
      if (trimmed !== entry.newName) {
        onDone();
        return;
      }
    }
    // Committing text identical to the rules' own output CLEARS the
    // override instead of pinning it (§14.2 commit guard).
    const derived = await commands.ruleDerivedName(entry.id);
    if (trimmed === derived) {
      await commands.setOverride(entry.id, null);
    } else {
      await commands.setOverride(entry.id, trimmed);
    }
    onDone();
  };

  return (
    <input
      ref={input}
      value={value}
      aria-label={`New name for ${entry.currentName}`}
      onChange={(event) => {
        setValue(event.target.value);
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          void commit();
        } else if (event.key === 'Escape') {
          event.stopPropagation();
          onDone();
        }
      }}
      onBlur={() => {
        // Focus loss commits (Finder behavior, §14.2).
        void commit();
      }}
      className="min-w-0 flex-1 rounded border px-1 text-[13px] outline-none"
      style={{
        borderColor: 'var(--accent)',
        background: 'var(--pane-bg)',
        color: 'var(--text-primary)',
      }}
    />
  );
}
