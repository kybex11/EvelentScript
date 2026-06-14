### Argument parsing and shebang (`#!`) lines

In EvelentScript 1.x, `--` was required after the path and filename of the script to be run, but before any arguments passed to that script. This convention is now deprecated. So instead of:

```bash
es [options] path/to/script.es -- [args]
```

Now you would just type:

```bash
es [options] path/to/script.es [args]
```

The deprecated version will still work, but it will print a warning before running the script.

On non-Windows platforms, a `.es` file can be made executable by adding a shebang (`#!`) line at the top of the file and marking the file as executable. For example:

```es
#!/usr/bin/env es

x = 2 + 2
console.log x
```

If this were saved as `executable.es`, it could be made executable and run:

```bash
▶ chmod +x ./executable.es
▶ ./executable.es
4
```

In EvelentScript 1.x, this used to fail when trying to pass arguments to the script. Some users on OS X worked around the problem by using `#!/usr/bin/env es --` as the first line of the file. That didn’t work on Linux, however, which cannot parse shebang lines with more than a single argument. While such scripts will still run on OS X, EvelentScript will now display a warning before compiling or evaluating files that begin with a too-long shebang line. Now that EvelentScript 2 supports passing arguments without needing `--`, we recommend simply changing the shebang lines in such scripts to just `#!/usr/bin/env es`.