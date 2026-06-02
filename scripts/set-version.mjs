#!/usr/bin/env node
import fs from 'node:fs';

const PACKAGE_JSON = 'package.json';
const TAURI_CONFIG = 'src-tauri/tauri.conf.json';
const CARGO_TOML = 'src-tauri/Cargo.toml';
const CARGO_LOCK = 'src-tauri/Cargo.lock';

const input = process.argv[2];

if (!input) {
  usage();
  process.exit(1);
}

const currentVersion = JSON.parse(fs.readFileSync(PACKAGE_JSON, 'utf8')).version;
const nextVersion = resolveVersion(input, currentVersion);

updateJson(PACKAGE_JSON, (pkg) => {
  pkg.version = nextVersion;
});

updateJson(TAURI_CONFIG, (config) => {
  config.version = nextVersion;
});

updateCargoToml(CARGO_TOML, nextVersion);
updateCargoLock(CARGO_LOCK, nextVersion);

console.log(`Updated atuin-bar version: ${currentVersion} -> ${nextVersion}`);
console.log(`Release tag will be: v${nextVersion}`);

function usage() {
  console.error(`Usage: pnpm set-version <version|major|minor|patch>

Examples:
  pnpm set-version 1.2.3
  pnpm set-version patch
  pnpm set-version minor
  pnpm set-version major`);
}

function resolveVersion(value, current) {
  const normalized = value.replace(/^v/, '');
  if (/^\d+\.\d+\.\d+$/.test(normalized)) {
    return normalized;
  }

  if (!['major', 'minor', 'patch'].includes(value)) {
    throw new Error(`Invalid version '${value}'. Use x.y.z, vx.y.z, major, minor, or patch.`);
  }

  const match = current.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    throw new Error(`Current version '${current}' is not a plain semantic version.`);
  }

  let [, major, minor, patch] = match.map(Number);
  switch (value) {
    case 'major':
      major += 1;
      minor = 0;
      patch = 0;
      break;
    case 'minor':
      minor += 1;
      patch = 0;
      break;
    case 'patch':
      patch += 1;
      break;
  }

  return `${major}.${minor}.${patch}`;
}

function updateJson(path, mutate) {
  const data = JSON.parse(fs.readFileSync(path, 'utf8'));
  mutate(data);
  fs.writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
}

function updateCargoToml(path, version) {
  const content = fs.readFileSync(path, 'utf8');
  const pattern = /(\[package\][\s\S]*?^version\s*=\s*")([^"]+)(")/m;
  if (!pattern.test(content)) {
    throw new Error(`Could not find [package] version in ${path}`);
  }

  const updated = content.replace(pattern, `$1${version}$3`);
  fs.writeFileSync(path, updated);
}

function updateCargoLock(path, version) {
  const content = fs.readFileSync(path, 'utf8');
  const pattern = /(^name\s*=\s*"atuin-bar"\n^version\s*=\s*")([^"]+)(")/m;
  if (!pattern.test(content)) {
    throw new Error(`Could not find atuin-bar package entry in ${path}`);
  }

  const updated = content.replace(pattern, `$1${version}$3`);
  fs.writeFileSync(path, updated);
}
