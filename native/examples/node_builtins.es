# Demo: Node-like builtins on the native Rust VM

fs   = require 'fs'
path = require 'path'
os   = require 'os'
crypto = require 'crypto'
assert = require 'assert'

console.log 'platform =', os.platform()
console.log 'tmpdir   =', os.tmpdir()
console.log 'cwd      =', process.cwd()

file = path.join os.tmpdir(), 'evelent-demo.txt'
fs.writeFileSync file, 'hello from esc'
text = fs.readFileSync file, 'utf8'
console.log 'read =', text
assert.ok (fs.existsSync file)
fs.unlinkSync file

hash = crypto.createHash('sha256').update('evelent').digest('hex')
console.log 'sha256  =', hash

mod = require 'module'
console.log 'builtins =', mod.builtinModules.length