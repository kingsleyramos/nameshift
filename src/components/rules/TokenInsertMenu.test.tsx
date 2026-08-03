import { describe, expect, it } from 'vitest';
import { insertToken } from './TokenInsertMenu';

// The token menu inserts at the caret, not append (audit NS-04) — vectors
// ported from the reference suite.
describe('insertToken', () => {
  it('appends at a caret at the end', () => {
    expect(insertToken('{n}', 'photo-', 6, 6)).toEqual({ text: 'photo-{n}', caret: 9 });
  });

  it('prepends at a caret at the start', () => {
    expect(insertToken('{n}', 'photo-', 0, 0).text).toBe('{n}photo-');
  });

  it('inserts mid-string', () => {
    expect(insertToken('{created}', 'IMG b', 4, 4).text).toBe('IMG {created}b');
  });

  it('replaces a selection', () => {
    expect(insertToken('{created}', 'abcd', 0, 2).text).toBe('{created}cd');
  });

  it('clamps a stale out-of-range selection instead of trapping', () => {
    expect(insertToken('{n}', 'ab', 99, 104)).toEqual({ text: 'ab{n}', caret: 5 });
  });
});
