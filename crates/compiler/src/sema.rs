//! Semantic analysis: scopes, name resolution and type inference.
//!
//! Runs on the syntax tree after parsing and finds problems the parser can
//! not see:
//!
//! - unknown names (with "Did you mean …?"), unknown members (`robot.drvie`)
//! - names declared twice in the same block, names that hide built-ins
//! - `let`/`const` used before the line that creates them
//! - assignments to `const`, functions and built-in values
//! - wrong use of built-ins (`robot.stop` without `()`, `Motor(Port.A)`
//!   without `new`, wrong number or type of arguments)
//!
//! It also records what every name refers to ([`Analysis::resolutions`])
//! and which variables are captured by inner functions
//! ([`Binding::captured`]). Code generation and the language server use
//! these results.
//!
//! Scoping follows JavaScript: `let`/`const` are block-scoped, functions
//! are hoisted to the top of their block, and inner functions may refer to
//! variables that are declared later in an outer block (they only run
//! later).

use std::collections::HashMap;

use crate::ast::*;
use crate::diagnostic::{Diagnostic, ErrorKind, MemberOwner};
use crate::span::Span;
use crate::stdlib::{
    self, Expect, Global, GlobalKind, LIST_MEMBERS, Member, MemberKind, Returns, Signature,
};
use crate::suggest::suggest_with;
use crate::types::{Dim, Ty};

/// The function a piece of code belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FnId {
    /// Top-level code (runs once before `main`)
    Top,
    /// A function declaration or arrow function, identified by its span
    Func(Span),
}

/// Where a variable lives at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Storage {
    /// Declared directly at the top level: a global
    Global,
    /// Declared inside the given function (or in a block of top-level code)
    Local(FnId),
}

/// What kind of declaration a binding comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// `const`, including `for (const x of …)`
    Const,
    /// `let`, including `for (let x of …)`
    Let,
    /// A function parameter
    Param,
    /// A function declaration
    Function,
}

/// Arity and `async`-ness of a function stored in a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FnInfo {
    /// Number of parameters
    pub arity: usize,
    /// Declared `async`
    pub is_async: bool,
}

/// A declared name.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    /// The name
    pub name: String,
    /// Span of the name in its declaration (also the key in [`Analysis::bindings`])
    pub decl: Span,
    /// Declaration kind
    pub kind: BindingKind,
    /// Global or local, and of which function
    pub storage: Storage,
    /// Used by an inner function (needs a heap cell at runtime)
    pub captured: bool,
    /// Inferred type of the value
    pub ty: Ty,
    /// Set if the value is known to be a function
    pub fn_info: Option<FnInfo>,
}

/// What a name in an expression refers to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolution {
    /// A declared variable, parameter or function (key into [`Analysis::bindings`])
    Binding(Span),
    /// A built-in name
    Builtin(&'static Global),
}

/// How a `.member` was resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemberRes {
    /// Member of a built-in namespace (`hub.beep`, `Port.A`): no receiver value
    Namespace(&'static Member),
    /// Member of a value whose class is known (`robot.drive`)
    Instance(&'static Member),
    /// Member of a value of unknown type; the VM picks the member by the
    /// receiver's class at runtime
    Dynamic,
}

/// Results of [`analyze`].
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    /// Problems found, in no particular order
    pub diagnostics: Vec<Diagnostic>,
    /// Every resolved name use, keyed by the span of the identifier
    pub resolutions: HashMap<Span, Resolution>,
    /// Every declaration, keyed by the span of the declared name
    pub bindings: HashMap<Span, Binding>,
    /// Every resolved `.member`, keyed by the span of the member name
    pub members: HashMap<Span, MemberRes>,
}

/// Analyzes a parsed program.
pub fn analyze(program: &Program) -> Analysis {
    let mut a = Analyzer::default();
    a.fns.push(FnCtx {
        id: FnId::Top,
        is_async: false,
    });
    a.scopes.push(Scope {
        names: HashMap::new(),
        fn_id: FnId::Top,
        is_module: true,
    });
    a.walk_stmts(&program.body);
    a.out
}

#[derive(Debug)]
struct Scope {
    names: HashMap<String, Span>,
    fn_id: FnId,
    is_module: bool,
}

#[derive(Debug, Clone, Copy)]
struct FnCtx {
    id: FnId,
    is_async: bool,
}

/// Result of looking up `object.property`.
enum Lookup {
    /// A single known member
    Found {
        member: &'static Member,
        full_name: String,
        /// The other members of the same class or namespace
        siblings: &'static [Member],
    },
    /// Receiver of unknown type; these members could match
    Dynamic(Vec<&'static Member>),
    /// Nothing usable (an error was reported)
    None,
}

#[derive(Default)]
struct Analyzer {
    out: Analysis,
    scopes: Vec<Scope>,
    fns: Vec<FnCtx>,
    /// Declarations whose line has not been reached yet
    pending: std::collections::HashSet<Span>,
}

/// Readable text for simple expressions like `robot.drive` (for messages).
pub(crate) fn path_text(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Ident(id) => Some(id.name.clone()),
        ExprKind::Member { object, property } => {
            Some(format!("{}.{}", path_text(object)?, property.name))
        }
        _ => None,
    }
}

/// `true` for a literal `0` without unit (fits every kind of number).
pub(crate) fn is_zero_literal(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Number { value, unit: None } if value == 0.0)
}

impl Analyzer {
    fn error(&mut self, kind: ErrorKind, span: Span) {
        self.out.diagnostics.push(Diagnostic::new(kind, span));
    }

    fn error_related(&mut self, kind: ErrorKind, span: Span, related: Span) {
        self.out
            .diagnostics
            .push(Diagnostic::new(kind, span).with_related(related));
    }

    fn current_fn(&self) -> FnCtx {
        *self.fns.last().expect("function stack is never empty")
    }

    // ------------------------------------------------------------------
    // Scopes and declarations
    // ------------------------------------------------------------------

    fn push_scope(&mut self) {
        let fn_id = self.current_fn().id;
        self.scopes.push(Scope {
            names: HashMap::new(),
            fn_id,
            is_module: false,
        });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Declares `name` in the innermost scope.
    fn declare(&mut self, name: &Ident, kind: BindingKind, ty: Ty, fn_info: Option<FnInfo>) {
        if stdlib::global(&name.name).is_some() {
            self.error(ErrorKind::ShadowsBuiltin(name.name.clone()), name.span);
        }
        let scope = self.scopes.last_mut().expect("scope stack is never empty");
        if let Some(&first) = scope.names.get(&name.name) {
            self.error_related(
                ErrorKind::DuplicateDeclaration(name.name.clone()),
                name.span,
                first,
            );
            return;
        }
        scope.names.insert(name.name.clone(), name.span);
        let storage = if scope.is_module {
            Storage::Global
        } else {
            Storage::Local(scope.fn_id)
        };
        self.out.bindings.insert(
            name.span,
            Binding {
                name: name.name.clone(),
                decl: name.span,
                kind,
                storage,
                captured: false,
                ty,
                fn_info,
            },
        );
    }

    /// Finds the binding for a name, marking it as captured if needed.
    fn lookup_binding(&mut self, name: &str) -> Option<(Span, FnId)> {
        let current = self.current_fn().id;
        for scope in self.scopes.iter().rev() {
            if let Some(&decl) = scope.names.get(name) {
                let fn_id = scope.fn_id;
                if let Some(b) = self.out.bindings.get_mut(&decl) {
                    if fn_id != current && b.storage != Storage::Global {
                        b.captured = true;
                    }
                }
                return Some((decl, fn_id));
            }
        }
        None
    }

    /// All names visible here, for suggestions.
    fn visible_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .scopes
            .iter()
            .flat_map(|s| s.names.keys().map(String::as_str))
            .collect();
        names.extend(stdlib::GLOBALS.iter().map(|g| g.name));
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Resolves an identifier and records the result.
    fn resolve(&mut self, id: &Ident) -> Option<Resolution> {
        if let Some((decl, fn_id)) = self.lookup_binding(&id.name) {
            if self.pending.contains(&decl) && fn_id == self.current_fn().id {
                self.error_related(
                    ErrorKind::UsedBeforeDeclaration(id.name.clone()),
                    id.span,
                    decl,
                );
            }
            let res = Resolution::Binding(decl);
            self.out.resolutions.insert(id.span, res);
            return Some(res);
        }
        if let Some(g) = stdlib::global(&id.name) {
            let res = Resolution::Builtin(g);
            self.out.resolutions.insert(id.span, res);
            return Some(res);
        }
        let aliases: Vec<(&str, &str)> = stdlib::GLOBALS
            .iter()
            .flat_map(|g| g.aliases.iter().map(move |a| (*a, g.name)))
            .collect();
        let names = self.visible_names();
        let suggestion = suggest_with(&id.name, &names, &aliases).map(str::to_string);
        self.error(
            ErrorKind::UnknownName {
                name: id.name.clone(),
                suggestion,
            },
            id.span,
        );
        None
    }

    fn binding(&self, decl: Span) -> &Binding {
        &self.out.bindings[&decl]
    }

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    /// Walks the statements of a block in the current scope.
    ///
    /// Functions are hoisted: they are declared first and their bodies are
    /// checked after the other statements, when all variables of the block
    /// (and their types) are known.
    fn walk_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Function(f) => {
                    let info = FnInfo {
                        arity: f.params.len(),
                        is_async: f.is_async,
                    };
                    self.declare(&f.name, BindingKind::Function, Ty::Function, Some(info));
                }
                StmtKind::VarDecl { kind, name, .. } => {
                    let bk = match kind {
                        DeclKind::Const => BindingKind::Const,
                        DeclKind::Let => BindingKind::Let,
                    };
                    let before = self.out.bindings.contains_key(&name.span);
                    self.declare(name, bk, Ty::Any, None);
                    if !before && self.out.bindings.contains_key(&name.span) {
                        self.pending.insert(name.span);
                    }
                }
                _ => {}
            }
        }
        let mut functions = Vec::new();
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Function(f) => functions.push(f),
                _ => self.walk_stmt(stmt),
            }
        }
        for f in functions {
            self.walk_function(f);
        }
    }

    /// Walks a statement in its own block scope (bodies of `if`, loops, …).
    fn walk_scoped(&mut self, stmt: &Stmt) {
        self.push_scope();
        match &stmt.kind {
            StmtKind::Block(stmts) => self.walk_stmts(stmts),
            _ => self.walk_stmts(std::slice::from_ref(stmt)),
        }
        self.pop_scope();
    }

    fn walk_function(&mut self, f: &Function) {
        self.fns.push(FnCtx {
            id: FnId::Func(f.span),
            is_async: f.is_async,
        });
        self.push_scope();
        for p in &f.params {
            self.declare(p, BindingKind::Param, Ty::Any, None);
        }
        self.walk_stmts(&f.body);
        self.pop_scope();
        self.fns.pop();
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::VarDecl { name, init, .. } => {
                let (ty, fn_info) = match init {
                    Some(e) => {
                        let ty = self.expr(e);
                        let info = match &e.kind {
                            ExprKind::Arrow {
                                params, is_async, ..
                            } => Some(FnInfo {
                                arity: params.len(),
                                is_async: *is_async,
                            }),
                            _ => None,
                        };
                        // A plain `0` is how counters and sums start; its
                        // kind is decided by what is added later.
                        let ty = if is_zero_literal(e) { Ty::Any } else { ty };
                        (ty, info)
                    }
                    None => (Ty::Any, None),
                };
                self.pending.remove(&name.span);
                if let Some(b) = self.out.bindings.get_mut(&name.span) {
                    b.ty = ty;
                    b.fn_info = fn_info;
                }
            }
            StmtKind::Function(f) => {
                // Only reached for a function outside of a statement list
                // (e.g. the body of an `if` without braces).
                self.walk_function(f);
            }
            StmtKind::Expr(e) => {
                self.expr_statement(e);
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                self.walk_scoped(then);
                if let Some(o) = otherwise {
                    self.walk_scoped(o);
                }
            }
            StmtKind::While { cond, body } => {
                self.expr(cond);
                self.walk_scoped(body);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.push_scope();
                if let Some(init) = init {
                    self.walk_stmts(std::slice::from_ref(init.as_ref()));
                }
                if let Some(c) = cond {
                    self.expr(c);
                }
                if let Some(u) = update {
                    self.expr(u);
                }
                self.walk_scoped(body);
                self.pop_scope();
            }
            StmtKind::ForOf {
                kind,
                name,
                iterable,
                body,
            } => {
                self.expr(iterable);
                self.push_scope();
                let bk = match kind {
                    DeclKind::Const => BindingKind::Const,
                    DeclKind::Let => BindingKind::Let,
                };
                self.declare(name, bk, Ty::Any, None);
                self.walk_scoped(body);
                self.pop_scope();
            }
            StmtKind::Block(stmts) => {
                self.push_scope();
                self.walk_stmts(stmts);
                self.pop_scope();
            }
            StmtKind::Return(value) => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            StmtKind::Break | StmtKind::Continue | StmtKind::Empty => {}
        }
    }

    /// An expression used as a statement.
    ///
    /// Warns when its value is a Promise: the Promise is then neither
    /// awaited, nor stored, nor passed to `Promise.all`/`Promise.race`, and
    /// the program goes on while the command is still running. (Storing it
    /// or passing it on are not expression statements, so they never warn.)
    fn expr_statement(&mut self, e: &Expr) {
        let ty = self.expr(e);
        if ty == Ty::Promise && !matches!(e.kind, ExprKind::Assign { .. }) {
            let name = match &e.kind {
                ExprKind::Call { callee, .. } => path_text(callee),
                _ => None,
            };
            let in_async = self.current_fn().is_async;
            self.error(ErrorKind::UnusedPromise { name, in_async }, e.span);
        }
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------

    /// Checks an expression and returns its inferred type.
    fn expr(&mut self, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Number { unit, .. } => Ty::Num(unit.map_or(Dim::Number, Unit::dim)),
            ExprKind::Str(_) => Ty::Text,
            ExprKind::Bool(_) => Ty::Bool,
            ExprKind::Null | ExprKind::Undefined => Ty::Nothing,
            ExprKind::Ident(id) => self.ident_value(id),
            ExprKind::Array(items) => {
                for item in items {
                    self.expr(item);
                }
                Ty::List
            }
            ExprKind::Member { object, property } => self.member_value(object, property),
            ExprKind::Index { object, index } => {
                self.expr(object);
                self.expr(index);
                Ty::Any
            }
            ExprKind::Call { callee, args } => self.call(e, callee, args),
            ExprKind::New { callee, args } => self.new_object(e, callee, args),
            ExprKind::Unary { op, operand } => {
                let t = self.expr(operand);
                match op {
                    UnaryOp::Not => Ty::Bool,
                    UnaryOp::Neg | UnaryOp::Plus => match t {
                        Ty::Num(_) => t,
                        _ => Ty::Any,
                    },
                }
            }
            ExprKind::Update { target, op, .. } => {
                let t = self.assign_target(target);
                self.check_update(e, *op, t);
                t
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.expr(lhs);
                let r = self.expr(rhs);
                self.binary(e, *op, lhs, rhs, l, r)
            }
            ExprKind::Conditional {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                let a = self.expr(then);
                let b = self.expr(otherwise);
                if a == b { a } else { Ty::Any }
            }
            ExprKind::Assign { op, target, value } => {
                let v = self.expr(value);
                let t = self.assign_target(target);
                self.check_assign(e, *op, target, value, t, v);
                if *op == AssignOp::Assign { v } else { t }
            }
            ExprKind::Await(inner) => match self.expr(inner) {
                Ty::Promise => Ty::Any,
                t => t,
            },
            ExprKind::Arrow {
                params,
                body,
                is_async,
            } => {
                self.fns.push(FnCtx {
                    id: FnId::Func(e.span),
                    is_async: *is_async,
                });
                self.push_scope();
                for p in params {
                    self.declare(p, BindingKind::Param, Ty::Any, None);
                }
                match body {
                    ArrowBody::Expr(body) => {
                        self.expr(body);
                    }
                    ArrowBody::Block(stmts) => self.walk_stmts(stmts),
                }
                self.pop_scope();
                self.fns.pop();
                Ty::Function
            }
            ExprKind::Error => Ty::Any,
        }
    }

    /// A name used as a value.
    fn ident_value(&mut self, id: &Ident) -> Ty {
        match self.resolve(id) {
            Some(Resolution::Binding(decl)) => self.binding(decl).ty,
            Some(Resolution::Builtin(g)) => {
                let kind = match g.kind {
                    GlobalKind::Function(_) => ErrorKind::MustCall(g.name.into()),
                    _ => ErrorKind::NotAValue(g.name.into()),
                };
                self.error(kind, id.span);
                Ty::Any
            }
            None => Ty::Any,
        }
    }

    /// Looks up `object.property` and records the result.
    fn lookup_member(&mut self, object: &Expr, property: &Ident) -> Lookup {
        // Built-in namespaces and classes: `hub.beep`, `Port.A`, `Motor.x`
        if let ExprKind::Ident(id) = &object.kind {
            if self.lookup_binding(&id.name).is_none() {
                if let Some(g) = stdlib::global(&id.name) {
                    self.out.resolutions.insert(id.span, Resolution::Builtin(g));
                    return match g.kind {
                        GlobalKind::Namespace { members } => self
                            .find_or_report(members, MemberOwner::Name(g.name.into()), property)
                            .map_or(Lookup::None, |member| {
                                self.out
                                    .members
                                    .insert(property.span, MemberRes::Namespace(member));
                                Lookup::Found {
                                    member,
                                    full_name: format!("{}.{}", g.name, property.name),
                                    siblings: members,
                                }
                            }),
                        GlobalKind::Class { .. } => {
                            self.error(ErrorKind::NotAValue(g.name.into()), id.span);
                            Lookup::None
                        }
                        GlobalKind::Function(_) => {
                            self.error(ErrorKind::MustCall(g.name.into()), id.span);
                            Lookup::None
                        }
                    };
                }
            }
        }

        let ty = self.expr(object);
        let full_name = match path_text(object) {
            Some(p) => format!("{p}.{}", property.name),
            None => property.name.clone(),
        };
        let members: &'static [Member] = match ty {
            Ty::Object(class) => stdlib::class_members(class),
            Ty::List | Ty::Text => LIST_MEMBERS,
            Ty::Any => {
                let found = stdlib::instance_members_named(&property.name);
                if found.is_empty() {
                    let mut names: Vec<&str> = stdlib::instance_member_sets()
                        .flat_map(|set| set.iter().map(|m| m.name))
                        .collect();
                    names.sort_unstable();
                    names.dedup();
                    let aliases = member_aliases(stdlib::instance_member_sets().flatten());
                    let suggestion =
                        suggest_with(&property.name, &names, &aliases).map(str::to_string);
                    self.error(
                        ErrorKind::UnknownMember {
                            owner: MemberOwner::Value(Ty::Any),
                            member: property.name.clone(),
                            suggestion,
                        },
                        property.span,
                    );
                    return Lookup::None;
                }
                self.out.members.insert(property.span, MemberRes::Dynamic);
                return Lookup::Dynamic(found);
            }
            other => {
                self.error(
                    ErrorKind::UnknownMember {
                        owner: MemberOwner::Value(other),
                        member: property.name.clone(),
                        suggestion: None,
                    },
                    property.span,
                );
                return Lookup::None;
            }
        };
        let owner = match ty {
            Ty::Object(class) => MemberOwner::Name(class.name().into()),
            other => MemberOwner::Value(other),
        };
        match self.find_or_report(members, owner, property) {
            Some(member) => {
                self.out
                    .members
                    .insert(property.span, MemberRes::Instance(member));
                Lookup::Found {
                    member,
                    full_name,
                    siblings: members,
                }
            }
            None => Lookup::None,
        }
    }

    fn find_or_report(
        &mut self,
        members: &'static [Member],
        owner: MemberOwner,
        property: &Ident,
    ) -> Option<&'static Member> {
        if let Some(m) = stdlib::find_member(members, &property.name) {
            return Some(m);
        }
        let names: Vec<&str> = members.iter().map(|m| m.name).collect();
        let aliases = member_aliases(members.iter());
        let suggestion = suggest_with(&property.name, &names, &aliases).map(str::to_string);
        self.error(
            ErrorKind::UnknownMember {
                owner,
                member: property.name.clone(),
                suggestion,
            },
            property.span,
        );
        None
    }

    /// `object.property` used as a value (not called).
    fn member_value(&mut self, object: &Expr, property: &Ident) -> Ty {
        match self.lookup_member(object, property) {
            Lookup::Found {
                member, full_name, ..
            } => match member.kind {
                MemberKind::Method(_) => {
                    self.error(ErrorKind::MustCall(full_name), property.span);
                    Ty::Any
                }
                MemberKind::Property { ty, .. } => ty,
                MemberKind::Constant(v) => v.ty(),
            },
            Lookup::Dynamic(found) => {
                let types: Vec<Ty> = found
                    .iter()
                    .filter_map(|m| match m.kind {
                        MemberKind::Property { ty, .. } => Some(ty),
                        _ => None,
                    })
                    .collect();
                if types.is_empty() {
                    let name = path_text(object)
                        .map_or(property.name.clone(), |p| format!("{p}.{}", property.name));
                    self.error(ErrorKind::MustCall(name), property.span);
                    return Ty::Any;
                }
                common(&types)
            }
            Lookup::None => Ty::Any,
        }
    }

    fn walk_args(&mut self, args: &[Expr]) -> Vec<Ty> {
        args.iter().map(|a| self.expr(a)).collect()
    }

    fn call(&mut self, call: &Expr, callee: &Expr, args: &[Expr]) -> Ty {
        match &callee.kind {
            ExprKind::Ident(id) => match self.resolve(id) {
                Some(Resolution::Builtin(g)) => match g.kind {
                    GlobalKind::Function(sig) => self.check_call(call, g.name, &sig, args, &[]),
                    GlobalKind::Class { .. } => {
                        self.error(ErrorKind::MissingNew(g.name.into()), id.span);
                        self.walk_args(args);
                        Ty::Any
                    }
                    GlobalKind::Namespace { .. } => {
                        self.error(ErrorKind::NotCallable(Some(g.name.into())), id.span);
                        self.walk_args(args);
                        Ty::Any
                    }
                },
                Some(Resolution::Binding(decl)) => {
                    let b = self.binding(decl).clone();
                    self.walk_args(args);
                    match b.ty {
                        Ty::Function | Ty::Any => {}
                        _ => {
                            self.error(ErrorKind::NotCallable(Some(b.name.clone())), id.span);
                            return Ty::Any;
                        }
                    }
                    match b.fn_info {
                        Some(info) => {
                            if args.len() != info.arity {
                                self.error(
                                    ErrorKind::WrongArgCount {
                                        name: b.name.clone(),
                                        min: info.arity,
                                        max: Some(info.arity),
                                        found: args.len(),
                                    },
                                    call.span,
                                );
                            }
                            if info.is_async { Ty::Promise } else { Ty::Any }
                        }
                        None => Ty::Any,
                    }
                }
                None => {
                    self.walk_args(args);
                    Ty::Any
                }
            },
            ExprKind::Member { object, property } => match self.lookup_member(object, property) {
                Lookup::Found {
                    member,
                    full_name,
                    siblings,
                } => match member.kind {
                    MemberKind::Method(sig) => {
                        self.check_call(call, &full_name, &sig, args, siblings)
                    }
                    _ => {
                        self.error(ErrorKind::NotCallable(Some(full_name)), property.span);
                        self.walk_args(args);
                        Ty::Any
                    }
                },
                Lookup::Dynamic(found) => {
                    self.walk_args(args);
                    let returns: Vec<Ty> = found
                        .iter()
                        .filter_map(|m| match m.kind {
                            MemberKind::Method(Signature {
                                returns: Returns::Is(t),
                                ..
                            }) => Some(t),
                            MemberKind::Method(_) => Some(Ty::Any),
                            _ => None,
                        })
                        .collect();
                    if returns.is_empty() {
                        let name = path_text(callee).unwrap_or(property.name.clone());
                        self.error(ErrorKind::NotCallable(Some(name)), property.span);
                        return Ty::Any;
                    }
                    common(&returns)
                }
                Lookup::None => {
                    self.walk_args(args);
                    Ty::Any
                }
            },
            _ => {
                let t = self.expr(callee);
                self.walk_args(args);
                if !matches!(t, Ty::Function | Ty::Any) {
                    self.error(ErrorKind::NotCallable(None), callee.span);
                }
                Ty::Any
            }
        }
    }

    /// Checks the arguments of a built-in call and returns the result type.
    ///
    /// `siblings` are the other members of the same class; they are used to
    /// suggest `robot.turn` for `robot.drive(90deg)`.
    fn check_call(
        &mut self,
        call: &Expr,
        name: &str,
        sig: &Signature,
        args: &[Expr],
        siblings: &'static [Member],
    ) -> Ty {
        let types = self.walk_args(args);
        let n = args.len();
        if n < sig.min_args() || sig.max_args().is_some_and(|max| n > max) {
            self.error(
                ErrorKind::WrongArgCount {
                    name: name.into(),
                    min: sig.min_args(),
                    max: sig.max_args(),
                    found: n,
                },
                call.span,
            );
        }
        let mut number_kind: Option<Ty> = None;
        for (i, (arg, &ty)) in args.iter().zip(&types).enumerate() {
            let Some(param) = sig.param(i) else { break };
            match param.expect {
                Expect::Any => {}
                Expect::Number => match ty {
                    Ty::Num(found) => match number_kind {
                        None => number_kind = Some(ty),
                        Some(Ty::Num(first)) if first != found && !is_zero_literal(arg) => {
                            self.error(
                                ErrorKind::WrongArgUnit {
                                    name: name.into(),
                                    param: param.name,
                                    expected: first,
                                    found,
                                    alternative: None,
                                },
                                arg.span,
                            );
                        }
                        Some(_) => {}
                    },
                    Ty::Any => {}
                    _ => self.wrong_arg_type(name, param.name, param.expect, ty, arg.span),
                },
                Expect::Is(expected) => {
                    let alternative = if i == 0 {
                        alternative_for(name, siblings, ty)
                    } else {
                        None
                    };
                    self.check_arg(name, param, expected, arg, ty, alternative);
                }
            }
        }
        match sig.returns {
            Returns::Is(t) => t,
            Returns::SameKindAsArgs => number_kind.unwrap_or(Ty::Any),
        }
    }

    fn check_arg(
        &mut self,
        name: &str,
        param: &stdlib::Param,
        expected: Ty,
        arg: &Expr,
        found: Ty,
        alternative: Option<String>,
    ) {
        if found == Ty::Any || found == expected {
            return;
        }
        if let (Ty::Num(expected), Ty::Num(found)) = (expected, found) {
            if !is_zero_literal(arg) {
                self.error(
                    ErrorKind::WrongArgUnit {
                        name: name.into(),
                        param: param.name,
                        expected,
                        found,
                        alternative,
                    },
                    arg.span,
                );
            }
            return;
        }
        self.wrong_arg_type(name, param.name, param.expect, found, arg.span);
    }

    fn wrong_arg_type(
        &mut self,
        name: &str,
        param: &'static str,
        expected: Expect,
        found: Ty,
        span: Span,
    ) {
        self.error(
            ErrorKind::WrongArgType {
                name: name.into(),
                param,
                expected,
                found,
            },
            span,
        );
    }

    fn new_object(&mut self, e: &Expr, callee: &Expr, args: &[Expr]) -> Ty {
        let ExprKind::Ident(id) = &callee.kind else {
            self.expr(callee);
            self.walk_args(args);
            self.error(ErrorKind::NotAClass(path_text(callee)), callee.span);
            return Ty::Any;
        };
        match self.resolve(id) {
            Some(Resolution::Builtin(Global {
                name,
                kind: GlobalKind::Class { ctor, .. },
                ..
            })) => self.check_call(e, name, ctor, args, &[]),
            Some(_) => {
                self.walk_args(args);
                self.error(ErrorKind::NotAClass(Some(id.name.clone())), id.span);
                Ty::Any
            }
            None => {
                self.walk_args(args);
                Ty::Any
            }
        }
    }

    /// Checks the left side of `=`, `+=`, `++`, … and returns its current type.
    fn assign_target(&mut self, target: &Expr) -> Ty {
        match &target.kind {
            ExprKind::Ident(id) => match self.resolve(id) {
                Some(Resolution::Binding(decl)) => {
                    let b = self.binding(decl).clone();
                    match b.kind {
                        BindingKind::Const => self.error_related(
                            ErrorKind::AssignToConst(b.name.clone()),
                            id.span,
                            decl,
                        ),
                        BindingKind::Function => {
                            self.error(ErrorKind::AssignToFunction(b.name.clone()), id.span)
                        }
                        BindingKind::Let | BindingKind::Param => {}
                    }
                    b.ty
                }
                Some(Resolution::Builtin(g)) => {
                    self.error(ErrorKind::ReadOnly(g.name.into()), id.span);
                    Ty::Any
                }
                None => Ty::Any,
            },
            ExprKind::Member { object, property } => {
                let ty = self.member_value(object, property);
                let name = path_text(target).unwrap_or(property.name.clone());
                self.error(ErrorKind::ReadOnly(name), property.span);
                ty
            }
            ExprKind::Index { object, index } => {
                self.expr(object);
                self.expr(index);
                Ty::Any
            }
            // Other targets were already reported by the parser.
            _ => {
                self.expr(target);
                Ty::Any
            }
        }
    }

    // ------------------------------------------------------------------
    // Units
    // ------------------------------------------------------------------

    /// `x++` / `x--`: only for plain numbers.
    fn check_update(&mut self, e: &Expr, _op: UpdateOp, t: Ty) {
        if let Ty::Num(d) = t {
            if d != Dim::Number {
                self.error(ErrorKind::UnitIncrement(d), e.span);
            }
        }
    }

    /// `x = v`, `x += v`, …: the variable keeps its kind of number.
    fn check_assign(&mut self, e: &Expr, op: AssignOp, target: &Expr, value: &Expr, t: Ty, v: Ty) {
        let new = match op {
            AssignOp::Assign => {
                if is_zero_literal(value) {
                    return;
                }
                v
            }
            _ => {
                let bop = match op {
                    AssignOp::Add => BinaryOp::Add,
                    AssignOp::Sub => BinaryOp::Sub,
                    AssignOp::Mul => BinaryOp::Mul,
                    AssignOp::Div => BinaryOp::Div,
                    AssignOp::Rem => BinaryOp::Rem,
                    AssignOp::Assign => unreachable!("handled above"),
                };
                let before = self.out.diagnostics.len();
                let result = self.binary(e, bop, target, value, t, v);
                if self.out.diagnostics.len() != before {
                    // The operator itself was already reported.
                    return;
                }
                result
            }
        };
        if let (Ty::Num(old), Ty::Num(new)) = (t, new) {
            if old != new {
                self.error(
                    ErrorKind::UnitAssign {
                        name: path_text(target),
                        old,
                        new,
                    },
                    e.span,
                );
            }
        }
    }

    /// Type of `l op r`, with unit checks.
    ///
    /// - `+`, `-`, `%`, comparisons: both sides must be the same kind
    ///   (a plain `0` fits every kind).
    /// - `*`: at most one side may have a unit.
    /// - `/`: by a plain number (keeps the kind) or by the same kind (gives
    ///   a plain number).
    fn binary(&mut self, e: &Expr, op: BinaryOp, lhs: &Expr, rhs: &Expr, l: Ty, r: Ty) -> Ty {
        use BinaryOp as B;
        if op == B::Add && (l == Ty::Text || r == Ty::Text) {
            return Ty::Text;
        }
        if matches!(op, B::And | B::Or) {
            return if l == r { l } else { Ty::Any };
        }
        let (Ty::Num(a), Ty::Num(b)) = (l, r) else {
            return match op {
                B::Eq | B::NotEq | B::Lt | B::Le | B::Gt | B::Ge => Ty::Bool,
                _ => Ty::Any,
            };
        };
        let zero = is_zero_literal(lhs) || is_zero_literal(rhs);
        let symbol = match op {
            B::Add => "+",
            B::Sub => "-",
            B::Rem => "%",
            B::Eq => "==",
            B::NotEq => "!=",
            B::Lt => "<",
            B::Le => "<=",
            B::Gt => ">",
            B::Ge => ">=",
            B::Mul => "*",
            B::Div => "/",
            B::And | B::Or => unreachable!("handled above"),
        };
        match op {
            B::Add | B::Sub | B::Rem | B::Eq | B::NotEq | B::Lt | B::Le | B::Gt | B::Ge => {
                if a != b && !zero {
                    self.error(
                        ErrorKind::UnitMismatch {
                            op: symbol,
                            left: a,
                            right: b,
                        },
                        e.span,
                    );
                }
                match op {
                    B::Add | B::Sub | B::Rem if a == b => l,
                    B::Add | B::Sub => Ty::Num(if is_zero_literal(lhs) { b } else { a }),
                    B::Rem => l,
                    _ => Ty::Bool,
                }
            }
            B::Mul => match (a, b) {
                (Dim::Number, _) => r,
                (_, Dim::Number) => l,
                _ => {
                    self.error(ErrorKind::UnitProduct { left: a, right: b }, e.span);
                    Ty::Any
                }
            },
            B::Div => match (a, b) {
                (_, Dim::Number) => l,
                _ if a == b => Ty::Num(Dim::Number),
                _ => {
                    self.error(ErrorKind::UnitQuotient { left: a, right: b }, e.span);
                    Ty::Any
                }
            },
            B::And | B::Or => unreachable!("handled above"),
        }
    }
}

/// For `robot.drive(90deg)`: a sibling command whose first parameter takes
/// the given kind of number (`robot.turn`).
fn alternative_for(name: &str, siblings: &'static [Member], found: Ty) -> Option<String> {
    let Ty::Num(found) = found else { return None };
    let sibling = siblings.iter().find(|m| match m.kind {
        MemberKind::Method(sig) => {
            sig.params.first().map(|p| p.expect) == Some(Expect::Is(Ty::Num(found)))
        }
        _ => false,
    })?;
    let owner = name.rsplit_once('.').map_or(name, |(owner, _)| owner);
    Some(format!("{owner}.{}", sibling.name))
}

/// `(alias, name)` pairs of members, for suggestions.
fn member_aliases<'a>(
    members: impl Iterator<Item = &'a Member>,
) -> Vec<(&'static str, &'static str)> {
    members
        .flat_map(|m| m.aliases.iter().map(move |a| (*a, m.name)))
        .collect()
}

/// The common type of several candidates, or `Any`.
fn common(types: &[Ty]) -> Ty {
    match types.split_first() {
        Some((first, rest)) if rest.iter().all(|t| t == first) => *first,
        _ => Ty::Any,
    }
}
