// The interface pass: the real production bundle, driven in a real browser
// engine, with the backend stood in for. Runs on every OS in both Chromium
// and WebKit.

import { test, expect, type Page } from '@playwright/test';
import { installFakeBackend } from './fake-backend';
// Copy comes from the one catalog the app renders from, so rewording a
// string updates the app and these tests together. A test here fails only
// when BEHAVIOUR changes.
import { strings } from '../src/lib/strings';

async function open(page: Page): Promise<void> {
  await page.addInitScript(installFakeBackend);
  await page.goto('/');
}

/** Invoke a command the way the app does, to arrange state for a test. */
async function invoke(page: Page, cmd: string, args?: Record<string, unknown>) {
  await page.evaluate(
    ([command, commandArgs]: [string, Record<string, unknown> | undefined]) =>
      (window as unknown as { __TAURI__: { core: { invoke: (c: string, a?: unknown) => Promise<unknown> } } })
        .__TAURI__.core.invoke(command, commandArgs),
    [cmd, args] as [string, Record<string, unknown> | undefined],
  );
}

async function importFiles(page: Page, names: string[]): Promise<void> {
  await invoke(page, 'import_paths', { paths: names.map((name) => `/tmp/demo/${name}`) });
}

test.describe('interface', () => {
  // The regression that matters most: a render loop leaves #root empty and
  // the window blank. Every prior test seeded state, so none could see it.
  test('mounts and renders the shell from the launch state', async ({ page }) => {
    const crashes: string[] = [];
    page.on('pageerror', (error) => crashes.push(error.message));

    await open(page);

    await expect(page.getByRole('button', { name: strings.optionsButton })).toBeVisible();
    await expect(page.getByRole('tab', { name: strings.rulesTab(0) })).toBeVisible();
    await expect(page.getByText(strings.dropPrompt)).toBeVisible();
    expect(crashes).toEqual([]);
  });

  test('lists imported files and previews the renamed result', async ({ page }) => {
    await open(page);
    await importFiles(page, ['one.txt', 'two.txt']);

    const rows = page.locator('[data-row-id]');
    await expect(rows).toHaveCount(2);

    await invoke(page, 'set_rules', {
      rules: [{ kind: 'addPrefix', text: 'x-', isEnabled: true }],
    });

    // The preview shows the new names; nothing has been applied.
    await expect(page.getByTestId('file-list')).toContainText('x-one.txt');
    await expect(page.getByTestId('file-list')).toContainText('x-two.txt');
  });

  test('flags naming conflicts and Skip Conflicted clears the block', async ({ page }) => {
    await open(page);
    await importFiles(page, ['a.txt', 'b.txt']);
    // Both names collapse onto one target.
    await invoke(page, 'set_rules', {
      rules: [{ kind: 'template', text: 'same', isEnabled: true }],
    });

    const actionBar = page.getByTestId('action-bar');
    await expect(actionBar).toContainText(strings.conflictCount(2));

    await page.getByRole('button', { name: strings.skipConflicted }).click();
    await expect(actionBar).not.toContainText(strings.conflictCount(2));
  });

  test('the filter changes visibility without changing inclusion', async ({ page }) => {
    await open(page);
    await importFiles(page, ['report.txt', 'photo.jpg']);
    await expect(page.locator('[data-row-id]')).toHaveCount(2);

    await page.getByRole('searchbox', { name: strings.filterFilesLabel }).fill('photo');
    await expect(page.locator('[data-row-id]')).toHaveCount(1);

    // Filtering hides rows; it never unchecks them.
    await expect(page.getByTestId('action-bar')).toContainText(strings.includedCount(2, 2));
  });

  test('scope tabs report the file and folder counts', async ({ page }) => {
    await open(page);
    await importFiles(page, ['one.txt']);
    await expect(page.getByRole('tab', { name: strings.filesTab(1) })).toBeVisible();
  });
});
