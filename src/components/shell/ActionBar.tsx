// The bottom action bar (§14.1): decision info and the primary action in
// one left-to-right scan. Counts are click-toggle filters; Revert never
// wears Rename's accent (§14.0).

import { Pencil, TriangleAlert, Undo2 } from 'lucide-react';
import { useAppStore } from '../../state/appState';
import { useUiStore, type ListFilter } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function ActionBar() {
  const snapshot = useAppStore((s) => s.snapshot);
  const preview = useAppStore((s) => s.preview);
  const revertPreview = useAppStore((s) => s.revertPreview);
  const processing = useAppStore((s) => s.processing);
  const filterMode = useUiStore((s) => s.filterMode);
  const toggleFilter = useUiStore((s) => s.toggleFilter);
  const showConfirm = useUiStore((s) => s.showConfirm);

  const revertMode = snapshot?.selectedSnapshotId != null;
  const isFolders = snapshot?.listMode === 'Folders';
  const counts = preview?.counts;
  const total = preview?.entries.length ?? 0;
  const overrideCount = snapshot?.overrideCount ?? 0;

  const chip = (
    label: string,
    mode: ListFilter,
    options: { warning?: boolean; icon?: React.ReactNode } = {},
  ) => {
    const active = filterMode === mode;
    return (
      <button
        onClick={() => {
          toggleFilter(mode);
        }}
        aria-pressed={active}
        className="flex h-6 items-center gap-1 rounded-full px-2.5 text-[12px]"
        style={
          active
            ? { background: 'var(--accent)', color: 'var(--on-accent)' }
            : {
                color: options.warning ? 'var(--warning-text)' : 'var(--link-text)',
              }
        }
      >
        {options.icon}
        {label}
        {active && <span aria-hidden>✕</span>}
      </button>
    );
  };

  if (revertMode) {
    const restorable = revertPreview?.restorableRenameCount ?? 0;
    const files = revertPreview?.restorableFileCount ?? 0;
    const versions = (revertPreview?.newerSnapshotCount ?? 0) + 1;
    return (
      <footer
        className="flex items-center gap-3 px-3 py-2"
        style={{ background: 'var(--window-bg)', borderTop: '1px solid var(--separator)' }}
      >
        <span className="text-[12px]" style={{ color: 'var(--warning-text)' }}>
          {strings.revertBannerSummary(files, restorable, versions)}
        </span>
        <div className="flex-1" />
        <button
          onClick={() => {
            showConfirm({
              title: strings.revertConfirmTitle,
              message: strings.revertConfirmMessage(
                files,
                restorable,
                versions,
                revertPreview?.newerSnapshotCount ?? 0,
              ),
              confirmLabel: strings.revertConfirmButton,
              destructive: true,
              onConfirm: () => {
                void commands.revertSelected();
              },
            });
          }}
          disabled={restorable === 0 || processing != null}
          className="flex h-7 items-center gap-1.5 rounded-full px-4 text-[13px] font-semibold disabled:opacity-40"
          style={{ background: 'var(--revert-action)', color: 'var(--revert-action-text)' }}
        >
          <Undo2 size={14} aria-hidden />
          {strings.revertButton}
        </button>
      </footer>
    );
  }

  const renameCount = counts?.changeCount ?? 0;
  const canApply = (counts?.canApply ?? false) && processing == null;

  const renameTooltip = canApply
    ? strings.renameTooltipEnabled(renameCount, isFolders)
    : (counts?.conflictCount ?? 0) > 0
      ? strings.renameTooltipConflicts(counts?.conflictCount ?? 0)
      : strings.renameTooltipEmpty(isFolders);

  return (
    <footer
      className="flex items-center gap-2 px-3 py-2"
      style={{ background: 'var(--window-bg)', borderTop: '1px solid var(--separator)' }}
    >
      {total > 0 && counts && (
        <>
          <span className="text-[12px]" style={{ color: 'var(--text-secondary)' }}>
            {strings.includedCount(counts.selectedCount, total)}
          </span>
          <span aria-hidden style={{ color: 'var(--text-secondary)' }}>
            ·
          </span>
          {chip(strings.willChangeCount(counts.changeCount), 'willChange')}
          {counts.conflictCount > 0 &&
            chip(strings.conflictCount(counts.conflictCount), 'conflicts', {
              warning: true,
              icon: <TriangleAlert size={12} aria-hidden />,
            })}
          {counts.conflictCount > 0 && (
            <button
              onClick={() => {
                void commands.deselectConflicted();
              }}
              title={strings.skipConflictedTooltip}
              className="h-6 rounded-md border px-2 text-[12px] hover:bg-[var(--wash-hover)]"
              style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
            >
              {strings.skipConflicted}
            </button>
          )}
          <button
            onClick={() => {
              void commands.setAutoResolve(!(snapshot?.autoResolvesConflicts ?? false));
            }}
            aria-pressed={snapshot?.autoResolvesConflicts ?? false}
            className="h-6 rounded-full border px-2.5 text-[12px] hover:bg-[var(--wash-hover)]"
            style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
          >
            {strings.autoResolveChip(snapshot?.autoResolvesConflicts ?? false)}
          </button>
        </>
      )}
      <div className="flex-1" />
      {total > 0 && !canApply && processing == null && preview?.applyDisabledReason && (
        <span className="text-[12px]" style={{ color: 'var(--text-secondary)' }}>
          {preview.applyDisabledReason}
        </span>
      )}
      {overrideCount > 0 &&
        chip(strings.editedCount(overrideCount), 'edited', {
          icon: <Pencil size={12} aria-hidden />,
        })}
      <button
        onClick={() => {
          void commands.apply();
        }}
        disabled={!canApply}
        title={renameTooltip}
        className="h-7 rounded-full px-4 text-[13px] font-semibold disabled:opacity-40"
        style={{ background: 'var(--accent)', color: 'var(--on-accent)' }}
      >
        {strings.renameButton(renameCount, isFolders)}
      </button>
    </footer>
  );
}
