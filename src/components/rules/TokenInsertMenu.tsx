// The { } insert-at-cursor menu (§14.3, audit CON-08): every token-capable
// field gets one; insertion lands at the caret, never appends blindly.

import { Braces } from 'lucide-react';
import Menu from '../shell/Menu';
import { TOKENS } from './ruleMeta';

/** Insert `token` into `text` at the selection, returning the new text and
 * caret position — clamped for stale selections (never traps). */
export function insertToken(
  token: string,
  text: string,
  selectionStart: number,
  selectionEnd: number,
): { text: string; caret: number } {
  const start = Math.min(Math.max(0, selectionStart), text.length);
  const end = Math.min(Math.max(start, selectionEnd), text.length);
  const next = text.slice(0, start) + token + text.slice(end);
  return { text: next, caret: start + token.length };
}

export default function TokenInsertMenu({
  inputRef,
  value,
  onInsert,
}: {
  inputRef: React.RefObject<HTMLInputElement | null>;
  value: string;
  onInsert: (nextValue: string, caret: number) => void;
}) {
  return (
    <Menu
      align="right"
      ariaLabel="Insert token"
      trigger={(_, toggle) => (
        <button
          onClick={toggle}
          title="Insert a token at the cursor"
          aria-label="Insert token"
          className="flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded hover:bg-[var(--wash-hover)]"
          style={{ color: 'var(--text-secondary)' }}
        >
          <Braces size={13} aria-hidden />
        </button>
      )}
    >
      {(close) => (
        <div className="max-h-72 overflow-auto">
          {TOKENS.map((token) => (
            <button
              key={token.insert}
              role="menuitem"
              className="block w-full px-3 py-1 text-left hover:bg-[var(--wash-hover)]"
              onClick={() => {
                close();
                const input = inputRef.current;
                const start = input?.selectionStart ?? value.length;
                const end = input?.selectionEnd ?? value.length;
                const result = insertToken(token.insert, value, start, end);
                onInsert(result.text, result.caret);
                requestAnimationFrame(() => {
                  input?.focus();
                  input?.setSelectionRange(result.caret, result.caret);
                });
              }}
            >
              <span className="font-mono text-[12px]" style={{ color: 'var(--text-primary)' }}>
                {token.insert}
              </span>
              <span className="ml-2 text-[11.5px]" style={{ color: 'var(--text-secondary)' }}>
                {token.description}
              </span>
            </button>
          ))}
        </div>
      )}
    </Menu>
  );
}
