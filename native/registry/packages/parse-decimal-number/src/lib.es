# Parse localized decimals — inspired by AndreasPizsa/parse-decimal-number

parse = (value, options) ->
  s = String(value).trim()
  return NaN if s.length is 0
  thousands = ','
  decimal = '.'
  if options
    thousands = options.thousands if options.thousands
    decimal = options.decimal if options.decimal
  cleaned = ''
  i = 0
  while i < s.length
    c = s[i]
    if c is thousands
      # skip grouping separators
    else if c is decimal
      cleaned += '.'
    else
      cleaned += c
    i += 1
  Number cleaned

exports.parse = parse
exports.parseDecimalNumber = parse
