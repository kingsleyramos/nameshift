// E2E helpers: fixture trees on the runner's disk, plus the invoke bridge
// into the app (withGlobalTauri).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { browser } from '@wdio/globals';

declare global {
  interface Window {
    __TAURI__: {
      core: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
    };
  }
}

/** Create a scratch directory of files; returns its absolute path. */
export function makeFixtureDir(names: string[]): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'nameshift-e2e-'));
  for (const name of names) {
    fs.writeFileSync(path.join(dir, name), name);
  }
  return dir;
}

export function listing(dir: string): string[] {
  return fs.readdirSync(dir).sort();
}

export function cleanup(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

/** Invoke a §12.1 command inside the app. */
export async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  return browser.execute(
    (command, commandArgs) => window.__TAURI__.core.invoke(command, commandArgs),
    cmd,
    args,
  );
}

/** Import loose files through the one import gate. */
export async function importFiles(dir: string, names: string[]): Promise<void> {
  await invoke('import_paths', { paths: names.map((name) => path.join(dir, name)) });
}

/** Wait until the preview shows the expected number of rows. */
export async function waitForRows(count: number): Promise<void> {
  await browser.waitUntil(
    async () => {
      const rows = await browser.$$('[data-row-id]').getElements();
      return rows.length === count;
    },
    { timeoutMsg: `expected ${count} rows` },
  );
}

/** Replace the whole rule stack (idempotent editing path). */
export async function setRules(rules: Record<string, unknown>[]): Promise<void> {
  const withDefaults = rules.map((rule) => ({
    id: crypto.randomUUID().toUpperCase(),
    isEnabled: true,
    includesExtension: false,
    caseSensitive: true,
    text: '',
    replacement: '',
    caseStyle: 'lowercase',
    numberPosition: 'after',
    numberStart: 1,
    numberPadding: 3,
    stripsDiacritics: false,
    restartPerFolder: false,
    removesEmoji: false,
    ...rule,
  }));
  await invoke('set_rules', { rules: withDefaults });
}

/** Clear the workspace between specs. */
/**
 * Click a button until the confirm dialog is actually open. A re-render can
 * swap the button's DOM node mid-click (e.g. the revert preview arriving),
 * silently swallowing it — retrying until the dialog exists removes the race.
 */
export async function clickForDialog(selector: string): Promise<void> {
  await browser.waitUntil(
    async () => {
      if (await browser.$('[role="dialog"]').isExisting()) return true;
      const btn = await browser.$(selector);
      if ((await btn.isExisting()) && (await btn.isEnabled())) {
        await btn.click();
      }
      return await browser.$('[role="dialog"]').isExisting();
    },
    { timeoutMsg: `dialog never opened after clicking ${selector}` },
  );
}

/** Dump visible buttons + dialog state — diagnostic for selector failures. */
export async function dumpUi(tag: string): Promise<void> {
  const dump = await browser.execute(() => ({
    buttons: Array.from(document.querySelectorAll('button'))
      .map((b) => b.textContent?.trim() ?? '')
      .filter(Boolean),
    dialog: document.querySelector('[role="dialog"]')?.outerHTML.slice(0, 500) ?? 'NO DIALOG',
  }));
  console.log(`E2E-UI-DUMP ${tag}`, JSON.stringify(dump));
}

export async function resetWorkspace(): Promise<void> {
  await invoke('set_rules', { rules: [] });
  await invoke('clear_all');
  await invoke('clear_history');
  // clear_all keeps view prefs; tests must not leak Folders mode forward.
  await invoke('set_list_mode', { mode: 'Files' });
  // Reload the page so frontend-only ui state (active tab, row selection,
  // open dialogs) can't leak between tests — each spec starts at launch state.
  await browser.refresh();
  await browser.waitUntil(async () => (await browser.$$('#root > *').getElements()).length > 0, {
    timeoutMsg: 'app re-rendered after reset',
  });
}
