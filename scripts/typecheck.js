#!/usr/bin/env node

/**
 * Type-check EvelentScript sources using the TypeScript compiler.
 *
 * Compiles .es files to JavaScript, collects interface/type declarations,
 * merges them into a .ts file, and runs `tsc --noEmit`.
 *
 * Usage:
 *   node scripts/typecheck.js path/to/file.es
 *   node scripts/typecheck.js src/
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const os = require('os');

const ROOT = path.resolve(__dirname, '..');
const EvelentScript = require(path.join(ROOT, 'lib/evelentscript'));
const {
  collectTypeDeclarations,
  emitTypeScriptDeclarations,
} = require(path.join(ROOT, 'lib/evelentscript/typesyntax'));

function collectFiles(target) {
  const abs = path.resolve(target);
  if (!fs.existsSync(abs)) {
    throw new Error(`Path not found: ${target}`);
  }
  if (fs.statSync(abs).isFile()) {
    return abs.endsWith('.es') || abs.endsWith('.lites') ? [abs] : [];
  }
  const files = [];
  for (const entry of fs.readdirSync(abs, { withFileTypes: true })) {
    if (entry.isDirectory() && entry.name !== 'node_modules') {
      files.push(...collectFiles(path.join(abs, entry.name)));
    } else if (/\.(es|lites)$/.test(entry.name)) {
      files.push(path.join(abs, entry.name));
    }
  }
  return files;
}

function compileToTypeScript(file) {
  const source = fs.readFileSync(file, 'utf8');
  const result = EvelentScript.compile(source, {
    filename: file,
    bare: true,
    header: false,
    sourceMap: false,
  });
  const js = result.js || result;
  const root = EvelentScript.nodes(source, { filename: file });
  const declarations = collectTypeDeclarations(root.body || root);
  const header = emitTypeScriptDeclarations(declarations);
  const body = js.replace(/^\/\/[^\n]*\n/, '');
  return `${header ? `${header}\n\n` : ''}${body}`;
}

function main() {
  const targets = process.argv.slice(2);
  if (!targets.length) {
    console.error('Usage: node scripts/typecheck.js <file-or-directory...>');
    process.exit(1);
  }

  const files = targets.flatMap(collectFiles);
  if (!files.length) {
    console.error('No .es files found.');
    process.exit(1);
  }

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'evelent-types-'));
  const tsconfig = {
    compilerOptions: {
      target: 'ES2020',
      module: 'CommonJS',
      strict: true,
      noEmit: true,
      skipLibCheck: true,
      esModuleInterop: true,
    },
    include: ['**/*.ts'],
  };
  fs.writeFileSync(path.join(tmpDir, 'tsconfig.json'), JSON.stringify(tsconfig, null, 2));

  for (const file of files) {
    const ts = compileToTypeScript(file);
    const out = path.join(tmpDir, path.basename(file).replace(/\.(es|lites)$/, '.ts'));
    fs.writeFileSync(out, ts);
    console.log(`→ ${path.relative(ROOT, file)}`);
  }

  const tsc = require.resolve('typescript/bin/tsc', { paths: [ROOT] });
  const result = spawnSync(process.execPath, [tsc, '-p', tmpDir], {
    stdio: 'inherit',
  });

  process.exit(result.status || 0);
}

main();
