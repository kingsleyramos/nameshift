import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import FileRow from './FileRow';
import { makeEntry } from '../../test/fixtures';

vi.mock('../../ipc/commands', () => ({
  removeItems: vi.fn(() => Promise.resolve()),
  ruleDerivedName: vi.fn(() => Promise.resolve('derived.txt')),
  setOverride: vi.fn(() => Promise.resolve()),
}));

const noop = () => undefined;

function renderRow(entry = makeEntry(), props: Partial<Parameters<typeof FileRow>[0]> = {}) {
  return render(
    <FileRow
      entry={entry}
      isDirectory={false}
      folderSuffix={null}
      selected={false}
      focused={false}
      editing={false}
      inlineDiff={false}
      os="macos"
      onRowClick={noop}
      onCheckboxClick={noop}
      onStartEdit={noop}
      onEndEdit={noop}
      onContextMenu={noop}
      {...props}
    />,
  );
}

describe('FileRow (§14.2 anatomy & precedence)', () => {
  it('shows the full-opacity Skipped pill on deselected rows', () => {
    renderRow(makeEntry({ isSelected: false }));
    expect(screen.getByText('Skipped')).toBeInTheDocument();
  });

  it('marks conflict rows with the wash and the ⚠ before the new name', () => {
    const entry = makeEntry({ newName: 'dup.txt', isChanged: true, problem: 'DuplicateTarget' });
    renderRow(entry);
    const row = screen.getByRole('option');
    expect(row.style.background).toContain('--wash-conflict');
    expect(
      screen.getByTitle('Two or more files would end up with the same name.'),
    ).toBeInTheDocument();
  });

  it('selection wash renders ON TOP of the conflict wash with the accent bar', () => {
    const entry = makeEntry({ newName: 'dup.txt', isChanged: true, problem: 'DuplicateTarget' });
    renderRow(entry, { selected: true });
    const row = screen.getByRole('option');
    // Conflict wash owns the row background…
    expect(row.style.background).toContain('--wash-conflict');
    // …selection adds its overlay + 3px leading accent bar on top.
    const overlays = row.querySelectorAll('div[aria-hidden]');
    expect(overlays.length).toBeGreaterThanOrEqual(2);
  });

  it('announces name, inclusion, and the conflict reason to screen readers', () => {
    const entry = makeEntry({
      currentName: 'photo.jpg',
      newName: 'dup.jpg',
      isChanged: true,
      isSelected: false,
      problem: 'ExistingFileCollision',
    });
    renderRow(entry);
    expect(
      screen.getByRole('option', {
        name: /photo\.jpg, skipped, naming conflict: A different file with this name already exists in the folder\./,
      }),
    ).toBeInTheDocument();
  });

  it('shows the pencil badge on overridden rows', () => {
    const entry = makeEntry({ newName: 'mine.txt', isChanged: true, hasOverride: true });
    renderRow(entry);
    expect(screen.getByRole('option').innerHTML).toContain('lucide-pencil');
  });
});
