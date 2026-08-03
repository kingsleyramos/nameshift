// UI-side user-facing copy (§21.6): one greppable module, §A verbatim.
// The term is always “naming conflict”, never bare “conflict”.

export const plural = (count: number, singular: string, pluralForm?: string): string =>
  count === 1 ? singular : (pluralForm ?? `${singular}s`);

export const strings = {
  appName: 'Name Shift',

  // Action bar & chips (§A)
  includedCount: (n: number, m: number) => `${n} of ${m} included`,
  willChangeCount: (n: number) => `${n} will change`,
  conflictCount: (n: number) => `${n} naming ${plural(n, 'conflict')}`,
  editedCount: (n: number) => `${n} edited`,
  autoResolveChip: (on: boolean) => `Auto-resolve: ${on ? 'On' : 'Off'}`,
  skipConflicted: 'Skip Conflicted',
  skipConflictedTooltip: 'Uncheck the conflicted files so the rest can be renamed',
  showingOf: (shown: number, total: number) => `Showing ${shown} of ${total}`,
  matchingMenu: (n: number) => `${n} matching`,
  selectOnlyThese: 'Select Only These',
  removeTheseFromList: 'Remove These from List',

  // Primary action (§A)
  renameButton: (n: number, isFolders: boolean) =>
    n === 0 ? `Rename ${isFolders ? 'Folders' : 'Files'}` : `Rename ${n} ${isFolders ? plural(n, 'Folder') : plural(n, 'File')}`,
  renameTooltipEnabled: (n: number, isFolders: boolean) =>
    `Rename ${n} ${isFolders ? plural(n, 'folder') : plural(n, 'file')} on disk (⌘↩)`,
  renameTooltipConflicts: (n: number) =>
    `Fix or skip the ${n} naming ${plural(n, 'conflict')} — see the warnings in the list`,
  renameTooltipEmpty: (isFolders: boolean) =>
    `Nothing to rename yet — add a rule that changes at least one included ${isFolders ? 'folder' : 'file'}`,
  revertButton: 'Revert…',

  // Banner (§A)
  bannerRenamed: (n: number, isFolders: boolean) =>
    `Renamed ${n} ${isFolders ? plural(n, 'folder') : plural(n, 'file')}`,
  bannerRulesCleared: '· Rules cleared (⌘Z restores them)',
  bannerRulesKept: '· Kept rules; renamed files deselected',

  // Scope & rows (§14.2)
  filesTab: (n: number) => `Files · ${n}`,
  foldersTab: (n: number) => `Folders · ${n}`,
  skippedPill: 'Skipped',
  watchingLabel: 'Watching:',
  currentNameHeader: 'Current Name',
  newNameHeader: 'New Name',
  noFilesMatchFilter: 'No files match the filter',
  clearFilter: 'Clear Filter',
  dropPrompt: 'Drag files or folders here',
  dropHint: (shortcut: string) => `or press ${shortcut} to add them`,

  // Rules panel (§A, §14.3)
  rulesTab: (n: number) => (n > 0 ? `Rules (${n})` : 'Rules'),
  historyTab: (n: number) => (n > 0 ? `History (${n})` : 'History'),
  addRule: '＋ Add Rule',
  addRuleGhost: '＋ Add Rule…',
  afterAllRules: 'AFTER ALL RULES',
  trimSpacesFromNames: 'Trim spaces from names',
  presets: 'Presets',
  tokens: '{ } Tokens',
  clearAllRules: 'Clear All Rules',
  affectsCount: (a: number, e: number) => `affects ${a} of ${e}`,
  noInputYet: 'No input yet — rule is skipped',
  invalidRegex: 'This pattern isn’t a valid regular expression.',
  syntaxReference: 'Syntax reference',
  skippedForFolders: 'Skipped for folders',
  folderNamesHaveNoExtension: 'Folder names have no extension',
  includeExtension: 'Include extension',
  matchCase: 'Match case',
  noRulesYet: 'No rules yet',
  noRulesHint: 'Add a rule to start renaming, or load a preset.',
  seeExampleRecipes: 'See Example Recipes',
  rulesAppliedAndCleared: 'Rules applied and cleared',
  rulesAppliedHint:
    'The renames are saved in History. Press ⌘Z to bring the rules back, or turn on “Keep rules after applying” in Settings.',

  // History / revert (§A)
  revertBannerSummary: (files: number, renames: number, versions: number) =>
    `This will restore ${files} ${plural(files, 'file')} (${renames} ${plural(renames, 'rename')} across ${versions} ${plural(versions, 'version')})`,
  revertStatusOk: 'Will be restored',
  revertStatusMissing: 'Not found (nothing to restore)',
  revertStatusMissingTooltip:
    'It may have been moved or deleted since this version was applied. This rename will be skipped.',
  revertStatusNameTaken: 'The original name is taken by a different file — kept as is.',
  clearHistory: 'Clear History…',
  renamesCount: (n: number) => `${n} ${plural(n, 'rename')}`,

  // Confirmations (§A — real counts in every message)
  revertConfirmTitle: 'Revert these renames?',
  revertConfirmMessage: (n: number, renames: number, versions: number, newer: number) => {
    let message = `${n} ${plural(n, 'file')} will be renamed back on disk.`;
    if (renames !== n) {
      message += ` (${renames} ${plural(renames, 'rename')} across ${versions} ${plural(versions, 'version')}.)`;
    }
    if (newer > 0) {
      message += ` This also undoes the ${newer} newer ${plural(newer, 'version')}.`;
    }
    return message;
  },
  revertConfirmButton: 'Revert',
  clearListTitle: 'Clear the file list?',
  clearListMessage:
    'This empties the list and stops watching any folders. Files on disk aren’t changed.',
  clearListButton: 'Clear List',
  stopWatchingTitle: (folder: string) => `Stop watching “${folder}”?`,
  stopWatchingMessage: (n: number, edits: number) => {
    const editsPart =
      edits > 0 ? `, and ${edits} manual ${plural(edits, 'edit')} ${edits === 1 ? 'is' : 'are'} discarded` : '';
    return `Its ${n} ${plural(n, 'file')} leave the list${editsPart}. Files on disk aren’t changed.`;
  },
  stopWatchingButton: 'Stop Watching',
  bulkRemoveTitle: (n: number, isFolders: boolean) =>
    `Remove ${n} ${isFolders ? 'folders' : 'files'} from the list?`,
  bulkRemoveMessage: (edits: number) =>
    `${edits > 0 ? `${edits} manual ${plural(edits, 'edit')} will be discarded. ` : ''}Files on disk aren’t changed.`,
  bulkRemoveButton: 'Remove',
  clearHistoryTitle: 'Clear rename history?',
  clearHistoryMessage:
    'Past rename batches can no longer be previewed or reverted. Files on disk aren’t changed.',
  clearHistoryButton: 'Clear History',
  clearRulesTitle: 'Clear all rules?',
  clearRulesMessage: 'Every rule leaves the stack. ⌘Z brings them back.',
  clearRulesButton: 'Clear Rules',
  deletePresetTitle: (name: string) => `Delete “${name}”?`,
  deletePresetMessage: 'Presets have no undo.',
  deletePresetButton: 'Delete',
  cancel: 'Cancel',

  // Options menu (§14.1) — four titled sections, destructive last
  optionsImporting: 'Importing',
  optionsRenaming: 'Renaming',
  optionsSpreadsheet: 'Spreadsheet',
  optionsList: 'List',
  includeSubfolders: 'Include Subfolders',
  autoResolveNamingConflicts: 'Auto-Resolve Naming Conflicts',
  keepRulesAfterApplying: 'Keep Rules After Applying',
  trimSpacesOption: 'Trim Spaces from Names',
  renameByCsv: 'Rename by CSV…',
  clearAllManualEdits: (n: number) => `Clear All Manual Edits (${n})`,
  removeSkippedFromList: 'Remove Skipped from List',
  removeIncludedFromList: 'Remove Included from List',
  clearFileList: 'Clear File List…',

  // CSV modal (§14.7 structure, §A strings)
  csvTitle: 'Rename by CSV',
  csvSubtitle: 'Edit new names in Excel, Numbers, or Google Sheets, then bring the file back here.',
  csvDownload: (n: number, isFolders: boolean) =>
    `Download Template CSV (${n} ${isFolders ? plural(n, 'folder') : plural(n, 'file')})…`,
  csvDropHere: 'Drop the edited CSV here',
  csvChoose: 'Choose CSV…',
  csvWillChange: (a: number, n: number) => `${a} of ${n} names will change.`,
  csvRowsDidntMatch: (c: number) => `${c} ${plural(c, 'row')} didn’t match`,
  csvMissNotInList: 'Not in the list',
  csvMissDuplicate: 'Duplicate rows',
  csvMissUnreadable: 'Couldn’t read',
  csvApplyChanges: (a: number | null) => (a === null ? 'Apply Changes' : `Apply ${a} Changes`),
  csvZeroMatch: 'No rows matched — was this CSV exported from a different list?',
  csvReadError: 'Couldn’t read that file.',
  csvHeaderError: 'Its first row must be the exported header “Current Name, New Name”.',
  csvOneFilePlease: 'One CSV file, please',

  // Inspector (§14.10)
  inspectorEmpty: 'Click a file to see its info.',
  inspectorGeneral: 'General',
  inspectorKind: 'Kind',
  inspectorSize: 'Size',
  inspectorCreated: 'Created',
  inspectorModified: 'Modified',
  inspectorFolder: 'Folder',
  metadataSectionMac: 'Spotlight Metadata',
  metadataSectionWindows: 'File Properties',
  metadataSectionLinux: 'Metadata',

  // Settings (§14.11)
  settingsRenaming: 'Renaming',
  settingsFolders: 'Folders',
  settingsTrim: 'Trim leading and trailing spaces',
  settingsAutoResolve: 'Auto-resolve naming conflicts',
  settingsKeepRules: 'Keep rules after applying',
  settingsKeepRulesHelp:
    'Instead of clearing the rules after Apply, keep them for the next batch. The files you just renamed are unchecked, so re-applying only affects new files.',
  settingsIncludeSubfolders: 'Include subfolders when watching a folder',

  // Misc
  softCapWarning: 'Name Shift works best under 50,000 files; the preview may be slow.',
  processingRefused: 'Renaming is in progress. Try again when it finishes.',
  editNewName: 'Edit New Name…',
  clearManualEdit: 'Clear Manual Edit',
  copyName: 'Copy Name',
  copyNewName: 'Copy New Name',
  copyPath: 'Copy Path',
  removeFromList: 'Remove from List',
} as const;
