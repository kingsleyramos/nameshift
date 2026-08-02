// Watched-folder chips (§14.2): folder name + depth menu (§9) + undoable ✕
// with the §14.0 guard.

import Menu, { MenuItem } from '../shell/Menu';
import { useAppStore } from '../../state/appState';
import { useUiStore } from '../../state/uiState';
import { strings } from '../../lib/strings';
import * as commands from '../../ipc/commands';

export default function WatchedFolderChips() {
  const snapshot = useAppStore((s) => s.snapshot);
  const showConfirm = useUiStore((s) => s.showConfirm);
  if (!snapshot || snapshot.watchedFolders.length === 0) return null;

  const stopWatching = (id: string) => {
    const folder = snapshot.watchedFolders.find((f) => f.id === id);
    if (!folder) return;
    const items = snapshot.files.filter((f) => f.folderId === id);
    const edits = items.filter((f) => f.overrideName != null).length;
    const name = folder.path.split('/').pop() ?? folder.path;
    // §14.0 destructive policy: guard only when > 10 items or edits exist.
    if (items.length > 10 || edits > 0) {
      showConfirm({
        title: strings.stopWatchingTitle(name),
        message: strings.stopWatchingMessage(items.length, edits),
        confirmLabel: strings.stopWatchingButton,
        destructive: true,
        onConfirm: () => {
          void commands.removeWatchedFolder(id);
        },
      });
    } else {
      void commands.removeWatchedFolder(id);
    }
  };

  return (
    <div className="flex min-w-0 items-center gap-1.5 overflow-x-auto">
      <span className="shrink-0 text-[12px]" style={{ color: 'var(--text-secondary)' }}>
        {strings.watchingLabel}
      </span>
      {snapshot.watchedFolders.map((folder) => {
        const name = folder.path.split('/').pop() ?? folder.path;
        const depth = folder.includeSubfolders;
        return (
          <span
            key={folder.id}
            className="flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5 text-[12px]"
            style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
            title={folder.path}
          >
            <Menu
              ariaLabel={`Watch depth for ${name}`}
              trigger={(_, toggle) => (
                <button onClick={toggle} className="hover:underline">
                  {name}
                </button>
              )}
            >
              {(close) => (
                <>
                  <MenuItem
                    label="Follow Global"
                    checked={depth == null}
                    onSelect={() => {
                      close();
                      void commands.setWatchedFolderSubfolders(folder.id, null);
                    }}
                  />
                  <MenuItem
                    label="Include Subfolders"
                    checked={depth === true}
                    onSelect={() => {
                      close();
                      void commands.setWatchedFolderSubfolders(folder.id, true);
                    }}
                  />
                  <MenuItem
                    label="This Folder Only"
                    checked={depth === false}
                    onSelect={() => {
                      close();
                      void commands.setWatchedFolderSubfolders(folder.id, false);
                    }}
                  />
                </>
              )}
            </Menu>
            <button
              aria-label={`Stop watching ${name}`}
              onClick={() => {
                stopWatching(folder.id);
              }}
              className="flex h-4 w-4 items-center justify-center rounded-full hover:bg-[var(--wash-hover)]"
              style={{ color: 'var(--text-secondary)' }}
            >
              ✕
            </button>
          </span>
        );
      })}
    </div>
  );
}
