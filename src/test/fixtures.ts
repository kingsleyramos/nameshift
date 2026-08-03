// Shared test fixtures: builders for the IPC payload shapes.

import type { FileItem } from '../ipc/gen/FileItem';
import type { PreviewEntry } from '../ipc/gen/PreviewEntry';
import type { PreviewPayload } from '../ipc/gen/PreviewPayload';
import type { RenameRule } from '../ipc/gen/RenameRule';
import type { StateSnapshot } from '../ipc/gen/StateSnapshot';

let counter = 0;
export function nextId(): string {
  counter += 1;
  return `00000000-0000-4000-8000-${String(counter).padStart(12, '0')}`;
}

export function makeRule(partial: Partial<RenameRule> = {}): RenameRule {
  return {
    id: nextId(),
    kind: 'addPrefix',
    isEnabled: true,
    includesExtension: false,
    caseSensitive: true,
    text: 'x-',
    replacement: '',
    caseStyle: 'lowercase',
    numberPosition: 'after',
    numberStart: 1,
    numberPadding: 3,
    stripsDiacritics: false,
    restartPerFolder: false,
    removesEmoji: false,
    ...partial,
  };
}

export function makeItem(partial: Partial<FileItem> = {}): FileItem {
  const id = partial.id ?? nextId();
  return {
    id,
    path: `/tmp/demo/file-${id.slice(-4)}.txt`,
    folderId: null,
    isSelected: true,
    isDirectory: false,
    overrideName: null,
    ...partial,
  };
}

export function makeSnapshot(partial: Partial<StateSnapshot> = {}): StateSnapshot {
  return {
    files: [],
    rules: [],
    watchedFolders: [],
    presets: [],
    trimsWhitespace: false,
    autoResolvesConflicts: false,
    keepRulesAfterApply: true,
    includeSubfolders: false,
    sortKey: 'Order Added',
    sortAscending: true,
    listMode: 'Files',
    selectedSnapshotId: null,
    rulesClearedByApply: false,
    version: 1,
    rulesRevision: 0,
    isProcessing: false,
    overrideCount: 0,
    historyCount: 0,
    undoAction: null,
    redoAction: null,
    ...partial,
  };
}

export function makeEntry(partial: Partial<PreviewEntry> = {}): PreviewEntry {
  const id = partial.id ?? nextId();
  return {
    id,
    currentName: `file-${id.slice(-4)}.txt`,
    newName: `file-${id.slice(-4)}.txt`,
    isSelected: true,
    hasOverride: false,
    isChanged: false,
    problem: null,
    ...partial,
  };
}

export function makePreview(
  entries: PreviewEntry[],
  partial: Partial<Omit<PreviewPayload, 'entries'>> = {},
): PreviewPayload {
  const changeCount = entries.filter((e) => e.isChanged).length;
  const conflictCount = entries.filter((e) => e.problem != null).length;
  return {
    version: 1,
    entries,
    ruleImpact: [],
    counts: {
      selectedCount: entries.filter((e) => e.isSelected).length,
      changeCount,
      conflictCount,
      canApply: changeCount > 0 && conflictCount === 0,
    },
    applyDisabledReason: null,
    ...partial,
  };
}
