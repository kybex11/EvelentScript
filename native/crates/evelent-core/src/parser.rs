use crate::ast::*;
use crate::error::{Error, Result};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    path: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, path: impl Into<String>) -> Self {
        Self {
            tokens,
            pos: 0,
            path: path.into(),
        }
    }

    pub fn parse(mut self) -> Result<Program> {
        let mut body = Vec::new();
        self.skip_newlines();
        while !self.is(TokenKind::Eof) {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(Program { body })
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.kind() {
            TokenKind::Return => {
                self.bump();
                if self.is(TokenKind::Newline)
                    || self.is(TokenKind::Dedent)
                    || self.is(TokenKind::Eof)
                    || self.is(TokenKind::RBrace)
                {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            TokenKind::Break => {
                self.bump();
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.bump();
                Ok(Stmt::Continue)
            }
            TokenKind::Throw => {
                self.bump();
                Ok(Stmt::Throw(self.parse_expr()?))
            }
            TokenKind::If | TokenKind::Unless => self.parse_if(),
            TokenKind::While | TokenKind::Until => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Class => self.parse_class(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Export => self.parse_export(),
            TokenKind::Try => self.parse_try(),
            _ => {
                let expr = self.parse_expr()?;
                // postfix if/unless: `expr if cond`
                if matches!(self.kind(), TokenKind::If | TokenKind::Unless) {
                    let inverted = self.is(TokenKind::Unless);
                    self.bump();
                    let test = self.parse_expr()?;
                    return Ok(Stmt::If {
                        test,
                        body: vec![Stmt::Expr(expr)],
                        else_body: None,
                        inverted,
                    });
                }
                // assignment detected at expression level becomes Assign stmt when top-level
                if let Expr::AssignExpr { target, value, op } = expr {
                    Ok(Stmt::Assign {
                        target,
                        value: *value,
                        op,
                    })
                } else {
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        let inverted = self.is(TokenKind::Unless);
        self.bump();
        let test = self.parse_expr()?;
        let body = self.parse_block_or_then()?;
        let else_body = if self.is(TokenKind::Else) {
            self.bump();
            if matches!(self.kind(), TokenKind::If | TokenKind::Unless) {
                Some(vec![self.parse_if()?])
            } else {
                Some(self.parse_block_or_then()?)
            }
        } else {
            None
        };
        Ok(Stmt::If {
            test,
            body,
            else_body,
            inverted,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        let inverted = self.is(TokenKind::Until);
        self.bump();
        let test = self.parse_expr()?;
        let body = self.parse_block_or_then()?;
        Ok(Stmt::While {
            test,
            body,
            inverted,
        })
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        self.bump();
        let own = if self.is(TokenKind::Own) {
            self.bump();
            true
        } else {
            false
        };
        let name = self.expect_ident()?;
        let index = if self.is(TokenKind::Comma) {
            self.bump();
            Some(self.expect_ident()?)
        } else {
            None
        };
        if !matches!(self.kind(), TokenKind::In | TokenKind::Of) {
            return Err(self.err("expected `in` or `of` in for loop"));
        }
        self.bump();
        let iter = self.parse_expr()?;
        let body = self.parse_block_or_then()?;
        Ok(Stmt::For {
            name,
            index,
            iter,
            body,
            own,
        })
    }

    fn parse_class(&mut self) -> Result<Stmt> {
        self.bump();
        let name = self.expect_ident()?;
        let superclass = if self.is(TokenKind::Extends) {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        let body = self.parse_class_body()?;
        Ok(Stmt::Class {
            name,
            superclass,
            body,
        })
    }

    fn parse_class_body(&mut self) -> Result<Vec<ClassItem>> {
        self.skip_newlines();
        self.expect(TokenKind::Indent)?;
        let mut items = Vec::new();
        while !self.is(TokenKind::Dedent) && !self.is(TokenKind::Eof) {
            self.skip_newlines();
            if self.is(TokenKind::Dedent) {
                break;
            }
            let is_static = if self.check_ident("static") {
                self.bump();
                true
            } else {
                false
            };
            let name = self.expect_ident()?;
            if self.is(TokenKind::Colon) || self.is(TokenKind::Equals) {
                self.bump();
                let value = self.parse_expr()?;
                items.push(ClassItem::Property {
                    name,
                    value,
                    is_static,
                });
            } else if self.is(TokenKind::LParen)
                || self.is(TokenKind::Arrow)
                || self.is(TokenKind::FatArrow)
            {
                let (params, body, _) = self.parse_function_tail()?;
                items.push(ClassItem::Method {
                    name,
                    params,
                    body,
                    is_static,
                });
            } else {
                return Err(self.err("expected method or property in class body"));
            }
            self.skip_newlines();
        }
        if self.is(TokenKind::Dedent) {
            self.bump();
        }
        Ok(items)
    }

    fn parse_import(&mut self) -> Result<Stmt> {
        self.bump();
        // import 'mod'
        if self.is(TokenKind::String) {
            let source = self.bump().lexeme;
            return Ok(Stmt::Import(ImportDecl::SideEffect { source }));
        }
        // import * as ns from 'mod'
        if self.is(TokenKind::Star) {
            self.bump();
            self.expect(TokenKind::As)?;
            let name = self.expect_ident()?;
            self.expect(TokenKind::From)?;
            let source = self.expect_string()?;
            return Ok(Stmt::Import(ImportDecl::Namespace { name, source }));
        }
        // import { a, b as c } from 'mod'
        if self.is(TokenKind::LBrace) {
            self.bump();
            let mut specs = Vec::new();
            while !self.is(TokenKind::RBrace) {
                let imported = self.expect_ident()?;
                let local = if self.is(TokenKind::As) {
                    self.bump();
                    self.expect_ident()?
                } else {
                    imported.clone()
                };
                specs.push((imported, local));
                if self.is(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
            self.expect(TokenKind::From)?;
            let source = self.expect_string()?;
            return Ok(Stmt::Import(ImportDecl::Named { specs, source }));
        }
        // import x from 'mod'  OR  import default as x from 'mod'
        if self.is(TokenKind::Default) {
            self.bump();
            self.expect(TokenKind::As)?;
            let name = self.expect_ident()?;
            self.expect(TokenKind::From)?;
            let source = self.expect_string()?;
            return Ok(Stmt::Import(ImportDecl::Default { name, source }));
        }
        let name = self.expect_ident()?;
        self.expect(TokenKind::From)?;
        let source = self.expect_string()?;
        Ok(Stmt::Import(ImportDecl::Default { name, source }))
    }

    fn parse_export(&mut self) -> Result<Stmt> {
        self.bump();
        if self.is(TokenKind::Default) {
            self.bump();
            return Ok(Stmt::Export(ExportDecl::Default(self.parse_expr()?)));
        }
        if self.is(TokenKind::Star) {
            self.bump();
            self.expect(TokenKind::From)?;
            let source = self.expect_string()?;
            return Ok(Stmt::Export(ExportDecl::AllFrom(source)));
        }
        if self.is(TokenKind::LBrace) {
            self.bump();
            let mut specs = Vec::new();
            while !self.is(TokenKind::RBrace) {
                let local = self.expect_ident()?;
                let exported = if self.is(TokenKind::As) {
                    self.bump();
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                specs.push((local, exported));
                if self.is(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
            if self.is(TokenKind::From) {
                self.bump();
                let source = self.expect_string()?;
                return Ok(Stmt::Export(ExportDecl::NamedFrom { specs, source }));
            }
            return Ok(Stmt::Export(ExportDecl::Named(specs)));
        }
        // export class/assignment etc.
        let stmt = self.parse_stmt()?;
        Ok(Stmt::Export(ExportDecl::Decl(Box::new(stmt))))
    }

    fn parse_try(&mut self) -> Result<Stmt> {
        self.bump();
        let body = self.parse_block_or_then()?;
        let catch = if self.is(TokenKind::Catch) {
            self.bump();
            let name = if self.is(TokenKind::Ident) {
                Some(self.bump().lexeme)
            } else {
                None
            };
            Some((name, self.parse_block_or_then()?))
        } else {
            None
        };
        let finally = if self.is(TokenKind::Finally) {
            self.bump();
            Some(self.parse_block_or_then()?)
        } else {
            None
        };
        Ok(Stmt::Try {
            body,
            catch,
            finally,
        })
    }

    fn parse_block_or_then(&mut self) -> Result<Vec<Stmt>> {
        self.skip_newlines();
        if self.is(TokenKind::Then) {
            self.bump();
            return Ok(vec![self.parse_stmt()?]);
        }
        if self.is(TokenKind::Indent) {
            return self.parse_indented_block();
        }
        // single-line body
        Ok(vec![self.parse_stmt()?])
    }

    fn parse_indented_block(&mut self) -> Result<Vec<Stmt>> {
        self.expect(TokenKind::Indent)?;
        let mut body = Vec::new();
        while !self.is(TokenKind::Dedent) && !self.is(TokenKind::Eof) {
            self.skip_newlines();
            if self.is(TokenKind::Dedent) || self.is(TokenKind::Eof) {
                break;
            }
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        if self.is(TokenKind::Dedent) {
            self.bump();
        }
        Ok(body)
    }

    // ── Expressions ──────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr> {
        let left = self.parse_or()?;
        let op = match self.kind() {
            TokenKind::Equals => Some(AssignOp::Eq),
            TokenKind::ColonEquals => Some(AssignOp::ColonEq),
            TokenKind::PlusEq => Some(AssignOp::PlusEq),
            TokenKind::MinusEq => Some(AssignOp::MinusEq),
            TokenKind::StarEq => Some(AssignOp::StarEq),
            TokenKind::SlashEq => Some(AssignOp::SlashEq),
            TokenKind::Colon => {
                // object-style key: value only handled in object literal;
                // bare `name: value` at expression-start is treated as assign in statement context
                // via Indent objects — here colon alone isn't assignment.
                None
            }
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            self.skip_newlines();
            let value = self.parse_assignment()?;
            let target = expr_to_assign_target(left)?;
            return Ok(Expr::AssignExpr {
                target,
                value: Box::new(value),
                op,
            });
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while matches!(self.kind(), TokenKind::Or | TokenKind::PipePipe) {
            self.bump();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_equality()?;
        while matches!(self.kind(), TokenKind::And | TokenKind::AmpAmp) {
            self.bump();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.kind() {
                TokenKind::Is | TokenKind::EqEq => BinaryOp::StrictEq,
                TokenKind::Isnt | TokenKind::NotEq => BinaryOp::StrictNotEq,
                _ => break,
            };
            self.bump();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.kind() {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::GtEq => BinaryOp::GtEq,
                _ => break,
            };
            self.bump();
            let right = self.parse_term()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.parse_factor()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.bump();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if matches!(self.kind(), TokenKind::Not | TokenKind::Bang) {
            self.bump();
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                arg: Box::new(self.parse_unary()?),
            });
        }
        if self.is(TokenKind::Minus) {
            self.bump();
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                arg: Box::new(self.parse_unary()?),
            });
        }
        if self.is(TokenKind::Plus) {
            self.bump();
            return Ok(Expr::Unary {
                op: UnaryOp::Pos,
                arg: Box::new(self.parse_unary()?),
            });
        }
        if self.is(TokenKind::Await) {
            self.bump();
            return Ok(Expr::Await(Box::new(self.parse_unary()?)));
        }
        if self.is(TokenKind::New) {
            self.bump();
            let primary = self.parse_primary()?;
            let callee = self.parse_call_chain(primary)?;
            let (callee, args) = match callee {
                Expr::Call { callee, args } => (*callee, args),
                other => (other, vec![]),
            };
            return Ok(Expr::New {
                callee: Box::new(callee),
                args,
            });
        }
        let primary = self.parse_primary()?;
        self.parse_call_chain(primary)
    }

    fn parse_call_chain(&mut self, mut expr: Expr) -> Result<Expr> {
        loop {
            // Existence / soaked call / binary existential: `x?` | `x? args` | `x? y`
            if self.is(TokenKind::Question) {
                self.bump();
                if self.is(TokenKind::LParen) {
                    self.bump();
                    let args = self.parse_arg_list(TokenKind::RParen)?;
                    expr = Expr::SoakedCall {
                        callee: Box::new(expr),
                        args,
                    };
                    continue;
                }
                if is_call_start(self.kind()) {
                    let args = self.parse_implicit_args()?;
                    expr = Expr::SoakedCall {
                        callee: Box::new(expr),
                        args,
                    };
                    continue;
                }
                // Binary: `a ? b`
                if !matches!(
                    self.kind(),
                    TokenKind::Newline
                        | TokenKind::Dedent
                        | TokenKind::Eof
                        | TokenKind::RParen
                        | TokenKind::RBrace
                        | TokenKind::RBracket
                        | TokenKind::Comma
                        | TokenKind::Else
                        | TokenKind::Then
                        | TokenKind::Indent
                ) {
                    let default = self.parse_unary()?;
                    expr = Expr::ExistentialDefault {
                        value: Box::new(expr),
                        default: Box::new(default),
                    };
                    continue;
                }
                expr = Expr::Existence(Box::new(expr));
                continue;
            }
            match self.kind() {
                TokenKind::LParen => {
                    self.bump();
                    let args = self.parse_arg_list(TokenKind::RParen)?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                TokenKind::Dot => {
                    self.bump();
                    let optional = false;
                    let property = self.expect_ident()?;
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property: Box::new(Expr::String(property)),
                        computed: false,
                        optional,
                    };
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property: Box::new(index),
                        computed: true,
                        optional: false,
                    };
                }
                // optional soak member: ?.
                // (handled if we later add QuestionDot token)

                // implicit call: ident/member followed by primary-looking token
                TokenKind::Ident
                | TokenKind::Number
                | TokenKind::String
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::At
                | TokenKind::LBrace
                | TokenKind::Arrow
                | TokenKind::FatArrow
                    if can_implicit_call(&expr) =>
                {
                    let args = self.parse_implicit_args()?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    /// First arg + optional `, arg, …` (CoffeeScript `f a, b`).
    fn parse_implicit_args(&mut self) -> Result<Vec<Expr>> {
        let first = if matches!(self.kind(), TokenKind::Arrow | TokenKind::FatArrow) {
            self.finish_function(vec![])?
        } else {
            self.parse_unary()?
        };
        let mut args = vec![first];
        while self.is(TokenKind::Comma) {
            self.bump();
            self.skip_newlines();
            if matches!(self.kind(), TokenKind::Arrow | TokenKind::FatArrow) {
                args.push(self.finish_function(vec![])?);
            } else {
                args.push(self.parse_assignment()?);
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.kind() {
            TokenKind::Number => Ok(Expr::Number(self.bump().lexeme)),
            TokenKind::String => Ok(Expr::String(self.bump().lexeme)),
            TokenKind::True => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.bump();
                Ok(Expr::Bool(false))
            }
            TokenKind::Null => {
                self.bump();
                Ok(Expr::Null)
            }
            TokenKind::Undefined => {
                self.bump();
                Ok(Expr::Undefined)
            }
            TokenKind::At => {
                self.bump();
                if self.is(TokenKind::Ident) {
                    let prop = self.bump().lexeme;
                    Ok(Expr::Member {
                        object: Box::new(Expr::This),
                        property: Box::new(Expr::String(prop)),
                        computed: false,
                        optional: false,
                    })
                } else {
                    Ok(Expr::This)
                }
            }
            TokenKind::Require => {
                self.bump();
                // `require? …` / bare `require` → identifier; `require 'x'` → require call
                if self.is(TokenKind::Question)
                    || self.is(TokenKind::Dot)
                    || self.is(TokenKind::LBracket)
                    || matches!(
                        self.kind(),
                        TokenKind::Newline
                            | TokenKind::Dedent
                            | TokenKind::Eof
                            | TokenKind::RParen
                            | TokenKind::RBrace
                            | TokenKind::RBracket
                            | TokenKind::Comma
                            | TokenKind::Else
                            | TokenKind::Then
                    )
                {
                    Ok(Expr::Ident("require".into()))
                } else if self.is(TokenKind::LParen) {
                    self.bump();
                    let s = self.expect_string()?;
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Require(s))
                } else if self.is(TokenKind::String) {
                    Ok(Expr::Require(self.bump().lexeme))
                } else {
                    // e.g. require somethingElse — treat as Ident and let call_chain continue
                    Ok(Expr::Ident("require".into()))
                }
            }
            TokenKind::Ident => {
                let name = self.bump().lexeme;
                Ok(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.bump();
                // could be grouped expr OR function params
                if self.is(TokenKind::RParen) {
                    self.bump();
                    return self.finish_function(vec![]);
                }
                // Look ahead: if we see ident (comma|paren) then arrow → params
                if self.is_function_params() {
                    let params = self.parse_param_list(TokenKind::RParen)?;
                    return self.finish_function(params);
                }
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                if matches!(self.kind(), TokenKind::Arrow | TokenKind::FatArrow) {
                    // (x) ->  where x was parsed as expr — only if Ident
                    let params = match expr {
                        Expr::Ident(n) => vec![Param {
                            name: n,
                            default: None,
                            rest: false,
                        }],
                        _ => return Err(self.err("invalid function parameters")),
                    };
                    return self.finish_function(params);
                }
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.bump();
                let mut els = Vec::new();
                self.skip_newlines();
                while !self.is(TokenKind::RBracket) {
                    els.push(self.parse_expr()?);
                    self.skip_newlines();
                    if self.is(TokenKind::Comma) {
                        self.bump();
                        self.skip_newlines();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::Array(els))
            }
            TokenKind::LBrace => self.parse_object_literal(),
            TokenKind::Indent => {
                // indented object / block
                self.parse_indented_object_or_block()
            }
            TokenKind::Arrow | TokenKind::FatArrow => self.finish_function(vec![]),
            TokenKind::Async => {
                self.bump();
                if self.is(TokenKind::LParen) {
                    self.bump();
                    let params = self.parse_param_list(TokenKind::RParen)?;
                    let mut f = self.finish_function(params)?;
                    if let Expr::Func { async_, .. } = &mut f {
                        *async_ = true;
                    }
                    Ok(f)
                } else {
                    Err(self.err("expected function after async"))
                }
            }
            _ => Err(self.err(&format!("unexpected token `{}`", self.current().lexeme))),
        }
    }

    fn parse_indented_object_or_block(&mut self) -> Result<Expr> {
        // Peek: if first stmt looks like `key: value`, treat as object
        self.expect(TokenKind::Indent)?;
        let save = self.pos;
        let is_object = self.is(TokenKind::Ident) || self.is(TokenKind::String);
        if is_object {
            let _ = self.bump();
            let is_obj = self.is(TokenKind::Colon);
            self.pos = save;
            if is_obj {
                let mut props = Vec::new();
                while !self.is(TokenKind::Dedent) && !self.is(TokenKind::Eof) {
                    self.skip_newlines();
                    if self.is(TokenKind::Dedent) {
                        break;
                    }
                    let key = if self.is(TokenKind::String) {
                        ObjectKey::String(self.bump().lexeme)
                    } else {
                        ObjectKey::Ident(self.expect_ident()?)
                    };
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expr()?;
                    props.push((key, value));
                    self.skip_newlines();
                }
                if self.is(TokenKind::Dedent) {
                    self.bump();
                }
                return Ok(Expr::Object(props));
            }
        }
        // block of statements as expression
        self.pos = save;
        let mut body = Vec::new();
        while !self.is(TokenKind::Dedent) && !self.is(TokenKind::Eof) {
            self.skip_newlines();
            if self.is(TokenKind::Dedent) {
                break;
            }
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        if self.is(TokenKind::Dedent) {
            self.bump();
        }
        Ok(Expr::Block(body))
    }

    fn parse_object_literal(&mut self) -> Result<Expr> {
        self.bump(); // {
        let mut props = Vec::new();
        self.skip_newlines();
        while !self.is(TokenKind::RBrace) {
            let key = if self.is(TokenKind::String) {
                ObjectKey::String(self.bump().lexeme)
            } else if self.is(TokenKind::LBracket) {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RBracket)?;
                ObjectKey::Computed(e)
            } else {
                ObjectKey::Ident(self.expect_ident()?)
            };
            if self.is(TokenKind::Colon) {
                self.bump();
                let value = self.parse_expr()?;
                props.push((key, value));
            } else {
                // shorthand
                match &key {
                    ObjectKey::Ident(n) => {
                        props.push((key.clone(), Expr::Ident(n.clone())));
                    }
                    _ => return Err(self.err("invalid object shorthand")),
                }
            }
            self.skip_newlines();
            if self.is(TokenKind::Comma) {
                self.bump();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::Object(props))
    }

    fn finish_function(&mut self, params: Vec<Param>) -> Result<Expr> {
        let bound = self.is(TokenKind::FatArrow);
        if !matches!(self.kind(), TokenKind::Arrow | TokenKind::FatArrow) {
            return Err(self.err("expected -> or =>"));
        }
        self.bump();
        self.skip_newlines();
        if self.is(TokenKind::Indent) {
            let body = self.parse_indented_block()?;
            Ok(Expr::Func {
                params,
                body,
                expression: false,
                bound,
                async_: false,
            })
        } else {
            let expr = self.parse_expr()?;
            Ok(Expr::Func {
                params,
                body: vec![Stmt::Return(Some(expr))],
                expression: true,
                bound,
                async_: false,
            })
        }
    }

    fn parse_function_tail(&mut self) -> Result<(Vec<Param>, Vec<Stmt>, bool)> {
        let params = if self.is(TokenKind::LParen) {
            self.bump();
            self.parse_param_list(TokenKind::RParen)?
        } else {
            vec![]
        };
        let bound = self.is(TokenKind::FatArrow);
        self.expect_arrow()?;
        self.skip_newlines();
        let body = if self.is(TokenKind::Indent) {
            self.parse_indented_block()?
        } else {
            let expr = self.parse_expr()?;
            vec![Stmt::Return(Some(expr))]
        };
        Ok((params, body, bound))
    }

    fn parse_param_list(&mut self, end: TokenKind) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        while !self.is(end.clone()) {
            let rest = if self.is(TokenKind::Ellipsis) {
                self.bump();
                true
            } else {
                false
            };
            let name = self.expect_ident()?;
            let default = if self.is(TokenKind::Equals) {
                self.bump();
                Some(self.parse_assignment()?)
            } else {
                None
            };
            params.push(Param {
                name,
                default,
                rest,
            });
            if self.is(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(end)?;
        Ok(params)
    }

    fn parse_arg_list(&mut self, end: TokenKind) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        self.skip_newlines();
        while !self.is(end.clone()) {
            args.push(self.parse_assignment()?);
            self.skip_newlines();
            if self.is(TokenKind::Comma) {
                self.bump();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.expect(end)?;
        Ok(args)
    }

    fn is_function_params(&self) -> bool {
        // Heuristic: Ident ( , Ident )* ) (->|=>)
        let mut i = self.pos;
        let kind_at = |i: usize| self.tokens.get(i).map(|t| &t.kind);
        if kind_at(i) != Some(&TokenKind::Ident) && kind_at(i) != Some(&TokenKind::Ellipsis) {
            // empty already handled; single expr group
            // if next is ) -> then params
            if kind_at(i) == Some(&TokenKind::RParen) {
                return matches!(
                    kind_at(i + 1),
                    Some(TokenKind::Arrow | TokenKind::FatArrow)
                );
            }
            return false;
        }
        // scan ahead for ) ->
        while i < self.tokens.len() {
            match kind_at(i) {
                Some(TokenKind::RParen) => {
                    return matches!(
                        kind_at(i + 1),
                        Some(TokenKind::Arrow | TokenKind::FatArrow)
                    );
                }
                Some(TokenKind::Eof) => return false,
                _ => i += 1,
            }
        }
        false
    }

    // ── helpers ──────────────────────────────────────────────────

    fn expect_arrow(&mut self) -> Result<()> {
        if matches!(self.kind(), TokenKind::Arrow | TokenKind::FatArrow) {
            self.bump();
            Ok(())
        } else {
            Err(self.err("expected -> or =>"))
        }
    }

    fn check_ident(&self, name: &str) -> bool {
        self.is(TokenKind::Ident) && self.current().lexeme == name
    }

    fn expect_ident(&mut self) -> Result<String> {
        // allow some keywords as property/ident names in limited contexts
        if self.is(TokenKind::Ident) {
            return Ok(self.bump().lexeme);
        }
        // from/as/default as idents when expected
        if matches!(
            self.kind(),
            TokenKind::From | TokenKind::As | TokenKind::Default | TokenKind::Own
        ) {
            return Ok(self.bump().lexeme);
        }
        Err(self.err("expected identifier"))
    }

    fn expect_string(&mut self) -> Result<String> {
        if self.is(TokenKind::String) {
            Ok(self.bump().lexeme)
        } else {
            Err(self.err("expected string"))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token> {
        if self.is(kind.clone()) {
            Ok(self.bump())
        } else {
            Err(self.err(&format!("expected {kind:?}")))
        }
    }

    fn skip_newlines(&mut self) {
        while self.is(TokenKind::Newline) {
            self.bump();
        }
    }

    fn is(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }

    fn kind(&self) -> TokenKind {
        self.current().kind.clone()
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("tokens"))
    }

    fn bump(&mut self) -> Token {
        let t = self.current().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn err(&self, msg: &str) -> Error {
        let t = self.current();
        Error::syntax(&self.path, t.span.line, t.span.column, msg)
    }
}

fn can_implicit_call(expr: &Expr) -> bool {
    // Do not continue after Call — otherwise
    //   test "a", ->
    //   test "b", ->
    // becomes test(...)(test(...)). Explicit `()` still chains.
    matches!(
        expr,
        Expr::Ident(_) | Expr::Member { .. } | Expr::This | Expr::Existence(_)
    )
}

fn is_call_start(kind: TokenKind) -> bool {
    // After `?`, Ident starts a binary existential (`a ? b`), not a soaked call.
    // Soaked calls use literals / parens: `require? 'vm'`, `fn?(x)`, `fn? ->`.
    matches!(
        kind,
        TokenKind::Number
            | TokenKind::String
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
            | TokenKind::LBrace
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::Arrow
            | TokenKind::FatArrow
    )
}

fn expr_to_assign_target(expr: Expr) -> Result<AssignTarget> {
    match expr {
        Expr::Ident(n) => Ok(AssignTarget::Ident(n)),
        Expr::Member {
            object,
            property,
            computed,
            ..
        } => Ok(AssignTarget::Member {
            object,
            property,
            computed,
        }),
        Expr::Array(els) => {
            let mut targets = Vec::new();
            for el in els {
                targets.push(Some(expr_to_assign_target(el)?));
            }
            Ok(AssignTarget::Array(targets))
        }
        other => Err(Error::Other(format!(
            "invalid assignment target: {other:?}"
        ))),
    }
}
