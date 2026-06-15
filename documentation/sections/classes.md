## Classes

EvelentScript provides the `class` and `extends` keywords, compiling them to native ES2015 classes.

```
codeFor('classes', true)
```

Static methods can be defined using `@` before the method name:

```
codeFor('static', 'Teenager.say("Are we there yet?")')
```

Finally, class definitions are blocks of executable code, which make for interesting metaprogramming possibilities. In the context of a class definition, `this` is the class object itself; therefore, you can assign static properties by using `@property: value`.
