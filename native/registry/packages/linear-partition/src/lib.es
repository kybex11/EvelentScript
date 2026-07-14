# Linear partition — EvelentScript port of crispymtn/linear-partition

partition = (seq, k) ->
  n = seq.length
  return [] if n is 0
  return [seq.slice()] if k <= 1
  k = n if k > n

  prefixes = []
  i = 0
  while i < n
    if i is 0
      prefixes.push seq[0]
    else
      prefixes.push prefixes[i - 1] + seq[i]
    i += 1

  table = []
  solution = []
  i = 0
  while i < n
    row = []
    srow = []
    j = 0
    while j < k
      row.push 0
      srow.push 0
      j += 1
    table.push row
    solution.push srow
    i += 1

  i = 0
  while i < n
    table[i][0] = prefixes[i]
    i += 1

  j = 0
  while j < k
    table[0][j] = seq[0]
    j += 1

  big = 1e100
  i = 1
  while i < n
    j = 1
    while j < k
      table[i][j] = big
      x = 0
      while x < i
        left = table[x][j - 1]
        right = prefixes[i] - prefixes[x]
        cost = if left > right then left else right
        if table[i][j] > cost
          table[i][j] = cost
          solution[i][j] = x
        x += 1
      j += 1
    i += 1

  parts = []
  ni = n - 1
  kj = k - 1
  while kj > 0
    start = solution[ni][kj] + 1
    chunk = seq.slice(start, ni + 1)
    # prepend without unshift
    next = [chunk]
    for p in parts
      next.push p
    parts = next
    ni = solution[ni][kj]
    kj -= 1
  first = [seq.slice(0, ni + 1)]
  for p in parts
    first.push p
  first

exports.partition = partition
exports.linearPartition = partition
