# Minimal sentiment — inspired by soops/sentimood

positives = {}
positives['awesome'] = 5
positives['good'] = 3
positives['great'] = 4
positives['love'] = 4
positives['happy'] = 3
positives['excellent'] = 5
positives['nice'] = 2
positives['best'] = 4
positives['wonderful'] = 4
positives['cool'] = 2

negatives = {}
negatives['bad'] = -3
negatives['awful'] = -5
negatives['hate'] = -4
negatives['terrible'] = -5
negatives['sad'] = -3
negatives['worst'] = -5
negatives['angry'] = -3
negatives['stupid'] = -3
negatives['poor'] = -2
negatives['fail'] = -3

analyze = (text) ->
  score = 0
  lower = ('' + text).toLowerCase()
  words = lower.split(' ')
  for w in words
    clean = ''
    i = 0
    while i < w.length
      c = w[i]
      if c >= 'a' and c <= 'z'
        clean += c
      else if c >= '0' and c <= '9'
        clean += c
      i += 1
    if clean.length is 0
      continue
    p = positives[clean]
    if p
      score += p
    nval = negatives[clean]
    if nval
      score += nval
  comparative = 0
  if words.length
    comparative = score / words.length
  result = {}
  result.score = score
  result.comparative = comparative
  result

exports.analyze = analyze
exports.positives = positives
exports.negatives = negatives
