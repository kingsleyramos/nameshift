// The revert preview (§14.5): orange summary banner, directory-grouped
// current → restored rows with §8.8 statuses. The Revert button lives in
// the action bar and never wears Rename's accent.

import { useMemo } from 'react';
import { TriangleAlert } from 'lucide-react';
import type { RevertEntry } from '../../ipc/gen/RevertEntry';
import { useAppStore } from '../../state/appState';
import { strings } from '../../lib/strings';

export default function RevertPreviewPane() {
  const revertPreview = useAppStore((s) => s.revertPreview);
  const entries = useMemo(() => revertPreview?.entries ?? [], [revertPreview]);

  const groups = useMemo(() => {
    const byDirectory = new Map<string, RevertEntry[]>();
    for (const entry of entries) {
      const list = byDirectory.get(entry.directoryPath) ?? [];
      list.push(entry);
      byDirectory.set(entry.directoryPath, list);
    }
    return [...byDirectory.entries()];
  }, [entries]);

  if (!revertPreview) return null;
  const skipped = entries.filter((e) => e.status === 'missing').length;
  const taken = revertPreview.nameTakenCount;

  return (
    <div className="flex h-full flex-col" data-testid="revert-preview">
      <div
        className="px-3 py-2 text-[13px]"
        style={{ background: 'var(--wash-conflict)', color: 'var(--warning-text)' }}
        role="status"
      >
        {strings.revertBannerSummary(
          revertPreview.restorableFileCount,
          revertPreview.restorableRenameCount,
          revertPreview.newerSnapshotCount + 1,
        )}
        {skipped > 0 && ` · ${String(skipped)} skipped`}
        {taken > 0 && ` · ${String(taken)} name${taken === 1 ? '' : 's'} taken`}
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {groups.map(([directory, rows]) => (
          <section key={directory}>
            <h3
              className="sticky top-0 truncate px-3 py-1 text-[11.5px] font-semibold"
              style={{ background: 'var(--window-bg)', color: 'var(--text-secondary)' }}
              title={directory}
            >
              {directory}
            </h3>
            {rows.map((entry) => (
              <div
                key={entry.id}
                className="flex items-center gap-2 px-3 py-1 text-[13px]"
                style={{ opacity: entry.status === 'ok' ? 1 : 0.75 }}
              >
                <span className="min-w-0 flex-1 truncate" style={{ color: 'var(--text-primary)' }}>
                  {entry.currentName}
                </span>
                <span aria-hidden style={{ color: 'var(--text-secondary)' }}>
                  →
                </span>
                <span className="min-w-0 flex-1 truncate" style={{ color: 'var(--text-primary)' }}>
                  {entry.restoredName}
                </span>
                <StatusLabel status={entry.status} />
              </div>
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}

function StatusLabel({ status }: { status: RevertEntry['status'] }) {
  if (status === 'ok') {
    return (
      <span className="shrink-0 text-[11.5px]" style={{ color: 'var(--success-text)' }}>
        {strings.revertStatusOk}
      </span>
    );
  }
  if (status === 'missing') {
    return (
      <span
        className="flex shrink-0 items-center gap-1 text-[11.5px]"
        style={{ color: 'var(--text-secondary)' }}
        title={strings.revertStatusMissingTooltip}
      >
        <TriangleAlert size={11} aria-hidden />
        {strings.revertStatusMissing}
      </span>
    );
  }
  return (
    <span
      className="flex shrink-0 items-center gap-1 text-[11.5px]"
      style={{ color: 'var(--warning-text)' }}
    >
      <TriangleAlert size={11} aria-hidden />
      {strings.revertStatusNameTaken}
    </span>
  );
}
