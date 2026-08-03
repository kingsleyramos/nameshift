// A lightweight dropdown menu primitive: trigger + positioned panel,
// closed by outside click or Esc. Menu items are real buttons (§14.0 —
// no bare-text buttons; every interactive element keyboard-reachable).

import { ReactNode, useEffect, useRef, useState } from 'react';

interface MenuProps {
  trigger: (open: boolean, toggle: () => void) => ReactNode;
  children: ReactNode | ((close: () => void) => ReactNode);
  align?: 'left' | 'right';
  ariaLabel?: string;
}

export default function Menu({ trigger, children, align = 'left', ariaLabel }: MenuProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (root.current && !root.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpen(false);
      }
    };
    window.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [open]);

  const close = () => {
    setOpen(false);
  };

  return (
    <div ref={root} className="relative inline-block">
      {trigger(open, () => {
        setOpen((value) => !value);
      })}
      {open && (
        <div
          role="menu"
          aria-label={ariaLabel}
          className="absolute z-50 mt-1 min-w-44 overflow-hidden py-1 shadow-lg"
          style={{
            [align === 'right' ? 'right' : 'left']: 0,
            background: 'var(--pane-bg)',
            border: '1px solid var(--separator)',
            borderRadius: 'var(--radius-overlay)',
          }}
        >
          {typeof children === 'function' ? children(close) : children}
        </div>
      )}
    </div>
  );
}

export function MenuItem({
  label,
  onSelect,
  disabled,
  checked,
  destructive,
}: {
  label: string;
  onSelect: () => void;
  disabled?: boolean;
  checked?: boolean;
  destructive?: boolean;
}) {
  return (
    <button
      role="menuitem"
      disabled={disabled}
      onClick={onSelect}
      className="block w-full px-3 py-1.5 text-left text-[13px] hover:enabled:bg-[var(--wash-hover)] disabled:opacity-40"
      style={{ color: destructive ? 'var(--warning-text)' : 'var(--text-primary)' }}
    >
      <span className="inline-block w-4">{checked ? '✓' : ''}</span>
      {label}
    </button>
  );
}

export function MenuSection({ title }: { title: string }) {
  return (
    <div
      className="px-3 pt-2 pb-0.5 text-[11px] font-semibold uppercase tracking-wide"
      style={{ color: 'var(--text-secondary)' }}
    >
      {title}
    </div>
  );
}

export function MenuSeparator() {
  return <div className="my-1 h-px" style={{ background: 'var(--separator)' }} />;
}
