// The sort menu (§14.2): mirrors the header sort and never disagrees with
// it. Extension is hidden in Folders mode (§7.2).

import { ArrowDownUp } from 'lucide-react';
import Menu, { MenuItem, MenuSeparator } from '../shell/Menu';
import type { FileSortKey } from '../../ipc/gen/FileSortKey';
import { useAppStore } from '../../state/appState';
import * as commands from '../../ipc/commands';

const LABELS: [FileSortKey, string][] = [
  ['Order Added', 'Order Added'],
  ['Name', 'Name'],
  ['Extension', 'Extension'],
  ['Folder', 'Folder'],
  ['Date Created', 'Date Created'],
  ['Date Modified', 'Date Modified'],
];

export default function SortMenu() {
  const snapshot = useAppStore((s) => s.snapshot);
  if (!snapshot) return null;
  const isFolders = snapshot.listMode === 'Folders';

  return (
    <Menu
      align="right"
      ariaLabel="Sort"
      trigger={(_, toggle) => (
        <button
          onClick={toggle}
          title="Sort"
          className="flex h-6 items-center gap-1 rounded-md border px-2 text-[12px] hover:bg-[var(--wash-hover)]"
          style={{ borderColor: 'var(--separator)', color: 'var(--text-primary)' }}
        >
          <ArrowDownUp size={12} aria-hidden />
          {LABELS.find(([key]) => key === snapshot.sortKey)?.[1] ?? 'Sort'}
        </button>
      )}
    >
      {(close) => (
        <>
          {LABELS.filter(([key]) => !(isFolders && key === 'Extension')).map(([key, label]) => (
            <MenuItem
              key={key}
              label={label}
              checked={snapshot.sortKey === key}
              onSelect={() => {
                close();
                void commands.setSort(key, snapshot.sortAscending);
              }}
            />
          ))}
          <MenuSeparator />
          <MenuItem
            label="Ascending"
            checked={snapshot.sortAscending}
            onSelect={() => {
              close();
              void commands.setSort(snapshot.sortKey, true);
            }}
          />
          <MenuItem
            label="Descending"
            checked={!snapshot.sortAscending}
            onSelect={() => {
              close();
              void commands.setSort(snapshot.sortKey, false);
            }}
          />
        </>
      )}
    </Menu>
  );
}
