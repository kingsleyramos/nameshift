import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SettingsWindow from './SettingsWindow';
import HelpWindow from '../help/HelpWindow';
import { useAppStore } from '../../state/appState';
import { makeSnapshot } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));
vi.mock('../../ipc/commands', () => ({
  setTrimsWhitespace: vi.fn(() => Promise.resolve()),
  setAutoResolve: vi.fn(() => Promise.resolve()),
  setKeepRules: vi.fn(() => Promise.resolve()),
  setIncludeSubfolders: vi.fn(() => Promise.resolve()),
}));

describe('SettingsWindow (§14.11)', () => {
  it('renders both groups with the keep-rules help text and live toggles', async () => {
    const user = userEvent.setup();
    useAppStore.setState({ snapshot: makeSnapshot({ keepRulesAfterApply: true }) });
    render(<SettingsWindow />);
    expect(screen.getByText('Renaming')).toBeInTheDocument();
    expect(screen.getByText('Folders')).toBeInTheDocument();
    expect(screen.getByText(/keep them for the next batch/)).toBeInTheDocument();
    await user.click(screen.getByRole('checkbox', { name: /Keep rules after applying/ }));
    expect(vi.mocked(commands.setKeepRules)).toHaveBeenCalledWith(false);
    await user.click(
      screen.getByRole('checkbox', { name: 'Include subfolders when watching a folder' }),
    );
    expect(vi.mocked(commands.setIncludeSubfolders)).toHaveBeenCalledWith(true);
  });
});

describe('HelpWindow (§14.12, §22.5)', () => {
  it('renders the topic sidebar and switches articles', async () => {
    const user = userEvent.setup();
    render(<HelpWindow />);
    expect(screen.getByRole('heading', { name: 'Getting Started' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Renaming & Naming Conflicts' }));
    expect(
      screen.getByRole('heading', { name: 'Renaming & Naming Conflicts' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/two phases/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Recipes' }));
    expect(screen.getByText(/Date-stamp a photo shoot/)).toBeInTheDocument();
  });

  it('deep-links from the topic query parameter', () => {
    const original = window.location.href;
    window.history.replaceState(null, '', '?topic=tokens');
    render(<HelpWindow />);
    expect(screen.getByRole('heading', { name: 'Tokens' })).toBeInTheDocument();
    window.history.replaceState(null, '', original);
  });
});
