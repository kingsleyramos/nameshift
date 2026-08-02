import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import PresetsMenu from './PresetsMenu';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { makeRule, makeSnapshot, nextId } from '../../test/fixtures';
import * as commands from '../../ipc/commands';

vi.mock('../../ipc/commands', () => ({
  presetNameExists: vi.fn(() => Promise.resolve(false)),
  savePreset: vi.fn(() => Promise.resolve()),
  applyPreset: vi.fn(() => Promise.resolve()),
  deletePreset: vi.fn(() => Promise.resolve()),
  importPresets: vi.fn(() => Promise.resolve()),
  exportPresets: vi.fn(() => Promise.resolve()),
}));

const preset = {
  id: nextId(),
  name: 'Photo import',
  rules: [makeRule()],
  trimsWhitespace: true,
};

describe('PresetsMenu (§14.8)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUiStore.setState({ confirm: null });
    useAppStore.setState({
      snapshot: makeSnapshot({ presets: [preset], rules: [makeRule()] }),
    });
  });

  it('loads a preset from the menu (undoable server-side)', async () => {
    const user = userEvent.setup();
    render(<PresetsMenu />);
    await user.click(screen.getByRole('button', { name: /Presets/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Photo import' }));
    expect(vi.mocked(commands.applyPreset)).toHaveBeenCalledWith(preset.id);
  });

  it('saves the current rules under a typed name', async () => {
    const user = userEvent.setup();
    render(<PresetsMenu />);
    await user.click(screen.getByRole('button', { name: /Presets/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Save Current Rules as Preset…' }));
    await user.type(screen.getByRole('textbox', { name: 'Preset name' }), 'Weekly cleanup');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(vi.mocked(commands.savePreset)).toHaveBeenCalledWith('Weekly cleanup', 'replace');
  });

  it('offers Replace / Keep Both on a name collision', async () => {
    const user = userEvent.setup();
    vi.mocked(commands.presetNameExists).mockResolvedValue(true);
    render(<PresetsMenu />);
    await user.click(screen.getByRole('button', { name: /Presets/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Save Current Rules as Preset…' }));
    await user.type(screen.getByRole('textbox', { name: 'Preset name' }), 'Photo import');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(
      await screen.findByRole('dialog', { name: 'Preset name exists' }),
    ).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Keep Both' }));
    expect(vi.mocked(commands.savePreset)).toHaveBeenCalledWith('Photo import', 'keepBoth');
  });

  it('deleting confirms — presets have no undo', async () => {
    const user = userEvent.setup();
    render(<PresetsMenu />);
    await user.click(screen.getByRole('button', { name: /Presets/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Delete “Photo import”…' }));
    const confirm = useUiStore.getState().confirm;
    expect(confirm?.title).toBe('Delete “Photo import”?');
    expect(confirm?.message).toBe('Presets have no undo.');
    confirm?.onConfirm();
    expect(vi.mocked(commands.deletePreset)).toHaveBeenCalledWith(preset.id);
  });

  it('imports and exports through the dialogs', async () => {
    const user = userEvent.setup();
    render(<PresetsMenu />);
    await user.click(screen.getByRole('button', { name: /Presets/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Import Presets…' }));
    expect(vi.mocked(commands.importPresets)).toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: /Presets/ }));
    await user.click(screen.getByRole('menuitem', { name: 'Export Presets…' }));
    expect(vi.mocked(commands.exportPresets)).toHaveBeenCalled();
  });
});
