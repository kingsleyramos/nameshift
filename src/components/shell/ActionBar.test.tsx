import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ActionBar from './ActionBar';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeEntry, makePreview, makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  apply: vi.fn(() => Promise.resolve()),
  deselectConflicted: vi.fn(() => Promise.resolve()),
  setAutoResolve: vi.fn(() => Promise.resolve()),
  revertSelected: vi.fn(() => Promise.resolve()),
}));

describe('ActionBar (§14.1, §7.7 gating)', () => {
  beforeEach(() => {
    useUiStore.setState({ filterMode: 'all', confirm: null });
    useAppStore.setState({ processing: null, revertPreview: null });
  });

  it('shows live counts and an enabled primary when applyable', () => {
    const entries = [
      makeEntry({ currentName: 'a.txt', newName: 'x-a.txt', isChanged: true }),
      makeEntry({ currentName: 'b.txt', newName: 'x-b.txt', isChanged: true }),
      makeEntry({ currentName: 'c.txt' }),
    ];
    useAppStore.setState({ snapshot: makeSnapshot(), preview: makePreview(entries) });
    render(<ActionBar />);
    expect(screen.getByText('3 of 3 included')).toBeInTheDocument();
    expect(screen.getByText('2 will change')).toBeInTheDocument();
    const primary = screen.getByRole('button', { name: /Rename 2 Files/ });
    expect(primary).toBeEnabled();
  });

  it('uses the singular/plural §A copy and the Folders noun', () => {
    const entries = [makeEntry({ newName: 'x.txt', isChanged: true })];
    useAppStore.setState({
      snapshot: makeSnapshot({ listMode: 'Folders' }),
      preview: makePreview(entries),
    });
    render(<ActionBar />);
    expect(screen.getByRole('button', { name: /Rename 1 Folder$/ })).toBeInTheDocument();
  });

  it('disables the primary and shows the reason when conflicts block Apply', () => {
    const entries = [
      makeEntry({ newName: 'dup.txt', isChanged: true, problem: 'DuplicateTarget' }),
      makeEntry({ newName: 'dup.txt', isChanged: true, problem: 'DuplicateTarget' }),
    ];
    useAppStore.setState({
      snapshot: makeSnapshot(),
      preview: makePreview(entries, {
        applyDisabledReason: 'Fix or skip the naming conflicts to rename',
      }),
    });
    render(<ActionBar />);
    expect(screen.getByRole('button', { name: /Rename/ })).toBeDisabled();
    expect(screen.getByText('2 naming conflicts')).toBeInTheDocument();
    expect(screen.getByText('Fix or skip the naming conflicts to rename')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Skip Conflicted' })).toBeInTheDocument();
  });

  it('never says bare “conflict” without “naming”', () => {
    const entries = [makeEntry({ newName: 'x', isChanged: true, problem: 'EmptyName' })];
    useAppStore.setState({ snapshot: makeSnapshot(), preview: makePreview(entries) });
    const { container } = render(<ActionBar />);
    // “Skip Conflicted” is the §A-sanctioned button label; every other
    // occurrence must read “naming conflict”.
    const text = container.textContent.replaceAll('Skip Conflicted', '');
    for (const match of text.matchAll(/conflict/gi)) {
      const before = text.slice(Math.max(0, match.index - 10), match.index);
      expect(before.toLowerCase()).toContain('naming');
    }
  });

  it('clicking a count toggles its filter', async () => {
    const user = userEvent.setup();
    const entries = [makeEntry({ newName: 'x-a.txt', isChanged: true })];
    useAppStore.setState({ snapshot: makeSnapshot(), preview: makePreview(entries) });
    render(<ActionBar />);
    await user.click(screen.getByText('1 will change'));
    expect(useUiStore.getState().filterMode).toBe('willChange');
    await user.click(screen.getByText('1 will change'));
    expect(useUiStore.getState().filterMode).toBe('all');
  });

  it('invokes apply from the primary button', async () => {
    const user = userEvent.setup();
    const entries = [makeEntry({ newName: 'x.txt', isChanged: true })];
    useAppStore.setState({ snapshot: makeSnapshot(), preview: makePreview(entries) });
    render(<ActionBar />);
    await user.click(screen.getByRole('button', { name: /Rename 1 File$/ }));
    expect(vi.mocked(commands.apply)).toHaveBeenCalledTimes(1);
  });

  it('revert mode shows the revert action, never the accent primary', () => {
    useAppStore.setState({
      snapshot: makeSnapshot({ selectedSnapshotId: 'ABC' }),
      preview: makePreview([]),
      revertPreview: {
        version: 1,
        entries: [],
        restorableRenameCount: 3,
        restorableFileCount: 2,
        nameTakenCount: 0,
        newerSnapshotCount: 1,
      },
    });
    render(<ActionBar />);
    const revert = screen.getByRole('button', { name: /Revert…/ });
    expect(revert).toBeEnabled();
    expect(revert.style.background).toContain('--revert-action');
    expect(screen.queryByRole('button', { name: /Rename/ })).not.toBeInTheDocument();
    expect(
      screen.getByText('This will restore 2 files (3 renames across 2 versions)'),
    ).toBeInTheDocument();
  });
});
