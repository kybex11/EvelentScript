#!/usr/bin/env node

/**
 * Automatic build script for EvelentScript.
 *
 * Usage:
 *   node scripts/build.js              # compile src -> lib
 *   node scripts/build.js --full       # build twice + run tests
 *   node scripts/build.js --browser    # compile + browser bundles
 *   node scripts/build.js --all        # full build + browser bundles
 *   node scripts/build.js --release    # full release pipeline (via cake)
 *   node scripts/build.js --watch      # watch src and rebuild + test
 *   node scripts/build.js --install    # npm install before build
 *   node scripts/build.js --help
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');
const CAKE = path.join(ROOT, 'bin', 'cake');
const NODE_MODULES = path.join(ROOT, 'node_modules');

const colors = process.stdout.isTTY && !process.env.NO_COLOR
  ? { reset: '\x1B[0m', green: '\x1B[32m', yellow: '\x1B[33m', red: '\x1B[31m', bold: '\x1B[1m' }
  : { reset: '', green: '', yellow: '', red: '', bold: '' };

function log(message, color = '') {
  console.log(`${color}${message}${colors.reset}`);
}

function fail(message, code = 1) {
  log(`\n✗ ${message}`, colors.red);
  process.exit(code);
}

function parseArgs(argv) {
  const flags = {
    help: false,
    install: false,
    full: false,
    browser: false,
    all: false,
    release: false,
    watch: false,
    harmony: false,
    tasks: [],
  };

  for (const arg of argv) {
    switch (arg) {
      case '--help':
      case '-h':
        flags.help = true;
        break;
      case '--install':
      case '-i':
        flags.install = true;
        break;
      case '--full':
      case '-f':
        flags.full = true;
        break;
      case '--browser':
      case '-b':
        flags.browser = true;
        break;
      case '--all':
      case '-a':
        flags.all = true;
        break;
      case '--release':
      case '-r':
        flags.release = true;
        break;
      case '--watch':
      case '-w':
        flags.watch = true;
        break;
      case '--harmony':
        flags.harmony = true;
        break;
      default:
        if (arg.startsWith('-')) {
          fail(`Unknown option: ${arg}\nRun with --help for usage.`);
        } else {
          flags.tasks.push(arg);
        }
    }
  }

  if (flags.all) {
    flags.full = true;
    flags.browser = true;
  }

  return flags;
}

function printHelp() {
  console.log(`
${colors.bold}EvelentScript build script${colors.reset}

Usage:
  node scripts/build.js [options] [cake-task...]

Options:
  -h, --help       Show this help
  -i, --install    Run npm install before building
  -f, --full       Run build:full (build twice + tests)
  -b, --browser    Build browser compiler bundles
  -a, --all        Full build + browser bundles + tests
  -r, --release    Run the full release pipeline (cake release)
  -w, --watch      Watch src/ and rebuild on changes
      --harmony    Use harmony mode for watch builds

Default (no options):
  Compiles EvelentScript sources from src/ into lib/evelentscript/

Examples:
  npm run build
  npm run build -- --full
  npm run build -- --all
  node scripts/build.js build:parser
`);
}

function checkNodeVersion() {
  const major = parseInt(process.versions.node.split('.')[0], 10);
  if (major < 6) {
    fail('Node.js 6 or later is required.');
  }
}

function ensureProjectLayout() {
  const required = [
    path.join(ROOT, 'src'),
    path.join(ROOT, 'bin', 'es'),
    path.join(ROOT, 'bin', 'cake'),
    path.join(ROOT, 'Cakefile'),
  ];

  for (const filePath of required) {
    if (!fs.existsSync(filePath)) {
      fail(`Missing required path: ${path.relative(ROOT, filePath)}`);
    }
  }
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: ROOT,
    stdio: 'inherit',
    shell: process.platform === 'win32',
    env: process.env,
    ...options,
  });

  if (result.error) {
    fail(`Failed to run ${command}: ${result.error.message}`);
  }

  if (result.status !== 0) {
    fail(`${command} ${args.join(' ')} exited with code ${result.status}`, result.status);
  }
}

function npmInstall() {
  log('\n→ Installing dependencies...', colors.yellow);
  run('npm', ['install'], { stdio: 'inherit' });
}

function runCake(tasks, nodeArgs = []) {
  const args = nodeArgs.concat([CAKE]).concat(tasks);
  log(`\n→ node ${args.slice(1).join(' ')}`, colors.green);
  run(process.execPath, args);
}

function resolveTasks(flags) {
  if (flags.tasks.length > 0) {
    return flags.tasks;
  }

  if (flags.release) {
    return ['release'];
  }

  if (flags.watch) {
    return [flags.harmony ? 'build:watch:harmony' : 'build:watch'];
  }

  const tasks = [];

  if (flags.full) {
    tasks.push('build:full');
  } else {
    tasks.push('build');
  }

  if (flags.browser) {
    tasks.push('build:browser');
  }

  return tasks;
}

function main() {
  const flags = parseArgs(process.argv.slice(2));

  if (flags.help) {
    printHelp();
    return;
  }

  process.chdir(ROOT);

  log(`${colors.bold}EvelentScript build${colors.reset}`);
  checkNodeVersion();
  ensureProjectLayout();

  if (flags.install || !fs.existsSync(NODE_MODULES)) {
    if (!fs.existsSync(NODE_MODULES)) {
      log('\n→ node_modules not found, installing dependencies...', colors.yellow);
    }
    npmInstall();
  }

  const tasks = resolveTasks(flags);

  for (const task of tasks) {
    runCake([task]);
  }

  log('\n✓ Build completed successfully.', colors.green);
}

main();
