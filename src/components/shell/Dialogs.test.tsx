import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Dialogs from './Dialogs';
import ProcessingOverlay from './ProcessingOverlay';
import DropOverlay from './DropOverlay';
import SplitLayout from './SplitLayout';
import { useUiStore } from '../../state/uiState';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  cancelProcessing: vi.fn(() => Promise.resolve()),
}));

describe('Dialogs (§14.9)', () => {
  beforeEach(() => {
    useUiStore.setState({ confirm: null, alert: null, dragOver: false });
  });

  it('renders nothing without a request', () => {
    render(<Dialogs />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('confirmations show title, message, and both buttons', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    useUiStore.setState({
      confirm: {
        title: 'Revert these renames?',
        message: '3 files will be renamed back on disk.',
        confirmLabel: 'Revert',
        destructive: true,
        onConfirm,
      },
    });
    render(<Dialogs />);
    expect(screen.getByText('Revert these renames?')).toBeInTheDocument();
    expect(screen.getByText('3 files will be renamed back on disk.')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Revert' }));
    expect(onConfirm).toHaveBeenCalled();
    expect(useUiStore.getState().confirm).toBeNull();
  });

  it('cancel dismisses without running the action', async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    useUiStore.setState({
      confirm: {
        title: 'Clear the file list?',
        message: 'msg',
        confirmLabel: 'Clear List',
        destructive: true,
        onConfirm,
      },
    });
    render(<Dialogs />);
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('alerts show a single OK', async () => {
    const user = userEvent.setup();
    useUiStore.setState({
      alert: { kind: 'info', title: 'Some files were missing', message: '2 files…' },
    });
    render(<Dialogs />);
    await user.click(screen.getByRole('button', { name: 'OK' }));
    expect(useUiStore.getState().alert).toBeNull();
  });
});

describe('ProcessingOverlay (§14.1)', () => {
  it('appears only after 350 ms with determinate progress and Esc-bound Cancel', async () => {
    vi.useFakeTimers();
    render(
      <ProcessingOverlay processing={{ title: 'Renaming files…', completed: 4, total: 10 }} />,
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await act(() => vi.advanceTimersByTimeAsync(400));
    expect(screen.getByRole('dialog', { name: 'Renaming files…' })).toBeInTheDocument();
    expect(screen.getByText('4 of 10')).toBeInTheDocument();
    vi.useRealTimers();
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(vi.mocked(commands.cancelProcessing)).toHaveBeenCalled();
  });
});

describe('DropOverlay + SplitLayout', () => {
  it('drop overlay renders only during a drag', () => {
    useUiStore.setState({ dragOver: false });
    const { rerender, container } = render(<DropOverlay />);
    expect(container.firstChild).toBeNull();
    useUiStore.setState({ dragOver: true });
    rerender(<DropOverlay />);
    expect(container.firstChild).not.toBeNull();
  });

  it('split layout renders both panes with a resize separator', () => {
    render(<SplitLayout side={<p>side</p>} detail={<p>detail</p>} />);
    expect(screen.getByText('side')).toBeInTheDocument();
    expect(screen.getByText('detail')).toBeInTheDocument();
    expect(screen.getByRole('separator', { name: 'Resize side panel' })).toBeInTheDocument();
  });
});
