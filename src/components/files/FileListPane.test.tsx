import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import FileListPane from './FileListPane';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeEntry, makeItem, makePreview, makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

// jsdom has no layout: render every row.
vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: (options: { count: number }) => ({
    getTotalSize: () => options.count * 28,
    getVirtualItems: () =>
      Array.from({ length: options.count }, (_, index) => ({
        index,
        start: index * 28,
        size: 28,
        key: index,
      })),
    scrollToIndex: () => undefined,
  }),
}));
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: vi.fn(() => Promise.resolve()),
}));
vi.mock('../../ipc/commands', () => ({
  setSelected: vi.fn(() => Promise.resolve()),
  setSelectedMany: vi.fn(() => Promise.resolve()),
  setAllSelected: vi.fn(() => Promise.resolve()),
  selectOnly: vi.fn(() => Promise.resolve()),
  removeItems: vi.fn(() => Promise.resolve()),
  setSort: vi.fn(() => Promise.resolve()),
  setListMode: vi.fn(() => Promise.resolve()),
  setWatchedFolderSubfolders: vi.fn(() => Promise.resolve()),
  removeWatchedFolder: vi.fn(() => Promise.resolve()),
  revealInFileManager: vi.fn(() => Promise.resolve()),
  setOverride: vi.fn(() => Promise.resolve()),
  ruleDerivedName: vi.fn(() => Promise.resolve('derived')),
}));

function seed(count = 3) {
  const items = Array.from({ length: count }, (_, i) => makeItem({ path: `/d/file-${i}.txt` }));
  const entries = items.map((item, i) =>
    makeEntry({ id: item.id, currentName: `file-${i}.txt`, newName: `file-${i}.txt` }),
  );
  useAppStore.setState({
    snapshot: makeSnapshot({ files: items }),
    preview: makePreview(entries),
    platform: { os: 'macos', channel: 'direct', hasSpotlight: true, version: '2.0.0' },
  });
  return { items, entries };
}

describe('FileListPane (§14.2)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUiStore.setState({
      filterText: '',
      filterMode: 'all',
      rowSelection: new Set(),
      focusedRow: null,
      selectionAnchor: null,
      checkboxAnchor: null,
      editingRow: null,
      inlineDiffView: false,
      confirm: null,
    });
  });

  it('shows the drop prompt when the list is empty', () => {
    useAppStore.setState({ snapshot: makeSnapshot(), preview: makePreview([]) });
    render(<FileListPane />);
    expect(screen.getByText('Drag files or folders here')).toBeInTheDocument();
  });

  it('renders rows with scope tabs and headers', () => {
    seed();
    render(<FileListPane />);
    expect(screen.getAllByRole('option')).toHaveLength(3);
    expect(screen.getByRole('tab', { name: 'Files · 3' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Current Name/ })).toBeInTheDocument();
  });

  it('click selects a row; ⌘-click toggles; ⇧-click extends (visible order)', async () => {
    const user = userEvent.setup();
    const { entries } = seed();
    render(<FileListPane />);
    const rows = screen.getAllByRole('option');
    await user.click(rows[0] as HTMLElement);
    expect(useUiStore.getState().rowSelection).toEqual(new Set([entries[0]?.id]));
    await user.keyboard('{Control>}');
    await user.click(rows[2] as HTMLElement);
    await user.keyboard('{/Control}');
    expect(useUiStore.getState().rowSelection.size).toBe(2);
    await user.keyboard('{Shift>}');
    await user.click(rows[1] as HTMLElement);
    await user.keyboard('{/Shift}');
    // Range from the ⌘-click anchor (row 2) to row 1.
    expect(useUiStore.getState().rowSelection.size).toBe(2);
  });

  it('Space toggles INCLUSION on the selection — selection and inclusion are distinct', async () => {
    const user = userEvent.setup();
    const { entries } = seed();
    render(<FileListPane />);
    await user.click(screen.getAllByRole('option')[0] as HTMLElement);
    const list = screen.getByRole('listbox');
    (list as HTMLElement).focus();
    await user.keyboard(' ');
    expect(vi.mocked(commands.setSelectedMany)).toHaveBeenCalledWith([entries[0]?.id], false);
  });

  it('⇧-clicking a checkbox applies its state to the whole range in ONE call', async () => {
    const user = userEvent.setup();
    const { entries } = seed(4);
    render(<FileListPane />);
    const checkboxes = screen
      .getAllByRole('option')
      .map((row) => within(row as HTMLElement).getByRole('checkbox'));
    await user.click(checkboxes[0] as HTMLElement);
    expect(vi.mocked(commands.setSelected)).toHaveBeenCalledWith(entries[0]?.id, false);
    await user.keyboard('{Shift>}');
    await user.click(checkboxes[3] as HTMLElement);
    await user.keyboard('{/Shift}');
    expect(vi.mocked(commands.setSelectedMany)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(commands.setSelectedMany)).toHaveBeenCalledWith(
      entries.map((e) => e.id),
      false,
    );
  });

  it('filters change visibility only, and the matching menu offers Select Only These', async () => {
    const user = userEvent.setup();
    const { entries } = seed();
    const changed = entries[1];
    if (changed) {
      changed.newName = 'renamed.txt';
      changed.isChanged = true;
    }
    useAppStore.setState({ preview: makePreview(entries) });
    useUiStore.setState({ filterMode: 'willChange' });
    render(<FileListPane />);
    expect(screen.getAllByRole('option')).toHaveLength(1);
    await user.click(screen.getByRole('button', { name: /1 matching/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Select Only These' }));
    expect(vi.mocked(commands.selectOnly)).toHaveBeenCalledWith([changed?.id]);
  });

  it('text filter matches current OR new name; empty state offers Clear Filter', async () => {
    const user = userEvent.setup();
    seed();
    useUiStore.setState({ filterText: 'no-such-file' });
    render(<FileListPane />);
    expect(screen.getByText('No files match the filter')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Clear Filter' }));
    expect(useUiStore.getState().filterText).toBe('');
  });

  it('header checkbox is tri-state over inclusion', () => {
    const { entries } = seed();
    const first = entries[0];
    if (first) first.isSelected = false;
    useAppStore.setState({ preview: makePreview(entries) });
    render(<FileListPane />);
    const header = screen.getByRole('checkbox', { name: 'Include all' });
    expect((header as HTMLInputElement).indeterminate).toBe(true);
  });

  it('header Current Name click sorts by Name, click again flips direction', async () => {
    const user = userEvent.setup();
    seed();
    render(<FileListPane />);
    await user.click(screen.getByRole('button', { name: /Current Name/ }));
    expect(vi.mocked(commands.setSort)).toHaveBeenCalledWith('Name', true);
    useAppStore.setState({
      snapshot: makeSnapshot({
        files: useAppStore.getState().snapshot?.files ?? [],
        sortKey: 'Name',
        sortAscending: true,
      }),
    });
    await user.click(screen.getByRole('button', { name: /Current Name/ }));
    expect(vi.mocked(commands.setSort)).toHaveBeenLastCalledWith('Name', false);
  });

  it('Delete removes the selection (small removals are instant + undoable)', async () => {
    const user = userEvent.setup();
    const { entries } = seed();
    render(<FileListPane />);
    await user.click(screen.getAllByRole('option')[0] as HTMLElement);
    (screen.getByRole('listbox') as HTMLElement).focus();
    await user.keyboard('{Delete}');
    expect(vi.mocked(commands.removeItems)).toHaveBeenCalledWith([entries[0]?.id]);
  });

  it('bulk removal (> 10 rows) routes through the §14.0 guard', async () => {
    const user = userEvent.setup();
    const { entries } = seed(12);
    render(<FileListPane />);
    useUiStore.setState({ rowSelection: new Set(entries.map((e) => e.id)) });
    (screen.getByRole('listbox') as HTMLElement).focus();
    await user.keyboard('{Delete}');
    expect(vi.mocked(commands.removeItems)).not.toHaveBeenCalled();
    expect(useUiStore.getState().confirm?.title).toBe('Remove 12 files from the list?');
  });

  it('context menu acts on the selection', async () => {
    const user = userEvent.setup();
    seed();
    render(<FileListPane />);
    await user.pointer({ keys: '[MouseRight]', target: screen.getAllByRole('option')[0] as HTMLElement });
    expect(screen.getByRole('menu')).toBeInTheDocument();
    await user.click(screen.getByRole('menuitem', { name: 'Reveal in Finder' }));
    expect(vi.mocked(commands.revealInFileManager)).toHaveBeenCalled();
  });

  it('shows the folder suffix only when the visible set spans > 1 directory', () => {
    const a = makeItem({ path: '/one/a.txt' });
    const b = makeItem({ path: '/two/b.txt' });
    useAppStore.setState({
      snapshot: makeSnapshot({ files: [a, b] }),
      preview: makePreview([
        makeEntry({ id: a.id, currentName: 'a.txt' }),
        makeEntry({ id: b.id, currentName: 'b.txt' }),
      ]),
    });
    render(<FileListPane />);
    expect(screen.getByText('· one')).toBeInTheDocument();
    expect(screen.getByText('· two')).toBeInTheDocument();
  });
});
