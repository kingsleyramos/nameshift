// The file list (§14.2): scope tabs, chips, real header controls, a
// virtualized listbox with roving focus, the filter model, and the two
// named concepts — row *selection* (ephemeral) vs checkbox *inclusion*
// (domain state, the only thing Rename reads).

import { useCallback, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import type { PreviewEntry } from '../../ipc/gen/PreviewEntry';
import FileRow, { ROW_HEIGHT } from './FileRow';
import ModeSwitch from './ModeSwitch';
import SortMenu from './SortMenu';
import WatchedFolderChips from './WatchedFolderChips';
import Menu, { MenuItem } from '../shell/Menu';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import { hasPrimaryModifier, revealLabel } from '../../lib/platform';
import * as commands from '../../ipc/commands';

export default function FileListPane() {
  const snapshot = useAppStore((s) => s.snapshot);
  const preview = useAppStore((s) => s.preview);
  const platform = useAppStore((s) => s.platform);
  const ui = useUiStore();
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const os = platform?.os ?? 'macos';
  const entries = useMemo(() => preview?.entries ?? [], [preview]);
  const itemsById = useMemo(() => {
    const map = new Map<string, { path: string; isDirectory: boolean; folder: string }>();
    for (const item of snapshot?.files ?? []) {
      const slash = item.path.lastIndexOf('/');
      map.set(item.id, {
        path: item.path,
        isDirectory: item.isDirectory,
        folder: slash > 0 ? item.path.slice(0, slash) : item.path,
      });
    }
    return map;
  }, [snapshot]);

  // Filtering changes visibility only — never what Rename does (§14.2).
  const visible = useMemo(() => {
    const text = ui.filterText.toLowerCase();
    return entries.filter((entry) => {
      if (
        text.length > 0 &&
        !entry.currentName.toLowerCase().includes(text) &&
        !entry.newName.toLowerCase().includes(text)
      ) {
        return false;
      }
      switch (ui.filterMode) {
        case 'willChange':
          return entry.isChanged;
        case 'conflicts':
          return entry.problem != null;
        case 'edited':
          return entry.hasOverride;
        default:
          return true;
      }
    });
  }, [entries, ui.filterText, ui.filterMode]);

  const spansMultipleFolders = useMemo(() => {
    const folders = new Set<string>();
    for (const entry of visible) {
      const folder = itemsById.get(entry.id)?.folder;
      if (folder != null) folders.add(folder);
      if (folders.size > 1) return true;
    }
    return false;
  }, [visible, itemsById]);

  const virtualizer = useVirtualizer({
    count: visible.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  const filterActive = ui.filterText.length > 0 || ui.filterMode !== 'all';

  // ---- row selection (frontend-only, §24 Q28) -----------------------------

  const rowClick = useCallback(
    (entry: PreviewEntry, event: React.MouseEvent) => {
      const state = useUiStore.getState();
      const selection = new Set(state.rowSelection);
      if (event.shiftKey && state.selectionAnchor != null) {
        const ids = visible.map((e) => e.id);
        const from = ids.indexOf(state.selectionAnchor);
        const to = ids.indexOf(entry.id);
        if (from !== -1 && to !== -1) {
          selection.clear();
          for (const id of ids.slice(Math.min(from, to), Math.max(from, to) + 1)) {
            selection.add(id);
          }
        }
        state.set({ rowSelection: selection, focusedRow: entry.id, inspectedId: entry.id });
      } else if (hasPrimaryModifier(event)) {
        if (selection.has(entry.id)) {
          selection.delete(entry.id);
        } else {
          selection.add(entry.id);
        }
        state.set({
          rowSelection: selection,
          focusedRow: entry.id,
          selectionAnchor: entry.id,
          inspectedId: entry.id,
        });
      } else {
        state.selectSingle(entry.id);
      }
    },
    [visible],
  );

  // ⇧-clicking a checkbox applies its new state to the whole visible range
  // from the last-toggled anchor (§14.2 FEAT-02); one call = one undo entry.
  const checkboxClick = useCallback(
    (entry: PreviewEntry, event: React.MouseEvent) => {
      event.stopPropagation();
      const state = useUiStore.getState();
      const next = !entry.isSelected;
      if (event.shiftKey && state.checkboxAnchor != null) {
        const ids = visible.map((e) => e.id);
        const from = ids.indexOf(state.checkboxAnchor);
        const to = ids.indexOf(entry.id);
        if (from !== -1 && to !== -1) {
          const range = ids.slice(Math.min(from, to), Math.max(from, to) + 1);
          void commands.setSelectedMany(range, next);
          state.set({ checkboxAnchor: entry.id });
          return;
        }
      }
      void commands.setSelected(entry.id, next);
      state.set({ checkboxAnchor: entry.id });
    },
    [visible],
  );

  const removeSelection = useCallback(
    (ids: string[]) => {
      const affected = snapshot?.files.filter((f) => ids.includes(f.id)) ?? [];
      const edits = affected.filter((f) => f.overrideName != null).length;
      if (ids.length > 10 || edits > 0) {
        useUiStore.getState().showConfirm({
          title: strings.bulkRemoveTitle(ids.length, snapshot?.listMode === 'Folders'),
          message: strings.bulkRemoveMessage(edits),
          confirmLabel: strings.bulkRemoveButton,
          destructive: true,
          onConfirm: () => {
            void commands.removeItems(ids);
          },
        });
      } else {
        void commands.removeItems(ids);
      }
      useUiStore.getState().clearSelection();
    },
    [snapshot],
  );

  const keyDown = useCallback(
    (event: React.KeyboardEvent) => {
      const state = useUiStore.getState();
      const ids = visible.map((e) => e.id);
      if (ids.length === 0) return;
      const focusIndex = state.focusedRow != null ? ids.indexOf(state.focusedRow) : -1;
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        const nextIndex =
          event.key === 'ArrowDown'
            ? Math.min(ids.length - 1, focusIndex + 1)
            : Math.max(0, focusIndex === -1 ? 0 : focusIndex - 1);
        const id = ids[nextIndex];
        if (id == null) return;
        virtualizer.scrollToIndex(nextIndex);
        if (event.shiftKey && state.selectionAnchor != null) {
          const from = ids.indexOf(state.selectionAnchor);
          const selection = new Set<string>(
            ids.slice(Math.min(from, nextIndex), Math.max(from, nextIndex) + 1),
          );
          state.set({ rowSelection: selection, focusedRow: id, inspectedId: id });
        } else {
          state.selectSingle(id);
        }
      } else if (event.key === ' ') {
        event.preventDefault();
        // Space toggles INCLUSION on the selection (§14.2).
        const targets = state.rowSelection.size > 0 ? [...state.rowSelection] : ids.slice(focusIndex, focusIndex + 1);
        const anyOff = entries.some((e) => targets.includes(e.id) && !e.isSelected);
        void commands.setSelectedMany(targets, anyOff);
      } else if (event.key === 'Enter' && state.focusedRow != null) {
        event.preventDefault();
        state.set({ editingRow: state.focusedRow });
      } else if ((event.key === 'Delete' || event.key === 'Backspace') && state.rowSelection.size > 0) {
        event.preventDefault();
        removeSelection([...state.rowSelection]);
      } else if (event.key === 'a' && hasPrimaryModifier(event)) {
        event.preventDefault();
        // ⌘A selects ROWS, never checkboxes (§15).
        state.set({ rowSelection: new Set(ids), focusedRow: ids[0] ?? null });
      }
    },
    [visible, entries, virtualizer, removeSelection],
  );

  // Header tri-state (§14.2): checked / unchecked / mixed over active items.
  const selectedCount = preview?.counts.selectedCount ?? 0;
  const allChecked = entries.length > 0 && selectedCount === entries.length;
  const someChecked = selectedCount > 0 && !allChecked;

  if (!snapshot || entries.length === 0) {
    return (
      <div className="flex h-full flex-col">
        <ScopeRow />
        <div
          className="flex flex-1 flex-col items-center justify-center gap-1 text-[13px]"
          style={{ color: 'var(--text-secondary)' }}
        >
          <p>{strings.dropPrompt}</p>
          <p>{strings.dropHint('⌘O')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full min-w-0 flex-col" data-testid="file-list">
      <ScopeRow />

      {/* Headers are real controls (§14.2). */}
      <div
        className="flex items-center gap-2 px-2 py-1 text-[12px] font-medium"
        style={{ borderBottom: '1px solid var(--separator)', color: 'var(--text-secondary)' }}
      >
        <input
          type="checkbox"
          aria-label="Include all"
          checked={allChecked}
          ref={(el) => {
            if (el) el.indeterminate = someChecked;
          }}
          onChange={() => {
            void commands.setAllSelected(!allChecked);
          }}
          className="h-3.5 w-3.5 accent-[var(--accent)]"
        />
        <button
          className="flex items-center gap-1 hover:underline"
          onClick={() => {
            if (snapshot.sortKey === 'Name') {
              void commands.setSort('Name', !snapshot.sortAscending);
            } else {
              void commands.setSort('Name', true);
            }
          }}
        >
          {strings.currentNameHeader}
          {snapshot.sortKey === 'Name' && <span aria-hidden>{snapshot.sortAscending ? '▲' : '▼'}</span>}
        </button>
        <div className="flex-1" />
        <span>{strings.newNameHeader}</span>
        <div className="flex-1" />
        {filterActive && (
          <Menu
            align="right"
            ariaLabel="Matching items"
            trigger={(_, toggle) => (
              <button
                onClick={toggle}
                className="rounded-md border px-1.5 py-0.5 text-[12px]"
                style={{ borderColor: 'var(--separator)', color: 'var(--link-text)' }}
              >
                {strings.matchingMenu(visible.length)} ▾
              </button>
            )}
          >
            {(close) => (
              <>
                <MenuItem
                  label={strings.selectOnlyThese}
                  onSelect={() => {
                    close();
                    void commands.selectOnly(visible.map((e) => e.id));
                  }}
                />
                <MenuItem
                  label={strings.removeTheseFromList}
                  destructive
                  onSelect={() => {
                    close();
                    removeSelection(visible.map((e) => e.id));
                  }}
                />
              </>
            )}
          </Menu>
        )}
        <SortMenu />
      </div>

      {visible.length === 0 ? (
        <div
          className="flex flex-1 flex-col items-center justify-center gap-2 text-[13px]"
          style={{ color: 'var(--text-secondary)' }}
        >
          <p>{strings.noFilesMatchFilter}</p>
          <button
            onClick={() => {
              ui.set({ filterText: '', filterMode: 'all' });
            }}
            className="rounded-md border px-2 py-0.5 text-[12px]"
            style={{ borderColor: 'var(--separator)', color: 'var(--link-text)' }}
          >
            {strings.clearFilter}
          </button>
        </div>
      ) : (
        <div
          ref={scrollRef}
          role="listbox"
          aria-multiselectable="true"
          aria-label="Files"
          tabIndex={0}
          onKeyDown={keyDown}
          className="min-h-0 flex-1 overflow-auto outline-none"
        >
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
            {virtualizer.getVirtualItems().map((row) => {
              const entry = visible[row.index];
              if (!entry) return null;
              const info = itemsById.get(entry.id);
              const folderName = info?.folder.split('/').pop() ?? '';
              return (
                <div
                  key={entry.id}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: row.size,
                    transform: `translateY(${String(row.start)}px)`,
                  }}
                >
                  <FileRow
                    entry={entry}
                    isDirectory={info?.isDirectory ?? false}
                    folderSuffix={spansMultipleFolders ? `· ${folderName}` : null}
                    selected={ui.rowSelection.has(entry.id)}
                    focused={ui.focusedRow === entry.id}
                    editing={ui.editingRow === entry.id}
                    inlineDiff={ui.inlineDiffView}
                    os={os}
                    onRowClick={(event) => {
                      rowClick(entry, event);
                    }}
                    onCheckboxClick={(event) => {
                      checkboxClick(entry, event);
                    }}
                    onStartEdit={() => {
                      ui.set({ editingRow: entry.id });
                    }}
                    onEndEdit={() => {
                      ui.set({ editingRow: null });
                    }}
                    onContextMenu={(event) => {
                      event.preventDefault();
                      if (!ui.rowSelection.has(entry.id)) {
                        ui.selectSingle(entry.id);
                      }
                      setContextMenu({ x: event.clientX, y: event.clientY });
                    }}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      {contextMenu && (
        <RowContextMenu
          position={contextMenu}
          os={os}
          onClose={() => {
            setContextMenu(null);
          }}
          onRemove={() => {
            removeSelection([...useUiStore.getState().rowSelection]);
          }}
          itemsById={itemsById}
        />
      )}
    </div>
  );
}

function ScopeRow() {
  return (
    <div
      className="flex items-center gap-3 px-2 py-1.5"
      style={{ borderBottom: '1px solid var(--separator)' }}
    >
      <ModeSwitch />
      <WatchedFolderChips />
      <div className="flex-1" />
    </div>
  );
}

function RowContextMenu({
  position,
  os,
  onClose,
  onRemove,
  itemsById,
}: {
  position: { x: number; y: number };
  os: string;
  onClose: () => void;
  onRemove: () => void;
  itemsById: Map<string, { path: string }>;
}) {
  const selection = [...useUiStore.getState().rowSelection];
  const entries = useAppStore.getState().preview?.entries ?? [];
  const selected = entries.filter((e) => selection.includes(e.id));
  const first = selected[0];

  const item = (label: string, action: () => void, destructive = false) => (
    <MenuItem
      label={label}
      destructive={destructive}
      onSelect={() => {
        onClose();
        action();
      }}
    />
  );

  return (
    <div className="fixed inset-0 z-50" onClick={onClose} onContextMenu={onClose}>
      <div
        role="menu"
        className="absolute min-w-48 py-1 shadow-lg"
        style={{
          left: position.x,
          top: position.y,
          background: 'var(--pane-bg)',
          border: '1px solid var(--separator)',
          borderRadius: 'var(--radius-overlay)',
        }}
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        {first &&
          item(strings.editNewName, () => {
            useUiStore.getState().set({ editingRow: first.id });
          })}
        {first?.hasOverride &&
          item(strings.clearManualEdit, () => {
            selected.forEach((e) => {
              void commands.setOverride(e.id, null);
            });
          })}
        {first &&
          item(strings.copyName, () => {
            void writeText(selected.map((e) => e.currentName).join('\n'));
          })}
        {first &&
          item(strings.copyNewName, () => {
            void writeText(selected.map((e) => e.newName).join('\n'));
          })}
        {first &&
          item(strings.copyPath, () => {
            void writeText(
              selected.map((e) => itemsById.get(e.id)?.path ?? '').join('\n'),
            );
          })}
        {first && item(strings.removeFromList, onRemove, true)}
        {first &&
          item(revealLabel(os), () => {
            void commands.revealInFileManager(first.id);
          })}
      </div>
    </div>
  );
}
