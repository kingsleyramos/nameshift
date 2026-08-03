import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WatchedFolderChips from './WatchedFolderChips';
import SortMenu from './SortMenu';
import { OldName, NewName } from './NameDiff';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeItem, makeSnapshot, nextId } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  setWatchedFolderSubfolders: vi.fn(() => Promise.resolve()),
  removeWatchedFolder: vi.fn(() => Promise.resolve()),
  setSort: vi.fn(() => Promise.resolve()),
}));

const folder = { id: nextId(), path: '/Users/demo/Iceland', includeSubfolders: null };

describe('WatchedFolderChips (§14.2, §9)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUiStore.setState({ confirm: null });
  });

  it('renders the chip with its depth menu', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot({ watchedFolders: [folder] }) });
    render(<WatchedFolderChips />);
    expect(screen.getByText('Watching:')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Iceland' }));
    await user.click(screen.getByRole('menuitem', { name: 'Include Subfolders' }));
    expect(vi.mocked(commands.setWatchedFolderSubfolders)).toHaveBeenCalledWith(folder.id, true);
  });

  it('small folders stop watching instantly (undoable)', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot({ watchedFolders: [folder] }) });
    render(<WatchedFolderChips />);
    await user.click(screen.getByRole('button', { name: 'Stop watching Iceland' }));
    expect(vi.mocked(commands.removeWatchedFolder)).toHaveBeenCalledWith(folder.id);
  });

  it('guards when the folder contributed > 10 items or edits (§14.0)', async () => {
    const user = userEvent.setup();
    const items = Array.from({ length: 11 }, () => makeItem({ folderId: folder.id }));
    const edited = items[0];
    if (edited) edited.overrideName = 'custom.txt';
    useAppStore.setState({
      snapshot: makeSnapshot({ watchedFolders: [folder], files: items }),
    });
    render(<WatchedFolderChips />);
    await user.click(screen.getByRole('button', { name: 'Stop watching Iceland' }));
    const confirm = useUiStore.getState().confirm;
    expect(confirm?.title).toBe('Stop watching “Iceland”?');
    expect(confirm?.message).toBe(
      'Its 11 files leave the list, and 1 manual edit is discarded. Files on disk aren’t changed.',
    );
    confirm?.onConfirm();
    expect(vi.mocked(commands.removeWatchedFolder)).toHaveBeenCalledWith(folder.id);
  });
});

describe('SortMenu (§14.2 — mirrors the header, never disagrees)', () => {
  it('offers every key with direction, hiding Extension in Folders mode', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot({ listMode: 'Folders' }) });
    render(<SortMenu />);
    await user.click(screen.getByRole('button', { name: /Order Added/ }));
    expect(screen.queryByRole('menuitem', { name: 'Extension' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('menuitem', { name: 'Date Created' }));
    expect(vi.mocked(commands.setSort)).toHaveBeenCalledWith('Date Created', true);
  });

  it('direction entries flip ascending', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot({ sortKey: 'Name', sortAscending: true }) });
    render(<SortMenu />);
    await user.click(screen.getByRole('button', { name: /Name/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Descending' }));
    expect(vi.mocked(commands.setSort)).toHaveBeenCalledWith('Name', false);
  });
});

describe('NameDiff spans render (§14.6)', () => {
  it('old middle is struck through; added middle underlines non-whitespace runs only', () => {
    const { container } = render(
      <p>
        <OldName oldName="chapter2.md" newName="chapter 2.md" />
        <NewName oldName="chapter2.md" newName="chapter 2.md" />
      </p>,
    );
    const struck = container.querySelector('[style*="line-through"]');
    expect(struck).toBeNull(); // pure insertion: nothing removed
    const added = container.querySelector('[style*="--diff-added-bg-whitespace"]');
    expect(added).not.toBeNull();
    expect(added?.querySelector('[style*="underline"]')).toBeNull();
  });

  it('a text change strikes the old middle and underlines the new letters', () => {
    const { container } = render(
      <p>
        <OldName oldName="IMG_0421.jpg" newName="IMG_beach.jpg" />
        <NewName oldName="IMG_0421.jpg" newName="IMG_beach.jpg" />
      </p>,
    );
    expect(container.querySelector('[style*="line-through"]')?.textContent).toBe('0421');
    const underlined = container.querySelectorAll('[style*="underline"]');
    expect([...underlined].map((n) => n.textContent).join('')).toBe('beach');
  });
});
