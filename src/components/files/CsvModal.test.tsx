import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import CsvModal from './CsvModal';
import { useAppStore } from '../../state/appState';
import { makeEntry, makePreview, makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: () => Promise.resolve(() => undefined),
  }),
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(() => Promise.resolve('/tmp/edited.csv')),
}));
vi.mock('../../ipc/commands', () => ({
  exportCsvTemplate: vi.fn(() => Promise.resolve()),
  csvDryRun: vi.fn(),
  csvApply: vi.fn(() => Promise.resolve()),
}));

describe('CsvModal (§14.7 — one state, results as a sentence)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      snapshot: makeSnapshot(),
      preview: makePreview([makeEntry(), makeEntry(), makeEntry()]),
    });
  });

  it('renders the download button with the live count, drop zone, and fixed footer', () => {
    render(<CsvModal onClose={() => undefined} />);
    expect(
      screen.getByRole('button', { name: 'Download Template CSV (3 files)…' }),
    ).toBeInTheDocument();
    expect(screen.getByTestId('csv-drop-zone')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Choose CSV…' })).toBeInTheDocument();
    // Count-less disabled Apply until a read exists (§14.7 footer).
    const apply = screen.getByRole('button', { name: 'Apply Changes' });
    expect(apply).toBeDisabled();
  });

  it('reads a chosen file, shows the sentence, and enables Apply with the count', async () => {
    const user = userEvent.setup();
    vi.mocked(commands.csvDryRun).mockResolvedValue({
      matches: [
        { id: 'A', newName: 'new-a.txt', changes: true },
        { id: 'B', newName: 'b.txt', changes: false },
      ],
      misses: [],
      willChange: 1,
      unchanged: 1,
    });
    render(<CsvModal onClose={() => undefined} />);
    await user.click(screen.getByRole('button', { name: 'Choose CSV…' }));
    expect(await screen.findByText('1 of 3 names will change.')).toBeInTheDocument();
    // The zone collapses to a passive file chip; Choose stays.
    expect(screen.queryByTestId('csv-drop-zone')).not.toBeInTheDocument();
    expect(screen.getByText('edited.csv')).toBeInTheDocument();
    const apply = screen.getByRole('button', { name: 'Apply 1 Changes' });
    expect(apply).toBeEnabled();
  });

  it('groups misses behind the single disclosure', async () => {
    const user = userEvent.setup();
    vi.mocked(commands.csvDryRun).mockResolvedValue({
      matches: [{ id: 'A', newName: 'x.txt', changes: true }],
      misses: [
        { name: 'ghost.txt', reason: 'notInList' },
        { name: 'twice.txt', reason: 'duplicateRow' },
        { name: 'broken-line', reason: 'couldntRead' },
      ],
      willChange: 1,
      unchanged: 0,
    });
    render(<CsvModal onClose={() => undefined} />);
    await user.click(screen.getByRole('button', { name: 'Choose CSV…' }));
    const disclosure = await screen.findByText(/3 rows didn’t match/);
    expect(screen.queryByText('ghost.txt')).not.toBeInTheDocument();
    await user.click(disclosure);
    expect(screen.getByText('Not in the list')).toBeInTheDocument();
    expect(screen.getByText('Duplicate rows')).toBeInTheDocument();
    expect(screen.getByText('Couldn’t read')).toBeInTheDocument();
    expect(screen.getByText('ghost.txt')).toBeInTheDocument();
  });

  it('shows the zero-match note and keeps Apply disabled', async () => {
    const user = userEvent.setup();
    vi.mocked(commands.csvDryRun).mockResolvedValue({
      matches: [],
      misses: [{ name: 'ghost.txt', reason: 'notInList' }],
      willChange: 0,
      unchanged: 0,
    });
    render(<CsvModal onClose={() => undefined} />);
    await user.click(screen.getByRole('button', { name: 'Choose CSV…' }));
    expect(
      await screen.findByText('No rows matched — was this CSV exported from a different list?'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Apply 0 Changes' })).toBeDisabled();
  });

  it('shows read errors inline, never a modal', async () => {
    const user = userEvent.setup();
    vi.mocked(commands.csvDryRun).mockRejectedValue({
      title: 'Couldn’t complete that',
      message: 'Its first row must be the exported header “Current Name, New Name”.',
    });
    render(<CsvModal onClose={() => undefined} />);
    await user.click(screen.getByRole('button', { name: 'Choose CSV…' }));
    expect(
      await screen.findByText(
        'Its first row must be the exported header “Current Name, New Name”.',
      ),
    ).toBeInTheDocument();
  });

  it('applying commits only the changed matches and closes', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    vi.mocked(commands.csvDryRun).mockResolvedValue({
      matches: [
        { id: 'A', newName: 'new-a.txt', changes: true },
        { id: 'B', newName: 'b.txt', changes: false },
      ],
      misses: [],
      willChange: 1,
      unchanged: 1,
    });
    render(<CsvModal onClose={onClose} />);
    await user.click(screen.getByRole('button', { name: 'Choose CSV…' }));
    await user.click(await screen.findByRole('button', { name: 'Apply 1 Changes' }));
    expect(vi.mocked(commands.csvApply)).toHaveBeenCalledWith([
      { id: 'A', newName: 'new-a.txt', changes: true },
    ]);
    expect(onClose).toHaveBeenCalled();
  });
});
