// Version sync check (§20.6): the single source of truth is the root
// package.json; tauri.conf.json and the Cargo workspace must match.
// Run: node scripts/check-versions.mjs

import fs from 'node:fs';

const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8')).version;
const tauri = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8')).version;
const cargo = /\[workspace\.package\][^[]*?version\s*=\s*"([^"]+)"/s.exec(
  fs.readFileSync('Cargo.toml', 'utf8'),
)?.[1];

const versions = { 'package.json': pkg, 'src-tauri/tauri.conf.json': tauri, 'Cargo.toml': cargo };
const distinct = new Set(Object.values(versions));
if (distinct.size !== 1) {
  console.error('version mismatch:', versions);
  process.exit(1);
}
console.log(`versions in sync: ${pkg}`);
