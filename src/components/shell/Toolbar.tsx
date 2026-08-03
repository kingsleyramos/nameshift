// The toolbar (§14.1): Add, Options, File Info, filter field — and no
// primary action (that lives in the action bar, §24 Q27).

import { Ellipsis, FolderPlus, Info, Search } from 'lucide-react';
import Menu, { MenuItem, MenuSection, MenuSeparator } from './Menu';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function Toolbar() {
  const snapshot = useAppStore((s) => s.snapshot);
  const filterText = useUiStore((s) => s.filterText);
  const setUi = useUiStore((s) => s.set);
  const showConfirm = useUiStore((s) => s.showConfirm);
  const revertMode = snapshot?.selectedSnapshotId != null;
  const overrideCount = snapshot?.overrideCount ?? 0;
  const activeCount =
    snapshot?.files.filter((f) => f.isDirectory === (snapshot.listMode === 'Folders')).length ?? 0;

  const toolButton =
    'flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-[13px] hover:bg-[var(--wash-hover)] disabled:opacity-40';

  return (
    <header
      className="flex items-center gap-2 px-3 py-2"
      style={{ background: 'var(--window-bg)', borderBottom: '1px solid var(--separator)' }}
    >
      <Menu
        ariaLabel="Add files or folders"
        trigger={(_, toggle) => (
          <button
            className={toolButton}
            style={{ borderColor: 'var(--separator)' }}
            onClick={toggle}
            disabled={revertMode}
            title="Add files or folders (⌘O)"
          >
            <FolderPlus size={15} aria-hidden />
            Add
          </button>
        )}
      >
        {(close) => (
          <>
            <MenuItem
              label="Files…"
              onSelect={() => {
                close();
                void commands.pickAndImport();
              }}
            />
            <MenuItem
              label="Folders…"
              onSelect={() => {
                close();
                void commands.pickAndImportFolders();
              }}
            />
          </>
        )}
      </Menu>

      <Menu
        ariaLabel={strings.optionsButton}
        trigger={(_, toggle) => (
          <button
            className={toolButton}
            style={{ borderColor: 'var(--separator)' }}
            onClick={toggle}
            disabled={revertMode}
            title={strings.optionsButton}
          >
            <Ellipsis size={15} aria-hidden />
            Options
          </button>
        )}
      >
        {(close) => (
          <>
            <MenuSection title={strings.optionsImporting} />
            <MenuItem
              label={strings.includeSubfolders}
              checked={snapshot?.includeSubfolders}
              onSelect={() => {
                close();
                void commands.setIncludeSubfolders(!snapshot?.includeSubfolders);
              }}
            />
            <MenuSection title={strings.optionsRenaming} />
            <MenuItem
              label={strings.autoResolveNamingConflicts}
              checked={snapshot?.autoResolvesConflicts}
              onSelect={() => {
                close();
                void commands.setAutoResolve(!snapshot?.autoResolvesConflicts);
              }}
            />
            <MenuItem
              label={strings.keepRulesAfterApplying}
              checked={snapshot?.keepRulesAfterApply}
              onSelect={() => {
                close();
                void commands.setKeepRules(!snapshot?.keepRulesAfterApply);
              }}
            />
            <MenuItem
              label={strings.trimSpacesOption}
              checked={snapshot?.trimsWhitespace}
              onSelect={() => {
                close();
                void commands.setTrimsWhitespace(!snapshot?.trimsWhitespace);
              }}
            />
            <MenuSection title={strings.optionsSpreadsheet} />
            <MenuItem
              label={strings.renameByCsv}
              disabled={activeCount === 0}
              onSelect={() => {
                close();
                setUi({ csvModalOpen: true });
              }}
            />
            <MenuSection title={strings.optionsList} />
            <MenuItem
              label={strings.clearAllManualEdits(overrideCount)}
              disabled={overrideCount === 0}
              onSelect={() => {
                close();
                void commands.clearAllOverrides();
              }}
            />
            <MenuItem
              label={strings.removeSkippedFromList}
              disabled={activeCount === 0}
              onSelect={() => {
                close();
                removeByInclusion(false);
              }}
            />
            <MenuItem
              label={strings.removeIncludedFromList}
              disabled={activeCount === 0}
              onSelect={() => {
                close();
                removeByInclusion(true);
              }}
            />
            <MenuSeparator />
            <MenuItem
              label={strings.clearFileList}
              destructive
              disabled={(snapshot?.files.length ?? 0) === 0}
              onSelect={() => {
                close();
                showConfirm({
                  title: strings.clearListTitle,
                  message: strings.clearListMessage,
                  confirmLabel: strings.clearListButton,
                  destructive: true,
                  onConfirm: () => {
                    void commands.clearAll();
                  },
                });
              }}
            />
          </>
        )}
      </Menu>

      <button
        className={toolButton}
        style={{ borderColor: 'var(--separator)' }}
        onClick={() => {
          setUi({ inspectorOpen: !useUiStore.getState().inspectorOpen });
        }}
        title="File Info (⌘I)"
      >
        <Info size={15} aria-hidden />
        File Info
      </button>

      <div className="flex-1" />

      <label
        className="flex h-7 w-56 items-center gap-1.5 rounded-md border px-2"
        style={{ borderColor: 'var(--separator)', background: 'var(--pane-bg)' }}
      >
        <Search size={13} aria-hidden style={{ color: 'var(--text-secondary)' }} />
        <input
          type="search"
          value={filterText}
          data-filter-field
          onChange={(event) => {
            setUi({ filterText: event.target.value, checkboxAnchor: null });
          }}
          placeholder="Filter"
          aria-label={strings.filterFilesLabel}
          className="w-full bg-transparent text-[13px] outline-none"
          style={{ color: 'var(--text-primary)' }}
        />
      </label>
    </header>
  );
}

function removeByInclusion(included: boolean) {
  const app = useAppStore.getState();
  const snapshot = app.snapshot;
  if (!snapshot) return;
  const folders = snapshot.listMode === 'Folders';
  const ids = snapshot.files
    .filter((f) => f.isDirectory === folders && f.isSelected === included)
    .map((f) => f.id);
  if (ids.length === 0) return;
  const affectedEdits = snapshot.files.filter(
    (f) => ids.includes(f.id) && f.overrideName != null,
  ).length;
  // Destructive policy (§14.0): bulk (> 10 or any manual edit) confirms.
  if (ids.length > 10 || affectedEdits > 0) {
    useUiStore.getState().showConfirm({
      title: strings.bulkRemoveTitle(ids.length, folders),
      message: strings.bulkRemoveMessage(affectedEdits),
      confirmLabel: strings.bulkRemoveButton,
      destructive: true,
      onConfirm: () => {
        void commands.removeItems(ids);
      },
    });
  } else {
    void commands.removeItems(ids);
  }
}
