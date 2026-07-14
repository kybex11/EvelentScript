use crate::error::{Error, Result};
use crate::token::{Span, Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    path: String,
    indent_stack: Vec<usize>,
    pending: Vec<Token>,
    at_line_start: bool,
}

impl Lexer {
    pub fn new(source: &str, path: impl Into<String>) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            path: path.into(),
            indent_stack: vec![0],
            pending: Vec::new(),
            at_line_start: true,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        if let Some(t) = self.pending.pop() {
            return Ok(t);
        }

        if self.at_line_start {
            self.handle_indent()?;
            self.at_line_start = false;
            if let Some(t) = self.pending.pop() {
                return Ok(t);
            }
        }

        self.skip_inline_ws();

        if self.is_eof() {
            while self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                self.pending.push(self.make(TokenKind::Dedent, ""));
            }
            if let Some(t) = self.pending.pop() {
                self.pending.push(self.make(TokenKind::Eof, ""));
                return Ok(t);
            }
            return Ok(self.make(TokenKind::Eof, ""));
        }

        let span = self.span();
        let c = self.peek();

        if c == '\n' {
            self.advance();
            self.at_line_start = true;
            return Ok(Token {
                kind: TokenKind::Newline,
                lexeme: "\n".into(),
                span,
            });
        }

        if c == '#' {
            while !self.is_eof() && self.peek() != '\n' {
                self.advance();
            }
            return self.next_token();
        }

        // Strings / heredocs
        if c == '"' || c == '\'' {
            return self.lex_string();
        }

        // Numbers
        if c.is_ascii_digit() {
            return self.lex_number();
        }

        // Identifiers / keywords
        if is_ident_start(c) {
            return Ok(self.lex_ident());
        }

        // Multi-char operators
        if let Some(tok) = self.try_ops() {
            return Ok(tok);
        }

        Err(Error::syntax(
            &self.path,
            span.line,
            span.column,
            format!("unexpected character `{c}`"),
        ))
    }

    fn handle_indent(&mut self) -> Result<()> {
        let mut spaces = 0usize;
        while !self.is_eof() {
            match self.peek() {
                ' ' => {
                    spaces += 1;
                    self.advance();
                }
                '\t' => {
                    spaces += 2;
                    self.advance();
                }
                '\n' => {
                    // blank line
                    self.advance();
                    spaces = 0;
                    continue;
                }
                '#' => {
                    while !self.is_eof() && self.peek() != '\n' {
                        self.advance();
                    }
                    if !self.is_eof() && self.peek() == '\n' {
                        self.advance();
                    }
                    spaces = 0;
                    continue;
                }
                _ => break,
            }
        }

        if self.is_eof() {
            return Ok(());
        }

        let current = *self.indent_stack.last().unwrap();
        if spaces == current {
            return Ok(());
        }
        if spaces > current {
            self.indent_stack.push(spaces);
            self.pending.push(self.make(TokenKind::Indent, ""));
            return Ok(());
        }

        while let Some(&top) = self.indent_stack.last() {
            if top == spaces {
                break;
            }
            if top < spaces {
                return Err(Error::syntax(
                    &self.path,
                    self.line,
                    self.column,
                    "inconsistent indentation",
                ));
            }
            self.indent_stack.pop();
            self.pending.push(self.make(TokenKind::Dedent, ""));
        }
        Ok(())
    }

    fn lex_string(&mut self) -> Result<Token> {
        let span = self.span();
        let quote = self.advance();

        // Heredoc: '''...''' or """..."""
        if !self.is_eof() && self.peek() == quote && self.peek_ahead(1) == Some(quote) {
            self.advance();
            self.advance();
            return self.lex_heredoc(quote, span);
        }

        let mut buf = String::new();
        while !self.is_eof() {
            let c = self.advance();
            if c == quote {
                let lexeme = if buf.contains("#{") {
                    flatten_interpolation(&buf)
                } else {
                    buf
                };
                return Ok(Token {
                    kind: TokenKind::String,
                    lexeme,
                    span,
                });
            }
            if c == '\\' {
                if self.is_eof() {
                    break;
                }
                let esc = self.advance();
                buf.push(match esc {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' | '\'' | '"' => esc,
                    other => other,
                });
            } else {
                buf.push(c);
            }
        }
        Err(Error::syntax(
            &self.path,
            span.line,
            span.column,
            "unterminated string",
        ))
    }

    fn lex_heredoc(&mut self, quote: char, span: crate::token::Span) -> Result<Token> {
        let mut buf = String::new();
        while !self.is_eof() {
            // Closing ''' or """
            if self.peek() == quote
                && self.peek_ahead(1) == Some(quote)
                && self.peek_ahead(2) == Some(quote)
            {
                self.advance();
                self.advance();
                self.advance();
                // Trim leading newline (CoffeeScript-style)
                if buf.starts_with('\n') {
                    buf.remove(0);
                }
                // Dedent common leading whitespace from each line
                buf = dedent_heredoc(&buf);
                let lexeme = if buf.contains("#{") {
                    flatten_interpolation(&buf)
                } else {
                    buf
                };
                return Ok(Token {
                    kind: TokenKind::String,
                    lexeme,
                    span,
                });
            }
            let c = self.advance();
            if c == '\\' && !self.is_eof() {
                let esc = self.advance();
                buf.push(match esc {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' | '\'' | '"' => esc,
                    other => other,
                });
            } else {
                buf.push(c);
            }
        }
        Err(Error::syntax(
            &self.path,
            span.line,
            span.column,
            "unterminated heredoc",
        ))
    }

    fn lex_number(&mut self) -> Result<Token> {
        let span = self.span();
        let mut buf = String::new();
        while !self.is_eof() && (self.peek().is_ascii_digit() || self.peek() == '_') {
            let c = self.advance();
            if c != '_' {
                buf.push(c);
            }
        }
        if !self.is_eof() && self.peek() == '.' && self.peek_ahead(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
            buf.push(self.advance());
            while !self.is_eof() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                let c = self.advance();
                if c != '_' {
                    buf.push(c);
                }
            }
        }
        if !self.is_eof() && matches!(self.peek(), 'e' | 'E') {
            buf.push(self.advance());
            if !self.is_eof() && matches!(self.peek(), '+' | '-') {
                buf.push(self.advance());
            }
            while !self.is_eof() && self.peek().is_ascii_digit() {
                buf.push(self.advance());
            }
        }
        Ok(Token {
            kind: TokenKind::Number,
            lexeme: buf,
            span,
        })
    }

    fn lex_ident(&mut self) -> Token {
        let span = self.span();
        let mut buf = String::new();
        while !self.is_eof() && is_ident_continue(self.peek()) {
            buf.push(self.advance());
        }
        let kind = keyword(&buf).unwrap_or(TokenKind::Ident);
        Token {
            kind,
            lexeme: buf,
            span,
        }
    }

    fn try_ops(&mut self) -> Option<Token> {
        let span = self.span();
        let two = self.peek_str(2);
        let three = self.peek_str(3);

        let (kind, len) = match () {
            _ if three == "..." => (TokenKind::Ellipsis, 3),
            _ if two == "->" => (TokenKind::Arrow, 2),
            _ if two == "=>" => (TokenKind::FatArrow, 2),
            _ if two == ":=" => (TokenKind::ColonEquals, 2),
            _ if two == "==" => (TokenKind::EqEq, 2),
            _ if two == "!=" => (TokenKind::NotEq, 2),
            _ if two == "<=" => (TokenKind::LtEq, 2),
            _ if two == ">=" => (TokenKind::GtEq, 2),
            _ if two == "+=" => (TokenKind::PlusEq, 2),
            _ if two == "-=" => (TokenKind::MinusEq, 2),
            _ if two == "*=" => (TokenKind::StarEq, 2),
            _ if two == "/=" => (TokenKind::SlashEq, 2),
            _ if two == "&&" => (TokenKind::AmpAmp, 2),
            _ if two == "||" => (TokenKind::PipePipe, 2),
            _ => {
                let kind = match self.peek() {
                    '=' => TokenKind::Equals,
                    ':' => TokenKind::Colon,
                    ',' => TokenKind::Comma,
                    '.' => TokenKind::Dot,
                    '?' => TokenKind::Question,
                    '!' => TokenKind::Bang,
                    '@' => TokenKind::At,
                    '+' => TokenKind::Plus,
                    '-' => TokenKind::Minus,
                    '*' => TokenKind::Star,
                    '/' => TokenKind::Slash,
                    '%' => TokenKind::Percent,
                    '<' => TokenKind::Lt,
                    '>' => TokenKind::Gt,
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    '{' => TokenKind::LBrace,
                    '}' => TokenKind::RBrace,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    _ => return None,
                };
                (kind, 1)
            }
        };

        let mut lexeme = String::new();
        for _ in 0..len {
            lexeme.push(self.advance());
        }
        Some(Token { kind, lexeme, span })
    }

    fn skip_inline_ws(&mut self) {
        while !self.is_eof() && matches!(self.peek(), ' ' | '\t' | '\r') {
            self.advance();
        }
    }

    fn make(&self, kind: TokenKind, lexeme: &str) -> Token {
        Token {
            kind,
            lexeme: lexeme.into(),
            span: self.span(),
        }
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.column, self.pos)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn peek_str(&self, n: usize) -> String {
        self.chars[self.pos..].iter().take(n).collect()
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$'
}

fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

fn keyword(s: &str) -> Option<TokenKind> {
    Some(match s {
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "unless" => TokenKind::Unless,
        "then" => TokenKind::Then,
        "while" => TokenKind::While,
        "until" => TokenKind::Until,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "of" => TokenKind::Of,
        "when" => TokenKind::When,
        "return" => TokenKind::Return,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "class" => TokenKind::Class,
        "extends" => TokenKind::Extends,
        "super" => TokenKind::Super,
        "new" => TokenKind::New,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "finally" => TokenKind::Finally,
        "throw" => TokenKind::Throw,
        "switch" => TokenKind::Switch,
        "true" | "yes" | "on" => TokenKind::True,
        "false" | "no" | "off" => TokenKind::False,
        "null" => TokenKind::Null,
        "undefined" => TokenKind::Undefined,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "is" => TokenKind::Is,
        "isnt" => TokenKind::Isnt,
        "import" => TokenKind::Import,
        "export" => TokenKind::Export,
        "from" => TokenKind::From,
        "as" => TokenKind::As,
        "default" => TokenKind::Default,
        "require" => TokenKind::Require,
        "native" => TokenKind::Native,
        "do" => TokenKind::Do,
        "own" => TokenKind::Own,
        "await" => TokenKind::Await,
        "async" => TokenKind::Async,
        "yield" => TokenKind::Yield,
        _ => return None,
    })
}

/// Convert `hello, #{name}!` into a sentinel form `__TPL__hello, ${name}!`
/// so codegen can emit a template literal.
fn flatten_interpolation(s: &str) -> String {
    let mut out = String::from("__TPL__");
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' && chars.peek() == Some(&'{') {
            chars.next();
            out.push('$');
            out.push('{');
            for c2 in chars.by_ref() {
                out.push(c2);
                if c2 == '}' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip shared leading indentation from heredoc body (CoffeeScript-like).
fn dedent_heredoc(s: &str) -> String {
    let lines: Vec<&str> = s.split('\n').collect();
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                l.chars().skip(min_indent).collect()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
