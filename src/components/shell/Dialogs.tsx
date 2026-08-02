// Confirmation and alert dialogs (§14.9): title + message + buttons, real
// counts already baked into the §A copy by the caller.

import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';

export default function Dialogs() {
  const confirm = useUiStore((s) => s.confirm);
  const alert = useUiStore((s) => s.alert);
  const setUi = useUiStore((s) => s.set);

  if (!confirm && !alert) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: 'rgb(0 0 0 / 0.25)' }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          setUi({ confirm: null, alert: null });
        }
      }}
    >
      <div
        className="w-96 p-5"
        style={{ background: 'var(--pane-bg)', borderRadius: 'var(--radius-overlay)' }}
      >
        <h2 className="text-[14px] font-semibold" style={{ color: 'var(--text-primary)' }}>
          {confirm?.title ?? alert?.title}
        </h2>
        <p className="mt-2 whitespace-pre-wrap text-[13px]" style={{ color: 'var(--text-secondary)' }}>
          {confirm?.message ?? alert?.message}
        </p>
        <div className="mt-4 flex justify-end gap-2">
          {confirm ? (
            <>
              <button
                autoFocus
                onClick={() => {
                  setUi({ confirm: null });
                }}
                className="h-7 rounded-md border px-3 text-[13px] hover:bg-[var(--wash-hover)]"
                style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
              >
                {strings.cancel}
              </button>
              <button
                onClick={() => {
                  const action = confirm.onConfirm;
                  setUi({ confirm: null });
                  action();
                }}
                className="h-7 rounded-md px-3 text-[13px] font-semibold text-white"
                style={{
                  background: confirm.destructive ? 'var(--revert-action)' : 'var(--accent)',
                }}
              >
                {confirm.confirmLabel}
              </button>
            </>
          ) : (
            <button
              autoFocus
              onClick={() => {
                setUi({ alert: null });
              }}
              className="h-7 rounded-md px-3 text-[13px] font-semibold text-white"
              style={{ background: 'var(--accent)' }}
            >
              OK
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
