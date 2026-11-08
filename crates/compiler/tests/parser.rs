//! Parser tests: valid programs, operator precedence and error messages.

use ekips_compiler::ast::*;
use ekips_compiler::diagnostic::{ErrorKind, Expected, Found};
use ekips_compiler::{Lang, parse};

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

/// Wraps statements into `async function main() { … }`.
fn in_main(body: &str) -> String {
    format!("async function main() {{\n{body}\n}}\n")
}

/// Parses and asserts that there are no problems.
fn parse_ok(source: &str) -> Program {
    let result = parse(source);
    let rendered: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| d.render(source, "test.ekips", Lang::En))
        .collect();
    assert!(
        rendered.is_empty(),
        "unexpected problems:\n{}",
        rendered.join("\n")
    );
    result.program
}

/// Kinds of all problems found in `source`.
fn errors(source: &str) -> Vec<ErrorKind> {
    parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.kind)
        .collect()
}

/// Body of the `main` function.
fn main_body(program: &Program) -> &[Stmt] {
    program
        .body
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::Function(f) if f.name.name == "main" => Some(f.body.as_slice()),
            _ => None,
        })
        .expect("main function")
}

/// Parses a single expression (inside `main`) and returns it as an S-expression.
fn expr(source: &str) -> String {
    let program = parse_ok(&in_main(source));
    match &main_body(&program)[0].kind {
        StmtKind::Expr(e) => sexp(e),
        other => panic!("not an expression statement: {other:?}"),
    }
}

/// Compact, readable form of an expression for assertions.
fn sexp(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Number { value, unit } => match unit {
            Some(u) => format!("{value}{}", u.suffix()),
            None => format!("{value}"),
        },
        ExprKind::Str(s) => format!("{s:?}"),
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Null => "null".into(),
        ExprKind::Undefined => "undefined".into(),
        ExprKind::Ident(i) => i.name.clone(),
        ExprKind::Array(items) => format!("[{}]", list(items)),
        ExprKind::Member { object, property } => format!("{}.{}", sexp(object), property.name),
        ExprKind::Index { object, index } => format!("{}[{}]", sexp(object), sexp(index)),
        ExprKind::Call { callee, args } => format!("{}({})", sexp(callee), list(args)),
        ExprKind::New { callee, args } => format!("(new {}({}))", sexp(callee), list(args)),
        ExprKind::Unary { op, operand } => {
            let op = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
                UnaryOp::Plus => "+",
            };
            format!("({op} {})", sexp(operand))
        }
        ExprKind::Update { op, prefix, target } => {
            let op = if *op == UpdateOp::Inc { "++" } else { "--" };
            if *prefix {
                format!("({op}pre {})", sexp(target))
            } else {
                format!("({op}post {})", sexp(target))
            }
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let op = match op {
                BinaryOp::Or => "||",
                BinaryOp::And => "&&",
                BinaryOp::Eq => "==",
                BinaryOp::NotEq => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Rem => "%",
            };
            format!("({op} {} {})", sexp(lhs), sexp(rhs))
        }
        ExprKind::Conditional {
            cond,
            then,
            otherwise,
        } => format!("(? {} {} {})", sexp(cond), sexp(then), sexp(otherwise)),
        ExprKind::Assign { op, target, value } => {
            let op = match op {
                AssignOp::Assign => "=",
                AssignOp::Add => "+=",
                AssignOp::Sub => "-=",
                AssignOp::Mul => "*=",
                AssignOp::Div => "/=",
                AssignOp::Rem => "%=",
            };
            format!("({op} {} {})", sexp(target), sexp(value))
        }
        ExprKind::Await(inner) => format!("(await {})", sexp(inner)),
        ExprKind::Arrow {
            params,
            body,
            is_async,
        } => {
            let params: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            let body = match body {
                ArrowBody::Expr(e) => sexp(e),
                ArrowBody::Block(stmts) => format!("{{{} stmts}}", stmts.len()),
            };
            let prefix = if *is_async { "async " } else { "" };
            format!("({prefix}({}) => {body})", params.join(","))
        }
        ExprKind::Error => "<error>".into(),
    }
}

fn list(items: &[Expr]) -> String {
    items.iter().map(sexp).collect::<Vec<_>>().join(", ")
}

// ----------------------------------------------------------------------
// Example programs
// ----------------------------------------------------------------------

#[test]
fn example_programs_parse_cleanly() {
    let examples = [
        include_str!("../../../examples/square.ekips"),
        include_str!("../../../examples/drive_to_line.ekips"),
        include_str!("../../../examples/parallel.ekips"),
        include_str!("../../../examples/missions.ekips"),
    ];
    for source in examples {
        parse_ok(source);
    }
}

// ----------------------------------------------------------------------
// Expressions
// ----------------------------------------------------------------------

#[test]
fn precedence() {
    assert_eq!(expr("a + b * c"), "(+ a (* b c))");
    assert_eq!(expr("(a + b) * c"), "(* (+ a b) c)");
    assert_eq!(expr("a - b - c"), "(- (- a b) c)");
    assert_eq!(expr("a || b && c"), "(|| a (&& b c))");
    assert_eq!(expr("a < b == c >= d"), "(== (< a b) (>= c d))");
    assert_eq!(expr("!a && -b > 2"), "(&& (! a) (> (- b) 2))");
    assert_eq!(expr("x = y = 3"), "(= x (= y 3))");
    assert_eq!(expr("a ? b : c ? d : e"), "(? a b (? c d e))");
    assert_eq!(expr("x === 1 || y !== 2"), "(|| (== x 1) (!= y 2))");
    assert_eq!(expr("10 % 3"), "(% 10 3)");
}

#[test]
fn units() {
    assert_eq!(expr("robot.drive(20cm, 50%)"), "robot.drive(20cm, 50%)");
    assert_eq!(expr("-2.5rot"), "(- 2.5rot)");
    assert_eq!(expr("wait(1.5s + 300ms)"), "wait((+ 1.5s 300ms))");
    assert_eq!(expr("x = 90°"), "(= x 90deg)");
    assert_eq!(expr("speed -= 20%"), "(-= speed 20%)");
}

#[test]
fn members_calls_and_new() {
    assert_eq!(expr("await robot.drive(20cm)"), "(await robot.drive(20cm))");
    assert_eq!(expr("eye.sees(Color.Black)"), "eye.sees(Color.Black)");
    assert_eq!(expr("x = new Motor(Port.A)"), "(= x (new Motor(Port.A)))");
    assert_eq!(expr("list[i + 1].angle"), "list[(+ i 1)].angle");
    assert_eq!(expr("Math.max(1, 2)"), "Math.max(1, 2)");
    assert_eq!(expr("f()()"), "f()()");
    assert_eq!(expr("Promise.race([a(), b()])"), "Promise.race([a(), b()])");
}

#[test]
fn update_operators() {
    assert_eq!(expr("i++"), "(++post i)");
    assert_eq!(expr("--i"), "(--pre i)");
    assert_eq!(expr("arm.count++"), "(++post arm.count)");
}

#[test]
fn arrow_functions() {
    assert_eq!(
        expr("waitUntil(() => eye.sees(Color.Black))"),
        "waitUntil((() => eye.sees(Color.Black)))"
    );
    assert_eq!(expr("f = x => x * 2"), "(= f ((x) => (* x 2)))");
    assert_eq!(expr("f = (a, b) => a + b"), "(= f ((a,b) => (+ a b)))");
    assert_eq!(
        expr("f = async () => { await wait(1s); hub.beep(); }"),
        "(= f (async () => {2 stmts}))"
    );
    // A bracketed expression is not an arrow function.
    assert_eq!(expr("(a + b)"), "(+ a b)");
}

#[test]
fn arrays() {
    assert_eq!(expr("x = []"), "(= x [])");
    assert_eq!(expr("x = [1, 2, 3,]"), "(= x [1, 2, 3])");
}

// ----------------------------------------------------------------------
// Statements
// ----------------------------------------------------------------------

#[test]
fn declarations() {
    let program = parse_ok(&in_main("const a = 1;\nlet b;\nlet c = a + 1"));
    let body = main_body(&program);
    assert!(matches!(
        &body[0].kind,
        StmtKind::VarDecl { kind: DeclKind::Const, name, init: Some(_) } if name.name == "a"
    ));
    assert!(matches!(
        &body[1].kind,
        StmtKind::VarDecl {
            kind: DeclKind::Let,
            init: None,
            ..
        }
    ));
    assert_eq!(body.len(), 3);
}

#[test]
fn functions() {
    let program = parse_ok(
        "function double(x) { return x * 2 }\nasync function go(a, b,) { await wait(a) }\nasync function main() {}",
    );
    let StmtKind::Function(f) = &program.body[0].kind else {
        panic!("not a function");
    };
    assert_eq!(f.name.name, "double");
    assert!(!f.is_async);
    let StmtKind::Function(g) = &program.body[1].kind else {
        panic!("not a function");
    };
    assert!(g.is_async);
    assert_eq!(g.params.len(), 2);
}

#[test]
fn if_else_chain() {
    let program = parse_ok(&in_main(
        "if (a) {\n  x = 1\n} else if (b) {\n  x = 2\n} else {\n  x = 3\n}",
    ));
    let StmtKind::If { otherwise, .. } = &main_body(&program)[0].kind else {
        panic!("not an if");
    };
    let inner = otherwise.as_ref().expect("else part");
    assert!(matches!(
        &inner.kind,
        StmtKind::If {
            otherwise: Some(_),
            ..
        }
    ));
}

#[test]
fn loops() {
    let program = parse_ok(&in_main(
        "for (let i = 0; i < 4; i++) { await robot.turn(90deg) }\n\
         for (const m of missions) { await m() }\n\
         for (;;) { break }\n\
         while (true) { continue }",
    ));
    let body = main_body(&program);
    assert!(matches!(
        &body[0].kind,
        StmtKind::For {
            init: Some(_),
            cond: Some(_),
            update: Some(_),
            ..
        }
    ));
    assert!(matches!(&body[1].kind, StmtKind::ForOf { name, .. } if name.name == "m"));
    assert!(matches!(
        &body[2].kind,
        StmtKind::For {
            init: None,
            cond: None,
            update: None,
            ..
        }
    ));
    assert!(matches!(&body[3].kind, StmtKind::While { .. }));
}

#[test]
fn semicolons_are_optional_at_line_end() {
    let program = parse_ok(&in_main("let a = 1\nlet b = 2; let c = 3\nhub.beep()"));
    assert_eq!(main_body(&program).len(), 4);
}

#[test]
fn bracket_on_new_line_starts_new_statement() {
    // In JavaScript this would be `b(c)`; here it is two statements.
    let program = parse_ok(&in_main("let a = b\n(c)"));
    assert_eq!(main_body(&program).len(), 2);
}

// ----------------------------------------------------------------------
// Errors
// ----------------------------------------------------------------------

#[test]
fn misspelled_keyword_is_recovered() {
    let errs = errors(&in_main("cosnt speed = 50%\nawait robot.drive(10cm)"));
    assert_eq!(
        errs,
        vec![ErrorKind::UnknownWord {
            word: "cosnt".into(),
            suggestion: Some("const")
        }]
    );
}

#[test]
fn german_keywords_get_suggestions() {
    let errs = errors(&in_main("wenn (x > 3) {\n  hub.beep()\n}"));
    assert_eq!(
        errs,
        vec![ErrorKind::UnknownWord {
            word: "wenn".into(),
            suggestion: Some("if")
        }]
    );
    let errs = errors("funktion helper() {}\nasync function main() {}");
    assert_eq!(
        errs,
        vec![ErrorKind::UnknownWord {
            word: "funktion".into(),
            suggestion: Some("function")
        }]
    );
}

#[test]
fn var_and_types_are_explained() {
    let errs = errors(&in_main("var x = 1\nlet y: number = 2"));
    assert_eq!(errs, vec![ErrorKind::VarKeyword, ErrorKind::TypesNotNeeded]);
    let errs =
        errors("function f(a: number, b: Map<string, number>): void {}\nasync function main() {}");
    assert_eq!(
        errs,
        vec![
            ErrorKind::TypesNotNeeded,
            ErrorKind::TypesNotNeeded,
            ErrorKind::TypesNotNeeded
        ]
    );
}

#[test]
fn await_needs_async() {
    let errs = errors("function f() { await wait(1s) }\nasync function main() {}");
    assert_eq!(errs, vec![ErrorKind::AwaitOutsideAsync]);
    // An async arrow inside a normal function is fine.
    assert!(
        errors("function f() { return async () => { await wait(1s) } }\nasync function main() {}")
            .is_empty()
    );
    // A normal arrow inside an async function is not.
    let errs = errors(&in_main("const f = () => { await wait(1s) }"));
    assert_eq!(errs, vec![ErrorKind::AwaitOutsideAsync]);
}

#[test]
fn main_is_required_and_async() {
    assert_eq!(errors("let x = 1"), vec![ErrorKind::MissingMain]);
    assert_eq!(errors("function main() {}"), vec![ErrorKind::MainNotAsync]);
}

#[test]
fn braces() {
    assert_eq!(
        errors("async function main() {\n  hub.beep()\n"),
        vec![ErrorKind::MissingCloseBrace]
    );
    assert_eq!(
        errors("async function main() {}\n}"),
        vec![ErrorKind::UnmatchedCloseBrace]
    );
}

#[test]
fn context_errors() {
    assert_eq!(
        errors(&in_main("break\ncontinue")),
        vec![
            ErrorKind::OutsideLoop("break"),
            ErrorKind::OutsideLoop("continue")
        ]
    );
    assert_eq!(
        errors("return 1\nasync function main() {}"),
        vec![ErrorKind::ReturnOutsideFunction]
    );
    assert_eq!(
        errors(&in_main("5 = x")),
        vec![ErrorKind::InvalidAssignTarget]
    );
    assert_eq!(
        errors(&in_main("else { }")),
        vec![ErrorKind::UnexpectedElse]
    );
}

#[test]
fn missing_condition_brackets() {
    let errs = errors(&in_main("if x > 3 {\n  hub.beep()\n}"));
    assert_eq!(
        errs,
        vec![ErrorKind::Expected {
            expected: Expected::Token("("),
            found: Found::Word("x".into())
        }]
    );
}

#[test]
fn two_statements_on_one_line() {
    let errs = errors(&in_main("robot.drive(20cm) robot.turn(90deg)"));
    assert_eq!(
        errs,
        vec![ErrorKind::MissingSemicolon(Found::Word("robot".into()))]
    );
}

#[test]
fn recovery_reports_everything_at_once() {
    let source = "\
cosnt arm = new Motor(Port.C)
var speed = 50%

async function main() {
  wenn (speed > 3) { hub.show(1) }
  5 = speed
  break
  # comment
}
";
    let errs = errors(source);
    assert_eq!(errs.len(), 6, "{errs:#?}");
    // The broken lines must not close `main` too early.
    assert!(!errs.contains(&ErrorKind::UnmatchedCloseBrace));
}

#[test]
fn rendering_in_both_languages() {
    let source = in_main("  drvie(20cm)\n  cosnt x = 1");
    let result = parse(&source);
    let diag = result
        .diagnostics
        .iter()
        .find(|d| matches!(d.kind, ErrorKind::UnknownWord { .. }))
        .expect("unknown word error");

    let en = diag.render(&source, "mission.ekips", Lang::En);
    assert_eq!(
        en,
        "error: I don't understand the word `cosnt` here.\n\
         \x20--> mission.ekips:3:3\n\
         \x20 |\n\
         3 |   cosnt x = 1\n\
         \x20 |   ^^^^^\n\
         \x20 = help: Did you mean `const`?\n"
    );

    let de = diag.render(&source, "mission.ekips", Lang::De);
    assert!(de.starts_with("Fehler: Das Wort `cosnt` verstehe ich hier nicht."));
    assert!(de.contains("= Hilfe: Meintest du `const`?"));
}
