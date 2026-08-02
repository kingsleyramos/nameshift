// The Settings window (§14.11) — the discoverable home; in-context mirrors
// share the same state underneath.

import { useAppStore } from '../../state/appState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function SettingsWindow() {
  const snapshot = useAppStore((s) => s.snapshot);
  if (!snapshot) return null;

  return (
    <main className="h-full overflow-auto px-6 py-4" style={{ background: 'var(--window-bg)' }}>
      <section>
        <h2 className="text-[13px] font-semibold" style={{ color: 'var(--text-primary)' }}>
          {strings.settingsRenaming}
        </h2>
        <Toggle
          label={strings.settingsTrim}
          checked={snapshot.trimsWhitespace}
          onChange={(value) => {
            void commands.setTrimsWhitespace(value);
          }}
        />
        <Toggle
          label={strings.settingsAutoResolve}
          checked={snapshot.autoResolvesConflicts}
          onChange={(value) => {
            void commands.setAutoResolve(value);
          }}
        />
        <Toggle
          label={strings.settingsKeepRules}
          help={strings.settingsKeepRulesHelp}
          checked={snapshot.keepRulesAfterApply}
          onChange={(value) => {
            void commands.setKeepRules(value);
          }}
        />
      </section>
      <section className="mt-4">
        <h2 className="text-[13px] font-semibold" style={{ color: 'var(--text-primary)' }}>
          {strings.settingsFolders}
        </h2>
        <Toggle
          label={strings.settingsIncludeSubfolders}
          checked={snapshot.includeSubfolders}
          onChange={(value) => {
            void commands.setIncludeSubfolders(value);
          }}
        />
      </section>
    </main>
  );
}

function Toggle({
  label,
  help,
  checked,
  onChange,
}: {
  label: string;
  help?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="mt-2 block text-[13px]" style={{ color: 'var(--text-primary)' }}>
      <span className="flex items-center gap-2">
        <input
          type="checkbox"
          checked={checked}
          onChange={(event) => {
            onChange(event.target.checked);
          }}
          className="h-3.5 w-3.5 accent-[var(--accent)]"
        />
        {label}
      </span>
      {help != null && (
        <span className="mt-0.5 block pl-5 text-[12px]" style={{ color: 'var(--text-secondary)' }}>
          {help}
        </span>
      )}
    </label>
  );
}
