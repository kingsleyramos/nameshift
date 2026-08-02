import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Banner from './Banner';
import { useUiStore } from '../../state/uiState';

vi.mock('../../ipc/commands', () => ({
  selectSnapshot: vi.fn(() => Promise.resolve()),
}));

describe('Banner (§14.1, §A variants)', () => {
  beforeEach(() => {
    useUiStore.setState({ banner: null });
  });

  it('renders nothing without a banner', () => {
    render(<Banner />);
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('announces the renamed count with the cleared-rules suffix', () => {
    useUiStore.setState({
      banner: {
        count: 17,
        snapshotId: 'ABC',
        isFolders: false,
        clearedRules: true,
        keptRules: false,
      },
    });
    render(<Banner />);
    const status = screen.getByRole('status');
    expect(status).toHaveTextContent('Renamed 17 files');
    expect(status).toHaveTextContent('· Rules cleared (⌘Z restores them)');
    expect(screen.getByRole('button', { name: 'Revert…' })).toBeInTheDocument();
  });

  it('kept-rules and folders variants', () => {
    useUiStore.setState({
      banner: {
        count: 1,
        snapshotId: null,
        isFolders: true,
        clearedRules: false,
        keptRules: true,
      },
    });
    render(<Banner />);
    const status = screen.getByRole('status');
    expect(status).toHaveTextContent('Renamed 1 folder');
    expect(status).toHaveTextContent('· Kept rules; renamed files deselected');
    // No snapshot → no Revert button.
    expect(screen.queryByRole('button', { name: 'Revert…' })).not.toBeInTheDocument();
  });

  it('pure manual-edit applies claim neither cleared nor kept (§8.5.4)', () => {
    useUiStore.setState({
      banner: {
        count: 2,
        snapshotId: 'ABC',
        isFolders: false,
        clearedRules: false,
        keptRules: false,
      },
    });
    render(<Banner />);
    const status = screen.getByRole('status');
    expect(status).toHaveTextContent('Renamed 2 files');
    expect(status).not.toHaveTextContent('Rules cleared');
    expect(status).not.toHaveTextContent('Kept rules');
  });

  it('auto-dismisses after 6 s', () => {
    vi.useFakeTimers();
    useUiStore.setState({
      banner: { count: 3, snapshotId: null, isFolders: false, clearedRules: false, keptRules: false },
    });
    render(<Banner />);
    expect(screen.getByRole('status')).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(6100);
    });
    expect(useUiStore.getState().banner).toBeNull();
    vi.useRealTimers();
  });
});
