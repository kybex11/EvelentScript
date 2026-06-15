# Load and normalize esconfig.json (TypeScript-style project config for EvelentScript).

fs     = require 'fs'
path   = require 'path'
helpers = require './helpers'

CONFIG_NAMES = ['esconfig.json', 'esconfig.jsonc']

DEFAULT_INCLUDE = [
  '*.es'
  '**/*.es'
  '*.lites'
  '**/*.lites'
  '*.es.md'
  '**/*.es.md'
]

DEFAULT_EXCLUDE = [
  '**/node_modules/**'
  '**/.git/**'
]

DEFAULT_COMPILER_OPTIONS =
  rootDir: '.'
  outDir: 'dist'
  entry: 'index.es'
  bundle: no
  outFile: 'index.js'
  bare: no
  sourceMap: no
  header: no
  include: DEFAULT_INCLUDE
  exclude: DEFAULT_EXCLUDE

findConfigFile = (startDir) ->
  dir = path.resolve startDir
  loop
    for name in CONFIG_NAMES
      candidate = path.join dir, name
      return candidate if fs.existsSync candidate
    parent = path.dirname dir
    break if parent is dir
    dir = parent
  null

loadJson = (filePath) ->
  raw = fs.readFileSync filePath, 'utf8'
  # Allow // and /* */ comments in esconfig.jsonc-style files.
  raw = raw.replace /\/\/[^\n]*/g, ''
  raw = raw.replace /\/\*[\s\S]*?\*\//g, ''
  JSON.parse raw

normalizeOptions = (raw, configDir) ->
  flat = Object.assign {}, raw
  if raw.compilerOptions?
    Object.assign flat, raw.compilerOptions
    delete flat.compilerOptions

  opts = Object.assign {}, DEFAULT_COMPILER_OPTIONS, flat

  opts.rootDir = path.resolve configDir, opts.rootDir ? opts.inputDir ? '.'
  opts.outDir = path.resolve configDir, opts.outDir ? opts.outputDir ? 'dist'

  if opts.entry?
    entryRel = opts.entry
    opts.entry = if path.isAbsolute entryRel
      entryRel
    else
      path.resolve opts.rootDir, entryRel
  else
    opts.entry = findDefaultEntry opts.rootDir

  if flat.include?
    seen = {}
    opts.include = []
    for pattern in DEFAULT_INCLUDE.concat flat.include
      unless seen[pattern]
        seen[pattern] = yes
        opts.include.push pattern
  else
    opts.include = DEFAULT_INCLUDE.slice()

  opts.exclude ?= DEFAULT_EXCLUDE
  opts.exclude = opts.exclude.concat [
    path.relative(opts.rootDir, opts.outDir).replace(/\\/g, '/') + '/**'
  ]

  opts.configPath = null
  opts.configDir = configDir
  opts

findDefaultEntry = (rootDir) ->
  for name in ['index.es', 'main.es', 'app.es']
    candidate = path.join rootDir, name
    return candidate if fs.existsSync candidate
  path.join rootDir, 'index.es'

exports.findConfigFile = findConfigFile

exports.load = (configPath = null, cwd = process.cwd()) ->
  configPath ?= findConfigFile cwd
  unless configPath?
    throw new Error 'Could not find esconfig.json in this directory or any parent directory.'
  configDir = path.dirname path.resolve configPath
  raw = loadJson configPath
  opts = normalizeOptions raw, configDir
  opts.configPath = path.resolve configPath
  opts

exports.matchesGlob = (relPath, pattern) ->
  normalized = relPath.replace /\\/g, '/'
  regex = globToRegex pattern.replace /\\/g, '/'
  regex.test normalized

globToRegex = (pattern) ->
  normalized = pattern.replace /\\/g, '/'
  optionalPrefix = no
  if normalized.indexOf('**/') is 0
    optionalPrefix = yes
    normalized = normalized.substring 3
  escaped = normalized.replace /[.+^${}()|[\]\\]/g, '\\$&'
  escaped = escaped.replace /\*\*/g, '.*'
  escaped = escaped.replace /\*/g, '[^/]*'
  escaped = escaped.replace /\?/g, '[^/]'
  body = if optionalPrefix then "(?:.*/)?#{escaped}" else escaped
  new RegExp "^#{body}$"

exports.matchesAny = (relPath, patterns) ->
  for pattern in patterns
    return yes if exports.matchesGlob relPath, pattern
  no

exports.collectSources = (opts) ->
  files = []
  unless fs.existsSync opts.rootDir
    throw new Error "rootDir does not exist: #{opts.rootDir}"

  walk = (dir) ->
    for entry in fs.readdirSync dir, withFileTypes: yes
      name = entry.name
      continue if name is '.' or name is '..'
      full = path.join dir, name
      rel = path.relative(opts.rootDir, full).replace /\\/g, '/'

      if entry.isDirectory()
        continue if exports.matchesAny rel + '/', opts.exclude
        continue if exports.matchesAny rel + '/**', opts.exclude
        walk full
      else if helpers.isEvelentScript full
        continue if exports.matchesAny rel, opts.exclude
        continue unless exports.matchesAny rel, opts.include
        files.push full

  walk opts.rootDir
  files.sort()
