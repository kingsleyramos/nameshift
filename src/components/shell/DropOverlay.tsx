// The whole-window drop indicator (§14.1): an accent inset ring.

import { useUiStore } from '../../state/uiState';

export default function DropOverlay() {
  const dragOver = useUiStore((s) => s.dragOver);
  if (!dragOver) return null;
  return (
    <div
      aria-hidden
      className="pointer-events-none fixed inset-2 z-40 rounded-xl"
      style={{ border: '3px solid var(--accent)', background: 'var(--wash-selected)' }}
    />
  );
}
