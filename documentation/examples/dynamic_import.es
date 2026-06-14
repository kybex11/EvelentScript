# Your browser must support dynamic import to run this example.

do ->
  { run } = await import('./browser-compiler-modern/evelentscript.js')
  run '''
    if 5 < new Date().getHours() < 9
      alert 'Time to compile!'
    else
      alert 'Time to get some work done.'
  '''
