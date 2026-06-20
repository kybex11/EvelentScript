#!/usr/bin/env node
'use strict';

/**
 * Post-compile protection pipeline for EvelentScript projects.
 *
 *   1. Compiles .es / .lites sources to .js (optional).
 *   2. Obfuscates the generated .js with javascript-obfuscator (string
 *      encryption, control-flow flattening, dead code, self-defending, ...).
 *   3. Optionally protects native binaries (.dll / .exe) with VMProtect.
 *
 * IMPORTANT: VMProtect only protects native machine code (PE/ELF). It cannot
 * protect JavaScript. JS is protected by obfuscation; VMProtect is applied only
 * to native targets you explicitly list in protect.config.json.
 *
 * Usage:
 *   node scripts/protect.js                       # uses protect.config.json
 *   node scripts/protect.js --in src --out dist   # override paths
 *   node scripts/protect.js --preset balanced     # balanced | max
 *   node scripts/protect.js --no-compile          # inputs are already .js
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');

function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--no-compile') args.compile = false;
    else if (a === '--in') args.input = argv[++i];
    else if (a === '--out') args.output = argv[++i];
    else if (a === '--preset') args.preset = argv[++i];
    else if (a === '--config') args.configPath = argv[++i];
  }
  return args;
}

function loadConfig(cliArgs) {
  const configPath = path.resolve(ROOT, cliArgs.configPath || 'protect.config.json');
  let fileConfig = {};
  if (fs.existsSync(configPath)) {
    try {
      fileConfig = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    } catch (error) {
      fail(`Cannot parse ${configPath}: ${error.message}`);
    }
  }
  return {
    input: cliArgs.input || fileConfig.input || 'src',
    output: cliArgs.output || fileConfig.output || 'dist',
    compile: cliArgs.compile != null ? cliArgs.compile : fileConfig.compile !== false,
    preset: cliArgs.preset || fileConfig.preset || 'max',
    obfuscate: fileConfig.obfuscate || {},
    vmprotect: fileConfig.vmprotect || { enabled: false },
  };
}

function fail(message) {
  console.error(`\x1b[31m[protect] ERROR:\x1b[0m ${message}`);
  process.exit(1);
}

function log(message) {
  console.log(`\x1b[36m[protect]\x1b[0m ${message}`);
}

function walk(dir, exts, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name.startsWith('.')) continue;
      walk(full, exts, out);
    } else if (exts.some((ext) => entry.name.endsWith(ext))) {
      out.push(full);
    }
  }
  return out;
}

function isEsFile(file) {
  return /\.(lites|es\.md|es)$/.test(file);
}

function outPathFor(file, inputRoot, outputRoot, asJs) {
  const rel = path.relative(inputRoot, file);
  let target = path.join(outputRoot, rel);
  if (asJs) {
    target = target.replace(/\.(lites|es\.md|es)$/, '.js');
  }
  return target;
}

// ---- Step 1: compile .es -> .js -------------------------------------------

function compileSources(config, inputRoot, files) {
  let EvelentScript;
  try {
    EvelentScript = require(path.join(ROOT, 'lib/evelentscript'));
  } catch (error) {
    fail(`Cannot load EvelentScript compiler: ${error.message}`);
  }
  const helpers = EvelentScript.helpers;
  const produced = [];

  for (const file of files) {
    const source = fs.readFileSync(file, 'utf8');
    const literate = helpers?.isLiterate?.(file) ?? false;
    let js;
    try {
      const result = EvelentScript.compile(source, {
        filename: file,
        literate,
        header: false,
        sourceMap: false,
      });
      js = typeof result === 'string' ? result : result.js;
    } catch (error) {
      fail(`Compile failed for ${file}: ${error.message}`);
    }
    const target = outPathFor(file, inputRoot, config.output, true);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, js, 'utf8');
    produced.push(target);
    log(`compiled  ${path.relative(ROOT, file)} -> ${path.relative(ROOT, target)}`);
  }
  return produced;
}

// ---- Step 2: obfuscate .js -------------------------------------------------

function obfuscatorOptions(config) {
  const base = {
    compact: true,
    simplify: true,
    target: 'node',
    identifierNamesGenerator: 'hexadecimal',
    numbersToExpressions: true,
    transformObjectKeys: true,
    splitStrings: true,
    splitStringsChunkLength: 8,
    stringArray: true,
    stringArrayThreshold: 1,
    stringArrayEncoding: ['base64'],
    controlFlowFlattening: true,
    controlFlowFlatteningThreshold: 0.6,
    deadCodeInjection: false,
    selfDefending: false,
    // domainLock is browser-only and breaks Node/alt:V — never enable here.
    debugProtection: false,
    disableConsoleOutput: false,
  };

  if (config.preset === 'max') {
    Object.assign(base, {
      controlFlowFlatteningThreshold: 1,
      deadCodeInjection: true,
      deadCodeInjectionThreshold: 0.4,
      stringArrayEncoding: ['rc4'],
      stringArrayCallsTransform: true,
      stringArrayWrappersCount: 3,
      stringArrayWrappersType: 'function',
      splitStringsChunkLength: 5,
      selfDefending: true,
    });
  }

  // Caller overrides (e.g. enable debugProtection at your own risk).
  return Object.assign(base, config.obfuscate || {});
}

function obfuscateFiles(config, files) {
  let JavaScriptObfuscator;
  try {
    JavaScriptObfuscator = require('javascript-obfuscator');
  } catch (error) {
    fail(
      'javascript-obfuscator is not installed. Run:\n' +
        '    npm install --save-dev javascript-obfuscator'
    );
  }
  const options = obfuscatorOptions(config);
  for (const file of files) {
    const code = fs.readFileSync(file, 'utf8');
    let result;
    try {
      result = JavaScriptObfuscator.obfuscate(code, options).getObfuscatedCode();
    } catch (error) {
      fail(`Obfuscation failed for ${file}: ${error.message}`);
    }
    fs.writeFileSync(file, result, 'utf8');
    log(`obfuscated ${path.relative(ROOT, file)}`);
  }
}

// ---- Step 3: VMProtect native binaries ------------------------------------

function runVmProtect(config) {
  const vmp = config.vmprotect || {};
  if (!vmp.enabled) {
    log('VMProtect: disabled (no native targets). JS is protected by obfuscation.');
    return;
  }
  if (!vmp.cli || !fs.existsSync(vmp.cli)) {
    fail(
      `VMProtect enabled but CLI not found at "${vmp.cli}". ` +
        'Set "vmprotect.cli" to VMProtect_con.exe.'
    );
  }
  const targets = vmp.targets || [];
  if (!targets.length) {
    log('VMProtect: enabled but no "targets" listed — nothing to protect.');
    return;
  }
  for (const target of targets) {
    const input = path.resolve(ROOT, target);
    if (!fs.existsSync(input)) {
      fail(`VMProtect target not found: ${input}`);
    }
    const cliArgs = [input, input];
    if (vmp.project) cliArgs.push(path.resolve(ROOT, vmp.project));
    log(`VMProtect protecting ${path.relative(ROOT, input)} ...`);
    const res = spawnSync(vmp.cli, cliArgs, { stdio: 'inherit' });
    if (res.status !== 0) {
      fail(`VMProtect failed for ${input} (exit ${res.status}).`);
    }
    log(`VMProtect done: ${path.relative(ROOT, input)}`);
  }
}

// ---- Main ------------------------------------------------------------------

function main() {
  const cliArgs = parseArgs(process.argv);
  const config = loadConfig(cliArgs);
  const inputRoot = path.resolve(ROOT, config.input);

  if (!fs.existsSync(inputRoot)) {
    fail(`Input path does not exist: ${inputRoot}`);
  }

  log(`input=${config.input}  output=${config.output}  preset=${config.preset}  compile=${config.compile}`);

  let jsFiles = [];
  const isDir = fs.statSync(inputRoot).isDirectory();

  if (config.compile) {
    const sources = isDir ? walk(inputRoot, ['.es', '.lites', '.es.md']) : [inputRoot].filter(isEsFile);
    if (!sources.length) fail('No .es sources found to compile.');
    jsFiles = compileSources(config, isDir ? inputRoot : path.dirname(inputRoot), sources);
  } else {
    // Inputs are already .js — copy into output then obfuscate in place.
    const sources = isDir ? walk(inputRoot, ['.js']) : [inputRoot];
    for (const src of sources) {
      const target = outPathFor(src, isDir ? inputRoot : path.dirname(inputRoot), config.output, false);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.copyFileSync(src, target);
      jsFiles.push(target);
    }
  }

  if (!jsFiles.length) fail('No .js files to protect.');

  obfuscateFiles(config, jsFiles);
  runVmProtect(config);

  log(`\x1b[32mDone.\x1b[0m Protected ${jsFiles.length} file(s) into "${config.output}".`);
}

main();
