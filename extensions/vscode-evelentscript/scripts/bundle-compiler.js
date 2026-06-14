'use strict';

const fs = require('fs');
const path = require('path');

const extRoot = path.join(__dirname, '..');
const repoRoot = path.join(extRoot, '..', '..');
const src = path.join(repoRoot, 'lib', 'evelentscript');
const dest = path.join(extRoot, 'bundled', 'evelentscript');

function copyDir(from, to) {
  fs.mkdirSync(to, { recursive: true });
  for (const entry of fs.readdirSync(from, { withFileTypes: true })) {
    const srcPath = path.join(from, entry.name);
    const destPath = path.join(to, entry.name);
    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

if (!fs.existsSync(src)) {
  console.error('Run "npm run build" in the repo root before packaging the extension.');
  process.exit(1);
}

fs.rmSync(path.join(extRoot, 'bundled'), { recursive: true, force: true });
copyDir(src, dest);
console.log('Bundled EvelentScript compiler into extensions/vscode-evelentscript/bundled/');
