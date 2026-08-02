import { describe, expect, it } from 'vitest';
import { absoluteDate, relativeDate } from './format';
import { hasPrimaryModifier, metadataSectionTitle, revealLabel, shortcutLabel } from './platform';
import { strings } from './strings';

describe('format helpers', () => {
  const now = new Date('2026-08-01T12:00:00Z');
  it('relative dates for history rows', () => {
    expect(relativeDate('2026-08-01T11:00:00Z', now)).toBe('1 hour ago');
    expect(relativeDate('2026-07-30T12:00:00Z', now)).toBe('2 days ago');
    expect(relativeDate('2026-08-01T11:59:40Z', now)).toBe('just now');
    expect(relativeDate('2026-06-20T12:00:00Z', now)).toBe('last month');
  });
  it('absolute date renders locale text', () => {
    expect(absoluteDate('2026-08-01T11:00:00Z').length).toBeGreaterThan(0);
  });
});

describe('platform helpers (§15 per-OS labels)', () => {
  it('shortcut labels', () => {
    // jsdom is not a Mac.
    expect(shortcutLabel({ mod: true, key: 'O' })).toBe('Ctrl+O');
    expect(shortcutLabel({ mod: true, shift: true, key: 'Z' })).toBe('Ctrl+Shift+Z');
    expect(shortcutLabel({ mod: true, key: 'Enter' })).toBe('Ctrl+Enter');
  });
  it('reveal + metadata section per OS', () => {
    expect(revealLabel('macos')).toBe('Reveal in Finder');
    expect(revealLabel('windows')).toBe('Show in Explorer');
    expect(revealLabel('linux')).toBe('Show in File Manager');
    expect(metadataSectionTitle('macos')).toBe('Spotlight Metadata');
    expect(metadataSectionTitle('windows')).toBe('File Properties');
    expect(metadataSectionTitle('linux')).toBe('Metadata');
  });
  it('primary modifier check', () => {
    expect(hasPrimaryModifier({ metaKey: false, ctrlKey: true })).toBe(true);
    expect(hasPrimaryModifier({ metaKey: true, ctrlKey: false })).toBe(false);
  });
});

describe('§A copy builders', () => {
  it('revert confirmation composes real counts', () => {
    expect(strings.revertConfirmMessage(3, 3, 1, 0)).toBe(
      '3 files will be renamed back on disk.',
    );
    expect(strings.revertConfirmMessage(1, 4, 2, 1)).toBe(
      '1 file will be renamed back on disk. (4 renames across 2 versions.) This also undoes the 1 newer version.',
    );
  });
  it('stop-watching guard includes discarded edits only when present', () => {
    expect(strings.stopWatchingMessage(12, 0)).toBe(
      'Its 12 files leave the list. Files on disk aren’t changed.',
    );
    expect(strings.stopWatchingMessage(1, 2)).toBe(
      'Its 1 file leave the list, and 2 manual edits are discarded. Files on disk aren’t changed.',
    );
  });
  it('bulk-removal guard mentions edits only when affected', () => {
    expect(strings.bulkRemoveMessage(0)).toBe('Files on disk aren’t changed.');
    expect(strings.bulkRemoveMessage(1)).toBe(
      '1 manual edit will be discarded. Files on disk aren’t changed.',
    );
  });
  it('primary button copy at zero and with counts', () => {
    expect(strings.renameButton(0, false)).toBe('Rename Files');
    expect(strings.renameButton(1, false)).toBe('Rename 1 File');
    expect(strings.renameButton(2, true)).toBe('Rename 2 Folders');
  });
  it('csv copy', () => {
    expect(strings.csvDownload(3, false)).toBe('Download Template CSV (3 files)…');
    expect(strings.csvWillChange(2, 5)).toBe('2 of 5 names will change.');
    expect(strings.csvApplyChanges(null)).toBe('Apply Changes');
    expect(strings.csvApplyChanges(2)).toBe('Apply 2 Changes');
    expect(strings.csvRowsDidntMatch(1)).toBe('1 row didn’t match');
  });
  it('counts and tooltips', () => {
    expect(strings.includedCount(18, 20)).toBe('18 of 20 included');
    expect(strings.conflictCount(1)).toBe('1 naming conflict');
    expect(strings.conflictCount(2)).toBe('2 naming conflicts');
    expect(strings.renameTooltipEnabled(3, false)).toBe('Rename 3 files on disk (⌘↩)');
    expect(strings.renameTooltipConflicts(2)).toBe(
      'Fix or skip the 2 naming conflicts — see the warnings in the list',
    );
    expect(strings.affectsCount(14, 20)).toBe('affects 14 of 20');
  });
});
