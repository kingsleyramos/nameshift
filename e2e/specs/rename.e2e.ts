// The §18.5 end-to-end pass: import → rules → preview → Apply → REAL disk
// → revert → restored; conflict blocking + Skip Conflicted; CSV round-trip;
// Folders mode; session restore.

import fs from 'node:fs';
import path from 'node:path';
import { browser, $, expect } from '@wdio/globals';
import {
  cleanup,
  importFiles,
  invoke,
  listing,
  makeFixtureDir,
  setRules,
  resetWorkspace,
  waitForRows,
} from '../fixtures/helpers';

describe('rename end-to-end', () => {
  afterEach(async () => {
    await resetWorkspace();
  });

  it('imports, previews, applies to disk, and reverts', async () => {
    const dir = makeFixtureDir(['one.txt', 'two.txt']);
    try {
      await importFiles(dir, ['one.txt', 'two.txt']);
      await waitForRows(2);

      await setRules([{ kind: 'addPrefix', text: 'x-' }]);
      // The preview shows the new names before anything touches disk.
      await browser.waitUntil(async () =>
        (await $('[data-testid="file-list"]').getText()).includes('x-one.txt'),
      );
      expect(listing(dir)).toEqual(['one.txt', 'two.txt']);

      const rename = await $('button*=Rename 2 Files');
      await rename.click();
      await browser.waitUntil(() => listing(dir).includes('x-one.txt'), {
        timeoutMsg: 'apply reached the real filesystem',
      });
      expect(listing(dir)).toEqual(['x-one.txt', 'x-two.txt']);

      // Revert restores the original names byte-for-byte.
      const historyTab = await $('button*=History');
      await historyTab.click();
      const version = await $('button*=Prefix “x-”');
      await version.click();
      const revert = await $('button*=Revert…');
      await revert.click();
      const confirm = await $('button=Revert');
      await confirm.click();
      await browser.waitUntil(() => listing(dir).includes('one.txt'));
      expect(listing(dir)).toEqual(['one.txt', 'two.txt']);
    } finally {
      cleanup(dir);
    }
  });

  it('blocks on naming conflicts and Skip Conflicted unblocks', async () => {
    const dir = makeFixtureDir(['a.txt', 'b.txt']);
    try {
      await importFiles(dir, ['a.txt', 'b.txt']);
      await waitForRows(2);
      // Both rename to the same target → duplicate targets block Apply.
      await setRules([{ kind: 'template', text: 'same' }]);
      await browser.waitUntil(async () =>
        (await $('footer').getText()).includes('naming conflict'),
      );
      const skip = await $('button=Skip Conflicted');
      await skip.click();
      // Skipping both leaves nothing to rename; disk is untouched.
      expect(listing(dir)).toEqual(['a.txt', 'b.txt']);
    } finally {
      cleanup(dir);
    }
  });

  it('round-trips the CSV template as manual edits', async () => {
    const dir = makeFixtureDir(['photo.jpg']);
    try {
      await importFiles(dir, ['photo.jpg']);
      await waitForRows(1);
      // Dry-run a CSV mapping and apply it as one undoable batch.
      const csv = path.join(dir, 'mapping.csv');
      fs.writeFileSync(csv, 'Current Name,New Name\nphoto.jpg,beach day.jpg\n');
      const report = (await invoke('csv_dry_run', { path: csv })) as {
        matches: { id: string; newName: string; changes: boolean }[];
        willChange: number;
      };
      expect(report.willChange).toBe(1);
      await invoke('csv_apply', { matches: report.matches });
      const rename = await $('button*=Rename 1 File');
      await rename.click();
      await browser.waitUntil(() => listing(dir).includes('beach day.jpg'));
    } finally {
      cleanup(dir);
    }
  });

  it('renames folders in Folders mode and reverts', async () => {
    const dir = makeFixtureDir([]);
    try {
      fs.mkdirSync(path.join(dir, 'Shoot'));
      fs.writeFileSync(path.join(dir, 'Shoot/img.jpg'), 'x');
      await invoke('import_paths', { paths: [path.join(dir, 'Shoot')] });
      await browser.waitUntil(async () =>
        (await $('[role="tablist"][aria-label="Scope"]').getText()).includes('Folders · 1'),
      );
      await invoke('set_list_mode', { mode: 'Folders' });
      await setRules([{ kind: 'addPrefix', text: '2026 ' }]);
      const rename = await $('button*=Rename 1 Folder');
      await rename.click();
      await browser.waitUntil(() => listing(dir).includes('2026 Shoot'));
      expect(fs.existsSync(path.join(dir, '2026 Shoot/img.jpg'))).toBe(true);

      const historyTab = await $('button*=History');
      await historyTab.click();
      const version = await $('button*=Folders');
      await version.click();
      await (await $('button*=Revert…')).click();
      await (await $('button=Revert')).click();
      await browser.waitUntil(() => listing(dir).includes('Shoot'));
    } finally {
      cleanup(dir);
    }
  });

  it('restores the session across a relaunch', async () => {
    const dir = makeFixtureDir(['keep.txt']);
    try {
      await importFiles(dir, ['keep.txt']);
      await waitForRows(1);
      await setRules([{ kind: 'addSuffix', text: '-v2' }]);
      await invoke('save_session_now');

      await browser.reloadSession();
      await browser.waitUntil(async () =>
        (await $('[data-testid="file-list"]').getText()).includes('keep.txt'),
      );
      const rulesText = await $('[data-testid="rules-stack"]').getText();
      expect(rulesText).toContain('Add Suffix');
    } finally {
      cleanup(dir);
    }
  });
});
