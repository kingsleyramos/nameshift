// WebdriverIO + tauri-driver (§18.4/§18.5): Linux & Windows CI only —
// tauri-driver has no macOS support (docs/MANUAL_TESTING.md covers macOS).

import { spawn, type ChildProcess } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
// Cargo workspace: the binary builds to the workspace-root target dir, not
// src-tauri/target.
const binary =
  process.platform === 'win32'
    ? path.resolve(here, '../target/release/nameshift.exe')
    : path.resolve(here, '../target/release/nameshift');

let tauriDriver: ChildProcess | undefined;

export const config: WebdriverIO.Config = {
  specs: ['./specs/**/*.e2e.ts'],
  maxInstances: 1,
  hostname: '127.0.0.1',
  port: 4444,
  capabilities: [
    {
      // @ts-expect-error tauri-specific capability
      'tauri:options': {
        application: binary,
      },
      browserName: 'wry',
      // WebdriverIO 9 negotiates WebDriver BiDi by default, but the
      // WebKitWebDriver/Edge driver tauri-driver proxies to is classic-only
      // and rejects the session ("Failed to match capabilities"). Force the
      // classic protocol.
      'wdio:enforceWebDriverClassic': true,
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 120_000 },
  waitforTimeout: 15_000,
  connectionRetryCount: 3,

  beforeSession: () => {
    const args: string[] = [];
    // Windows runners preinstall the matching Edge WebDriver.
    const edge = process.env.EDGEWEBDRIVER;
    if (process.platform === 'win32' && edge) {
      args.push('--native-driver', path.join(edge, 'msedgedriver.exe'));
    }
    tauriDriver = spawn(path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'), args, {
      stdio: [null, process.stdout, process.stderr],
    });
  },
  afterSession: () => {
    tauriDriver?.kill();
  },
};
