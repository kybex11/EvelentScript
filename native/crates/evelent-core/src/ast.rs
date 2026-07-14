#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Assign {
        target: AssignTarget,
        value: Expr,
        op: AssignOp,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    If {
        test: Expr,
        body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        inverted: bool, // unless
    },
    While {
        test: Expr,
        body: Vec<Stmt>,
        inverted: bool, // until
    },
    For {
        name: String,
        index: Option<String>,
        iter: Expr,
        body: Vec<Stmt>,
        own: bool,
    },
    Class {
        name: String,
        superclass: Option<Expr>,
        body: Vec<ClassItem>,
    },
    Import(ImportDecl),
    Export(ExportDecl),
    Throw(Expr),
    Try {
        body: Vec<Stmt>,
        catch: Option<(Option<String>, Vec<Stmt>)>,
        finally: Option<Vec<Stmt>>,
    },
}

#[derive(Debug, Clone)]
pub enum ClassItem {
    Method {
        name: String,
        params: Vec<Param>,
        body: Vec<Stmt>,
        is_static: bool,
    },
    Property {
        name: String,
        value: Expr,
        is_static: bool,
    },
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Ident(String),
    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
    },
    Array(Vec<Option<AssignTarget>>),
    Object(Vec<(String, AssignTarget)>),
}

#[derive(Debug, Clone, Copy)]
pub enum AssignOp {
    Eq,
    ColonEq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub rest: bool,
}

#[derive(Debug, Clone)]
pub enum ImportDecl {
    /// `import x from 'mod'` / `import default as x from 'mod'`
    Default { name: String, source: String },
    /// `import { a, b as c } from 'mod'`
    Named { specs: Vec<(String, String)>, source: String },
    /// `import * as ns from 'mod'`
    Namespace { name: String, source: String },
    /// `import 'mod'` side-effect
    SideEffect { source: String },
}

#[derive(Debug, Clone)]
pub enum ExportDecl {
    Default(Expr),
    Named(Vec<(String, Option<String>)>),
    AllFrom(String),
    NamedFrom {
        specs: Vec<(String, Option<String>)>,
        source: String,
    },
    Decl(Box<Stmt>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(String),
    Number(String),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    This,
    Array(Vec<Expr>),
    Object(Vec<(ObjectKey, Expr)>),
    Unary {
        op: UnaryOp,
        arg: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: Box<Expr>,
        computed: bool,
        optional: bool,
    },
    Func {
        params: Vec<Param>,
        body: Vec<Stmt>,
        expression: bool, // single-expr arrow body
        bound: bool,      // => vs ->
        async_: bool,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Require(String),
    /// Soaked call: `fn? args` → call only if fn exists
    SoakedCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Await(Box<Expr>),
    Existence(Box<Expr>), // x?
    /// Binary existential: `a ? b` → `(a != null ? a : b)`
    ExistentialDefault {
        value: Box<Expr>,
        default: Box<Expr>,
    },
    AssignExpr {
        target: AssignTarget,
        value: Box<Expr>,
        op: AssignOp,
    },
    IfExpr {
        test: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        inverted: bool,
    },
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub enum ObjectKey {
    Ident(String),
    String(String),
    Computed(Expr),
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
    TypeOf,
    Delete,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}
