import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import HistoryPanel from './HistoryPanel';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  selectSnapshot: vi.fn(() => Promise.resolve()),
  clearHistory: vi.fn(() => Promise.resolve()),
}));

const meta = {
  id: 'AAAA',
  date: new Date(Date.now() - 3 * 3600 * 1000).toISOString(),
  summary: 'Prefix “x-” · Trim spaces',
  entryCount: 17,
};

describe('HistoryPanel (§14.5)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUiStore.setState({ confirm: null });
  });

  it('renders rows with relative date, summary, and rename count', () => {
    useAppStore.setState({ history: [meta], snapshot: makeSnapshot({ historyCount: 1 }) });
    render(<HistoryPanel />);
    expect(screen.getByText('3 hours ago')).toBeInTheDocument();
    expect(screen.getByText('Prefix “x-” · Trim spaces')).toBeInTheDocument();
    expect(screen.getByText('17 renames')).toBeInTheDocument();
  });

  it('click selects the version; clicking again deselects', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ history: [meta], snapshot: makeSnapshot() });
    render(<HistoryPanel />);
    await user.click(screen.getByText('Prefix “x-” · Trim spaces'));
    expect(vi.mocked(commands.selectSnapshot)).toHaveBeenCalledWith('AAAA');
    useAppStore.setState({ snapshot: makeSnapshot({ selectedSnapshotId: 'AAAA' }) });
    await user.click(screen.getByText('Prefix “x-” · Trim spaces'));
    expect(vi.mocked(commands.selectSnapshot)).toHaveBeenLastCalledWith(null);
  });

  it('Clear History confirms before destroying stored data (🗑 policy)', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ history: [meta], snapshot: makeSnapshot() });
    render(<HistoryPanel />);
    await user.click(screen.getByRole('button', { name: 'Clear History…' }));
    const confirm = useUiStore.getState().confirm;
    expect(confirm?.title).toBe('Clear rename history?');
    confirm?.onConfirm();
    expect(vi.mocked(commands.clearHistory)).toHaveBeenCalled();
  });

  it('shows the empty state without history', () => {
    useAppStore.setState({ history: [], snapshot: makeSnapshot() });
    render(<HistoryPanel />);
    expect(screen.getByText(/appear here as versions/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Clear History…' })).toBeDisabled();
  });
});
