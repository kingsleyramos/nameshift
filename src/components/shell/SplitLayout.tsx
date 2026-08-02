// Side panel | detail split (§14.1: side 320–460 px, draggable).

import { ReactNode, useCallback, useRef, useState } from 'react';

export default function SplitLayout({ side, detail }: { side: ReactNode; detail: ReactNode }) {
  const [width, setWidth] = useState(360);
  const dragging = useRef(false);

  const onPointerDown = useCallback((event: React.PointerEvent) => {
    dragging.current = true;
    (event.target as HTMLElement).setPointerCapture(event.pointerId);
  }, []);
  const onPointerMove = useCallback((event: React.PointerEvent) => {
    if (!dragging.current) return;
    setWidth(Math.min(460, Math.max(320, event.clientX)));
  }, []);
  const onPointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  return (
    <div className="flex min-h-0 flex-1">
      <aside
        className="flex h-full flex-col overflow-hidden"
        style={{ width, background: 'var(--window-bg)' }}
      >
        {side}
      </aside>
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize side panel"
        className="w-1 shrink-0 cursor-col-resize"
        style={{ background: 'var(--separator)' }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
      />
      <section className="min-w-0 flex-1" style={{ background: 'var(--pane-bg)' }}>
        {detail}
      </section>
    </div>
  );
}
