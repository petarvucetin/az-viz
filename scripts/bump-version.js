#!/usr/bin/env node

const { execFileSync } = require('child_process');
const fs = require('fs');

function git(...args) {
  return execFileSync('git', args, { encoding: 'utf8' });
}
function gitSafe(...args) {
  try {
    return execFileSync('git', args, { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] });
  } catch {
    return null;
  }
}

const hasHead = gitSafe('rev-parse', '--verify', 'HEAD') !== null;

if (hasHead) {
  const stagedDiff = git('diff', '--cached').trim();
  if (stagedDiff === '') {
    console.log('[bump-version] No staged changes (amend/no-op); skipping.');
    process.exit(0);
  }
}

const cargoVersionRe = /^version\s*=\s*"([^"]+)"/m;

const stagedCargo = hasHead ? gitSafe('show', ':Cargo.toml') : fs.readFileSync('Cargo.toml', 'utf8');
const stagedMatch = stagedCargo && stagedCargo.match(cargoVersionRe);
if (!stagedMatch) {
  console.error('[bump-version] Could not find version in staged Cargo.toml.');
  process.exit(1);
}
const stagedVersion = stagedMatch[1];

let headVersion = null;
if (hasHead) {
  const headCargo = gitSafe('show', 'HEAD:Cargo.toml');
  const m = headCargo && headCargo.match(cargoVersionRe);
  headVersion = m ? m[1] : null;
}

let newVersion;
if (headVersion !== null && stagedVersion !== headVersion) {
  console.log(`[bump-version] Manual version change detected (${headVersion} -> ${stagedVersion}); syncing sibling files.`);
  newVersion = stagedVersion;
} else {
  const parts = stagedVersion.split('.');
  if (parts.length !== 3 || parts.some(p => !/^\d+$/.test(p))) {
    console.error(`[bump-version] Expected semver X.Y.Z, got "${stagedVersion}".`);
    process.exit(1);
  }
  parts[2] = String(Number(parts[2]) + 1);
  newVersion = parts.join('.');
  console.log(`[bump-version] Auto-bumping patch: ${stagedVersion} -> ${newVersion}`);
}

const cargoPath = 'Cargo.toml';
const tauriConfPath = 'src-tauri/tauri.conf.json';
const uiPkgPath = 'ui/package.json';

{
  const s = fs.readFileSync(cargoPath, 'utf8');
  fs.writeFileSync(cargoPath, s.replace(cargoVersionRe, `version = "${newVersion}"`));
}
{
  const s = fs.readFileSync(tauriConfPath, 'utf8');
  fs.writeFileSync(
    tauriConfPath,
    s.replace(/("package"\s*:\s*\{[^}]*?"version"\s*:\s*)"[^"]+"/, `$1"${newVersion}"`),
  );
}
{
  const s = fs.readFileSync(uiPkgPath, 'utf8');
  fs.writeFileSync(uiPkgPath, s.replace(/(^  "version"\s*:\s*)"[^"]+"/m, `$1"${newVersion}"`));
}

git('add', cargoPath, tauriConfPath, uiPkgPath);
console.log(`[bump-version] Version set to ${newVersion}.`);
