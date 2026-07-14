# Shellword splitter — EvelentScript port inspired by jimmycuadra/shellwords

isSpace = (ch) ->
  ch is ' ' or ch is '\t' or ch is '\n' or ch is '\r'

split = (line) ->
  words = []
  field = ''
  i = 0
  len = line.length
  while i < len
    ch = line[i]
    if ch is '\\' and i + 1 < len
      field += line[i + 1]
      i += 2
      continue
    if ch is "'" or ch is '"'
      quote = ch
      i += 1
      while i < len and line[i] isnt quote
        if line[i] is '\\' and quote is '"' and i + 1 < len
          field += line[i + 1]
          i += 2
        else
          field += line[i]
          i += 1
      i += 1
      continue
    if isSpace ch
      if field.length > 0
        words.push field
        field = ''
      while i < len and isSpace(line[i])
        i += 1
      continue
    field += ch
    i += 1
  if field.length > 0
    words.push field
  words

escape = (str) ->
  s = '' + str
  needs = false
  i = 0
  while i < s.length
    c = s[i]
    if isSpace(c) or c is "'" or c is '"'
      needs = true
      break
    i += 1
  if not needs
    return s
  out = "'"
  i = 0
  while i < s.length
    if s[i] is "'"
      out += "'\\''"
    else
      out += s[i]
    i += 1
  out + "'"

join = (words) ->
  parts = []
  for w in words
    parts.push escape(w)
  parts.join ' '

exports.split = split
exports.escape = escape
exports.join = join
