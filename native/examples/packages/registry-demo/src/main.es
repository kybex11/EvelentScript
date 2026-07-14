# Demo: require registry packages (native VM)

heap = require 'heap'
h = heap.create()
h.push 5
h.push 1
h.push 3
console.log 'heap.peek', h.peek()
console.log 'heap.pop', h.pop(), h.pop(), h.pop()

easie = require 'easie'
console.log 'ease', easie.quadOut(500, 0, 100, 1000)

shellwords = require 'shellwords'
words = shellwords.split "hello world"
console.log 'words', words.join('|')

sentimood = require 'sentimood'
mood = sentimood.analyze 'this is awesome and great'
console.log 'sentiment', mood.score

normat = require 'normat'
console.log 'bytes', normat.bytes(1536000)
console.log 'ms', normat.ms(125000)
