#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Span {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident,
    Number,
    String,
    Regex,

    // Keywords
    If,
    Else,
    Unless,
    Then,
    While,
    Until,
    For,
    In,
    Of,
    When,
    Return,
    Break,
    Continue,
    Class,
    Extends,
    Super,
    New,
    Try,
    Catch,
    Finally,
    Throw,
    Switch,
    WhenArm, // reserved
    True,
    False,
    Null,
    Undefined,
    Yes,
    No,
    On,
    Off,
    And,
    Or,
    Not,
    Is,
    Isnt,
    Import,
    Export,
    From,
    As,
    Default,
    Require,
    Native,
    Do,
    Own,
    Await,
    Async,
    Yield,

    // Punctuation / operators
    Arrow,       // ->
    FatArrow,    // =>
    Equals,      // =
    ColonEquals, // :=
    Colon,
    Comma,
    Dot,
    Question,    // ?
    Bang,        // !
    At,          // @
    Ellipsis,    // ...
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AmpAmp,
    PipePipe,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Indent,
    Dedent,
    Newline,
    Eof,
}
