import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Toolbar from './Toolbar';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeItem, makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  pickAndImport: vi.fn(() => Promise.resolve()),
  pickAndImportFolders: vi.fn(() => Promise.resolve()),
  setIncludeSubfolders: vi.fn(() => Promise.resolve()),
  setAutoResolve: vi.fn(() => Promise.resolve()),
  setKeepRules: vi.fn(() => Promise.resolve()),
  setTrimsWhitespace: vi.fn(() => Promise.resolve()),
  clearAllOverrides: vi.fn(() => Promise.resolve()),
  removeItems: vi.fn(() => Promise.resolve()),
  clearAll: vi.fn(() => Promise.resolve()),
}));

describe('Toolbar (§14.1)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUiStore.setState({ filterText: '', confirm: null, csvModalOpen: false });
  });

  it('has Add, Options, File Info, and the filter — and no primary action', () => {
    useAppStore.setState({ snapshot: makeSnapshot() });
    render(<Toolbar />);
    expect(screen.getByRole('button', {  name: 'Add'  })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Options' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /File Info/ })).toBeInTheDocument();
    expect(screen.getByRole('searchbox', { name: 'Filter files' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Rename/ })).not.toBeInTheDocument();
  });

  it('the Add menu offers Files… and Folders…', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot() });
    render(<Toolbar />);
    await user.click(screen.getByRole('button', {  name: 'Add'  }));
    await user.click(screen.getByRole('menuitem', { name: 'Files…' }));
    expect(vi.mocked(commands.pickAndImport)).toHaveBeenCalled();
  });

  it('Options has the four titled sections with working toggles', async () => {
    const user = userEvent.setup();
    useAppStore.setState({
      snapshot: makeSnapshot({ files: [makeItem()], overrideCount: 1 }),
    });
    render(<Toolbar />);
    await user.click(screen.getByRole('button', { name: 'Options' }));
    for (const section of ['Importing', 'Renaming', 'Spreadsheet', 'List']) {
      expect(screen.getByText(section)).toBeInTheDocument();
    }
    await user.click(screen.getByRole('menuitem', { name: /Keep Rules After Applying/ }));
    expect(vi.mocked(commands.setKeepRules)).toHaveBeenCalledWith(false);
  });

  it('Rename by CSV opens the modal; Clear All Manual Edits invokes the command', async () => {
    const user = userEvent.setup();
    useAppStore.setState({
      snapshot: makeSnapshot({ files: [makeItem()], overrideCount: 2 }),
    });
    render(<Toolbar />);
    await user.click(screen.getByRole('button', { name: 'Options' }));
    await user.click(screen.getByRole('menuitem', { name: 'Rename by CSV…' }));
    expect(useUiStore.getState().csvModalOpen).toBe(true);
    await user.click(screen.getByRole('button', { name: 'Options' }));
    await user.click(screen.getByRole('menuitem', { name: 'Clear All Manual Edits (2)' }));
    expect(vi.mocked(commands.clearAllOverrides)).toHaveBeenCalled();
  });

  it('Clear File List routes through the confirmation with §A copy', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot({ files: [makeItem()] }) });
    render(<Toolbar />);
    await user.click(screen.getByRole('button', { name: 'Options' }));
    await user.click(screen.getByRole('menuitem', { name: 'Clear File List…' }));
    const confirm = useUiStore.getState().confirm;
    expect(confirm?.title).toBe('Clear the file list?');
    expect(confirm?.message).toBe(
      'This empties the list and stops watching any folders. Files on disk aren’t changed.',
    );
    confirm?.onConfirm();
    expect(vi.mocked(commands.clearAll)).toHaveBeenCalled();
  });

  it('typing in the filter updates ui state and resets the checkbox anchor', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot() });
    useUiStore.setState({ checkboxAnchor: 'stale' });
    render(<Toolbar />);
    await user.type(screen.getByRole('searchbox', { name: 'Filter files' }), 'img');
    expect(useUiStore.getState().filterText).toBe('img');
    expect(useUiStore.getState().checkboxAnchor).toBeNull();
  });

  it('File Info toggles the inspector', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot() });
    useUiStore.setState({ inspectorOpen: false });
    render(<Toolbar />);
    await user.click(screen.getByRole('button', { name: /File Info/ }));
    expect(useUiStore.getState().inspectorOpen).toBe(true);
  });

  it('Remove Skipped routes small sets straight to removal', async () => {
    const user = userEvent.setup();
    const skipped = makeItem({ isSelected: false });
    useAppStore.setState({ snapshot: makeSnapshot({ files: [skipped, makeItem()] }) });
    render(<Toolbar />);
    await user.click(screen.getByRole('button', { name: 'Options' }));
    await user.click(screen.getByRole('menuitem', { name: 'Remove Skipped from List' }));
    expect(vi.mocked(commands.removeItems)).toHaveBeenCalledWith([skipped.id]);
  });
});
