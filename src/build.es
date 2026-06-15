# Project build: esconfig.json → compiled JavaScript in outDir (mirror or bundle).

fs          = require 'fs'
path        = require 'path'
helpers     = require './helpers'
EvelentScript = require './evelentscript'
esconfig    = require './esconfig'

useWinPathSep = path.sep is '\\'

IMPORT_RE = /// ^import\s+(?:[\w\*\s{},.]+\s+from\s+)?['"]([^'"]+)['"]\s*;?\s*$ ///gm
EXPORT_FROM_RE = /// ^export\s+(?:\{[^}]*\}|\*(?:\s+as\s+[\w$]+)?)\s+from\s+['"]([^'"]+)['"]\s*;?\s*$ ///gm
DYNAMIC_IMPORT_RE = /// import\s*\(\s*['"]([^'"]+)['"]\s*\) ///g

log = (message) -> console.log message
fail = (message) -> console.error message; process.exit 1

mkdirp = (dir) ->
  fs.mkdirSync dir, recursive: yes unless fs.existsSync dir

jsOutputPath = (sourceFile, rootDir, outDir) ->
  rel = path.relative rootDir, sourceFile
  base = helpers.baseFileName rel, yes, useWinPathSep
  dir = path.dirname rel
  outRel = if dir is '.' then base + '.js' else path.join dir, base + '.js'
  path.join outDir, outRel

readSource = (file) ->
  raw = fs.readFileSync file, 'utf8'
  raw = raw.substring 1 if raw.charCodeAt(0) is 0xFEFF
  if helpers.isLiterate file
    helpers.invertLiterate raw
  else
    raw

getModuleSpecifiers = (code) ->
  specs = []
  collect = (re) ->
    code.replace re, (_, spec) ->
      specs.push spec
      ''
  collect IMPORT_RE
  collect EXPORT_FROM_RE
  collect DYNAMIC_IMPORT_RE
  specs

resolveLocalModule = (specifier, fromFile, rootDir, sourceSet) ->
  return null unless specifier.startsWith '.'
  fromDir = path.dirname fromFile
  requested = path.resolve fromDir, specifier
  candidates = []
  if /\.js$/i.test specifier
    candidates.push requested.replace /\.js$/i, '.es'
  candidates.push requested
  candidates.push requested + '.es'
  candidates.push path.join requested, 'index.es'
  for candidate in candidates
    resolved = path.resolve candidate
    return resolved if sourceSet.has resolved
  null

REQUIRE_LINE_RE = /// ^[^\n]*require\s+['"](\.[^'"]+)['"][^\n]*$ ///gm

stripLocalRequires = (code, fromFile, rootDir, sourceSet) ->
  code.replace REQUIRE_LINE_RE, (line, spec) ->
    if resolveLocalModule spec, fromFile, rootDir, sourceSet
      ''
    else
      line

stripLocalImports = (code, fromFile, rootDir, sourceSet) ->
  strip = (re) ->
    code = code.replace re, (line, spec) ->
      if resolveLocalModule spec, fromFile, rootDir, sourceSet
        ''
      else
        line
  code = strip IMPORT_RE
  code = strip EXPORT_FROM_RE
  stripLocalRequires code, fromFile, rootDir, sourceSet

stripExports = (code) ->
  code
    .replace /^export\s+default\s+/gm, ''
    .replace /^export\s+/gm, ''

sortDependencyOrder = (entry, rootDir, sourceSet) ->
  order = []
  visiting = {}
  visited = {}

  visit = (file) ->
    resolved = path.resolve file
    return if visited[resolved]
    if visiting[resolved]
      throw new Error "Circular dependency involving #{resolved}"
    visiting[resolved] = yes
    code = readSource resolved
    for spec in getModuleSpecifiers code
      dep = resolveLocalModule spec, resolved, rootDir, sourceSet
      visit dep if dep?
    delete visiting[resolved]
    visited[resolved] = yes
    order.push resolved

  entryResolved = path.resolve entry
  unless sourceSet.has entryResolved
    throw new Error "Entry file is not part of the project: #{entryResolved}"
  visit entryResolved
  order

compileOptions = (opts, filename, jsPath = null) ->
  answer =
    filename: filename
    literate: helpers.isLiterate filename
    bare: opts.bare
    header: opts.header
    sourceMap: opts.sourceMap
  if opts.sourceMap and jsPath?
    answer =
      filename: filename
      literate: answer.literate
      bare: opts.bare
      header: opts.header
      sourceMap: yes
      generatedFile: helpers.baseFileName jsPath, no, useWinPathSep
      sourceFiles: [filename]
  answer

writeOutput = (jsPath, js, sourceMap = null) ->
  mkdirp path.dirname jsPath
  if sourceMap?
    mapPath = "#{jsPath}.map"
    js = "#{js}\n//# sourceMappingURL=#{helpers.baseFileName mapPath, no, useWinPathSep}\n"
    fs.writeFileSync mapPath, sourceMap
  fs.writeFileSync jsPath, js

compileOne = (sourceFile, jsPath, opts) ->
  code = readSource sourceFile
  options = compileOptions opts, sourceFile, jsPath
  result = EvelentScript.compile code, options
  if opts.sourceMap
    writeOutput jsPath, result.js, result.v3SourceMap
  else
    js = if typeof result is 'string' then result else result.js
    writeOutput jsPath, js

exports.buildProject = (config) ->
  sources = esconfig.collectSources config
  sourceSet = new Set sources.map (f) -> path.resolve f

  if config.bundle
    entry = path.resolve config.entry
    unless fs.existsSync entry
      throw new Error "Entry file not found: #{entry}"

    order = sortDependencyOrder entry, config.rootDir, sourceSet
    parts = []
    for file in order
      code = readSource file
      code = stripExports code unless file is entry
      code = stripLocalImports code, file, config.rootDir, sourceSet
      parts.push code
    merged = parts.join '\n\n'
    outPath = path.join config.outDir, config.outFile
    options = compileOptions config, entry
    options.bare = yes unless config.bare is no
    result = EvelentScript.compile merged, options
    js = if typeof result is 'string' then result else result.js
    writeOutput outPath, js
    log "bundled #{order.length} file(s) → #{outPath}"
    return {files: order.length, outPath}

  if sources.length is 0
    throw new Error "No EvelentScript source files found under #{config.rootDir}"

  for source in sources
    jsPath = jsOutputPath source, config.rootDir, config.outDir
    compileOne source, jsPath, config

  log "compiled #{sources.length} file(s) → #{config.outDir}"
  {files: sources.length, outDir: config.outDir}

exports.run = (args = []) ->
  configPath = null
  watch = no
  i = 0
  while i < args.length
    switch args[i]
      when '-w', '--watch' then watch = yes; i += 1
      when '-c', '--config'
        configPath = args[i + 1]
        unless configPath?
          fail '--config requires a path'
        i += 2
      when '-h', '--help'
        console.log '''
          Usage: es build [options]

            Read esconfig.json and compile the project.

          Options:
            -c, --config PATH   Path to esconfig.json (default: search upward from cwd)
            -w, --watch         Rebuild when source files change
            -h, --help          Show this help message

          esconfig.json (compilerOptions):

            rootDir     Source root (default: ".")
            outDir      Output directory (default: "dist")
            entry       Entry file for bundle mode (default: "index.es")
            bundle      true = single outFile; false = mirror tree (default: false)
            outFile     Bundle output filename (default: "index.js")
            bare        Compile without function wrapper
            sourceMap   Emit .js.map files (project mode only)
            include     Glob patterns (default: **/*.es, etc.)
            exclude     Glob patterns (default: node_modules, outDir)
        '''
        return
      else
        fail "Unknown option: #{args[i]}"

  config = esconfig.load configPath
  doBuild = ->
    try
      exports.buildProject config
    catch err
      fail err.stack ? err.message ? err

  if watch
    log "watching #{config.rootDir}..."
    doBuild()
    timeout = null
    schedule = ->
      clearTimeout timeout
      timeout = setTimeout doBuild, 100
    sources = esconfig.collectSources config
    for source in sources
      fs.watch source, schedule
  else
    doBuild()
