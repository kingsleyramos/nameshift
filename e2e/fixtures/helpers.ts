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
    { timeoutMsg: `expected ${count} rows`, timeout: 8000 },
  ).catch(async (err: unknown) => {
    // DIAGNOSTIC: why doesn't React mount? Capture scripts + any load/JS error.
    const diag = await browser.execute(() => {
      const scripts = Array.from(document.querySelectorAll('script')).map((s) => ({
        src: (s as HTMLScriptElement).src,
        type: (s as HTMLScriptElement).type,
      }));
      return {
        rootLen: document.querySelector('#root')?.innerHTML.length ?? -1,
        headHtml: document.head.innerHTML.slice(0, 600),
        scripts,
        // @ts-expect-error diagnostic global
        capturedError: String(window.__e2eError ?? 'none'),
      };
    });
    let logs: unknown = 'unavailable';
    try {
      logs = await browser.getLogs('browser');
    } catch {
      /* driver may not support logs */
    }
    console.log('E2E-DIAG', JSON.stringify(diag));
    console.log('E2E-LOGS', JSON.stringify(logs).slice(0, 1500));
    throw err;
  });
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
export async function resetWorkspace(): Promise<void> {
  await invoke('set_rules', { rules: [] });
  await invoke('clear_all');
  await invoke('clear_history');
}
