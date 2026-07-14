# Native EvelentScript — runs on the Rust VM

console.log 'Hello from native esc!'

square = (x) -> x * x
console.log '5^2 =', (square 5)

list = [1, 2, 3]
for n in list
  console.log 'n =', n

math =
  cube: (x) -> x * x * x

console.log 'cube(3) =', (math.cube 3)
