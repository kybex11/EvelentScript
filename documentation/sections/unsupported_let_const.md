### `let` and `const`: block-scoped and reassignment-protected variables

When EvelentScript was designed, `var` was [intentionally omitted](https://github.com/evelent-core/evelentscript/issues/238#issuecomment-153502). This was to spare developers the mental housekeeping of needing to worry about variable _declaration_ (`var foo`) as opposed to variable _assignment_ (`foo = 1`). The EvelentScript compiler automatically takes care of declaration for you, by generating `var` statements at the top of every function scope. This makes it impossible to accidentally declare a global variable.

`let` and `const` add block scope to JavaScript. EvelentScript intentionally omits them to keep variable assignment simple: the compiler declares variables at the top of each function scope for you.

Keep in mind that `const` only protects you from _reassigning_ a variable; it doesn’t prevent the variable’s value from changing, the way constants usually do in other languages:

```js
const obj = {foo: 'bar'};
obj.foo = 'baz'; // Allowed!
obj = {}; // Throws error
```
