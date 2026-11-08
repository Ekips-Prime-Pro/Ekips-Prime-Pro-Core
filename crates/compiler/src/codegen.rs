//! Code generation: syntax tree + analysis results → bytecode.
//!
//! Runs only on programs without errors. Uses the name resolution of
//! [`crate::sema`] (which declaration each name means, which variables are
//! captured, how `.member` was resolved) and the standard library table for
//! native function ids. The instruction set is described in
//! `docs/en/bytecode.md`.

use std::collections::HashMap;

use ekips_bytecode::{self as bc, Capture, Constant, Native, Op};

use crate::ast::*;
use crate::diagnostic::{Diagnostic, ErrorKind, Limit};
use crate::sema::{Analysis, Binding, FnId, MemberRes, Resolution, Storage};
use crate::span::{LineIndex, Span};
use crate::stdlib::{self, ConstValue, GlobalKind, MemberKind};
use crate::types::Dim;

/// Generates bytecode for an analyzed, error-free program.
pub fn generate(
    program: &Program,
    analysis: &Analysis,
    source: &str,
) -> Result<bc::Program, Diagnostic> {
    let mut globals: Vec<&Binding> = analysis
        .bindings
        .values()
        .filter(|b| b.storage == Storage::Global)
        .collect();
    globals.sort_by_key(|b| b.decl.start);
    if globals.len() > usize::from(u16::MAX) {
        return Err(Diagnostic::new(
            ErrorKind::TooBig(Limit::Program),
            Span::default(),
        ));
    }
    let main = globals
        .iter()
        .find(|b| b.name == "main")
        .map(|b| b.decl)
        .expect("the parser ensures that `main` exists");
    let mut g = Gen {
        analysis,
        lines: LineIndex::new(source),
        globals: globals
            .iter()
            .enumerate()
            .map(|(i, b)| (b.decl, i as u16))
            .collect(),
        constants: Vec::new(),
        functions: Vec::new(),
        codes: Vec::new(),
        builders: Vec::new(),
    };

    g.begin_function(FnId::Top, "<top>".into(), &[], false, Span::default())?;
    g.stmt_list(&program.body)?;
    // Run `main` and wait until it is finished.
    g.load_binding(main, Span::default())?;
    g.op(Op::Call);
    g.u8(0);
    g.op(Op::Await);
    g.op(Op::Pop);
    g.op(Op::Undefined);
    g.op(Op::Return);
    g.end_function(Span::default())?;

    let mut code = Vec::new();
    let mut functions = Vec::new();
    for (f, c) in g.functions.into_iter().zip(g.codes) {
        let mut f = f.expect("every started function is finished");
        f.offset = code.len() as u32;
        f.len = c.len() as u32;
        code.extend_from_slice(&c);
        functions.push(f);
    }
    if functions.len() > usize::from(u16::MAX) || g.constants.len() > usize::from(u16::MAX) {
        return Err(Diagnostic::new(
            ErrorKind::TooBig(Limit::Program),
            Span::default(),
        ));
    }
    Ok(bc::Program {
        globals: globals.len() as u16,
        functions,
        constants: g.constants,
        code,
    })
}

type GResult<T = ()> = Result<T, Diagnostic>;

#[derive(Default)]
struct LoopCtx {
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

struct FnBuilder {
    id: FnId,
    index: usize,
    code: Vec<u8>,
    slots: HashMap<Span, u8>,
    next_slot: usize,
    params: u8,
    is_async: bool,
    name: String,
    captures: Vec<(Span, Capture)>,
    loops: Vec<LoopCtx>,
}

struct Gen<'a> {
    analysis: &'a Analysis,
    lines: LineIndex<'a>,
    globals: HashMap<Span, u16>,
    constants: Vec<Constant>,
    functions: Vec<Option<bc::Function>>,
    codes: Vec<Vec<u8>>,
    builders: Vec<FnBuilder>,
}

fn too_big(limit: Limit, span: Span) -> Diagnostic {
    Diagnostic::new(ErrorKind::TooBig(limit), span)
}

impl<'a> Gen<'a> {
    // ------------------------------------------------------------------
    // Emitting
    // ------------------------------------------------------------------

    fn b(&mut self) -> &mut FnBuilder {
        self.builders.last_mut().expect("inside a function")
    }

    fn op(&mut self, op: Op) {
        self.b().code.push(op as u8);
    }

    fn u8(&mut self, v: u8) {
        self.b().code.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.b().code.extend_from_slice(&v.to_le_bytes());
    }

    fn op_u8(&mut self, op: Op, v: u8) {
        self.op(op);
        self.u8(v);
    }

    fn op_u16(&mut self, op: Op, v: u16) {
        self.op(op);
        self.u16(v);
    }

    fn pos(&mut self) -> usize {
        self.b().code.len()
    }

    /// Emits a jump with a placeholder target and returns its position.
    fn jump(&mut self, op: Op) -> usize {
        self.op(op);
        let at = self.pos();
        self.u16(0);
        at
    }

    fn patch(&mut self, at: usize, target: usize, span: Span) -> GResult {
        let target = u16::try_from(target).map_err(|_| too_big(Limit::Code, span))?;
        self.b().code[at..at + 2].copy_from_slice(&target.to_le_bytes());
        Ok(())
    }

    /// Patches a jump to the current position.
    fn patch_here(&mut self, at: usize, span: Span) -> GResult {
        let here = self.pos();
        self.patch(at, here, span)
    }

    fn jump_to(&mut self, op: Op, target: usize, span: Span) -> GResult {
        let at = self.jump(op);
        self.patch(at, target, span)
    }

    fn constant(&mut self, c: Constant) -> u16 {
        let index = match self.constants.iter().position(|x| *x == c) {
            Some(i) => i,
            None => {
                self.constants.push(c);
                self.constants.len() - 1
            }
        };
        // Checked in `generate`; saturate so we never panic here.
        u16::try_from(index).unwrap_or(u16::MAX)
    }

    fn push_const(&mut self, c: Constant) {
        let index = self.constant(c);
        self.op_u16(Op::Const, index);
    }

    fn line(&mut self, span: Span) {
        let line = self.lines.line_col(span.start).line;
        self.op_u16(Op::Line, u16::try_from(line).unwrap_or(u16::MAX));
    }

    fn call_native(&mut self, native: Native, argc: usize, span: Span) -> GResult {
        let argc = u8::try_from(argc).map_err(|_| too_big(Limit::Arguments, span))?;
        self.op_u16(Op::CallNative, native as u16);
        self.u8(argc);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Functions and variables
    // ------------------------------------------------------------------

    fn begin_function(
        &mut self,
        id: FnId,
        name: String,
        params: &[Ident],
        is_async: bool,
        span: Span,
    ) -> GResult {
        let index = self.functions.len();
        self.functions.push(None);
        self.codes.push(Vec::new());
        self.builders.push(FnBuilder {
            id,
            index,
            code: Vec::new(),
            slots: HashMap::new(),
            next_slot: 0,
            params: u8::try_from(params.len()).map_err(|_| too_big(Limit::Variables, span))?,
            is_async,
            name,
            captures: Vec::new(),
            loops: Vec::new(),
        });
        for p in params {
            self.alloc_slot(p.span)?;
        }
        // Captured parameters move into a cell.
        for p in params {
            if self.is_captured(p.span) {
                let slot = self.b().slots[&p.span];
                self.op_u8(Op::LoadLocal, slot);
                self.op_u8(Op::NewCell, slot);
            }
        }
        Ok(())
    }

    /// Finishes the innermost function and returns its index.
    fn end_function(&mut self, span: Span) -> GResult<u16> {
        let b = self.builders.pop().expect("inside a function");
        if b.code.len() > usize::from(u16::MAX) {
            return Err(too_big(Limit::Code, span));
        }
        let locals = u8::try_from(b.next_slot).map_err(|_| too_big(Limit::Variables, span))?;
        self.functions[b.index] = Some(bc::Function {
            name: b.name,
            offset: 0,
            len: 0,
            params: b.params,
            locals,
            is_async: b.is_async,
            captures: b.captures.into_iter().map(|(_, c)| c).collect(),
        });
        self.codes[b.index] = b.code;
        u16::try_from(b.index).map_err(|_| too_big(Limit::Program, span))
    }

    /// A fresh local slot for a declaration (or a hidden temporary).
    fn alloc_slot(&mut self, decl: Span) -> GResult<u8> {
        let b = self.b();
        if let Some(&slot) = b.slots.get(&decl) {
            return Ok(slot);
        }
        let slot = u8::try_from(b.next_slot).map_err(|_| too_big(Limit::Variables, decl))?;
        b.next_slot += 1;
        b.slots.insert(decl, slot);
        Ok(slot)
    }

    /// A hidden temporary slot that belongs to no declaration.
    fn temp_slot(&mut self, span: Span) -> GResult<u8> {
        let b = self.b();
        let slot = u8::try_from(b.next_slot).map_err(|_| too_big(Limit::Variables, span))?;
        b.next_slot += 1;
        Ok(slot)
    }

    fn binding(&self, decl: Span) -> &'a Binding {
        &self.analysis.bindings[&decl]
    }

    fn is_captured(&self, decl: Span) -> bool {
        self.analysis
            .bindings
            .get(&decl)
            .is_some_and(|b| b.captured)
    }

    /// Index of the captured variable `decl` in builder `b`, adding it
    /// (and to all functions in between) if needed.
    fn upvalue(&mut self, b: usize, decl: Span, owner: FnId) -> u8 {
        if let Some(i) = self.builders[b]
            .captures
            .iter()
            .position(|(d, _)| *d == decl)
        {
            return i as u8;
        }
        let parent = b - 1;
        let source = if self.builders[parent].id == owner {
            Capture::Local(self.builders[parent].slots[&decl])
        } else {
            Capture::Upval(self.upvalue(parent, decl, owner))
        };
        self.builders[b].captures.push((decl, source));
        (self.builders[b].captures.len() - 1) as u8
    }

    /// Where a binding lives, as seen from the current function.
    fn place(&mut self, decl: Span) -> Place {
        let binding = self.binding(decl);
        match binding.storage {
            Storage::Global => Place::Global(self.globals[&decl]),
            Storage::Local(owner) => {
                let current = self.builders.len() - 1;
                if self.builders[current].id == owner {
                    let slot = self.builders[current].slots[&decl];
                    if binding.captured {
                        Place::Cell(slot)
                    } else {
                        Place::Local(slot)
                    }
                } else {
                    Place::Upval(self.upvalue(current, decl, owner))
                }
            }
        }
    }

    fn load_binding(&mut self, decl: Span, _span: Span) -> GResult {
        match self.place(decl) {
            Place::Global(i) => self.op_u16(Op::LoadGlobal, i),
            Place::Local(s) => self.op_u8(Op::LoadLocal, s),
            Place::Cell(s) => self.op_u8(Op::LoadCell, s),
            Place::Upval(i) => self.op_u8(Op::LoadUpval, i),
        }
        Ok(())
    }

    /// Pops the top value into a binding.
    fn store_binding(&mut self, decl: Span) {
        match self.place(decl) {
            Place::Global(i) => self.op_u16(Op::StoreGlobal, i),
            Place::Local(s) => self.op_u8(Op::StoreLocal, s),
            Place::Cell(s) => self.op_u8(Op::StoreCell, s),
            Place::Upval(i) => self.op_u8(Op::StoreUpval, i),
        }
    }

    fn resolved(&self, id: &Ident) -> Resolution {
        self.analysis.resolutions[&id.span]
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    /// A list of statements forming one block.
    ///
    /// On entry, every local declared in the block gets its slot, captured
    /// ones get a fresh cell (so each loop round has its own), and hoisted
    /// functions are created before any other statement runs.
    fn stmt_list(&mut self, stmts: &[Stmt]) -> GResult {
        for stmt in stmts {
            let decl = match &stmt.kind {
                StmtKind::VarDecl { name, .. } => name.span,
                StmtKind::Function(f) => f.name.span,
                _ => continue,
            };
            let Some(b) = self.analysis.bindings.get(&decl) else {
                continue;
            };
            if b.storage == Storage::Global {
                continue;
            }
            let slot = self.alloc_slot(decl)?;
            if b.captured {
                self.op(Op::Undefined);
                self.op_u8(Op::NewCell, slot);
            }
        }
        for stmt in stmts {
            if let StmtKind::Function(f) = &stmt.kind {
                self.function(f)?;
                self.store_binding(f.name.span);
            }
        }
        for stmt in stmts {
            if !matches!(stmt.kind, StmtKind::Function(_)) {
                self.stmt(stmt)?;
            }
        }
        Ok(())
    }

    /// A statement in its own block (bodies of `if` and loops).
    fn body(&mut self, stmt: &Stmt) -> GResult {
        match &stmt.kind {
            StmtKind::Block(stmts) => self.stmt_list(stmts),
            _ => self.stmt_list(std::slice::from_ref(stmt)),
        }
    }

    fn stmt(&mut self, stmt: &Stmt) -> GResult {
        if !matches!(stmt.kind, StmtKind::Block(_) | StmtKind::Empty) {
            self.line(stmt.span);
        }
        match &stmt.kind {
            StmtKind::VarDecl { name, init, .. } => {
                match init {
                    Some(e) => self.expr(e)?,
                    None => self.op(Op::Undefined),
                }
                self.store_binding(name.span);
            }
            StmtKind::Function(f) => {
                // A function outside a statement list (`if (x) function f() {}`).
                self.alloc_slot(f.name.span)?;
                self.function(f)?;
                self.store_binding(f.name.span);
            }
            StmtKind::Expr(e) => {
                self.expr(e)?;
                self.op(Op::Pop);
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond)?;
                let to_else = self.jump(Op::JumpIfFalse);
                self.body(then)?;
                match otherwise {
                    Some(o) => {
                        let to_end = self.jump(Op::Jump);
                        self.patch_here(to_else, stmt.span)?;
                        self.body(o)?;
                        self.patch_here(to_end, stmt.span)?;
                    }
                    None => self.patch_here(to_else, stmt.span)?,
                }
            }
            StmtKind::While { cond, body } => {
                let start = self.pos();
                self.expr(cond)?;
                let to_end = self.jump(Op::JumpIfFalse);
                self.b().loops.push(LoopCtx::default());
                self.body(body)?;
                self.jump_to(Op::Jump, start, stmt.span)?;
                let ctx = self.b().loops.pop().expect("loop context");
                for at in ctx.continues {
                    self.patch(at, start, stmt.span)?;
                }
                self.patch_here(to_end, stmt.span)?;
                for at in ctx.breaks {
                    self.patch_here(at, stmt.span)?;
                }
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                if let Some(init) = init {
                    self.stmt_list(std::slice::from_ref(init.as_ref()))?;
                }
                let start = self.pos();
                let to_end = match cond {
                    Some(c) => {
                        self.expr(c)?;
                        Some(self.jump(Op::JumpIfFalse))
                    }
                    None => None,
                };
                self.b().loops.push(LoopCtx::default());
                self.body(body)?;
                let ctx = self.b().loops.pop().expect("loop context");
                for at in ctx.continues {
                    self.patch_here(at, stmt.span)?;
                }
                if let Some(u) = update {
                    self.expr(u)?;
                    self.op(Op::Pop);
                }
                self.jump_to(Op::Jump, start, stmt.span)?;
                if let Some(at) = to_end {
                    self.patch_here(at, stmt.span)?;
                }
                for at in ctx.breaks {
                    self.patch_here(at, stmt.span)?;
                }
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                let list = self.temp_slot(stmt.span)?;
                let index = self.temp_slot(stmt.span)?;
                let var = self.alloc_slot(name.span)?;
                self.expr(iterable)?;
                self.op_u8(Op::StoreLocal, list);
                self.push_const(Constant::Num(0.0, Dim::Number));
                self.op_u8(Op::StoreLocal, index);
                let start = self.pos();
                self.op_u8(Op::LoadLocal, index);
                self.op_u8(Op::LoadLocal, list);
                self.call_native(Native::Length, 1, stmt.span)?;
                self.op(Op::Lt);
                let to_end = self.jump(Op::JumpIfFalse);
                self.op_u8(Op::LoadLocal, list);
                self.op_u8(Op::LoadLocal, index);
                self.op(Op::GetIndex);
                if self.is_captured(name.span) {
                    self.op_u8(Op::NewCell, var);
                } else {
                    self.op_u8(Op::StoreLocal, var);
                }
                self.b().loops.push(LoopCtx::default());
                self.body(body)?;
                let ctx = self.b().loops.pop().expect("loop context");
                for at in ctx.continues {
                    self.patch_here(at, stmt.span)?;
                }
                self.op_u8(Op::LoadLocal, index);
                self.push_const(Constant::Num(1.0, Dim::Number));
                self.op(Op::Add);
                self.op_u8(Op::StoreLocal, index);
                self.jump_to(Op::Jump, start, stmt.span)?;
                self.patch_here(to_end, stmt.span)?;
                for at in ctx.breaks {
                    self.patch_here(at, stmt.span)?;
                }
            }
            StmtKind::Block(stmts) => self.stmt_list(stmts)?,
            StmtKind::Return(value) => {
                match value {
                    Some(v) => self.expr(v)?,
                    None => self.op(Op::Undefined),
                }
                self.op(Op::Return);
            }
            StmtKind::Break => {
                let at = self.jump(Op::Jump);
                self.b()
                    .loops
                    .last_mut()
                    .expect("the parser ensures `break` is inside a loop")
                    .breaks
                    .push(at);
            }
            StmtKind::Continue => {
                let at = self.jump(Op::Jump);
                self.b()
                    .loops
                    .last_mut()
                    .expect("the parser ensures `continue` is inside a loop")
                    .continues
                    .push(at);
            }
            StmtKind::Empty => {}
        }
        Ok(())
    }

    /// Compiles a function declaration and pushes its closure.
    fn function(&mut self, f: &Function) -> GResult {
        self.begin_function(
            FnId::Func(f.span),
            f.name.name.clone(),
            &f.params,
            f.is_async,
            f.span,
        )?;
        self.stmt_list(&f.body)?;
        self.op(Op::Undefined);
        self.op(Op::Return);
        let index = self.end_function(f.span)?;
        self.op_u16(Op::Closure, index);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    /// Compiles an expression; leaves exactly one value on the stack.
    fn expr(&mut self, e: &Expr) -> GResult {
        match &e.kind {
            ExprKind::Number { value, unit } => {
                let (v, dim) = match unit {
                    Some(u) => (u.to_base(*value), u.dim()),
                    None => (*value, Dim::Number),
                };
                self.push_const(Constant::Num(v as f32, dim));
            }
            ExprKind::Str(s) => self.push_const(Constant::Str(s.clone())),
            ExprKind::Bool(true) => self.op(Op::True),
            ExprKind::Bool(false) => self.op(Op::False),
            ExprKind::Null => self.op(Op::Null),
            ExprKind::Undefined => self.op(Op::Undefined),
            ExprKind::Ident(id) => match self.resolved(id) {
                Resolution::Binding(decl) => self.load_binding(decl, id.span)?,
                Resolution::Builtin(_) => unreachable!("rejected by semantic analysis"),
            },
            ExprKind::Array(items) => {
                for item in items {
                    self.expr(item)?;
                }
                let n =
                    u16::try_from(items.len()).map_err(|_| too_big(Limit::ListItems, e.span))?;
                self.op_u16(Op::MakeList, n);
            }
            ExprKind::Member { object, property } => self.member(object, property)?,
            ExprKind::Index { object, index } => {
                self.expr(object)?;
                self.expr(index)?;
                self.op(Op::GetIndex);
            }
            ExprKind::Call { callee, args } => self.call(e, callee, args)?,
            ExprKind::New { callee, args } => {
                let ExprKind::Ident(id) = &callee.kind else {
                    unreachable!("rejected by semantic analysis")
                };
                let Resolution::Builtin(g) = self.resolved(id) else {
                    unreachable!("rejected by semantic analysis")
                };
                let GlobalKind::Class { ctor, .. } = g.kind else {
                    unreachable!("rejected by semantic analysis")
                };
                for a in args {
                    self.expr(a)?;
                }
                self.call_native(ctor.native, args.len(), e.span)?;
            }
            ExprKind::Unary { op, operand } => {
                self.expr(operand)?;
                self.op(match op {
                    UnaryOp::Not => Op::Not,
                    UnaryOp::Neg => Op::Neg,
                    UnaryOp::Plus => Op::ToNumber,
                });
            }
            ExprKind::Update { op, prefix, target } => self.update(e, *op, *prefix, target)?,
            ExprKind::Binary { op, lhs, rhs } => match op {
                BinaryOp::And | BinaryOp::Or => {
                    self.expr(lhs)?;
                    let jump = if *op == BinaryOp::And {
                        Op::JumpIfFalseKeep
                    } else {
                        Op::JumpIfTrueKeep
                    };
                    let to_end = self.jump(jump);
                    self.expr(rhs)?;
                    self.patch_here(to_end, e.span)?;
                }
                _ => {
                    self.expr(lhs)?;
                    self.expr(rhs)?;
                    self.op(binary_op(*op));
                }
            },
            ExprKind::Conditional {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond)?;
                let to_else = self.jump(Op::JumpIfFalse);
                self.expr(then)?;
                let to_end = self.jump(Op::Jump);
                self.patch_here(to_else, e.span)?;
                self.expr(otherwise)?;
                self.patch_here(to_end, e.span)?;
            }
            ExprKind::Assign { op, target, value } => self.assign(e, *op, target, value)?,
            ExprKind::Await(inner) => {
                self.expr(inner)?;
                self.op(Op::Await);
            }
            ExprKind::Arrow {
                params,
                body,
                is_async,
            } => {
                self.begin_function(
                    FnId::Func(e.span),
                    "<arrow>".into(),
                    params,
                    *is_async,
                    e.span,
                )?;
                match body {
                    ArrowBody::Expr(body) => {
                        self.expr(body)?;
                        self.op(Op::Return);
                    }
                    ArrowBody::Block(stmts) => {
                        self.stmt_list(stmts)?;
                        self.op(Op::Undefined);
                        self.op(Op::Return);
                    }
                }
                let index = self.end_function(e.span)?;
                self.op_u16(Op::Closure, index);
            }
            ExprKind::Error => unreachable!("code is only generated without errors"),
        }
        Ok(())
    }

    /// `object.property` used as a value.
    fn member(&mut self, object: &Expr, property: &Ident) -> GResult {
        match self.analysis.members[&property.span] {
            MemberRes::Namespace(m) => match m.kind {
                MemberKind::Constant(v) => self.push_const(const_value(v)),
                MemberKind::Property { native, .. } => {
                    self.call_native(native, 0, property.span)?
                }
                MemberKind::Method(_) => unreachable!("rejected by semantic analysis"),
            },
            MemberRes::Instance(m) => {
                self.expr(object)?;
                match m.kind {
                    MemberKind::Property { native, .. } => {
                        self.call_native(native, 1, property.span)?
                    }
                    _ => unreachable!("rejected by semantic analysis"),
                }
            }
            MemberRes::Dynamic => {
                self.expr(object)?;
                let set = self.method_set(&property.name);
                self.op_u16(Op::CallMethod, set);
                self.u8(0);
            }
        }
        Ok(())
    }

    fn method_set(&mut self, name: &str) -> u16 {
        let natives = stdlib::instance_members_named(name)
            .into_iter()
            .filter_map(|m| match m.kind {
                MemberKind::Method(sig) => Some(sig.native),
                MemberKind::Property { native, .. } => Some(native),
                MemberKind::Constant(_) => None,
            })
            .collect();
        self.constant(Constant::MethodSet(natives))
    }

    fn args(&mut self, args: &[Expr], span: Span) -> GResult<u8> {
        for a in args {
            self.expr(a)?;
        }
        u8::try_from(args.len()).map_err(|_| too_big(Limit::Arguments, span))
    }

    fn call(&mut self, e: &Expr, callee: &Expr, args: &[Expr]) -> GResult {
        match &callee.kind {
            ExprKind::Ident(id) => match self.resolved(id) {
                Resolution::Builtin(g) => {
                    let GlobalKind::Function(sig) = g.kind else {
                        unreachable!("rejected by semantic analysis")
                    };
                    let argc = self.args(args, e.span)?;
                    self.call_native(sig.native, usize::from(argc), e.span)?;
                    return Ok(());
                }
                Resolution::Binding(_) => {}
            },
            ExprKind::Member { object, property } => {
                match self.analysis.members.get(&property.span) {
                    Some(MemberRes::Namespace(m)) => {
                        let MemberKind::Method(sig) = m.kind else {
                            unreachable!("rejected by semantic analysis")
                        };
                        let argc = self.args(args, e.span)?;
                        self.call_native(sig.native, usize::from(argc), e.span)?;
                        return Ok(());
                    }
                    Some(MemberRes::Instance(m)) => {
                        let MemberKind::Method(sig) = m.kind else {
                            unreachable!("rejected by semantic analysis")
                        };
                        self.expr(object)?;
                        let argc = self.args(args, e.span)?;
                        self.call_native(sig.native, usize::from(argc) + 1, e.span)?;
                        return Ok(());
                    }
                    Some(MemberRes::Dynamic) => {
                        self.expr(object)?;
                        let argc = self.args(args, e.span)?;
                        let set = self.method_set(&property.name);
                        self.op_u16(Op::CallMethod, set);
                        self.u8(argc);
                        return Ok(());
                    }
                    None => {}
                }
            }
            _ => {}
        }
        self.expr(callee)?;
        let argc = self.args(args, e.span)?;
        self.op_u8(Op::Call, argc);
        Ok(())
    }

    /// `x++`, `--x`, `list[i]++`
    fn update(&mut self, e: &Expr, op: UpdateOp, prefix: bool, target: &Expr) -> GResult {
        let arith = match op {
            UpdateOp::Inc => Op::Add,
            UpdateOp::Dec => Op::Sub,
        };
        let one = Constant::Num(1.0, Dim::Number);
        match &target.kind {
            ExprKind::Ident(id) => {
                let Resolution::Binding(decl) = self.resolved(id) else {
                    unreachable!("rejected by semantic analysis")
                };
                self.load_binding(decl, id.span)?;
                if !prefix {
                    self.op(Op::Dup);
                }
                self.push_const(one);
                self.op(arith);
                if prefix {
                    self.op(Op::Dup);
                }
                self.store_binding(decl);
            }
            ExprKind::Index { object, index } => {
                self.expr(object)?;
                self.expr(index)?;
                self.op(Op::Dup2);
                self.op(Op::GetIndex);
                let old = if prefix {
                    None
                } else {
                    let slot = self.temp_slot(e.span)?;
                    self.op(Op::Dup);
                    self.op_u8(Op::StoreLocal, slot);
                    Some(slot)
                };
                self.push_const(one);
                self.op(arith);
                self.op(Op::SetIndex);
                if let Some(slot) = old {
                    self.op(Op::Pop);
                    self.op_u8(Op::LoadLocal, slot);
                }
            }
            _ => unreachable!("rejected by parser or semantic analysis"),
        }
        Ok(())
    }

    /// `x = v`, `x += v`, `list[i] = v`, …
    fn assign(&mut self, e: &Expr, op: AssignOp, target: &Expr, value: &Expr) -> GResult {
        let arith = match op {
            AssignOp::Assign => None,
            AssignOp::Add => Some(Op::Add),
            AssignOp::Sub => Some(Op::Sub),
            AssignOp::Mul => Some(Op::Mul),
            AssignOp::Div => Some(Op::Div),
            AssignOp::Rem => Some(Op::Rem),
        };
        match &target.kind {
            ExprKind::Ident(id) => {
                let Resolution::Binding(decl) = self.resolved(id) else {
                    unreachable!("rejected by semantic analysis")
                };
                if let Some(arith) = arith {
                    self.load_binding(decl, id.span)?;
                    self.expr(value)?;
                    self.op(arith);
                } else {
                    self.expr(value)?;
                }
                self.op(Op::Dup);
                self.store_binding(decl);
            }
            ExprKind::Index { object, index } => {
                self.expr(object)?;
                self.expr(index)?;
                if let Some(arith) = arith {
                    self.op(Op::Dup2);
                    self.op(Op::GetIndex);
                    self.expr(value)?;
                    self.op(arith);
                } else {
                    self.expr(value)?;
                }
                self.op(Op::SetIndex);
            }
            _ => unreachable!("rejected by parser or semantic analysis"),
        }
        let _ = e;
        Ok(())
    }
}

enum Place {
    Global(u16),
    Local(u8),
    Cell(u8),
    Upval(u8),
}

fn binary_op(op: BinaryOp) -> Op {
    match op {
        BinaryOp::Add => Op::Add,
        BinaryOp::Sub => Op::Sub,
        BinaryOp::Mul => Op::Mul,
        BinaryOp::Div => Op::Div,
        BinaryOp::Rem => Op::Rem,
        BinaryOp::Eq => Op::Eq,
        BinaryOp::NotEq => Op::NotEq,
        BinaryOp::Lt => Op::Lt,
        BinaryOp::Le => Op::Le,
        BinaryOp::Gt => Op::Gt,
        BinaryOp::Ge => Op::Ge,
        BinaryOp::And | BinaryOp::Or => unreachable!("compiled with jumps"),
    }
}

fn const_value(v: ConstValue) -> Constant {
    match v {
        ConstValue::Port(p) => Constant::Port(p),
        ConstValue::Color(c) => Constant::Color(c),
        ConstValue::Num(n, d) => Constant::Num(n as f32, d),
    }
}
