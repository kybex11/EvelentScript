; EvelentScript syntax highlights for Zed

; Comments
"#" @comment

; Strings
"\"" @string
"'" @string

; Keywords
"if" @keyword
"else" @keyword
"unless" @keyword
"switch" @keyword
"when" @keyword
"try" @keyword
"catch" @keyword
"finally" @keyword
"throw" @keyword
"return" @keyword
"break" @keyword
"continue" @keyword
"while" @keyword
"until" @keyword
"loop" @keyword
"for" @keyword
"of" @keyword
"in" @keyword
"by" @keyword
"new" @keyword
"delete" @keyword
"typeof" @keyword
"instanceof" @keyword
"super" @keyword
"extends" @keyword
"this" @keyword
"class" @keyword
"import" @keyword
"export" @keyword
"default" @keyword
"from" @keyword
"as" @keyword
"await" @keyword
"async" @keyword
"yield" @keyword
"do" @keyword

; Constants
"true" @constant
"false" @constant
"null" @constant
"undefined" @constant
"yes" @constant
"no" @constant
"on" @constant
"off" @constant

; Functions / arrows
"->" @function
"=>" @function

; Operators
"=" @operator
"==" @operator
"===" @operator
"!=" @operator
"!==" @operator
"?" @operator
":" @operator
"&&" @operator
"||" @operator
".." @operator
"..." @operator

; Variables
"@" @variable

; Numbers
@number

; Regex
@string.special
