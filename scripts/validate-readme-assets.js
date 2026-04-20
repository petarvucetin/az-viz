#!/usr/bin/env node
// Validates that all image assets referenced by README.md exist on disk,
// and that the version badge matches tauri.conf.json.

const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const readmePath = path.join(repoRoot, 'README.md');
const tauriConfPath = path.join(repoRoot, 'src-tauri', 'tauri.conf.json');

const readme = fs.readFileSync(readmePath, 'utf8');

let errors = 0;

// 1. Verify every local src="..." path exists.
const srcRe = /src="([^"]+)"/g;
for (const m of readme.matchAll(srcRe)) {
  const src = m[1];
  if (src.startsWith('http://') || src.startsWith('https://')) continue;
  const abs = path.join(repoRoot, src);
  if (!fs.existsSync(abs)) {
    console.error(`[readme] missing asset: ${src}`);
    errors++;
  }
}

// 2. Verify the version badge matches tauri.conf.json.
const conf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
const version = conf.package && conf.package.version;
if (!version) {
  console.error('[readme] could not read version from tauri.conf.json');
  errors++;
} else {
  const badgeRe = /shields\.io\/badge\/version-([^-]+)-/;
  const m = readme.match(badgeRe);
  if (!m) {
    console.error('[readme] version badge not found');
    errors++;
  } else if (m[1] !== version) {
    console.error(`[readme] version badge mismatch: badge=${m[1]} tauri.conf=${version}`);
    errors++;
  }
}

if (errors > 0) {
  console.error(`[readme] ${errors} error(s)`);
  process.exit(1);
}
console.log('[readme] OK');
