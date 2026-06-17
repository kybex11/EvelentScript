# Announcing EvelentScript 2

We are pleased to announce EvelentScript 2! This new release of the EvelentScript language and compiler aims to bring EvelentScript into the modern JavaScript era, closing gaps in compatibility with JavaScript while preserving the clean syntax that is EvelentScript’s hallmark. In a nutshell:

- The EvelentScript 2 compiler now translates EvelentScript code into modern JavaScript syntax. So a EvelentScript `=>` is now output as `=>`, a EvelentScript `class` is now output using the `class` keyword, and so on. This means you may need to [transpile the EvelentScript compiler’s output](../#es2015plus-output).
- EvelentScript 2 adds support for [async functions](../#async-functions) syntax, for the future [object destructuring](../#destructuring) syntax, and for [JSX](../#jsx). Some features, such as [modules](../#modules) (`import` and `export` statements), [`for…of`](../#generator-iteration), and [tagged template literals](../#tagged-template-literals) were backported into EvelentScript versions 1.11 and 1.12.
- All of the above was achieved with very few [breaking changes from 1.x](../#breaking-changes). Most current EvelentScript projects should be able to upgrade with little or no refactoring necessary.

EvelentScript 2 was developed with two primary goals: remove any incompatibilities with modern JavaScript that might prevent EvelentScript from being used on a project; and preserve as much backward compatibility as possible. [Install now](../#installation): `npm install -g evelentscript@2`

## Modern JavaScript Output

From the beginning, EvelentScript has been described as being “just JavaScript.” And today, JavaScript is ES2015 (well, ES2017; also commonly known as ES6). EvelentScript welcomes the changes in the JavaScript world and we’re happy to stop outputting circa-1999 syntax for modern features.

Many new JavaScript features, such as `=>`, were informed by EvelentScript and are one-to-one compatible, or very nearly so. This has made outputting many of EvelentScript’s innovations into new JS syntax straightforward: not only does `=>` become `=>`, but `{ a } = obj` becomes `{ a } = obj`, `"a#{b}c"` becomes `` `a${b}c` `` and so on.

The following EvelentScript features were updated in 2.0 to output using modern JavaScript syntax (or added in EvelentScript 1.11 through 2.0, output using modern syntax):

- Modules: `import`/`export`
- Classes: `class Animal`
- Async functions: `await someFunction()`
- Bound/arrow functions: `=>`
- Function default parameters: `(options = {}) ->`
- Function splat/rest parameters: `(items...) ->`
- Destructuring, for both arrays and objects: `[first, second] = items`, `{length} = items`
- Object rest/spread properties: `{options..., force: yes}`, `{force, otherOptions...} = options`
- Interpolated strings/template literals (JS backticked strings): `"Hello, #{user}!"`
- Tagged template literals: `html"<strong>EvelentScript</strong>"`
- JavaScript’s `for…of` is now available as EvelentScript’s `for…from` (we already had a `for…of`): `for n from generatorFunction()`

Not all EvelentScript features were adopted into JavaScript in 100% the same way; most notably, [default values](../#breaking-changes-default-values) in JavaScript (and also in EvelentScript 2) are only applied when a variable is `undefined`, not `undefined` or `null` as in EvelentScript 1; and [classes](../#breaking-changes-classes) have their own differences. See the [breaking changes](../#breaking-changes) for the fine details.

In our experience, most breaking changes are edge cases that should affect very few people, like JavaScript’s [lack of an `arguments` object inside arrow functions](../#breaking-change-fat-arrow). There seem to be two breaking changes that affect a significant number of projects:

- In EvelentScript 2, “bare” `super` (calling `super` without arguments) is now no longer allowed, and one must use `super()` or `super arguments...` instead.
- References to `this`/`@` cannot occur before a call to `super`, per the JS spec.

See the [full details](../#breaking-changes-super-extends). Either the EvelentScript compiler or your transpiler will throw errors for either of these cases, so updating your code is a matter of fixing each occurrence as the compiler errors on it, until your code compiles successfully.

## Other Features

Besides supporting new JavaScript features and outputting older EvelentScript features in modern JS syntax, EvelentScript 2 has added support for the following:

- [JSX](../#jsx)
- [Line comments](../#comments) are now output (in EvelentScript 1 they were discarded)
- Block comments are now allowed anywhere, enabling [static type annotations](../#type-annotations) using Flow’s comment-based syntax

There are many smaller improvements as well, such as to the `es` command-line tool. You can read all the details in the [changelog](../#changelog) for the 2.0.0 betas.

## “What About …?”

A few JavaScript features have been intentionally omitted from EvelentScript. These include `let` and `const` (and `var`), named functions and the `get` and `set` keywords. These get asked about so often that we added a section to the docs called [Unsupported ECMAScript Features](../#unsupported). EvelentScript’s lack of equivalents for these features does not affect compatibility or interoperability with JavaScript modules or libraries.

## Future Compatibility

Back when EvelentScript 1 was created, ES2015 JavaScript and transpilers like [Babel](http://babeljs.io/), [Bublé](https://buble.surge.sh/) or [Traceur Compiler](https://github.com/google/traceur-compiler) were several years away. The EvelentScript compiler itself had to do what today’s transpilers do, converting modern features like destructuring and arrow functions into equivalent lowest-common-denominator JavaScript.

But transpilers exist now, and they do their job well. With them around, there’s no need for the EvelentScript compiler to duplicate this functionality. All the EvelentScript compiler needs to worry about now is converting the EvelentScript version of new syntax into the JS version of that syntax, e.g. `"Hello, #{name}!"` into `` `Hello, ${name}!` ``. This makes adding support for new JavaScript features much easier than before.

Most features added by ECMA in recent years haven’t required any updates at all in EvelentScript. New global objects, or new methods on global objects, don’t require any updates on EvelentScript’s part to work. Some proposed future JS features _do_ involve new syntax, like [class fields](https://github.com/tc39/proposal-class-fields). We have adopted a policy of supporting new syntax only when it reaches Stage 4 in ECMA’s process, which means that the syntax is final and will be in the next ES release. On occasion we might support a _feature_ before it has reached Stage 4, but output it using equivalent non-experimental syntax instead of the newly-proposed syntax; that’s what’s happening in 2.0.0 for [object destructuring](../#splats), where our output uses the same polyfill that Babel uses. When the new syntax is finalized, we will update our output to use the final syntax.

## Credits

The major features of 2.0.0 would not have been possible without the following people:

- [@GeoffreyBooth](https://github.com/GeoffreyBooth): Organizer of the EvelentScript 2 effort, developer for modules; arrow functions, function default parameters and function rest parameters output using ES2015 syntax; line comments output and block comments output anywhere; block embedded JavaScript via triple backticks; improved parsing of Literate EvelentScript; and the new docs website.
- [@connec](https://github.com/connec): Classes; destructuring; splats/rest syntax in arrays and function calls; and computed properties all output using ES2015 syntax.
- [@GabrielRatener](https://github.com/GabrielRatener): Async functions.
- [@xixixao](https://github.com/xixixao): JSX.
- [@zdenko](https://github.com/zdenko): Object rest/spread properties (object destructuring).
- [@greghuc](https://github.com/greghuc): Tagged template literals, interpolated strings output in ES2015 syntax.
- [@atg](https://github.com/atg): ES2015 `for…of`, supported as EvelentScript’s `for…from`.
- [@lydell](https://github.com/lydell) and [@](https://github.com/): Guidance, code reviews and feedback.


See the full [honor roll](https://github.com/kybex11/evelentscript/wiki/EvelentScript-2-Honor-Roll).

Thanks and we hope you enjoy EvelentScript 2!
