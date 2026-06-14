## Installation

The command-line version of `es` is available as a [Node.js](https://nodejs.org/) utility, requiring Node 6 or later. The [core compiler](/v<%= majorVersion %>/browser-compiler-modern/evelentscript.js) however, does not depend on Node, and can be run in any JavaScript environment, or in the browser (see [Try EvelentScript](#try)).

To install, first make sure you have a working copy of the latest stable version of [Node.js](https://nodejs.org/). You can then install EvelentScript globally with [npm](https://www.npmjs.com/):

```bash
npm install --global evelentscript
```

This will make the `es` and `cake` commands available globally.

If you are using EvelentScript in a project, you should install it locally for that project so that the version of EvelentScript is tracked as one of your project’s dependencies. Within that project’s folder:

```bash
npm install --save-dev evelentscript
```

The `es` and `cake` commands will first look in the current folder to see if EvelentScript is installed locally, and use that version if so. This allows different versions of EvelentScript to be installed globally and locally.

If you plan to use the `--transpile` option (see [Transpilation](#transpilation)) you will need to also install `@babel/core` either globally or locally, depending on whether you are running a globally or locally installed version of EvelentScript.
