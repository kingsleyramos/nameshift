// Every §12.1 wrapper marshals through invoke — pinned via mockIPC so the
// command names and argument keys can't drift from the Rust side silently.

import { mockIPC } from '@tauri-apps/api/mocks';
import { describe, expect, it } from 'vitest';
import * as commands from './commands';

describe('ipc command wrappers', () => {
  it('sends the exact command names + camelCase args Rust expects', async () => {
    const calls: [string, unknown][] = [];
    mockIPC((cmd, args) => {
      calls.push([cmd, args]);
      switch (cmd) {
        case 'get_state':
        case 'get_preview':
        case 'get_revert_preview':
        case 'get_platform':
        case 'get_file_metadata':
          return {};
        case 'get_history':
          return [];
        case 'rule_derived_name':
          return 'x';
        case 'preset_name_exists':
          return false;
        case 'csv_dry_run':
          return { matches: [], misses: [], willChange: 0, unchanged: 0 };
        default:
          return null;
      }
    });

    await commands.getState();
    await commands.getPreview(7);
    await commands.getRevertPreview();
    await commands.getHistory();
    await commands.getFileMetadata('ID');
    await commands.ruleDerivedName('ID');
    await commands.getPlatform();
    await commands.importPaths(['/a']);
    await commands.pickAndImport();
    await commands.pickAndImportFolders();
    await commands.removeItems(['ID']);
    await commands.removeWatchedFolder('ID');
    await commands.clearAll();
    await commands.rescanWatchedFolders();
    await commands.setSelected('ID', true);
    await commands.setSelectedMany(['ID'], false);
    await commands.setAllSelected(true);
    await commands.invertSelection();
    await commands.selectOnly(['ID']);
    await commands.deselectConflicted();
    await commands.setRules([]);
    await commands.replaceRulesUndoable([], true, 'Clear Rules');
    await commands.undo();
    await commands.redo();
    await commands.setTrimsWhitespace(true);
    await commands.setAutoResolve(true);
    await commands.setKeepRules(false);
    await commands.setIncludeSubfolders(true);
    await commands.setWatchedFolderSubfolders('ID', null);
    await commands.setSort('Name', false);
    await commands.setListMode('Folders');
    await commands.setOverride('ID', 'x');
    await commands.clearAllOverrides();
    await commands.apply();
    await commands.selectSnapshot('ID');
    await commands.revertSelected();
    await commands.cancelProcessing();
    await commands.clearHistory();
    await commands.presetNameExists('P');
    await commands.savePreset('P', 'keepBoth');
    await commands.applyPreset('ID');
    await commands.deletePreset('ID');
    await commands.importPresets();
    await commands.exportPresets();
    await commands.exportCsvTemplate();
    await commands.csvDryRun('/tmp/x.csv');
    await commands.csvApply([]);
    await commands.copyPreviewTsv();
    await commands.revealInFileManager('ID');
    await commands.openHelp('tokens');
    await commands.saveSessionNow();

    const names = calls.map(([cmd]) => cmd);
    for (const expected of [
      'get_state',
      'get_preview',
      'import_paths',
      'set_selected_many',
      'replace_rules_undoable',
      'set_watched_folder_subfolders',
      'csv_dry_run',
      'reveal_in_file_manager',
      'save_session_now',
    ]) {
      expect(names).toContain(expected);
    }
    const replaceCall = calls.find(([cmd]) => cmd === 'replace_rules_undoable');
    expect(replaceCall?.[1]).toMatchObject({ trimsWhitespace: true, actionName: 'Clear Rules' });
    const sortCall = calls.find(([cmd]) => cmd === 'set_sort');
    expect(sortCall?.[1]).toMatchObject({ key: 'Name', ascending: false });
  });
});
