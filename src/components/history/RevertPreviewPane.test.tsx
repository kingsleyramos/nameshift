import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import RevertPreviewPane from './RevertPreviewPane';
import { useAppStore } from '../../state/appState';
import type { RevertEntry } from '../../ipc/gen/RevertEntry';

function entry(partial: Partial<RevertEntry>): RevertEntry {
  return {
    id: 'S-0',
    snapshotId: 'S',
    snapshotDate: '2026-07-27T18:04:11Z',
    directoryPath: '/d',
    currentName: 'x-a.txt',
    restoredName: 'a.txt',
    status: 'ok',
    ...partial,
  };
}

describe('RevertPreviewPane (§14.5, §8.8 statuses)', () => {
  it('groups by directory and shows §A status labels', () => {
    useAppStore.setState({
      revertPreview: {
        version: 1,
        entries: [
          entry({ id: 'S-0' }),
          entry({ id: 'S-1', currentName: 'x-b.txt', restoredName: 'b.txt', status: 'missing' }),
          entry({
            id: 'S-2',
            directoryPath: '/other',
            currentName: 'x-c.txt',
            restoredName: 'c.txt',
            status: 'nameTaken',
          }),
        ],
        restorableRenameCount: 1,
        restorableFileCount: 1,
        nameTakenCount: 1,
        newerSnapshotCount: 0,
      },
    });
    render(<RevertPreviewPane />);
    expect(screen.getByText('/d')).toBeInTheDocument();
    expect(screen.getByText('/other')).toBeInTheDocument();
    expect(screen.getByText('Will be restored')).toBeInTheDocument();
    expect(screen.getByText('Not found (nothing to restore)')).toBeInTheDocument();
    expect(
      screen.getByText('The original name is taken by a different file — kept as is.'),
    ).toBeInTheDocument();
    // The summary calls out skipped and name-taken counts when non-zero.
    expect(screen.getByRole('status')).toHaveTextContent(
      'This will restore 1 file (1 rename across 1 version) · 1 skipped · 1 name taken',
    );
  });
});
