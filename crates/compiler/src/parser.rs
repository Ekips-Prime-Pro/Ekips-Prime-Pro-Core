//! Recursive-descent parser with error recovery.
//!
//! The parser never stops at the first error. It reports the problem,
//! skips to a safe point (usually the next line) and continues, so a
//! student sees all problems at once instead of one per compile run.

use crate::ast::*;
use crate::diagnostic::{Diagnostic, ErrorKind, Expected};
use crate::keywords::{Keyword, STATEMENT_KEYWORDS};
use crate::lexer::{Punct, Token, TokenKind};
use crate::span::Span;
use crate::suggest::suggest;

/// Per-function context used to check `await`, `return`, `break` and `continue`.
#[derive(Debug, Clone, Copy)]
struct FnCtx {
    is_async: bool,
    is_function: bool,
    loop_depth: usize,
}

/// Parses a token stream (as produced by [`crate::lexer::lex`]).
pub struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
    last_end: usize,
    diags: Vec<Diagnostic>,
    ctx: Vec<FnCtx>,
}

type PResult<T> = Option<T>;

impl<'t> Parser<'t> {
    /// Creates a parser. `tokens` must end with an `Eof` token.
    pub fn new(tokens: &'t [Token]) -> Self {
        assert!(
            tokens.last().is_some_and(|t| t.kind == TokenKind::Eof),
            "token stream must end with Eof"
        );
        Parser {
            tokens,
            pos: 0,
            last_end: 0,
            diags: Vec::new(),
            ctx: vec![FnCtx {
                is_async: false,
                is_function: false,
                loop_depth: 0,
            }],
        }
    }

    /// Parses a whole program and returns it with all problems found.
    pub fn parse_program(mut self) -> (Program, Vec<Diagnostic>) {
        let mut body = Vec::new();
        loop {
            if self.at_eof() {
                break;
            }
            if self.at(Punct::RBrace) {
                let span = self.bump().span;
                self.error(ErrorKind::UnmatchedCloseBrace, span);
                continue;
            }
            match self.parse_statement() {
                Some(stmt) => body.push(stmt),
                None => self.synchronize(),
            }
        }
        let program = Program { body };
        self.check_main(&program);
        (program, self.diags)
    }

    // ------------------------------------------------------------------
    // Token helpers
    // ------------------------------------------------------------------

    fn peek(&self) -> &'t Token {
        &self.tokens[self.pos]
    }

    fn peek_at(&self, n: usize) -> &'t Token {
        let idx = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    fn at_eof(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn at(&self, p: Punct) -> bool {
        self.peek().is(p)
    }

    fn at_kw(&self, k: Keyword) -> bool {
        self.peek().is_kw(k)
    }

    fn bump(&mut self) -> &'t Token {
        let tok = self.peek();
        if tok.kind != TokenKind::Eof {
            self.pos += 1;
            self.last_end = tok.span.end;
        }
        tok
    }

    fn eat(&mut self, p: Punct) -> bool {
        if self.at(p) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, k: Keyword) -> bool {
        if self.at_kw(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn error(&mut self, kind: ErrorKind, span: Span) {
        self.diags.push(Diagnostic::new(kind, span));
    }

    fn error_expected(&mut self, expected: Expected) {
        let tok = self.peek();
        self.error(
            ErrorKind::Expected {
                expected,
                found: tok.found(),
            },
            tok.span,
        );
    }

    fn expect(&mut self, p: Punct, text: &'static str) -> PResult<Span> {
        if self.at(p) {
            Some(self.bump().span)
        } else {
            self.error_expected(Expected::Token(text));
            None
        }
    }

    /// Span from `start` to the end of the last consumed token.
    fn span_from(&self, start: Span) -> Span {
        Span::new(start.start, self.last_end.max(start.end))
    }

    fn ctx(&self) -> FnCtx {
        *self.ctx.last().expect("context stack is never empty")
    }

    /// Skips tokens until a safe point to continue: after a `;`, before a
    /// `}`, before the first token of the next line, or at the end.
    ///
    /// Braces opened while skipping are skipped together with their
    /// contents, so a broken line like `wenn (x) { … }` does not close the
    /// surrounding block by accident.
    fn synchronize(&mut self) {
        let start = self.pos;
        let mut depth = 0usize;
        loop {
            let tok = self.peek();
            if tok.kind == TokenKind::Eof {
                return;
            }
            if tok.is(Punct::RBrace) {
                if depth == 0 {
                    return;
                }
                depth -= 1;
                self.bump();
                continue;
            }
            if depth == 0 && self.pos > start && tok.newline_before {
                return;
            }
            self.bump();
            if tok.is(Punct::LBrace) {
                depth += 1;
            } else if depth == 0 && tok.is(Punct::Semi) {
                return;
            }
        }
    }

    /// Accepts the end of a statement: `;`, a line break, `}` or the end of file.
    fn consume_semicolon(&mut self) {
        if self.eat(Punct::Semi) {
            return;
        }
        let tok = self.peek();
        if tok.kind == TokenKind::Eof || tok.is(Punct::RBrace) || tok.newline_before {
            return;
        }
        self.error(ErrorKind::MissingSemicolon(tok.found()), tok.span);
        self.synchronize();
    }

    fn at_statement_end(&self) -> bool {
        let tok = self.peek();
        tok.is(Punct::Semi)
            || tok.is(Punct::RBrace)
            || tok.kind == TokenKind::Eof
            || tok.newline_before
    }

    fn expect_name(&mut self) -> PResult<Ident> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Ident(name) => {
                self.bump();
                Some(Ident {
                    name: name.clone(),
                    span: tok.span,
                })
            }
            _ => {
                self.error_expected(Expected::Name);
                None
            }
        }
    }

    /// Skips a TypeScript type annotation after `:` and reports that types
    /// are not needed. Stops before `=`, `,`, `)`, `;`, `{`, `=>` or a new line.
    fn skip_type_annotation(&mut self) {
        let colon = self.bump().span;
        let mut depth = 0usize;
        loop {
            let tok = self.peek();
            let stop = tok.kind == TokenKind::Eof
                || tok.newline_before
                || (depth == 0
                    && (tok.is(Punct::Assign)
                        || tok.is(Punct::Comma)
                        || tok.is(Punct::RParen)
                        || tok.is(Punct::Semi)
                        || tok.is(Punct::LBrace)
                        || tok.is(Punct::Arrow)));
            if stop {
                break;
            }
            if tok.is(Punct::Lt) {
                depth += 1;
            } else if tok.is(Punct::Gt) {
                depth = depth.saturating_sub(1);
            }
            self.bump();
        }
        let span = self.span_from(colon);
        self.error(ErrorKind::TypesNotNeeded, span);
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    fn parse_statement(&mut self) -> PResult<Stmt> {
        let tok = self.peek();
        let start = tok.span;
        match &tok.kind {
            TokenKind::Punct(Punct::LBrace) => {
                let body = self.parse_block()?;
                Some(Stmt {
                    kind: StmtKind::Block(body),
                    span: self.span_from(start),
                })
            }
            TokenKind::Punct(Punct::Semi) => {
                self.bump();
                Some(Stmt {
                    kind: StmtKind::Empty,
                    span: start,
                })
            }
            TokenKind::Keyword(Keyword::Else) => {
                self.bump();
                self.error(ErrorKind::UnexpectedElse, start);
                // Parse whatever follows (`if …` or `{ … }`) to keep the structure intact.
                self.parse_statement()
            }
            TokenKind::Keyword(
                kw @ (Keyword::Async
                | Keyword::Const
                | Keyword::Let
                | Keyword::Function
                | Keyword::If
                | Keyword::While
                | Keyword::For
                | Keyword::Return
                | Keyword::Break
                | Keyword::Continue),
            ) => {
                // `async` only starts a statement when `function` follows.
                if *kw == Keyword::Async && !self.peek_at(1).is_kw(Keyword::Function) {
                    return self.parse_expr_statement();
                }
                self.bump();
                self.parse_keyword_statement(*kw, start)
            }
            TokenKind::Ident(word) if word == "var" && self.next_is_name_on_same_line() => {
                self.bump();
                self.error(ErrorKind::VarKeyword, start);
                self.parse_keyword_statement(Keyword::Let, start)
            }
            TokenKind::Ident(word) if self.looks_like_block_header() => {
                // `wenn (x > 3) {` or `whiel (…) {`: a misspelled `if`, `while` or `for`.
                let suggestion = suggest(word, &["if", "while", "for"]);
                let Some(kw) = suggestion.and_then(Keyword::lookup) else {
                    return self.parse_expr_statement();
                };
                self.bump();
                self.error(
                    ErrorKind::UnknownWord {
                        word: word.clone(),
                        suggestion,
                    },
                    start,
                );
                self.parse_keyword_statement(kw, start)
            }
            TokenKind::Ident(word) if self.next_is_name_on_same_line() => {
                // Two words in a row: probably a misspelled keyword (`cosnt x = 1`).
                let suggestion = suggest(word, STATEMENT_KEYWORDS);
                self.bump();
                self.error(
                    ErrorKind::UnknownWord {
                        word: word.clone(),
                        suggestion,
                    },
                    start,
                );
                // Continue as if the suggested keyword had been written,
                // so one typo does not cause a cascade of errors.
                match suggestion.and_then(Keyword::lookup) {
                    Some(kw) => self.parse_keyword_statement(kw, start),
                    None => None,
                }
            }
            _ => self.parse_expr_statement(),
        }
    }

    /// `name ( … ) {` on one line – looks like `if`/`while`/`for`, not like a call.
    fn looks_like_block_header(&self) -> bool {
        let open = self.pos + 1;
        if !self.tokens[open].is(Punct::LParen) || self.tokens[open].newline_before {
            return false;
        }
        self.after_paren_group(open)
            .is_some_and(|t| t.is(Punct::LBrace) && !t.newline_before)
    }

    /// The token after the `)` matching the `(` at index `open`.
    fn after_paren_group(&self, open: usize) -> Option<&'t Token> {
        let mut depth = 0usize;
        for (j, t) in self.tokens.iter().enumerate().skip(open) {
            match t.kind {
                TokenKind::Punct(Punct::LParen) => depth += 1,
                TokenKind::Punct(Punct::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        return self.tokens.get(j + 1);
                    }
                }
                TokenKind::Eof => return None,
                _ => {}
            }
        }
        None
    }

    fn next_is_name_on_same_line(&self) -> bool {
        let next = self.peek_at(1);
        !next.newline_before && matches!(next.kind, TokenKind::Ident(_) | TokenKind::Keyword(_))
    }

    /// Parses a statement whose keyword (`kw`) has already been consumed.
    fn parse_keyword_statement(&mut self, kw: Keyword, start: Span) -> PResult<Stmt> {
        let kind = match kw {
            Keyword::Const | Keyword::Let => {
                let decl = self.parse_var_decl(kw, start)?;
                self.consume_semicolon();
                decl
            }
            Keyword::Async => {
                if !self.eat_kw(Keyword::Function) {
                    self.error_expected(Expected::Token("function"));
                    return None;
                }
                StmtKind::Function(self.parse_function(true, start)?)
            }
            Keyword::Function => StmtKind::Function(self.parse_function(false, start)?),
            Keyword::If => self.parse_if()?,
            Keyword::While => {
                let cond = self.parse_paren_condition();
                let body = self.parse_loop_body();
                StmtKind::While {
                    cond,
                    body: Box::new(body),
                }
            }
            Keyword::For => self.parse_for()?,
            Keyword::Return => {
                if !self.ctx().is_function {
                    self.error(ErrorKind::ReturnOutsideFunction, start);
                }
                let value = if self.at_statement_end() {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.consume_semicolon();
                StmtKind::Return(value)
            }
            Keyword::Break | Keyword::Continue => {
                if self.ctx().loop_depth == 0 {
                    self.error(ErrorKind::OutsideLoop(kw.as_str()), start);
                }
                self.consume_semicolon();
                if kw == Keyword::Break {
                    StmtKind::Break
                } else {
                    StmtKind::Continue
                }
            }
            // Other keywords never start a statement; they are handled as
            // expressions by the caller.
            _ => {
                self.error(
                    ErrorKind::Expected {
                        expected: Expected::Statement,
                        found: crate::diagnostic::Found::Word(kw.as_str().into()),
                    },
                    start,
                );
                return None;
            }
        };
        Some(Stmt {
            kind,
            span: self.span_from(start),
        })
    }

    fn parse_expr_statement(&mut self) -> PResult<Stmt> {
        let expr = self.parse_expr()?;
        self.consume_semicolon();
        let span = expr.span;
        Some(Stmt {
            kind: StmtKind::Expr(expr),
            span,
        })
    }

    /// `{ … }` – returns the statements inside.
    fn parse_block(&mut self) -> PResult<Vec<Stmt>> {
        let open = self.expect(Punct::LBrace, "{")?;
        let mut body = Vec::new();
        loop {
            if self.eat(Punct::RBrace) {
                return Some(body);
            }
            if self.at_eof() {
                self.error(ErrorKind::MissingCloseBrace, open);
                return Some(body);
            }
            match self.parse_statement() {
                Some(stmt) => body.push(stmt),
                None => self.synchronize(),
            }
        }
    }

    /// Body of `if`, `else`, `while` and `for`. Never fails: on error an
    /// empty statement is returned after skipping to a safe point.
    fn parse_body(&mut self) -> Stmt {
        let start = self.peek().span;
        match self.parse_statement() {
            Some(stmt) => stmt,
            None => {
                self.synchronize();
                Stmt {
                    kind: StmtKind::Empty,
                    span: start,
                }
            }
        }
    }

    fn parse_loop_body(&mut self) -> Stmt {
        self.ctx.last_mut().expect("context").loop_depth += 1;
        let body = self.parse_body();
        self.ctx.last_mut().expect("context").loop_depth -= 1;
        body
    }

    /// `( expr )` after `if` / `while`. Never fails.
    fn parse_paren_condition(&mut self) -> Expr {
        let start = self.peek().span;
        let has_paren = self.expect(Punct::LParen, "(").is_some();
        let cond = self.parse_expr().unwrap_or_else(|| Expr::error(start));
        if has_paren {
            self.expect(Punct::RParen, ")");
        }
        cond
    }

    /// `name [= value]` after `const` / `let` (keyword already consumed).
    fn parse_var_decl(&mut self, kw: Keyword, start: Span) -> PResult<StmtKind> {
        let kind = if kw == Keyword::Const {
            DeclKind::Const
        } else {
            DeclKind::Let
        };
        let name = self.expect_name()?;
        if self.at(Punct::Colon) {
            self.skip_type_annotation();
        }
        let init = if self.eat(Punct::Assign) {
            Some(self.parse_expr()?)
        } else {
            if kind == DeclKind::Const {
                self.error_expected(Expected::Token("="));
            }
            None
        };
        let _ = start;
        Some(StmtKind::VarDecl { kind, name, init })
    }

    /// Parameter list including the brackets: `(a, b)`.
    fn parse_params(&mut self) -> PResult<Vec<Ident>> {
        self.expect(Punct::LParen, "(")?;
        let mut params = Vec::new();
        if self.eat(Punct::RParen) {
            return Some(params);
        }
        loop {
            params.push(self.expect_name()?);
            if self.at(Punct::Colon) {
                self.skip_type_annotation();
            }
            if self.eat(Punct::Comma) {
                if self.eat(Punct::RParen) {
                    return Some(params);
                }
                continue;
            }
            if self.eat(Punct::RParen) {
                return Some(params);
            }
            self.error_expected(Expected::CommaOr(")"));
            return None;
        }
    }

    /// `name(params) { … }` after `function` (keywords already consumed).
    fn parse_function(&mut self, is_async: bool, start: Span) -> PResult<Function> {
        let name = self.expect_name()?;
        let params = self.parse_params()?;
        if self.at(Punct::Colon) {
            self.skip_type_annotation();
        }
        let span = self.span_from(start);
        self.ctx.push(FnCtx {
            is_async,
            is_function: true,
            loop_depth: 0,
        });
        let body = self.parse_block();
        self.ctx.pop();
        Some(Function {
            name,
            params,
            body: body?,
            is_async,
            span,
        })
    }

    /// `(cond) stmt [else stmt]` after `if`.
    fn parse_if(&mut self) -> PResult<StmtKind> {
        let cond = self.parse_paren_condition();
        let then = self.parse_body();
        let otherwise = if self.eat_kw(Keyword::Else) {
            Some(Box::new(self.parse_body()))
        } else {
            None
        };
        Some(StmtKind::If {
            cond,
            then: Box::new(then),
            otherwise,
        })
    }

    /// `(init; cond; update) stmt` or `(const x of list) stmt` after `for`.
    fn parse_for(&mut self) -> PResult<StmtKind> {
        self.expect(Punct::LParen, "(")?;

        // for (const x of list)
        let decl_kw = match self.peek().kind {
            TokenKind::Keyword(kw @ (Keyword::Const | Keyword::Let)) => Some(kw),
            _ => None,
        };
        if let Some(kw) = decl_kw {
            if self.peek_at(2).is_kw(Keyword::Of) {
                self.bump();
                let name = self.expect_name()?;
                self.bump(); // `of`
                let iterable = self.parse_expr()?;
                self.expect(Punct::RParen, ")")?;
                let body = self.parse_loop_body();
                let kind = if kw == Keyword::Const {
                    DeclKind::Const
                } else {
                    DeclKind::Let
                };
                return Some(StmtKind::ForOf {
                    kind,
                    name,
                    iterable,
                    body: Box::new(body),
                });
            }
        }

        // for (init; cond; update)
        let init = if self.at(Punct::Semi) {
            None
        } else if let Some(kw) = decl_kw {
            let start = self.bump().span;
            let kind = self.parse_var_decl(kw, start)?;
            Some(Box::new(Stmt {
                kind,
                span: self.span_from(start),
            }))
        } else {
            let expr = self.parse_expr()?;
            let span = expr.span;
            Some(Box::new(Stmt {
                kind: StmtKind::Expr(expr),
                span,
            }))
        };
        self.expect(Punct::Semi, ";")?;
        let cond = if self.at(Punct::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(Punct::Semi, ";")?;
        let update = if self.at(Punct::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(Punct::RParen, ")")?;
        let body = self.parse_loop_body();
        Some(StmtKind::For {
            init,
            cond,
            update,
            body: Box::new(body),
        })
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    /// Parses an expression.
    pub fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> PResult<Expr> {
        if self.arrow_ahead() {
            return self.parse_arrow();
        }
        let lhs = self.parse_conditional()?;
        let op = match self.peek().kind {
            TokenKind::Punct(Punct::Assign) => AssignOp::Assign,
            TokenKind::Punct(Punct::PlusAssign) => AssignOp::Add,
            TokenKind::Punct(Punct::MinusAssign) => AssignOp::Sub,
            TokenKind::Punct(Punct::StarAssign) => AssignOp::Mul,
            TokenKind::Punct(Punct::SlashAssign) => AssignOp::Div,
            TokenKind::Punct(Punct::PercentAssign) => AssignOp::Rem,
            _ => return Some(lhs),
        };
        self.check_assignable(&lhs);
        self.bump();
        let value = self.parse_assignment()?;
        let span = lhs.span.to(value.span);
        Some(Expr {
            kind: ExprKind::Assign {
                op,
                target: Box::new(lhs),
                value: Box::new(value),
            },
            span,
        })
    }

    fn check_assignable(&mut self, target: &Expr) {
        let ok = matches!(
            target.kind,
            ExprKind::Ident(_) | ExprKind::Member { .. } | ExprKind::Index { .. } | ExprKind::Error
        );
        if !ok {
            self.error(ErrorKind::InvalidAssignTarget, target.span);
        }
    }

    /// Looks ahead to see whether an arrow function starts here.
    fn arrow_ahead(&self) -> bool {
        let mut i = self.pos;
        if self.tokens[i].is_kw(Keyword::Async) {
            i += 1;
        }
        let tok = &self.tokens[i];
        match tok.kind {
            TokenKind::Ident(_) => self.tokens.get(i + 1).is_some_and(|t| t.is(Punct::Arrow)),
            TokenKind::Punct(Punct::LParen) => self
                .after_paren_group(i)
                .is_some_and(|t| t.is(Punct::Arrow)),
            _ => false,
        }
    }

    fn parse_arrow(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        let is_async = self.eat_kw(Keyword::Async);
        let params = if self.at(Punct::LParen) {
            self.parse_params()?
        } else {
            vec![self.expect_name()?]
        };
        self.expect(Punct::Arrow, "=>")?;
        self.ctx.push(FnCtx {
            is_async,
            is_function: true,
            loop_depth: 0,
        });
        let body = if self.at(Punct::LBrace) {
            self.parse_block().map(ArrowBody::Block)
        } else {
            self.parse_assignment()
                .map(|e| ArrowBody::Expr(Box::new(e)))
        };
        self.ctx.pop();
        Some(Expr {
            kind: ExprKind::Arrow {
                params,
                body: body?,
                is_async,
            },
            span: self.span_from(start),
        })
    }

    fn parse_conditional(&mut self) -> PResult<Expr> {
        let cond = self.parse_binary(1)?;
        if !self.eat(Punct::Question) {
            return Some(cond);
        }
        let then = self.parse_assignment()?;
        self.expect(Punct::Colon, ":")?;
        let otherwise = self.parse_assignment()?;
        let span = cond.span.to(otherwise.span);
        Some(Expr {
            kind: ExprKind::Conditional {
                cond: Box::new(cond),
                then: Box::new(then),
                otherwise: Box::new(otherwise),
            },
            span,
        })
    }

    fn binary_op(&self) -> Option<(BinaryOp, u8)> {
        let TokenKind::Punct(p) = self.peek().kind else {
            return None;
        };
        Some(match p {
            Punct::OrOr => (BinaryOp::Or, 1),
            Punct::AndAnd => (BinaryOp::And, 2),
            Punct::EqEq | Punct::EqEqEq => (BinaryOp::Eq, 3),
            Punct::NotEq | Punct::NotEqEq => (BinaryOp::NotEq, 3),
            Punct::Lt => (BinaryOp::Lt, 4),
            Punct::Le => (BinaryOp::Le, 4),
            Punct::Gt => (BinaryOp::Gt, 4),
            Punct::Ge => (BinaryOp::Ge, 4),
            Punct::Plus => (BinaryOp::Add, 5),
            Punct::Minus => (BinaryOp::Sub, 5),
            Punct::Star => (BinaryOp::Mul, 6),
            Punct::Slash => (BinaryOp::Div, 6),
            Punct::Percent => (BinaryOp::Rem, 6),
            _ => return None,
        })
    }

    /// Precedence climbing for left-associative binary operators.
    fn parse_binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let mut lhs = self.parse_unary()?;
        while let Some((op, prec)) = self.binary_op() {
            if prec < min_prec {
                break;
            }
            self.bump();
            let rhs = self.parse_binary(prec + 1)?;
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        Some(lhs)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let tok = self.peek();
        let start = tok.span;
        let unary = match tok.kind {
            TokenKind::Punct(Punct::Bang) => Some(UnaryOp::Not),
            TokenKind::Punct(Punct::Minus) => Some(UnaryOp::Neg),
            TokenKind::Punct(Punct::Plus) => Some(UnaryOp::Plus),
            _ => None,
        };
        if let Some(op) = unary {
            self.bump();
            let operand = self.parse_unary()?;
            return Some(Expr {
                span: start.to(operand.span),
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
            });
        }
        let update = match tok.kind {
            TokenKind::Punct(Punct::PlusPlus) => Some(UpdateOp::Inc),
            TokenKind::Punct(Punct::MinusMinus) => Some(UpdateOp::Dec),
            _ => None,
        };
        if let Some(op) = update {
            self.bump();
            let target = self.parse_unary()?;
            self.check_assignable(&target);
            return Some(Expr {
                span: start.to(target.span),
                kind: ExprKind::Update {
                    op,
                    prefix: true,
                    target: Box::new(target),
                },
            });
        }
        if tok.is_kw(Keyword::Await) {
            self.bump();
            if !self.ctx().is_async {
                self.error(ErrorKind::AwaitOutsideAsync, start);
            }
            let operand = self.parse_unary()?;
            return Some(Expr {
                span: start.to(operand.span),
                kind: ExprKind::Await(Box::new(operand)),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let expr = self.parse_call_member()?;
        let tok = self.peek();
        let op = match tok.kind {
            TokenKind::Punct(Punct::PlusPlus) => UpdateOp::Inc,
            TokenKind::Punct(Punct::MinusMinus) => UpdateOp::Dec,
            _ => return Some(expr),
        };
        // `x\n++y` is two statements, not `x++`.
        if tok.newline_before {
            return Some(expr);
        }
        self.bump();
        self.check_assignable(&expr);
        Some(Expr {
            span: expr.span.to(tok.span),
            kind: ExprKind::Update {
                op,
                prefix: false,
                target: Box::new(expr),
            },
        })
    }

    fn parse_property(&mut self) -> PResult<Ident> {
        let tok = self.peek();
        let name = match &tok.kind {
            TokenKind::Ident(name) => name.clone(),
            // Keywords are fine as property names (`Promise.race`, `x.new`).
            TokenKind::Keyword(kw) => kw.as_str().to_string(),
            _ => {
                self.error_expected(Expected::Property);
                return None;
            }
        };
        self.bump();
        Some(Ident {
            name,
            span: tok.span,
        })
    }

    fn parse_call_member(&mut self) -> PResult<Expr> {
        let mut expr = if self.at_kw(Keyword::New) {
            self.parse_new()?
        } else {
            self.parse_primary()?
        };
        loop {
            let tok = self.peek();
            if tok.is(Punct::Dot) {
                self.bump();
                let property = self.parse_property()?;
                expr = Expr {
                    span: expr.span.to(property.span),
                    kind: ExprKind::Member {
                        object: Box::new(expr),
                        property,
                    },
                };
            } else if tok.is(Punct::LBracket) && !tok.newline_before {
                self.bump();
                let index = self.parse_expr()?;
                self.expect(Punct::RBracket, "]")?;
                expr = Expr {
                    span: self.span_from(expr.span),
                    kind: ExprKind::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    },
                };
            } else if tok.is(Punct::LParen) && !tok.newline_before {
                // A `(` on a new line starts a new statement (unlike JavaScript,
                // where this is a famous source of bugs).
                let args = self.parse_args()?;
                expr = Expr {
                    span: self.span_from(expr.span),
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                };
            } else {
                return Some(expr);
            }
        }
    }

    /// `new Name(args)` / `new a.b.C(args)`
    fn parse_new(&mut self) -> PResult<Expr> {
        let start = self.bump().span;
        let mut callee = self.parse_primary()?;
        while self.eat(Punct::Dot) {
            let property = self.parse_property()?;
            callee = Expr {
                span: callee.span.to(property.span),
                kind: ExprKind::Member {
                    object: Box::new(callee),
                    property,
                },
            };
        }
        let args = if self.at(Punct::LParen) {
            self.parse_args()?
        } else {
            Vec::new()
        };
        Some(Expr {
            kind: ExprKind::New {
                callee: Box::new(callee),
                args,
            },
            span: self.span_from(start),
        })
    }

    /// Comma-separated list until `close`; the opening bracket is consumed here.
    fn parse_list(&mut self, close: Punct, close_text: &'static str) -> PResult<Vec<Expr>> {
        self.bump();
        let mut items = Vec::new();
        loop {
            if self.eat(close) {
                return Some(items);
            }
            items.push(self.parse_expr()?);
            if self.eat(Punct::Comma) {
                continue;
            }
            if self.eat(close) {
                return Some(items);
            }
            self.error_expected(Expected::CommaOr(close_text));
            return None;
        }
    }

    fn parse_args(&mut self) -> PResult<Vec<Expr>> {
        self.parse_list(Punct::RParen, ")")
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let tok = self.peek();
        let span = tok.span;
        let kind = match &tok.kind {
            TokenKind::Number { value, unit } => ExprKind::Number {
                value: *value,
                unit: *unit,
            },
            TokenKind::Str(s) => ExprKind::Str(s.clone()),
            TokenKind::Keyword(Keyword::True) => ExprKind::Bool(true),
            TokenKind::Keyword(Keyword::False) => ExprKind::Bool(false),
            TokenKind::Keyword(Keyword::Null) => ExprKind::Null,
            TokenKind::Keyword(Keyword::Undefined) => ExprKind::Undefined,
            TokenKind::Ident(name) => ExprKind::Ident(Ident {
                name: name.clone(),
                span,
            }),
            TokenKind::Punct(Punct::LParen) => {
                self.bump();
                let inner = self.parse_expr()?;
                self.expect(Punct::RParen, ")")?;
                return Some(Expr {
                    kind: inner.kind,
                    span: self.span_from(span),
                });
            }
            TokenKind::Punct(Punct::LBracket) => {
                let items = self.parse_list(Punct::RBracket, "]")?;
                return Some(Expr {
                    kind: ExprKind::Array(items),
                    span: self.span_from(span),
                });
            }
            _ => {
                self.error_expected(Expected::Value);
                return None;
            }
        };
        self.bump();
        Some(Expr { kind, span })
    }

    // ------------------------------------------------------------------
    // Whole-program checks
    // ------------------------------------------------------------------

    fn check_main(&mut self, program: &Program) {
        let main = program.body.iter().find_map(|stmt| match &stmt.kind {
            StmtKind::Function(f) if f.name.name == "main" => Some(f),
            _ => None,
        });
        match main {
            None => {
                let end = self.tokens.last().map(|t| t.span).unwrap_or_default();
                self.error(ErrorKind::MissingMain, end);
            }
            Some(f) if !f.is_async => self.error(ErrorKind::MainNotAsync, f.name.span),
            Some(_) => {}
        }
    }
}
