outer = ->
  inner = => Array.from arguments
  inner()

outer(1, 2)  # Returns '' in EvelentScript 1.x, '1, 2' in EvelentScript 2
