import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import FileRow, { problemLabel } from './FileRow';
import { makeEntry } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  removeItems: vi.fn(() => Promise.resolve()),
  ruleDerivedName: vi.fn(() => Promise.resolve('rules-output.txt')),
  setOverride: vi.fn(() => Promise.resolve()),
}));

const noop = () => undefined;

function renderEditing(entry = makeEntry({ currentName: 'a.txt', newName: 'a.txt' })) {
  const onEndEdit = vi.fn();
  render(
    <FileRow
      entry={entry}
      isDirectory={false}
      folderSuffix={null}
      selected={false}
      focused={false}
      editing
      inlineDiff={false}
      os="macos"
      onRowClick={noop}
      onCheckboxClick={noop}
      onStartEdit={noop}
      onEndEdit={onEndEdit}
      onContextMenu={noop}
    />,
  );
  return onEndEdit;
}

describe('inline new-name editor (§14.2)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('Return commits a typed override', async () => {
    const user = userEvent.setup();
    const entry = makeEntry({ currentName: 'a.txt', newName: 'a.txt' });
    const onEndEdit = renderEditing(entry);
    const input = screen.getByRole('textbox', { name: 'New name for a.txt' });
    await user.clear(input);
    await user.type(input, 'custom.txt{Enter}');
    expect(vi.mocked(commands.setOverride)).toHaveBeenCalledWith(entry.id, 'custom.txt');
    expect(onEndEdit).toHaveBeenCalled();
  });

  it('committing text identical to the rules output CLEARS the override', async () => {
    const user = userEvent.setup();
    const entry = makeEntry({ currentName: 'a.txt', newName: 'a.txt' });
    renderEditing(entry);
    const input = screen.getByRole('textbox', { name: 'New name for a.txt' });
    await user.clear(input);
    await user.type(input, 'rules-output.txt{Enter}');
    expect(vi.mocked(commands.setOverride)).toHaveBeenCalledWith(entry.id, null);
  });

  it('Esc cancels without committing', async () => {
    const user = userEvent.setup();
    const onEndEdit = renderEditing();
    await user.type(screen.getByRole('textbox'), 'zzz{Escape}');
    expect(vi.mocked(commands.setOverride)).not.toHaveBeenCalled();
    expect(onEndEdit).toHaveBeenCalled();
  });

  it('focus loss commits (Finder behavior)', async () => {
    const user = userEvent.setup();
    const entry = makeEntry({ currentName: 'a.txt', newName: 'a.txt' });
    renderEditing(entry);
    const input = screen.getByRole('textbox');
    await user.clear(input);
    await user.type(input, 'blurred.txt');
    (input as HTMLInputElement).blur();
    await vi.waitFor(() => {
      expect(vi.mocked(commands.setOverride)).toHaveBeenCalledWith(entry.id, 'blurred.txt');
    });
  });
});

describe('problemLabel — §A copy per OS', () => {
  it('covers every problem kind', () => {
    const kinds = [
      'EmptyName',
      'InvalidCharacters',
      'NameTooLong',
      'ReservedName',
      'EndsWithDotOrSpace',
      'DuplicateTarget',
      'ExistingFileCollision',
      'UnrenamableName',
      'NoFolderPermission',
    ] as const;
    for (const kind of kinds) {
      for (const os of ['macos', 'windows', 'linux']) {
        expect(problemLabel(kind, os).length).toBeGreaterThan(0);
      }
    }
    expect(problemLabel('InvalidCharacters', 'windows')).toContain('Windows doesn’t allow');
    expect(problemLabel('NameTooLong', 'linux')).toContain('this system');
  });
});

describe('inline diff view (§14.2 density mode)', () => {
  it('renders one combined diff line', () => {
    const entry = makeEntry({ currentName: 'a.txt', newName: 'b.txt', isChanged: true });
    render(
      <FileRow
        entry={entry}
        isDirectory={false}
        folderSuffix={null}
        selected={false}
        focused={false}
        editing={false}
        inlineDiff
        os="macos"
        onRowClick={noop}
        onCheckboxClick={noop}
        onStartEdit={noop}
        onEndEdit={noop}
        onContextMenu={noop}
      />,
    );
    // Single-column mode: no arrow between two columns.
    expect(screen.queryByText('→')).not.toBeInTheDocument();
  });
});
