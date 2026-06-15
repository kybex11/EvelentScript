fs                        = require 'fs'
os                        = require 'os'
path                      = require 'path'
_                         = require 'underscore'
{ spawn, exec, execSync } = require 'child_process'
EvelentScript              = require './lib/evelentscript'
helpers                   = require './lib/evelentscript/helpers'

# ANSI Terminal Colors.
bold = red = green = yellow = reset = ''
unless process.env.NODE_DISABLE_COLORS
  bold   = '\x1B[0;1m'
  red    = '\x1B[0;31m'
  green  = '\x1B[0;32m'
  yellow = '\x1B[0;33m'
  reset  = '\x1B[0m'

# Built file header.
header = """
  /**
   * EvelentScript Compiler v#{EvelentScript.VERSION}
   */
"""

# Used in folder names like `docs/v1`.
majorVersion = parseInt EvelentScript.VERSION.split('.')[0], 10

# Documentation site version (independent of compiler semver).
docsMajorVersion = 1
docsFullVersion = '1.0.0'


# Log a message with a color.
log = (message, color, explanation) ->
  console.log color + message + reset + ' ' + (explanation or '')


spawnNodeProcess = (args, output = 'stderr', callback) ->
  relayOutput = (buffer) -> console.log buffer.toString()
  proc =         spawn 'node', args
  proc.stdout.on 'data', relayOutput if output is 'both' or output is 'stdout'
  proc.stderr.on 'data', relayOutput if output is 'both' or output is 'stderr'
  proc.on        'exit', (status) -> callback(status) if typeof callback is 'function'

# Run a EvelentScript through our node/es interpreter.
run = (args, callback) ->
  spawnNodeProcess ['bin/es'].concat(args), 'stderr', (status) ->
    process.exit(1) if status isnt 0
    callback() if typeof callback is 'function'


# Build the EvelentScript language from source.
buildParser = ->
  helpers.extend global, require 'util'
  require 'jison'
  # We don't need `moduleMain`, since the parser is unlikely to be run standalone.
  parser = require('./lib/evelentscript/grammar').parser.generate(moduleMain: ->)
  fs.writeFileSync 'lib/evelentscript/parser.js', parser

buildExceptParser = (callback) ->
  files = fs.readdirSync 'src'
  files = ('src/' + file for file in files when file.match(/\.(lit)?es$/))
  run ['-c', '-o', 'lib/evelentscript'].concat(files), callback

build = (callback) ->
  buildParser()
  buildExceptParser callback

transpile = (code, options = {}) ->
  options.minify =      process.env.MINIFY    isnt 'false'
  options.transform =   process.env.TRANSFORM isnt 'false'
  options.sourceType ?= 'script'
  babel = require '@babel/core'
  presets = []
  # Exclude the `modules` plugin in order to not break the `}(this));`
  # at the end of the `build:browser` code block.
  presets.push ['@babel/env', {modules: no}] if options.transform
  presets.push ['minify', {mangle: no, evaluate: no, removeUndefined: no}] if options.minify
  babelOptions =
    presets: presets
    compact: options.minify
    minified: options.minify
    comments: not options.minify
    sourceType: options.sourceType
  { code } = babel.transformSync code, babelOptions unless presets.length is 0
  code

testBuiltCode = (watch = no) ->
  csPath = './lib/evelentscript'
  csDir  = path.dirname require.resolve csPath

  for mod of require.cache when csDir is mod[0 ... csDir.length]
    delete require.cache[mod]

  testResults = runTests require csPath
  unless watch
    process.exit 1 unless testResults

buildAndTest = (includingParser = yes, harmony = no) ->
  process.stdout.write '\x1Bc' # Clear terminal screen.
  execSync 'git checkout lib/*', stdio: 'inherit' # Reset the generated compiler.

  buildArgs = ['bin/cake']
  buildArgs.push if includingParser then 'build' else 'build:except-parser'
  log "building#{if includingParser then ', including parser' else ''}...", green
  spawnNodeProcess buildArgs, 'both', ->
    log 'testing...', green
    testArgs = if harmony then ['--harmony'] else []
    testArgs = testArgs.concat ['bin/cake', 'test']
    spawnNodeProcess testArgs, 'both'

watchAndBuildAndTest = (harmony = no) ->
  buildAndTest yes, harmony
  fs.watch 'src/', interval: 200, (eventType, filename) ->
    if eventType is 'change'
      log "src/#{filename} changed, rebuilding..."
      buildAndTest (filename is 'grammar.es'), harmony
  fs.watch 'test/', {interval: 200, recursive: yes}, (eventType, filename) ->
    if eventType is 'change'
      log "test/#{filename} changed, rebuilding..."
      buildAndTest no, harmony


task 'build', 'build the EvelentScript compiler from source', build

task 'build:parser', 'build the Jison parser only', buildParser

task 'build:except-parser', 'build the EvelentScript compiler, except for the Jison parser', buildExceptParser

task 'build:full', 'build the EvelentScript compiler from source twice, and run the tests', ->
  build ->
    build testBuiltCode

task 'build:browser', 'merge the built scripts into a single file for use in a browser', ->
  code = """
  require['../../package.json'] = (function() {
    return #{fs.readFileSync "./package.json"};
  })();
  """
  for name in ['helpers', 'rewriter', 'lexer', 'parser', 'scope', 'nodes', 'sourcemap', 'evelentscript', 'browser']
    code += """
      require['./#{name}'] = (function() {
        var exports = {}, module = {exports: exports};
        #{fs.readFileSync "lib/evelentscript/#{name}.js"}
        return module.exports;
      })();
    """
  # From here, we generate two outputs: a legacy script output for all browsers
  # and a module output for browsers that support `<script type="module">`.
  code = """
    var EvelentScript = function() {
      function require(path){ return require[path]; }
      #{code}
      return require['./browser'];
    }();
  """
  scriptCode = transpile """
    (function(root) {
      #{code}

      if (typeof define === 'function' && define.amd) {
        define(function() { return EvelentScript; });
      } else {
        root.EvelentScript = EvelentScript;
      }
    }(this));
  """
  moduleCode = transpile """
    #{code}

    export default EvelentScript;
    const { VERSION, compile, eval: evaluate, load, run, runScripts } = EvelentScript;
    export { VERSION, compile, evaluate as eval, load, run, runScripts };
  """, {sourceType: 'module'}
  outputFolders = [
    "docs/v#{docsMajorVersion}/browser-compiler-legacy"
    "docs/v#{docsMajorVersion}/browser-compiler-modern"
    "lib/evelentscript-browser-compiler-legacy"
    "lib/evelentscript-browser-compiler-modern"
  ]
  for outputFolder in outputFolders
    fs.mkdirSync outputFolder, recursive: yes unless fs.existsSync outputFolder
    fs.writeFileSync "#{outputFolder}/evelentscript.js", """
      #{header}
      #{if outputFolder.includes('legacy') then scriptCode else moduleCode}
    """

task 'build:browser:full', 'merge the built scripts into a single file for use in a browser, and test it', ->
  invoke 'build:browser'
  console.log "built ... running browser tests:"
  invoke 'test:browser'

task 'build:watch', 'watch and continually rebuild the EvelentScript compiler, running tests on each build', ->
  watchAndBuildAndTest()

task 'build:watch:harmony', 'watch and continually rebuild the EvelentScript compiler, running harmony tests on each build', ->
  watchAndBuildAndTest yes


buildDocs = (watch = no) ->
  # Constants
  indexFile             = 'documentation/site/index.html'
  siteSourceFolder      = "documentation/site"
  sectionsSourceFolder  = 'documentation/sections'
  changelogSourceFolder = 'documentation/sections/changelog'
  examplesSourceFolder  = 'documentation/examples'
  outputFolder          = "docs/v#{docsMajorVersion}"

  # Helpers
  releaseHeader = (date, version, prevVersion) ->
    """
      <h3>#{prevVersion and "<a href=\"https://github.com/evelent-core/evelentscript/compare/#{prevVersion}...#{version}\">#{version}</a>" or version}
        <span class="timestamp"> &mdash; <time datetime="#{date}">#{date}</time></span>
      </h3>
    """

  codeFor = require "./documentation/site/code.es"

  htmlFor = ->
    hljs = require 'highlight.js'
    hljs.configure classPrefix: ''
    markdownRenderer = require('markdown-it')
      html: yes
      typographer: yes
      highlight: (str, language) ->
        # From https://github.com/markdown-it/markdown-it#syntax-highlighting
        hlLanguage = if language is 'es' then 'coffeescript' else language
        if hlLanguage and hljs.getLanguage(hlLanguage)
          try
            return hljs.highlight(str, { language: hlLanguage }).value
          catch ex
        return '' # No syntax highlighting


    # Add some custom overrides to Markdown-It’s rendering, per
    # https://github.com/markdown-it/markdown-it/blob/master/docs/architecture.md#renderer
    defaultFence = markdownRenderer.renderer.rules.fence
    markdownRenderer.renderer.rules.fence = (tokens, idx, options, env, slf) ->
      code = tokens[idx].content
      if code.indexOf('codeFor(') is 0 or code.indexOf('releaseHeader(') is 0
        "<%= #{code} %>"
      else
        "<blockquote class=\"uneditable-code-block\">#{defaultFence.apply @, arguments}</blockquote>"

    (file, bookmark) ->
      md = fs.readFileSync "#{sectionsSourceFolder}/#{file.replace /\//g, path.sep}.md", 'utf-8'
      md = md.replace /<%= releaseHeader %>/g, releaseHeader
      md = md.replace /<%= majorVersion %>/g, docsMajorVersion
      md = md.replace /<%= fullVersion %>/g, docsFullVersion
      html = markdownRenderer.render md
      html = _.template(html)
        codeFor: codeFor()
        releaseHeader: releaseHeader

  includeScript = ->
    (file) ->
      file = "#{siteSourceFolder}/#{file}" unless '/' in file
      code = fs.readFileSync file, 'utf-8'
      code = EvelentScript.compile code
      code = transpile code
      code

  include = ->
    (file) ->
      file = "#{siteSourceFolder}/#{file}" unless '/' in file
      output = fs.readFileSync file, 'utf-8'
      if /\.html$/.test(file)
        render = _.template output
        output = render
          releaseHeader: releaseHeader
          majorVersion: docsMajorVersion
          fullVersion: docsFullVersion
          htmlFor: htmlFor()
          codeFor: codeFor()
          include: include()
          includeScript: includeScript()
      output

  siteAssets = ['icon.svg', 'logo.svg', 'manifest.json', 'codemirror-evelentscript.js']

  copySiteAssets = ->
    for asset in siteAssets
      src = "#{siteSourceFolder}/#{asset}"
      fs.copyFileSync src, "#{outputFolder}/#{asset}" if fs.existsSync src

  # Task
  do renderIndex = ->
    fs.mkdirSync outputFolder, recursive: yes unless fs.existsSync outputFolder
    fs.mkdirSync 'docs', recursive: yes unless fs.existsSync 'docs'
    render = _.template fs.readFileSync(indexFile, 'utf-8')
    output = render
      include: include()
    fs.writeFileSync "#{outputFolder}/index.html", output
    copySiteAssets()
    log 'compiled', green, "#{indexFile} → #{outputFolder}/index.html"
  try
    fs.symlinkSync "v#{docsMajorVersion}/index.html", 'docs/index.html'
  catch exception

  if watch
    for target in [indexFile, siteSourceFolder, examplesSourceFolder, sectionsSourceFolder, changelogSourceFolder]
      fs.watch target, interval: 200, renderIndex
    log 'watching...', green

task 'doc:site', 'build the documentation for the website', ->
  buildDocs()

task 'doc:site:watch', 'watch and continually rebuild the documentation for the website', ->
  buildDocs yes


buildDocTests = (watch = no) ->
  # Constants
  testFile          = 'documentation/site/test.html'
  testsSourceFolder = 'test'
  outputFolder      = "docs/v#{docsMajorVersion}"

  # Included in test.html
  testHelpers = fs.readFileSync('test/support/helpers.es', 'utf-8').replace /exports\./g, '@'

  # Helpers
  testsInScriptBlocks = ->
    output = ''
    for filename in fs.readdirSync(testsSourceFolder).sort()
      if filename.indexOf('.es') isnt -1
        fileType = 'evelentscript'
      else if filename.indexOf('.lites') isnt -1
        fileType = 'literate-evelentscript'
      else
        continue

      # Set the type to text/x-evelentscript or text/x-literate-evelentscript
      # to prevent the browser compiler from automatically running the script
      output += """
        <script type="text/x-#{type}" class="test" id="#{filename.split('.')[0]}">
        #{fs.readFileSync "test/#{filename}", 'utf-8'}
        </script>\n
      """
    output

  # Task
  do renderTest = ->
    render = _.template fs.readFileSync(testFile, 'utf-8')
    output = render
      testHelpers: testHelpers
      tests: testsInScriptBlocks()
    fs.writeFileSync "#{outputFolder}/test.html", output
    log 'compiled', green, "#{testFile} → #{outputFolder}/test.html"

  if watch
    for target in [testFile, testsSourceFolder]
      fs.watch target, interval: 200, renderTest
    log 'watching...', green

task 'doc:test', 'build the browser-based tests', ->
  buildDocTests()

task 'doc:test:watch', 'watch and continually rebuild the browser-based tests', ->
  buildDocTests yes


buildAnnotatedSource = (watch = no) ->
  do generateAnnotatedSource = ->
    exec "cd src && ../node_modules/docco/bin/docco *.*es --output ../docs/v#{docsMajorVersion}/annotated-source", (err) -> throw err if err
    log 'generated', green, "annotated source in docs/v#{docsMajorVersion}/annotated-source/"

  if watch
    fs.watch 'src/', interval: 200, generateAnnotatedSource
    log 'watching...', green

task 'doc:source', 'build the annotated source documentation', ->
  buildAnnotatedSource()

task 'doc:source:watch', 'watch and continually rebuild the annotated source documentation', ->
  buildAnnotatedSource yes


task 'release', 'update dependencies, build and test the EvelentScript source, and build the documentation', ->
  execSync '''
    npm install --silent
    cake build:full
    cake build:browser
    cake doc:test
    cake test:browser:node
    cake test:browser
    cake test:integrations
    cake doc:site
    cake doc:source
  ''', stdio: 'inherit'


task 'bench', 'quick benchmark of compilation time', ->
  {Rewriter} = require './lib/evelentscript/rewriter'
  sources = ['evelentscript', 'grammar', 'helpers', 'lexer', 'nodes', 'rewriter']
  esSource  = sources.map((name) -> fs.readFileSync "src/#{name}.es").join '\n'
  liteSource = fs.readFileSync("src/scope.lites").toString()
  fmt    = (ms) -> " #{bold}#{ "   #{ms}".slice -4 }#{reset} ms"
  total  = 0
  now    = Date.now()
  time   = -> total += ms = -(now - now = Date.now()); fmt ms
  tokens = EvelentScript.tokens esSource, rewrite: no
  littokens = EvelentScript.tokens liteSource, rewrite: no, literate: yes
  tokens = tokens.concat(littokens)
  console.log "Lex    #{time()} (#{tokens.length} tokens)"
  tokens = new Rewriter().rewrite tokens
  console.log "Rewrite#{time()} (#{tokens.length} tokens)"
  nodes  = EvelentScript.nodes tokens
  console.log "Parse  #{time()}"
  js     = nodes.compile bare: yes
  console.log "Compile#{time()} (#{js.length} chars)"
  console.log "total  #{ fmt total }"


# Run the EvelentScript test suite.
runTests = (EvelentScript) ->
  EvelentScript.register() unless global.testingBrowser

  # These are attached to `global` so that they’re accessible from within
  # `test/async.es`, which has an async-capable version of
  # `global.test`.
  global.currentFile = null
  global.passedTests = 0
  global.failures    = []

  global[name] = func for name, func of require 'assert'

  # Convenience aliases.
  global.EvelentScript = EvelentScript
  global.Repl   = require './lib/evelentscript/repl'
  global.bold   = bold
  global.red    = red
  global.green  = green
  global.yellow = yellow
  global.reset  = reset

  asyncTests = []
  onFail = (description, fn, err) ->
    failures.push
      filename: global.currentFile
      error: err
      description: description
      source: fn.toString() if fn.toString?

  # Our test helper function for delimiting different test cases.
  global.test = (description, fn) ->
    try
      fn.test = {description, currentFile}
      result = fn.call(fn)
      if result instanceof Promise # An async test.
        asyncTests.push result
        result.then ->
          passedTests++
        .catch (err) ->
          onFail description, fn, err
      else
        passedTests++
    catch err
      onFail description, fn, err

  helpers.extend global, require './test/support/helpers'

  # When all the tests have run, collect and print errors.
  # If a stacktrace is available, output the compiled function source.
  process.on 'exit', ->
    time = ((Date.now() - startTime) / 1000).toFixed(2)
    message = "passed #{passedTests} tests in #{time} seconds#{reset}"
    return log(message, green) unless failures.length
    log "failed #{failures.length} and #{message}", red
    for fail in failures
      {error, filename, description, source}  = fail
      console.log ''
      log "  #{description}", red if description
      log "  #{error.stack}", red
      console.log "  #{source}" if source
    return

  # Run every test in the `test` folder, recording failures, except for files
  # we’re skipping because the features to be tested are unsupported in the
  # current Node runtime.
  testFilesToSkip = []
  skipUnless = (featureDetect, filenames) ->
    unless (try new Function featureDetect)
      testFilesToSkip = testFilesToSkip.concat filenames
  skipUnless 'async () => {}', ['async.es', 'async_iterators.es']
  skipUnless 'async function* generator() { yield 42; }', ['async_iterators.es']
  skipUnless 'var a = 2 ** 2; a **= 3', ['exponentiation.es']
  skipUnless 'var {...a} = {}', ['object_rest_spread.es']
  skipUnless '/foo.bar/s.test("foo\tbar")', ['regex_dotall.es']
  skipUnless '1_2_3', ['numeric_literal_separators.es']
  skipUnless '1n', ['numbers_bigint.es']
  skipUnless 'async () => { await import(\'data:application/json,{"foo":"bar"}\', { assert: { type: "json" } }) }', ['import_assertions.es']
  files = fs.readdirSync('test').filter (filename) ->
    filename not in testFilesToSkip

  startTime = Date.now()
  for file in files when helpers.isEvelentScript file
    literate = helpers.isLiterate file
    currentFile = filename = path.join 'test', file
    code = fs.readFileSync filename
    try
      EvelentScript.run code.toString(), {filename, literate}
    catch error
      failures.push {filename, error}

  Promise.all(asyncTests).then ->
    Promise.reject() if failures.length isnt 0


task 'test', 'run the EvelentScript language test suite', ->
  runTests(EvelentScript).catch -> process.exit 1


task 'test:browser', 'run the test suite against the modern browser compiler in a headless browser', ->
  # Create very simple web server to serve the two files we need.
  http = require 'http'
  serveFile = (res, fileToServe, mimeType) ->
    res.statusCode = 200
    res.setHeader 'Content-Type', mimeType
    fs.createReadStream(fileToServe).pipe res
  server = http.createServer (req, res) ->
    if req.url is '/'
      serveFile res, path.join(__dirname, 'docs', "v#{docsMajorVersion}", 'test.html'), 'text/html'
    else if req.url is '/browser-compiler-modern/evelentscript.js'
      # The `text/javascript` MIME type is required for an ES module file to be
      # loaded in a browser.
      serveFile res, path.join(__dirname, 'docs', "v#{docsMajorVersion}", 'browser-compiler-modern', 'evelentscript.js'), 'text/javascript'
    else
      res.statusCode = 404
      res.end()

  server.listen 8080, ->
    puppeteer = require 'puppeteer'
    browser   = await puppeteer.launch()
    page      = await browser.newPage()
    result    = ""

    try
      await page.goto 'http://localhost:8080/'

      element = await page.waitForSelector '#result',
        visible: yes
        polling: 'mutation'
        timeout: 60000

      result = await page.evaluate ((el) => el.textContent), element
    catch e
      log e, red
    finally
      try browser.close()
      server.close()

    if result and not result.includes('failed')
      log result, green
    else
      log result, red
      process.exit 1


task 'test:browser:node', 'run the test suite against the legacy browser compiler in Node', ->
  source = fs.readFileSync "lib/evelentscript-browser-compiler-legacy/evelentscript.js", 'utf-8'
  result = {}
  global.testingBrowser = yes
  (-> eval source).call result
  runTests(EvelentScript).catch -> process.exit 1


task 'test:integrations', 'test the module integrated with other libraries and environments', ->
  # Tools like Webpack and Browserify generate builds intended for a browser
  # environment where Node modules are not available. We want to ensure that
  # the EvelentScript module as presented by the `browser` key in `package.json`
  # can be built by such tools; if such a build succeeds, it verifies that no
  # Node modules are required as part of the compiler (as opposed to the tests)
  # and that therefore the compiler will run in a browser environment.
  # Webpack 5 requires Node >= 10.13.0.
  [major, minor] = process.versions.node.split('.').map (n) -> parseInt(n, 10)
  return if major < 10 or (major is 10 and minor < 13)
  tmpdir = os.tmpdir()
  webpack = require 'webpack'
  webpack {
    entry: './'
    optimization:
      # Webpack’s minification causes the EvelentScript module to fail some tests.
      minimize: off
    output:
      path: tmpdir
      filename: 'evelentscript.js'
      library: 'EvelentScript'
      libraryTarget: 'commonjs2'
  }, (err, stats) ->
    if err or stats.hasErrors()
      if err
        console.error err.stack or err
        console.error err.details if err.details
      if stats.hasErrors()
        console.error error for error in stats.compilation.errors
      if stats.hasWarnings()
        console.warn warning for warning in stats.compilation.warnings
      process.exit 1

    builtCompiler = path.join tmpdir, 'evelentscript.js'
    { EvelentScript } = require builtCompiler
    global.testingBrowser = yes
    testResults = runTests EvelentScript
    fs.unlinkSync builtCompiler
    process.exit 1 unless testResults
