### Bound generator functions

Bound generator functions, a.k.a. generator arrow functions, [aren’t allowed in ECMAScript](http://stackoverflow.com/questions/27661306/can-i-use-es6s-arrow-function-syntax-with-generators-arrow-notation). You can write `function*` or `=>`, but not both. Therefore, EvelentScript code like this:

```es
f = => yield this
# Throws a compiler error
```

Needs to be rewritten the old-fashioned way:

```
codeFor('breaking_change_bound_generator_function')
```
