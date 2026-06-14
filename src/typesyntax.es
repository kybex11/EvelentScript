# Type syntax nodes for EvelentScript.
# Parsed type expressions compile away in JavaScript output and can be emitted
# as TypeScript for static checking via `es --check-types`.

{compact, flatten, fragmentsToText} = require './helpers'

exports.makeTypeReference = (name, typeArguments = null) ->
  new exports.TypeReference name, typeArguments

exports.makeFunctionType = (params, returnType) ->
  new exports.FunctionType params, returnType

#### TypeBase

exports.TypeBase = class TypeBase
  toTypeScript: -> ''
  compileNode: -> []
  unwrap: -> this
  unfoldSoak: -> no
  isStatement: -> no
  jumps: -> no

  compileToFragments: (o, lvl) ->
    @compileNode o

  compile: (o, lvl) ->
    fragmentsToText @compileToFragments o, lvl

  eachChild: (func) ->
    return this unless @children
    for attr in @children when @[attr]
      for child in flatten [@[attr]]
        return this if func(child) is false
    this

  updateLocationDataIfMissing: (locationData, force) ->
    @forceUpdateLocation = yes if force
    return this if @locationData and not @forceUpdateLocation
    delete @forceUpdateLocation
    @locationData = locationData
    @eachChild (child) ->
      child.updateLocationDataIfMissing? locationData
    this

  traverseChildren: (crossScope, func) ->
    @eachChild (child) ->
      recur = func child
      child.traverseChildren? crossScope, func unless recur is no
    this

#### TypeReference

exports.TypeReference = class TypeReference extends exports.TypeBase
  constructor: (@name, @typeArguments = null) ->
    super()

  children: ['typeArguments']

  toTypeScript: ->
    base = if @name instanceof exports.TypeBase then @name.toTypeScript() else "#{@name}"
    if @typeArguments?.length
      args = (@arg.toTypeScript() for arg in @typeArguments).join ', '
      "#{base}<#{args}>"
    else
      base

#### UnionType

exports.UnionType = class UnionType extends exports.TypeBase
  constructor: (@left, @right) ->
    super()

  children: ['left', 'right']

  toTypeScript: ->
    "#{@left.toTypeScript()} | #{@right.toTypeScript()}"

#### IntersectionType

exports.IntersectionType = class IntersectionType extends exports.TypeBase
  constructor: (@left, @right) ->
    super()

  children: ['left', 'right']

  toTypeScript: ->
    "#{@left.toTypeScript()} & #{@right.toTypeScript()}"

#### OptionalType

exports.OptionalType = class OptionalType extends exports.TypeBase
  constructor: (@type) ->
    super()

  children: ['type']

  toTypeScript: ->
    "#{@type.toTypeScript()}?"

#### ArrayType

exports.ArrayType = class ArrayType extends exports.TypeBase
  constructor: (@elementType) ->
    super()

  children: ['elementType']

  toTypeScript: ->
    "#{@elementType.toTypeScript()}[]"

#### FunctionType

exports.FunctionType = class FunctionType extends exports.TypeBase
  constructor: (@params = [], @returnType = null) ->
    super()

  children: ['params', 'returnType']

  toTypeScript: ->
    paramText = (@paramTypeText param for param in @params).join ', '
    ret = @returnType?.toTypeScript() ? 'void'
    "(#{paramText}): #{ret}"

  paramTypeText: (param) ->
    name = param.name?.value ? param.name ? '_'
    typeText = param.typeAnnotation?.toTypeScript() ? 'any'
    optional = if param.optional then '?' else ''
    "#{name}#{optional}: #{typeText}"

#### TypeParameter

exports.TypeParameter = class TypeParameter extends exports.TypeBase
  constructor: (@name, @constraint = null, @defaultType = null) ->
    super()

  toTypeScript: ->
    text = @name.value ? @name
    text += " extends #{@constraint.toTypeScript()}" if @constraint
    text += " = #{@defaultType.toTypeScript()}" if @defaultType
    text

#### TypeParameterList

exports.typeParametersToTypeScript = (params) ->
  return '' unless params?.length
  "<#{(p.toTypeScript() for p in params).join ', '}>"

#### InterfaceMember

exports.InterfaceMember = class InterfaceMember extends exports.TypeBase
  constructor: (@name, @typeAnnotation, options = {}) ->
    super()
    @optional = options.optional is yes
    @readonly = options.readonly is yes
    @method = options.method is yes

  children: ['name', 'typeAnnotation']

  toTypeScript: ->
    prefix = if @readonly then 'readonly ' else ''
    name = @name.value ? @name.name ? @name
    optional = if @optional and not @method then '?' else ''
    "#{prefix}#{name}#{optional}: #{@typeAnnotation.toTypeScript()}"

#### InterfaceDeclaration

exports.InterfaceDeclaration = class InterfaceDeclaration extends exports.TypeBase
  constructor: (@name, @typeParameters = [], @extendsType = null, @members = []) ->
    super()

  children: ['name', 'typeParameters', 'extendsType', 'members']

  isStatement: -> yes

  jumps: -> no

  compileNode: -> []

  toTypeScript: ->
    params = exports.typeParametersToTypeScript @typeParameters
    head = "interface #{@name.value}#{params}"
    head += " extends #{@extendsType.toTypeScript()}" if @extendsType
    body = (member.toTypeScript() for member in @members).join '\n  '
    if body
      "#{head} {\n  #{body}\n}"
    else
      "#{head} {}"

#### TypeAliasDeclaration

exports.TypeAliasDeclaration = class TypeAliasDeclaration extends exports.TypeBase
  constructor: (@name, @typeParameters = [], @typeAnnotation) ->
    super()

  children: ['name', 'typeParameters', 'typeAnnotation']

  isStatement: -> yes

  jumps: -> no

  compileNode: -> []

  toTypeScript: ->
    params = exports.typeParametersToTypeScript @typeParameters
    "type #{@name.value}#{params} = #{@typeAnnotation.toTypeScript()}"

#### TypeProgram

exports.collectTypeDeclarations = (body) ->
  declarations = []
  body.traverseChildren no, (node) ->
    if node instanceof exports.InterfaceDeclaration or node instanceof exports.TypeAliasDeclaration
      declarations.push node
  declarations

exports.emitTypeScriptDeclarations = (nodes) ->
  (node.toTypeScript() for node in nodes).join '\n\n'

exports.mergeTypeScriptWithJavaScript = (js, declarations) ->
  header = exports.emitTypeScriptDeclarations declarations
  return js unless header
  "#{header}\n\n#{js}"
