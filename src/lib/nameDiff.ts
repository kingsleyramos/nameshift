// The before → after diff highlighting (§14.6): longest common prefix and
// suffix in Unicode grapheme clusters, suffix limited to len − prefix.

export interface DiffSpans {
  prefix: string;
  middle: string;
  suffix: string;
}

export interface NameDiff {
  old: DiffSpans;
  new: DiffSpans;
  /** The new-name middle is entirely whitespace → stronger tint (§14.6). */
  middleAllWhitespace: boolean;
  changed: boolean;
}

const segmenter = new Intl.Segmenter('en', { granularity: 'grapheme' });

function graphemes(value: string): string[] {
  return Array.from(segmenter.segment(value), (s) => s.segment);
}

export function computeNameDiff(oldName: string, newName: string): NameDiff {
  if (oldName === newName) {
    return {
      old: { prefix: oldName, middle: '', suffix: '' },
      new: { prefix: newName, middle: '', suffix: '' },
      middleAllWhitespace: false,
      changed: false,
    };
  }
  const before = graphemes(oldName);
  const after = graphemes(newName);
  let prefix = 0;
  while (prefix < before.length && prefix < after.length && before[prefix] === after[prefix]) {
    prefix += 1;
  }
  // The suffix may not overlap the prefix.
  let suffix = 0;
  while (
    suffix < before.length - prefix &&
    suffix < after.length - prefix &&
    before[before.length - 1 - suffix] === after[after.length - 1 - suffix]
  ) {
    suffix += 1;
  }
  const spans = (parts: string[]): DiffSpans => ({
    prefix: parts.slice(0, prefix).join(''),
    middle: parts.slice(prefix, parts.length - suffix).join(''),
    suffix: parts.slice(parts.length - suffix).join(''),
  });
  const next = spans(after);
  return {
    old: spans(before),
    new: next,
    middleAllWhitespace: next.middle.length > 0 && next.middle.trim().length === 0,
    changed: true,
  };
}

/** Split a string into alternating whitespace / non-whitespace runs, so the
 * added-span underline can skip spaces (an underlined space reads as an
 * underscore — §14.6). */
export function whitespaceRuns(value: string): { text: string; isWhitespace: boolean }[] {
  const runs: { text: string; isWhitespace: boolean }[] = [];
  for (const char of value) {
    const isWhitespace = char.trim().length === 0;
    const last = runs[runs.length - 1];
    if (last?.isWhitespace === isWhitespace) {
      last.text += char;
    } else {
      runs.push({ text: char, isWhitespace });
    }
  }
  return runs;
}
