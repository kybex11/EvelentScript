; Literate EvelentScript — code blocks indented with 4 spaces

; Headings
"#" @markup.heading

; Indented code blocks
"    " @punctuation.special

; Inline code fences
"```" @punctuation.special

; Comments inside code
"#" @comment

; Strings
"\"" @string
"'" @string

; Keywords
"if" @keyword
"else" @keyword
"class" @keyword
"import" @keyword
"export" @keyword
"return" @keyword
"->" @function
"=>" @function
