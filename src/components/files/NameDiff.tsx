// Rendered diff spans (§14.6): red strikethrough removals, green additions
// with underline only on non-whitespace runs. Memoized per (old, new).

import { memo, useMemo } from 'react';
import { computeNameDiff, whitespaceRuns } from '../../lib/nameDiff';

export const OldName = memo(function OldName({ oldName, newName }: { oldName: string; newName: string }) {
  const diff = useMemo(() => computeNameDiff(oldName, newName), [oldName, newName]);
  if (!diff.changed) {
    return <span>{oldName}</span>;
  }
  return (
    <span>
      {diff.old.prefix}
      {diff.old.middle.length > 0 && (
        <span
          style={{
            background: 'var(--diff-removed-bg)',
            textDecoration: 'line-through',
            color: 'var(--text-secondary)',
            borderRadius: 2,
          }}
        >
          {diff.old.middle}
        </span>
      )}
      {diff.old.suffix}
    </span>
  );
});

export const NewName = memo(function NewName({ oldName, newName }: { oldName: string; newName: string }) {
  const diff = useMemo(() => computeNameDiff(oldName, newName), [oldName, newName]);
  if (!diff.changed) {
    return <span>{newName}</span>;
  }
  return (
    <span>
      {diff.new.prefix}
      {diff.new.middle.length > 0 && (
        <span
          style={{
            background: diff.middleAllWhitespace
              ? 'var(--diff-added-bg-whitespace)'
              : 'var(--diff-added-bg)',
            borderRadius: 2,
          }}
        >
          {/* Underline only non-whitespace runs — an underlined space reads
              as an underscore (§14.6). */}
          {whitespaceRuns(diff.new.middle).map((run, index) => (
            <span
              key={index}
              style={run.isWhitespace ? undefined : { textDecoration: 'underline' }}
            >
              {run.text}
            </span>
          ))}
        </span>
      )}
      {diff.new.suffix}
    </span>
  );
});
