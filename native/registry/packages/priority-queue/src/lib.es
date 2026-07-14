# Min priority queue — inspired by STRd6/priority_queue

create = ->
  items = []
  api = {}
  api.push = (value, priority) ->
    if not priority and priority isnt 0
      priority = value
    items.push { value: value, priority: priority }
    i = items.length - 1
    while i > 0
      parent = Math.floor((i - 1) / 2)
      break if items[parent].priority <= items[i].priority
      tmp = items[parent]
      items[parent] = items[i]
      items[i] = tmp
      i = parent
    value
  api.pop = ->
    return undefined if items.length is 0
    top = items[0].value
    last = items.pop()
    if items.length > 0
      items[0] = last
      i = 0
      while true
        left = 2 * i + 1
        right = 2 * i + 2
        smallest = i
        if left < items.length and items[left].priority < items[smallest].priority
          smallest = left
        if right < items.length and items[right].priority < items[smallest].priority
          smallest = right
        break if smallest is i
        tmp = items[i]
        items[i] = items[smallest]
        items[smallest] = tmp
        i = smallest
    top
  api.peek = ->
    if items.length then items[0].value else undefined
  api.size = -> items.length
  api.empty = -> items.length is 0
  return api

exports.create = create
exports.PriorityQueue = create
