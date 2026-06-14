## Block Regular Expressions

Similar to block strings and comments, EvelentScript supports block regexes — extended regular expressions that ignore internal whitespace and can contain comments and interpolation. Modeled after Perl’s `/x` modifier, EvelentScript’s block regexes are delimited by `///` and go a long way towards making complex regular expressions readable. To quote from the EvelentScript source:

```
codeFor('heregexes')
```
