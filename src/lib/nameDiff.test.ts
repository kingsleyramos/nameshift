import { describe, expect, it } from 'vitest';
import { computeNameDiff, whitespaceRuns } from './nameDiff';

describe('computeNameDiff (§14.6)', () => {
  it('reports unchanged names plainly', () => {
    const diff = computeNameDiff('a.txt', 'a.txt');
    expect(diff.changed).toBe(false);
    expect(diff.old.prefix).toBe('a.txt');
    expect(diff.old.middle).toBe('');
  });

  it('finds the longest common prefix and suffix', () => {
    const diff = computeNameDiff('IMG_0421.jpg', 'IMG_beach.jpg');
    expect(diff.old.prefix).toBe('IMG_');
    expect(diff.old.suffix).toBe('.jpg');
    expect(diff.old.middle).toBe('0421');
    expect(diff.new.middle).toBe('beach');
  });

  it('limits the suffix to len − prefix (no overlap)', () => {
    const diff = computeNameDiff('aaa', 'aa');
    expect(diff.old.prefix.length + diff.old.middle.length + diff.old.suffix.length).toBe(3);
    expect(diff.new.prefix.length + diff.new.middle.length + diff.new.suffix.length).toBe(2);
  });

  it('works in grapheme clusters, not code units', () => {
    const diff = computeNameDiff('👩‍👩‍👧 a.txt', '👩‍👩‍👧 b.txt');
    expect(diff.old.prefix).toBe('👩‍👩‍👧 ');
    expect(diff.old.middle).toBe('a');
    expect(diff.new.middle).toBe('b');
  });

  it('flags an all-whitespace added middle for the stronger tint', () => {
    const diff = computeNameDiff('chapter2.md', 'chapter 2.md');
    expect(diff.new.middle).toBe(' ');
    expect(diff.middleAllWhitespace).toBe(true);
  });

  it('does not flag mixed middles as whitespace-only', () => {
    const diff = computeNameDiff('a.md', 'a b.md');
    expect(diff.middleAllWhitespace).toBe(false);
  });
});

describe('whitespaceRuns (underline skips spaces)', () => {
  it('splits alternating runs', () => {
    expect(whitespaceRuns('ab cd')).toEqual([
      { text: 'ab', isWhitespace: false },
      { text: ' ', isWhitespace: true },
      { text: 'cd', isWhitespace: false },
    ]);
  });

  it('handles leading whitespace and empty strings', () => {
    expect(whitespaceRuns(' x')).toEqual([
      { text: ' ', isWhitespace: true },
      { text: 'x', isWhitespace: false },
    ]);
    expect(whitespaceRuns('')).toEqual([]);
  });
});
