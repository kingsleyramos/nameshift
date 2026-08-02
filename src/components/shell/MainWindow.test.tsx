import { render, screen } from '@testing-library/react';
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import MainWindow from './MainWindow';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeEntry, makePreview, makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

const listeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((name: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(name, handler);
    return Promise.resolve(() => undefined);
  }),
}));
vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn(() => Promise.resolve(() => undefined)),
  }),
}));
vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: (options: { count: number }) => ({
    getTotalSize: () => options.count * 28,
    getVirtualItems: () => [],
    scrollToIndex: () => undefined,
  }),
}));
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
  writeText: vi.fn(() => Promise.resolve()),
}));
vi.mock('../../ipc/commands', () => {
  const actual: Record<string, unknown> = {};
  const names = [
    'importPaths', 'pickAndImport', 'undo', 'redo', 'clearAllOverrides', 'cancelProcessing',
    'revertSelected', 'importPresets', 'exportPresets', 'selectSnapshot', 'setListMode',
    'setSort', 'setTrimsWhitespace', 'openHelp', 'replaceRulesUndoable', 'setRules',
    'presetNameExists', 'savePreset', 'applyPreset', 'deletePreset', 'clearHistory',
    'setSelected', 'setSelectedMany', 'setAllSelected', 'selectOnly', 'removeItems',
    'removeWatchedFolder', 'setWatchedFolderSubfolders', 'revealInFileManager',
    'ruleDerivedName', 'setOverride', 'exportCsvTemplate', 'csvDryRun', 'csvApply',
    'setAutoResolve', 'setKeepRules', 'setIncludeSubfolders', 'clearAll', 'apply',
    'deselectConflicted', 'pickAndImportFolders',
  ];
  for (const name of names) {
    actual[name] = vi.fn(() => Promise.resolve());
  }
  return actual;
});

describe('MainWindow (§14.1 shell + §15 menu routing)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
    useAppStore.setState({
      snapshot: makeSnapshot(),
      preview: makePreview([]),
      processing: null,
      history: [],
      revertPreview: null,
    });
    useUiStore.setState({
      banner: null,
      confirm: null,
      alert: null,
      csvModalOpen: false,
      sideTab: 'rules',
      filterText: '',
      filterMode: 'all',
      editingRow: null,
      inspectorOpen: false,
    });
  });

  it('renders toolbar, side tabs, list, and action bar', () => {
    render(<MainWindow />);
    expect(screen.getByRole('tab', { name: 'Rules' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Options' })).toBeInTheDocument();
    expect(screen.getByText('Drag files or folders here')).toBeInTheDocument();
  });

  it('routes alert events into the dialog and open-paths into the import gate', () => {
    render(<MainWindow />);
    act(() => {
      listeners.get('alert')?.({
        payload: { kind: 'info', title: 'Some files were missing', message: '2 files…' },
      });
    });
    expect(useUiStore.getState().alert?.title).toBe('Some files were missing');
    act(() => {
      listeners.get('open-paths')?.({ payload: { paths: ['/a', '/b'] } });
    });
    expect(vi.mocked(commands.importPaths)).toHaveBeenCalledWith(['/a', '/b']);
  });

  it('apply-finished shows the banner', () => {
    render(<MainWindow />);
    act(() => {
      listeners.get('apply-finished')?.({
        payload: {
          count: 5,
          snapshotId: null,
          isFolders: false,
          clearedRules: false,
          keptRules: true,
        },
      });
    });
    expect(useUiStore.getState().banner?.count).toBe(5);
  });

  it('menu undo/redo apply to the workspace when focus is not a text field', () => {
    render(<MainWindow />);
    act(() => {
      listeners.get('menu')?.({ payload: 'undo' });
      listeners.get('menu')?.({ payload: 'redo' });
    });
    expect(vi.mocked(commands.undo)).toHaveBeenCalled();
    expect(vi.mocked(commands.redo)).toHaveBeenCalled();
  });

  it('menu select-all selects rows, never checkboxes', () => {
    const entries = [makeEntry(), makeEntry()];
    useAppStore.setState({ preview: makePreview(entries) });
    render(<MainWindow />);
    act(() => {
      listeners.get('menu')?.({ payload: 'select-all' });
    });
    expect(useUiStore.getState().rowSelection.size).toBe(2);
    expect(vi.mocked(commands.setAllSelected)).not.toHaveBeenCalled();
  });

  it('menu actions open the CSV modal, toggle the inspector and diff view', () => {
    render(<MainWindow />);
    act(() => {
      listeners.get('menu')?.({ payload: 'rename-by-csv' });
      listeners.get('menu')?.({ payload: 'toggle-inspector' });
      listeners.get('menu')?.({ payload: 'inline-diff' });
    });
    expect(useUiStore.getState().csvModalOpen).toBe(true);
    expect(useUiStore.getState().inspectorOpen).toBe(true);
    expect(useUiStore.getState().inlineDiffView).toBe(true);
  });

  it('menu revert confirms with real counts before reverting', () => {
    useAppStore.setState({
      revertPreview: {
        version: 1,
        entries: [],
        restorableRenameCount: 4,
        restorableFileCount: 2,
        nameTakenCount: 0,
        newerSnapshotCount: 1,
      },
    });
    render(<MainWindow />);
    act(() => {
      listeners.get('menu')?.({ payload: 'revert' });
    });
    const confirm = useUiStore.getState().confirm;
    expect(confirm?.message).toBe(
      '2 files will be renamed back on disk. (4 renames across 2 versions.) This also undoes the 1 newer version.',
    );
    confirm?.onConfirm();
    expect(vi.mocked(commands.revertSelected)).toHaveBeenCalled();
  });

  it('Esc clears the filter last in the priority chain', () => {
    useUiStore.setState({ filterText: 'abc' });
    render(<MainWindow />);
    act(() => {
      const root = screen.getByLabelText('Name Shift');
      root.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    expect(useUiStore.getState().filterText).toBe('');
  });

  it('revert preview mode swaps the detail pane', () => {
    useAppStore.setState({
      snapshot: makeSnapshot({ selectedSnapshotId: 'ABC' }),
      revertPreview: {
        version: 1,
        entries: [],
        restorableRenameCount: 0,
        restorableFileCount: 0,
        nameTakenCount: 0,
        newerSnapshotCount: 0,
      },
    });
    render(<MainWindow />);
    expect(screen.getByTestId('revert-preview')).toBeInTheDocument();
  });
});
