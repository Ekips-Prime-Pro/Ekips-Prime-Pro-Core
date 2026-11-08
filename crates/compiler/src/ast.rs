//! Syntax tree produced by the parser.

use crate::span::Span;

/// A complete source file: a list of top-level statements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    /// Top-level statements in source order
    pub body: Vec<Stmt>,
}

/// A name written by the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    /// The name (case-sensitive)
    pub name: String,
    /// Position in the source
    pub span: Span,
}

/// `const` or `let`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    /// `const` – can not be changed later
    Const,
    /// `let` – can be changed later
    Let,
}

/// A function declaration: `[async] function name(params) { … }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Function name
    pub name: Ident,
    /// Parameter names
    pub params: Vec<Ident>,
    /// Function body
    pub body: Vec<Stmt>,
    /// Declared with `async`
    pub is_async: bool,
    /// Position of the header (`async function name(…)`)
    pub span: Span,
}

/// A statement with its position.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// What the statement does
    pub kind: StmtKind,
    /// Position in the source
    pub span: Span,
}

/// The different statements.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `const x = …;` / `let x = …;` / `let x;`
    VarDecl {
        /// `const` or `let`
        kind: DeclKind,
        /// Variable name
        name: Ident,
        /// Initial value
        init: Option<Expr>,
    },
    /// A function declaration
    Function(Function),
    /// An expression used as a statement, e.g. `await robot.drive(20cm);`
    Expr(Expr),
    /// `if (cond) … else …`
    If {
        /// Condition
        cond: Expr,
        /// Statement run when the condition is true
        then: Box<Stmt>,
        /// Statement run otherwise
        otherwise: Option<Box<Stmt>>,
    },
    /// `while (cond) …`
    While {
        /// Condition
        cond: Expr,
        /// Loop body
        body: Box<Stmt>,
    },
    /// `for (init; cond; update) …`
    For {
        /// Runs once before the loop
        init: Option<Box<Stmt>>,
        /// Checked before every run
        cond: Option<Expr>,
        /// Runs after every run
        update: Option<Expr>,
        /// Loop body
        body: Box<Stmt>,
    },
    /// `for (const x of list) …`
    ForOf {
        /// `const` or `let`
        kind: DeclKind,
        /// Loop variable
        name: Ident,
        /// The list
        iterable: Expr,
        /// Loop body
        body: Box<Stmt>,
    },
    /// `{ … }`
    Block(Vec<Stmt>),
    /// `return …;`
    Return(Option<Expr>),
    /// `break;`
    Break,
    /// `continue;`
    Continue,
    /// A lone `;`
    Empty,
}

/// Units that can be written directly after a number (`20cm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// `mm`
    Mm,
    /// `cm`
    Cm,
    /// `deg` or `°`
    Deg,
    /// `rot` (full motor rotations)
    Rot,
    /// `s`
    S,
    /// `ms`
    Ms,
    /// `%`
    Percent,
}

impl Unit {
    /// Looks up a unit suffix.
    pub fn from_suffix(suffix: &str) -> Option<Unit> {
        Some(match suffix {
            "mm" => Unit::Mm,
            "cm" => Unit::Cm,
            "deg" | "°" => Unit::Deg,
            "rot" => Unit::Rot,
            "s" => Unit::S,
            "ms" => Unit::Ms,
            "%" => Unit::Percent,
            _ => return None,
        })
    }

    /// The physical kind of the unit.
    pub fn dim(self) -> ekips_bytecode::Dim {
        use ekips_bytecode::Dim;
        match self {
            Unit::Mm | Unit::Cm => Dim::Length,
            Unit::Deg | Unit::Rot => Dim::Angle,
            Unit::S | Unit::Ms => Dim::Time,
            Unit::Percent => Dim::Percent,
        }
    }

    /// Converts a value in this unit to the base unit of its kind
    /// (mm, degree, ms, percent).
    pub fn to_base(self, value: f64) -> f64 {
        match self {
            Unit::Mm | Unit::Deg | Unit::Ms | Unit::Percent => value,
            Unit::Cm => value * 10.0,
            Unit::Rot => value * 360.0,
            Unit::S => value * 1000.0,
        }
    }

    /// Canonical suffix.
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Cm => "cm",
            Unit::Deg => "deg",
            Unit::Rot => "rot",
            Unit::S => "s",
            Unit::Ms => "ms",
            Unit::Percent => "%",
        }
    }
}

/// Prefix operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `!x`
    Not,
    /// `-x`
    Neg,
    /// `+x`
    Plus,
}

/// `++` / `--`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    /// `++`
    Inc,
    /// `--`
    Dec,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `||`
    Or,
    /// `&&`
    And,
    /// `==` or `===` (both compare strictly)
    Eq,
    /// `!=` or `!==`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    Add,
    /// `-=`
    Sub,
    /// `*=`
    Mul,
    /// `/=`
    Div,
    /// `%=`
    Rem,
}

/// Body of an arrow function.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    /// `() => expr`
    Expr(Box<Expr>),
    /// `() => { … }`
    Block(Vec<Stmt>),
}

/// An expression with its position.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// What the expression is
    pub kind: ExprKind,
    /// Position in the source
    pub span: Span,
}

impl Expr {
    /// Placeholder for an expression that could not be parsed.
    pub fn error(span: Span) -> Self {
        Expr {
            kind: ExprKind::Error,
            span,
        }
    }
}

/// The different expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// `42`, `2.5`, `20cm`, `50%`
    Number {
        /// The value as written
        value: f64,
        /// Optional unit suffix
        unit: Option<Unit>,
    },
    /// `"hello"` or `'hello'`
    Str(String),
    /// `true` / `false`
    Bool(bool),
    /// `null`
    Null,
    /// `undefined`
    Undefined,
    /// A variable, function or built-in name
    Ident(Ident),
    /// `[a, b, c]`
    Array(Vec<Expr>),
    /// `object.property`
    Member {
        /// Left side
        object: Box<Expr>,
        /// Property name
        property: Ident,
    },
    /// `object[index]`
    Index {
        /// Left side
        object: Box<Expr>,
        /// Index
        index: Box<Expr>,
    },
    /// `callee(args)`
    Call {
        /// Function being called
        callee: Box<Expr>,
        /// Arguments
        args: Vec<Expr>,
    },
    /// `new Class(args)`
    New {
        /// Class being created
        callee: Box<Expr>,
        /// Arguments
        args: Vec<Expr>,
    },
    /// `!x`, `-x`, `+x`
    Unary {
        /// Operator
        op: UnaryOp,
        /// Operand
        operand: Box<Expr>,
    },
    /// `i++`, `--i`
    Update {
        /// `++` or `--`
        op: UpdateOp,
        /// `true` for `++i`, `false` for `i++`
        prefix: bool,
        /// Variable or property being changed
        target: Box<Expr>,
    },
    /// `a + b`, `a && b`, …
    Binary {
        /// Operator
        op: BinaryOp,
        /// Left side
        lhs: Box<Expr>,
        /// Right side
        rhs: Box<Expr>,
    },
    /// `cond ? a : b`
    Conditional {
        /// Condition
        cond: Box<Expr>,
        /// Value if true
        then: Box<Expr>,
        /// Value if false
        otherwise: Box<Expr>,
    },
    /// `x = …`, `x += …`
    Assign {
        /// Operator
        op: AssignOp,
        /// Variable or property being changed
        target: Box<Expr>,
        /// New value
        value: Box<Expr>,
    },
    /// `await x`
    Await(Box<Expr>),
    /// `(a, b) => …`, `async () => { … }`
    Arrow {
        /// Parameter names
        params: Vec<Ident>,
        /// Body
        body: ArrowBody,
        /// Declared with `async`
        is_async: bool,
    },
    /// Could not be parsed (an error was already reported)
    Error,
}
