## Async Functions

ES2017’s [async functions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/async_function) are supported through the `await` keyword. Like with generators, there's no need for an `async` keyword; an async function in EvelentScript is simply a function that awaits.

Similar to how `yield return` forces a generator, `await return` may be used to force a function to be async.

```
codeFor('async', true)
```
