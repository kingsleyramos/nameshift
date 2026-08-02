// The determinate processing overlay (§14.1): appears only after 350 ms;
// Cancel is bound to Esc.

import { useEffect, useState } from 'react';
import type { Processing } from '../../ipc/gen/Processing';
import * as commands from '../../ipc/commands';

export default function ProcessingOverlay({ processing }: { processing: Processing }) {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => {
      setVisible(true);
    }, 350);
    return () => {
      clearTimeout(timer);
    };
  }, []);
  if (!visible) return null;
  const fraction = processing.total > 0 ? processing.completed / processing.total : 0;
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={processing.title}
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'var(--scrim)' }}
    >
      <div
        className="flex w-80 flex-col gap-3 p-5"
        style={{ background: 'var(--pane-bg)', borderRadius: 'var(--radius-overlay)' }}
      >
        <p className="text-[13px] font-medium" style={{ color: 'var(--text-primary)' }}>
          {processing.title}
        </p>
        <div
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={processing.total}
          aria-valuenow={processing.completed}
          className="h-1.5 overflow-hidden rounded-full"
          style={{ background: 'var(--wash-hover)' }}
        >
          <div
            className="h-full rounded-full transition-[width]"
            style={{ width: `${String(fraction * 100)}%`, background: 'var(--accent)' }}
          />
        </div>
        <div className="flex items-center justify-between">
          <span className="text-[12px]" style={{ color: 'var(--text-secondary)' }}>
            {processing.completed} of {processing.total}
          </span>
          <button
            onClick={() => {
              void commands.cancelProcessing();
            }}
            className="h-6 rounded-md border px-2.5 text-[12px] hover:bg-[var(--wash-hover)]"
            style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
