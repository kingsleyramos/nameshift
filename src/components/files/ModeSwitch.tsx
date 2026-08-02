// The first-class scope tabs (§14.2): Files · N | Folders · N. They change
// what Apply does — never styled like the filters.

import { useAppStore } from '../../state/appState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function ModeSwitch() {
  const snapshot = useAppStore((s) => s.snapshot);
  if (!snapshot) return null;
  const fileCount = snapshot.files.filter((f) => !f.isDirectory).length;
  const folderCount = snapshot.files.filter((f) => f.isDirectory).length;
  const mode = snapshot.listMode;

  const tab = (id: 'Files' | 'Folders', label: string) => (
    <button
      role="tab"
      aria-selected={mode === id}
      onClick={() => {
        void commands.setListMode(id);
      }}
      className="rounded-md px-2.5 py-1 text-[12px] font-medium"
      style={
        mode === id
          ? { background: 'var(--accent)', color: '#fff' }
          : { color: 'var(--text-secondary)' }
      }
    >
      {label}
    </button>
  );

  return (
    <div role="tablist" aria-label="Scope" className="flex items-center gap-1">
      {tab('Files', strings.filesTab(fileCount))}
      {tab('Folders', strings.foldersTab(folderCount))}
    </div>
  );
}
