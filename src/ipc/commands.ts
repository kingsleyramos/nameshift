// Typed invoke wrappers — one per §12.1 command. The payload types are
// generated from the Rust structs by ts-rs (src/ipc/gen); never hand-edit
// those.

import { invoke } from '@tauri-apps/api/core';
import type { CsvMatch } from './gen/CsvMatch';
import type { CsvMatchReport } from './gen/CsvMatchReport';
import type { FileSortKey } from './gen/FileSortKey';
import type { InspectorPayload } from './gen/InspectorPayload';
import type { ListMode } from './gen/ListMode';
import type { PlatformInfo } from './gen/PlatformInfo';
import type { PreviewPayload } from './gen/PreviewPayload';
import type { RenameRule } from './gen/RenameRule';
import type { RevertPreviewPayload } from './gen/RevertPreviewPayload';
import type { SnapshotMeta } from './gen/SnapshotMeta';
import type { StateSnapshot } from './gen/StateSnapshot';

// ---- state-reading ---------------------------------------------------------

export const getState = () => invoke<StateSnapshot>('get_state');
export const getPreview = (knownVersion?: number) =>
  invoke<PreviewPayload>('get_preview', { knownVersion: knownVersion ?? null });
export const getRevertPreview = () => invoke<RevertPreviewPayload>('get_revert_preview');
export const getHistory = () => invoke<SnapshotMeta[]>('get_history');
export const getFileMetadata = (id: string) =>
  invoke<InspectorPayload>('get_file_metadata', { id });
export const ruleDerivedName = (id: string) => invoke<string>('rule_derived_name', { id });
export const getPlatform = () => invoke<PlatformInfo>('get_platform');

// ---- import / list ---------------------------------------------------------

export const importPaths = (paths: string[]) => invoke<void>('import_paths', { paths });
export const pickAndImport = () => invoke<void>('pick_and_import');
export const pickAndImportFolders = () => invoke<void>('pick_and_import_folders');
export const removeItems = (ids: string[]) => invoke<void>('remove_items', { ids });
export const removeWatchedFolder = (id: string) => invoke<void>('remove_watched_folder', { id });
export const clearAll = () => invoke<void>('clear_all');
export const rescanWatchedFolders = () => invoke<void>('rescan_watched_folders');

// ---- inclusion (checkboxes) ------------------------------------------------

export const setSelected = (id: string, selected: boolean) =>
  invoke<void>('set_selected', { id, selected });
export const setSelectedMany = (ids: string[], selected: boolean) =>
  invoke<void>('set_selected_many', { ids, selected });
export const setAllSelected = (selected: boolean) =>
  invoke<void>('set_all_selected', { selected });
export const invertSelection = () => invoke<void>('invert_selection');
export const selectOnly = (ids: string[]) => invoke<void>('select_only', { ids });
export const deselectConflicted = () => invoke<void>('deselect_conflicted');

// ---- rules -----------------------------------------------------------------

export const setRules = (rules: RenameRule[]) => invoke<void>('set_rules', { rules });
export const replaceRulesUndoable = (
  rules: RenameRule[],
  trimsWhitespace: boolean,
  actionName: string,
) => invoke<void>('replace_rules_undoable', { rules, trimsWhitespace, actionName });
export const undo = () => invoke<void>('undo');
export const redo = () => invoke<void>('redo');
export const setTrimsWhitespace = (trims: boolean) =>
  invoke<void>('set_trims_whitespace', { trims });

// ---- options ---------------------------------------------------------------

export const setAutoResolve = (enabled: boolean) => invoke<void>('set_auto_resolve', { enabled });
export const setKeepRules = (enabled: boolean) => invoke<void>('set_keep_rules', { enabled });
export const setIncludeSubfolders = (enabled: boolean) =>
  invoke<void>('set_include_subfolders', { enabled });
export const setWatchedFolderSubfolders = (id: string, include: boolean | null) =>
  invoke<void>('set_watched_folder_subfolders', { id, include });
export const setSort = (key: FileSortKey, ascending: boolean) =>
  invoke<void>('set_sort', { key, ascending });
export const setListMode = (mode: ListMode) => invoke<void>('set_list_mode', { mode });

// ---- overrides -------------------------------------------------------------

export const setOverride = (id: string, name: string | null) =>
  invoke<void>('set_override', { id, name });
export const clearAllOverrides = () => invoke<void>('clear_all_overrides');

// ---- apply / revert --------------------------------------------------------

export const apply = () => invoke<void>('apply');
export const selectSnapshot = (id: string | null) => invoke<void>('select_snapshot', { id });
export const revertSelected = () => invoke<void>('revert_selected');
export const cancelProcessing = () => invoke<void>('cancel_processing');
export const clearHistory = () => invoke<void>('clear_history');

// ---- presets ---------------------------------------------------------------

export const presetNameExists = (name: string) =>
  invoke<boolean>('preset_name_exists', { name });
export const savePreset = (name: string, resolution: 'replace' | 'keepBoth') =>
  invoke<void>('save_preset', { name, resolution });
export const applyPreset = (id: string) => invoke<void>('apply_preset', { id });
export const deletePreset = (id: string) => invoke<void>('delete_preset', { id });
export const importPresets = () => invoke<void>('import_presets');
export const exportPresets = () => invoke<void>('export_presets');

// ---- spreadsheet -----------------------------------------------------------

export const exportCsvTemplate = () => invoke<void>('export_csv_template');
export const csvDryRun = (path: string) => invoke<CsvMatchReport>('csv_dry_run', { path });
export const csvApply = (matches: CsvMatch[]) => invoke<void>('csv_apply', { matches });
export const copyPreviewTsv = () => invoke<void>('copy_preview_tsv');

// ---- misc ------------------------------------------------------------------

export const revealInFileManager = (id: string) =>
  invoke<void>('reveal_in_file_manager', { id });
export const openHelp = (topic?: string) => invoke<void>('open_help', { topic: topic ?? null });
export const saveSessionNow = () => invoke<void>('save_session_now');
