// Rename by CSV (§14.7): ONE modal, one state — download stays available,
// the drop zone is live from open, results read as a sentence, and the
// footer never changes composition. Nothing mutates until Apply.

import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import type { CsvMatchReport } from '../../ipc/gen/CsvMatchReport';
import { useAppStore } from '../../state/appState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

interface ReadResult {
  filename: string;
  report: CsvMatchReport;
}

export default function CsvModal({ onClose }: { onClose: () => void }) {
  const snapshot = useAppStore((s) => s.snapshot);
  const total = useAppStore(
    (s) => s.preview?.entries.length ?? 0,
  );
  const [result, setResult] = useState<ReadResult | null>(null);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [missesOpen, setMissesOpen] = useState(false);
  const [shake, setShake] = useState(false);
  const zoneRef = useRef<HTMLButtonElement | null>(null);
  const isFolders = snapshot?.listMode === 'Folders';

  const readFile = useCallback(async (path: string) => {
    try {
      const report = await commands.csvDryRun(path);
      setResult({ filename: path.split('/').pop() ?? path, report });
      setInlineError(null);
      setMissesOpen(false);
    } catch (error) {
      // Inline, never a modal (§14.7); a later good read clears it.
      const raw = (error as { message?: unknown } | null)?.message;
      setInlineError(typeof raw === 'string' && raw.length > 0 ? raw : strings.csvReadError);
    }
  }, []);

  // The whole modal stays a drop target; re-dropping replaces the read.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== 'drop') return;
        const paths = event.payload.paths;
        // CSV only, everywhere: any other drop (or > 1 file) shakes the
        // zone — no error text (§14.7).
        const first = paths[0];
        if (paths.length !== 1 || !first?.toLowerCase().endsWith('.csv')) {
          setShake(true);
          setTimeout(() => {
            setShake(false);
          }, 400);
          return;
        }
        void readFile(first);
      })
      .then((d) => {
        dispose = d;
      });
    return () => {
      dispose?.();
    };
  }, [readFile]);

  const choose = async () => {
    const picked = await openFileDialog({
      multiple: false,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
      title: 'Choose a CSV with two columns: current name, new name',
    });
    if (typeof picked === 'string') {
      void readFile(picked);
    }
  };

  const matches = result?.report.matches.filter((m) => m.changes) ?? [];
  const missCount = result?.report.misses.length ?? 0;
  const applyCount = result ? result.report.willChange : null;

  const missGroups = (['notInList', 'duplicateRow', 'couldntRead'] as const)
    .map((reason) => ({
      reason,
      label:
        reason === 'notInList'
          ? strings.csvMissNotInList
          : reason === 'duplicateRow'
            ? strings.csvMissDuplicate
            : strings.csvMissUnreadable,
      rows: result?.report.misses.filter((m) => m.reason === reason) ?? [],
    }))
    .filter((group) => group.rows.length > 0);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={strings.csvTitle}
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'rgb(0 0 0 / 0.25)' }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') onClose();
      }}
    >
      <div
        className="flex max-h-[80vh] w-[480px] flex-col gap-3 overflow-auto p-5"
        style={{ background: 'var(--pane-bg)', borderRadius: 'var(--radius-overlay)' }}
      >
        <div>
          <h2 className="text-[15px] font-semibold" style={{ color: 'var(--text-primary)' }}>
            {strings.csvTitle}
          </h2>
          <p className="mt-0.5 text-[12.5px]" style={{ color: 'var(--text-secondary)' }}>
            {strings.csvSubtitle}
          </p>
        </div>

        <button
          onClick={() => {
            // The modal stays open after export (§14.7).
            void commands.exportCsvTemplate();
          }}
          disabled={total === 0}
          className="h-8 rounded-md border px-3 text-left text-[13px] hover:enabled:bg-[var(--wash-hover)] disabled:opacity-40"
          style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
        >
          {strings.csvDownload(total, isFolders)}
        </button>

        <div className="flex items-center gap-2">
          {result == null ? (
            <button
              ref={zoneRef}
              onClick={() => {
                void choose();
              }}
              onKeyDown={(event) => {
                if (event.key === ' ') {
                  event.preventDefault();
                  void choose();
                }
              }}
              data-testid="csv-drop-zone"
              className="flex-1 rounded-[10px] border border-dashed px-3 py-6 text-center text-[13px]"
              style={{
                borderColor: 'var(--separator)',
                color: 'var(--text-secondary)',
                animation: shake ? 'ns-shake 0.35s' : undefined,
              }}
            >
              {strings.csvDropHere}
            </button>
          ) : (
            <span
              className="flex-1 truncate rounded-md border px-2 py-1 text-[13px]"
              style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
              title={result.filename}
            >
              {result.filename}
            </span>
          )}
          <button
            onClick={() => {
              void choose();
            }}
            className="h-7 shrink-0 rounded-md border px-2.5 text-[13px] hover:bg-[var(--wash-hover)]"
            style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
          >
            {strings.csvChoose}
          </button>
        </div>

        {inlineError != null && (
          <p className="text-[12.5px]" style={{ color: 'var(--warning-text)' }} role="alert">
            {inlineError}
          </p>
        )}

        {result != null && (
          <div aria-live="polite">
            <p className="text-[13px] font-semibold" style={{ color: 'var(--text-primary)' }}>
              {strings.csvWillChange(result.report.willChange, total)}
            </p>
            {missCount > 0 && (
              <div className="mt-1">
                <button
                  aria-expanded={missesOpen}
                  onClick={() => {
                    setMissesOpen((open) => !open);
                  }}
                  onKeyDown={(event) => {
                    if (event.key === ' ' || event.key === 'Enter') {
                      event.preventDefault();
                      setMissesOpen((open) => !open);
                    }
                  }}
                  className="text-[12.5px]"
                  style={{ color: 'var(--warning-text)' }}
                >
                  {missesOpen ? '▾' : '▸'} {strings.csvRowsDidntMatch(missCount)}
                </button>
                {missesOpen && (
                  <div className="mt-1 max-h-48 overflow-auto">
                    {missGroups.map((group) => (
                      <section key={group.reason}>
                        <h4
                          className="py-0.5 text-[11px] font-semibold uppercase tracking-wide"
                          style={{ color: 'var(--text-secondary)' }}
                        >
                          {group.label}
                        </h4>
                        {group.rows.map((miss, index) => (
                          <p
                            key={`${miss.name}-${String(index)}`}
                            className="truncate py-0.5 text-[12.5px]"
                            style={{ color: 'var(--text-primary)' }}
                            title={miss.name}
                          >
                            {miss.name}
                          </p>
                        ))}
                      </section>
                    ))}
                  </div>
                )}
              </div>
            )}
            {result.report.willChange === 0 && (
              <p className="mt-1 text-[12.5px]" style={{ color: 'var(--text-secondary)' }}>
                {strings.csvZeroMatch}
              </p>
            )}
          </div>
        )}

        {/* Footer: fixed composition in every state (§14.7). */}
        <div className="mt-1 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="h-7 rounded-md border px-3 text-[13px] hover:bg-[var(--wash-hover)]"
            style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
          >
            {strings.cancel}
          </button>
          <button
            disabled={applyCount == null || applyCount === 0}
            onClick={() => {
              if (!result) return;
              // Commit sets overrides + selects — one ⌘Z removes the whole
              // batch (§14.7).
              void commands.csvApply(matches);
              onClose();
            }}
            className="h-7 rounded-md px-3 text-[13px] font-semibold text-white disabled:opacity-40"
            style={{ background: 'var(--accent)' }}
          >
            {strings.csvApplyChanges(applyCount)}
          </button>
        </div>
      </div>
    </div>
  );
}
