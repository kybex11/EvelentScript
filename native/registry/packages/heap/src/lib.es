# Binary heap — EvelentScript port of qiao/heap.js (native-safe)

defaultCmp = (x, y) ->
  return -1 if x < y
  return 1 if x > y
  0

siftdown = (array, startpos, pos, cmp) ->
  newitem = array[pos]
  while pos > startpos
    parentpos = Math.floor((pos - 1) / 2)
    parent = array[parentpos]
    if cmp(newitem, parent) < 0
      array[pos] = parent
      pos = parentpos
    else
      break
  array[pos] = newitem

siftup = (array, pos, cmp) ->
  endpos = array.length
  startpos = pos
  newitem = array[pos]
  childpos = 2 * pos + 1
  while childpos < endpos
    rightpos = childpos + 1
    if rightpos < endpos and not (cmp(array[childpos], array[rightpos]) < 0)
      childpos = rightpos
    array[pos] = array[childpos]
    pos = childpos
    childpos = 2 * pos + 1
  array[pos] = newitem
  siftdown array, startpos, pos, cmp

heappush = (array, item, cmp) ->
  cmp = cmp or defaultCmp
  array.push item
  siftdown array, 0, array.length - 1, cmp
  item

heappop = (array, cmp) ->
  cmp = cmp or defaultCmp
  lastelt = array.pop()
  if array.length
    returnitem = array[0]
    array[0] = lastelt
    siftup array, 0, cmp
    return returnitem
  lastelt

heapify = (array, cmp) ->
  cmp = cmp or defaultCmp
  i = Math.floor(array.length / 2) - 1
  while i >= 0
    siftup array, i, cmp
    i -= 1
  array

create = (cmp) ->
  cmp = cmp or defaultCmp
  nodes = []
  api = {}
  api.push = (x) ->
    heappush nodes, x, cmp
    x
  api.pop = ->
    heappop nodes, cmp
  api.peek = ->
    nodes[0]
  api.size = ->
    nodes.length
  api.empty = ->
    nodes.length is 0
  api.clear = ->
    while nodes.length > 0
      nodes.pop()
  api.toArray = ->
    nodes.slice()
  api

exports.Heap = create
exports.create = create
exports.push = heappush
exports.pop = heappop
exports.heapify = heapify
exports.defaultCmp = defaultCmp
