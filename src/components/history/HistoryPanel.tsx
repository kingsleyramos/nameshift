// The history panel (§14.5): relative dates, summaries, click to select a
// version (revert preview), footer Clear History (confirmed).

import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { absoluteDate, relativeDate } from '../../lib/format';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function HistoryPanel() {
  const history = useAppStore((s) => s.history);
  const selectedId = useAppStore((s) => s.snapshot?.selectedSnapshotId ?? null);
  const showConfirm = useUiStore((s) => s.showConfirm);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 overflow-auto px-2 pb-2">
        {history.length === 0 && (
          <p className="px-2 py-6 text-center text-[13px]" style={{ color: 'var(--text-secondary)' }}>
            Renames you apply appear here as versions you can revert.
          </p>
        )}
        {history.map((snapshot) => {
          const selected = selectedId === snapshot.id;
          return (
            <button
              key={snapshot.id}
              aria-pressed={selected}
              onClick={() => {
                // Click selects (revert preview); click again deselects.
                void commands.selectSnapshot(selected ? null : snapshot.id);
              }}
              className="mb-1.5 block w-full rounded-[10px] border px-3 py-2 text-left"
              style={{
                borderColor: selected ? 'var(--accent)' : 'var(--separator)',
                background: selected ? 'var(--wash-selected)' : 'var(--pane-bg)',
              }}
            >
              <div className="flex items-baseline justify-between gap-2">
                <span
                  className="text-[12px]"
                  style={{ color: 'var(--text-secondary)' }}
                  title={absoluteDate(snapshot.date)}
                >
                  {relativeDate(snapshot.date)}
                </span>
                <span className="text-[11.5px]" style={{ color: 'var(--text-secondary)' }}>
                  {strings.renamesCount(snapshot.entryCount)}
                </span>
              </div>
              <p className="mt-0.5 truncate text-[13px]" style={{ color: 'var(--text-primary)' }}>
                {snapshot.summary}
              </p>
            </button>
          );
        })}
      </div>
      <footer className="px-2 py-2" style={{ borderTop: '1px solid var(--separator)' }}>
        <button
          disabled={history.length === 0}
          onClick={() => {
            showConfirm({
              title: strings.clearHistoryTitle,
              message: strings.clearHistoryMessage,
              confirmLabel: strings.clearHistoryButton,
              destructive: true,
              onConfirm: () => {
                void commands.clearHistory();
              },
            });
          }}
          className="h-6 rounded-md border px-2 text-[12px] hover:enabled:bg-[var(--wash-hover)] disabled:opacity-40"
          style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
        >
          {strings.clearHistory}
        </button>
      </footer>
    </div>
  );
}
