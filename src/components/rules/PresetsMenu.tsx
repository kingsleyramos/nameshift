// The presets menu (§14.8): load (undoable), save with Replace/Keep Both,
// manage (confirmed deletes — presets have no undo), import/export.

import { useState } from 'react';
import Menu, { MenuItem, MenuSeparator } from '../shell/Menu';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function PresetsMenu() {
  const presets = useAppStore((s) => s.snapshot?.presets ?? []);
  const rules = useAppStore((s) => s.snapshot?.rules ?? []);
  const showConfirm = useUiStore((s) => s.showConfirm);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState('');
  const [collision, setCollision] = useState<string | null>(null);

  const savePreset = async () => {
    const trimmed = name.trim();
    if (trimmed.length === 0) return;
    const exists = await commands.presetNameExists(trimmed);
    setNaming(false);
    setName('');
    if (exists) {
      // On collision offer Replace / Keep Both (§14.8).
      setCollision(trimmed);
    } else {
      void commands.savePreset(trimmed, 'replace');
    }
  };

  return (
    <>
      <Menu
        ariaLabel="Presets"
        trigger={(_, toggle) => (
          <button
            onClick={toggle}
            className="h-6 rounded-md border px-2 text-[12px] hover:bg-[var(--wash-hover)]"
            style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
          >
            {strings.presets} ▾
          </button>
        )}
      >
        {(close) => (
          <>
            {presets.map((preset) => (
              <MenuItem
                key={preset.id}
                label={preset.name}
                onSelect={() => {
                  close();
                  void commands.applyPreset(preset.id);
                }}
              />
            ))}
            {presets.length > 0 && <MenuSeparator />}
            <MenuItem
              label="Save Current Rules as Preset…"
              disabled={rules.length === 0}
              onSelect={() => {
                close();
                setNaming(true);
              }}
            />
            {presets.map((preset) => (
              <MenuItem
                key={`delete-${preset.id}`}
                label={`Delete “${preset.name}”…`}
                destructive
                onSelect={() => {
                  close();
                  showConfirm({
                    title: strings.deletePresetTitle(preset.name),
                    message: strings.deletePresetMessage,
                    confirmLabel: strings.deletePresetButton,
                    destructive: true,
                    onConfirm: () => {
                      void commands.deletePreset(preset.id);
                    },
                  });
                }}
              />
            ))}
            <MenuSeparator />
            <MenuItem
              label="Import Presets…"
              onSelect={() => {
                close();
                void commands.importPresets();
              }}
            />
            <MenuItem
              label="Export Presets…"
              disabled={presets.length === 0}
              onSelect={() => {
                close();
                void commands.exportPresets();
              }}
            />
          </>
        )}
      </Menu>
      {collision != null && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label="Preset name exists"
          className="fixed inset-0 z-50 flex items-center justify-center"
          style={{ background: 'rgb(0 0 0 / 0.25)' }}
        >
          <div
            className="w-96 p-4"
            style={{ background: 'var(--pane-bg)', borderRadius: 'var(--radius-overlay)' }}
          >
            <h2 className="text-[14px] font-semibold" style={{ color: 'var(--text-primary)' }}>
              A preset named “{collision}” already exists
            </h2>
            <div className="mt-4 flex justify-end gap-2">
              <button
                onClick={() => {
                  setCollision(null);
                }}
                className="h-7 rounded-md border px-3 text-[13px]"
                style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
              >
                {strings.cancel}
              </button>
              <button
                onClick={() => {
                  void commands.savePreset(collision, 'keepBoth');
                  setCollision(null);
                }}
                className="h-7 rounded-md border px-3 text-[13px]"
                style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
              >
                Keep Both
              </button>
              <button
                onClick={() => {
                  void commands.savePreset(collision, 'replace');
                  setCollision(null);
                }}
                className="h-7 rounded-md px-3 text-[13px] font-semibold text-white"
                style={{ background: 'var(--accent)' }}
              >
                Replace
              </button>
            </div>
          </div>
        </div>
      )}
      {naming && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label="Save preset"
          className="fixed inset-0 z-50 flex items-center justify-center"
          style={{ background: 'rgb(0 0 0 / 0.25)' }}
        >
          <div
            className="w-80 p-4"
            style={{ background: 'var(--pane-bg)', borderRadius: 'var(--radius-overlay)' }}
          >
            <h2 className="text-[14px] font-semibold" style={{ color: 'var(--text-primary)' }}>
              Save Current Rules as Preset
            </h2>
            <input
              autoFocus
              value={name}
              onChange={(event) => {
                setName(event.target.value);
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter') void savePreset();
                if (event.key === 'Escape') setNaming(false);
              }}
              placeholder="Preset name"
              aria-label="Preset name"
              className="mt-3 w-full rounded-md border px-2 py-1 text-[13px] outline-none focus:border-[var(--accent)]"
              style={{
                borderColor: 'var(--separator)',
                background: 'var(--pane-bg)',
                color: 'var(--text-primary)',
              }}
            />
            <div className="mt-3 flex justify-end gap-2">
              <button
                onClick={() => {
                  setNaming(false);
                }}
                className="h-7 rounded-md border px-3 text-[13px]"
                style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
              >
                {strings.cancel}
              </button>
              <button
                onClick={() => {
                  void savePreset();
                }}
                disabled={name.trim().length === 0}
                className="h-7 rounded-md px-3 text-[13px] font-semibold text-white disabled:opacity-40"
                style={{ background: 'var(--accent)' }}
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
