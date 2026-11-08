//! Error messages, written for students.
//!
//! Every message exists in English and German. Messages say what is
//! wrong in plain words and, where possible, how to fix it.

use std::fmt::Write as _;

use crate::span::{LineIndex, Span};
use crate::stdlib::Expect;
use crate::types::{Dim, Ty, describe_dim, example_dim};

/// Language of the messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// English
    #[default]
    En,
    /// German
    De,
}

/// How bad a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The program can not run.
    Error,
    /// The program can run, but probably does not do what was meant.
    Warning,
}

/// What the parser was looking for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expected {
    /// A name for a variable, function or parameter
    Name,
    /// A value or calculation
    Value,
    /// A statement
    Statement,
    /// A property or method name after `.`
    Property,
    /// A specific symbol or keyword
    Token(&'static str),
    /// `,` or a closing bracket in a list
    CommaOr(&'static str),
}

/// What the parser found instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A word (name or keyword)
    Word(String),
    /// A number
    Number(String),
    /// A text in quotes
    Text,
    /// A symbol such as `(` or `+`
    Symbol(String),
    /// The end of the file
    FileEnd,
}

/// A size limit of the bytecode format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// More than 255 variables in one function
    Variables,
    /// More than 64 KiB of code in one function
    Code,
    /// More than 65535 constants, functions or globals
    Program,
    /// More than 255 arguments in one call
    Arguments,
    /// More than 65535 items in one list
    ListItems,
}

/// Whose member a `.name` was looked up on (for "unknown member" messages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberOwner {
    /// A built-in name such as `hub` or `Port`, or a class such as `DriveBase`
    Name(String),
    /// A value of this type
    Value(Ty),
}

/// The different kinds of problems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// A character the language does not use
    UnexpectedChar(char),
    /// A text without closing quote
    UnterminatedString,
    /// A `/* …` comment without `*/`
    UnterminatedComment,
    /// A number with an unknown unit suffix (`20km`)
    UnknownUnit(String),
    /// Something specific was expected
    Expected {
        /// What should be here
        expected: Expected,
        /// What is here
        found: Found,
    },
    /// Two statements on one line without `;`
    MissingSemicolon(Found),
    /// An unknown word followed by more code, probably a misspelled keyword
    UnknownWord {
        /// The word
        word: String,
        /// A likely correct keyword
        suggestion: Option<&'static str>,
    },
    /// `var` was used
    VarKeyword,
    /// A TypeScript type annotation (`: number`) was written
    TypesNotNeeded,
    /// Something that can not be assigned to (`5 = x`)
    InvalidAssignTarget,
    /// `else` without `if`
    UnexpectedElse,
    /// A `{` that is never closed
    MissingCloseBrace,
    /// A `}` that closes nothing
    UnmatchedCloseBrace,
    /// `await` outside of an `async` function
    AwaitOutsideAsync,
    /// `break` / `continue` outside of a loop
    OutsideLoop(&'static str),
    /// `return` outside of a function
    ReturnOutsideFunction,
    /// No `main` function
    MissingMain,
    /// `main` exists but is not `async`
    MainNotAsync,

    // --- Semantic analysis ---
    /// A name that is not declared anywhere
    UnknownName {
        /// The name
        name: String,
        /// A similar name that exists
        suggestion: Option<String>,
    },
    /// `.name` that does not exist on the value
    UnknownMember {
        /// What the member was looked up on
        owner: MemberOwner,
        /// The member name
        member: String,
        /// A similar member that exists
        suggestion: Option<String>,
    },
    /// The same name is declared twice in the same block (related: first declaration)
    DuplicateDeclaration(String),
    /// A declaration uses the name of a built-in (`const hub = …`)
    ShadowsBuiltin(String),
    /// A `let`/`const` is used before the line that declares it (related: declaration)
    UsedBeforeDeclaration(String),
    /// Assignment to a `const` (related: declaration)
    AssignToConst(String),
    /// Assignment to a function name
    AssignToFunction(String),
    /// Assignment to something built in (`hub.heading = 0deg`)
    ReadOnly(String),
    /// A built-in command used without `()` (`robot.stop`)
    MustCall(String),
    /// A built-in name used on its own (`const h = hub`)
    NotAValue(String),
    /// A device class called without `new` (`Motor(Port.A)`)
    MissingNew(String),
    /// `new` used with something that is not a device class
    NotAClass(Option<String>),
    /// Something that is not a function is called (`motor.angle()`)
    NotCallable(Option<String>),
    /// Wrong number of arguments
    WrongArgCount {
        /// Function name as written
        name: String,
        /// Smallest allowed number
        min: usize,
        /// Largest allowed number (`None` = unlimited)
        max: Option<usize>,
        /// Number given
        found: usize,
    },
    /// An argument of the wrong type (`new Motor(5)`)
    WrongArgType {
        /// Function name as written
        name: String,
        /// Parameter name
        param: &'static str,
        /// What the parameter accepts
        expected: Expect,
        /// What was given
        found: Ty,
    },

    // --- Unit checking ---
    /// Two different kinds of numbers combined with `+`, `-`, `<`, `==`, … (`20cm + 3s`)
    UnitMismatch {
        /// The operator as written
        op: &'static str,
        /// Kind of the left side
        left: Dim,
        /// Kind of the right side
        right: Dim,
    },
    /// Two values with units multiplied (`20cm * 3cm`)
    UnitProduct {
        /// Kind of the left side
        left: Dim,
        /// Kind of the right side
        right: Dim,
    },
    /// A division that has no meaning (`3 / 2s`, `20cm / 2s`)
    UnitQuotient {
        /// Kind of the left side
        left: Dim,
        /// Kind of the right side
        right: Dim,
    },
    /// A variable gets a different kind of number than it had (`let d = 5cm; d = 2s;`)
    UnitAssign {
        /// Variable name, if simple
        name: Option<String>,
        /// Kind it had
        old: Dim,
        /// Kind it gets
        new: Dim,
    },
    /// `++`/`--` on a number with unit
    UnitIncrement(Dim),
    /// An argument with the wrong kind of number (`robot.drive(90deg)`)
    WrongArgUnit {
        /// Function name as written
        name: String,
        /// Parameter name
        param: &'static str,
        /// Kind the parameter needs
        expected: Dim,
        /// Kind given
        found: Dim,
        /// A similar command that takes this kind (`robot.turn`)
        alternative: Option<String>,
    },

    // --- Code generation ---
    /// A size limit of the bytecode format was exceeded
    TooBig(Limit),

    // --- Warnings ---
    /// A Promise that is neither awaited, stored nor passed on (`robot.drive(20cm);`)
    UnusedPromise {
        /// The command, e.g. `robot.drive`
        name: Option<String>,
        /// The statement is inside an `async` function (so `await` is possible)
        in_async: bool,
    },
}

/// One problem with its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// What went wrong
    pub kind: ErrorKind,
    /// Where it went wrong
    pub span: Span,
    /// A second place that belongs to the problem, e.g. the first
    /// declaration of a name that is declared twice
    pub related: Option<Span>,
}

fn code(s: &str) -> String {
    format!("`{s}`")
}

/// "Write the unit directly after the number, e.g. `20cm`."
fn unit_missing_hint(dim: Dim, lang: Lang) -> Option<String> {
    let ex = example_dim(dim);
    Some(match lang {
        Lang::En => format!("Write the unit directly after the number, e.g. `{ex}`."),
        Lang::De => format!("Schreibe die Einheit direkt hinter die Zahl, z. B. `{ex}`."),
    })
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Describes what a parameter accepts.
pub fn describe_expect(expect: Expect, lang: Lang) -> String {
    match expect {
        Expect::Any => match lang {
            Lang::En => "a value".into(),
            Lang::De => "ein Wert".into(),
        },
        Expect::Number => match lang {
            Lang::En => "a number".into(),
            Lang::De => "eine Zahl".into(),
        },
        Expect::Is(ty) => ty.describe(lang),
    }
}

impl Expected {
    fn describe(&self, lang: Lang) -> String {
        let de = lang == Lang::De;
        match self {
            Expected::Name => if de { "ein Name" } else { "a name" }.into(),
            Expected::Value => if de { "ein Wert" } else { "a value" }.into(),
            Expected::Statement => if de { "ein Befehl" } else { "a statement" }.into(),
            Expected::Property => {
                if de {
                    "ein Eigenschafts- oder Methodenname"
                } else {
                    "a property or method name"
                }
            }
            .into(),
            Expected::Token(t) => code(t),
            Expected::CommaOr(close) => {
                if de {
                    format!("`,` oder `{close}`")
                } else {
                    format!("`,` or `{close}`")
                }
            }
        }
    }
}

impl Found {
    fn describe(&self, lang: Lang) -> String {
        let de = lang == Lang::De;
        match self {
            Found::Word(w) => code(w),
            Found::Number(n) => code(n),
            Found::Symbol(s) => code(s),
            Found::Text => if de { "ein Text" } else { "a text" }.into(),
            Found::FileEnd => if de {
                "das Dateiende"
            } else {
                "the end of the file"
            }
            .into(),
        }
    }
}

impl Diagnostic {
    /// Creates a diagnostic.
    pub fn new(kind: ErrorKind, span: Span) -> Self {
        Diagnostic {
            kind,
            span,
            related: None,
        }
    }

    /// Adds a related position (shown as a note).
    pub fn with_related(mut self, span: Span) -> Self {
        self.related = Some(span);
        self
    }

    /// `true` for errors, `false` for warnings.
    pub fn is_error(&self) -> bool {
        self.severity() == Severity::Error
    }

    /// Whether this is an error or a warning.
    pub fn severity(&self) -> Severity {
        match self.kind {
            ErrorKind::UnusedPromise { .. } => Severity::Warning,
            _ => Severity::Error,
        }
    }

    /// The main message.
    pub fn message(&self, lang: Lang) -> String {
        use ErrorKind as K;
        let de = lang == Lang::De;
        let t = |en: &str, de_text: &str| if de { de_text } else { en }.to_string();
        match &self.kind {
            K::UnexpectedChar(c) => {
                if de {
                    format!("Das Zeichen `{c}` verstehe ich nicht.")
                } else {
                    format!("I don't understand the character `{c}`.")
                }
            }
            K::UnterminatedString => t(
                "This text is missing its closing quote.",
                "Diesem Text fehlt das schließende Anführungszeichen.",
            ),
            K::UnterminatedComment => t(
                "This comment is never closed with `*/`.",
                "Dieser Kommentar wird nie mit `*/` geschlossen.",
            ),
            K::UnknownUnit(u) => {
                if de {
                    format!("`{u}` ist keine bekannte Einheit.")
                } else {
                    format!("`{u}` is not a known unit.")
                }
            }
            K::Expected { expected, found } => {
                let (e, f) = (expected.describe(lang), found.describe(lang));
                if de {
                    format!("Hier fehlt {e}. Gefunden: {f}.")
                } else {
                    format!("Expected {e}, but found {f}.")
                }
            }
            K::MissingSemicolon(found) => {
                let f = found.describe(lang);
                if de {
                    format!("Vor {f} fehlt ein `;` oder ein Zeilenumbruch.")
                } else {
                    format!("Missing `;` or line break before {f}.")
                }
            }
            K::UnknownWord { word, .. } => {
                if de {
                    format!("Das Wort `{word}` verstehe ich hier nicht.")
                } else {
                    format!("I don't understand the word `{word}` here.")
                }
            }
            K::VarKeyword => t(
                "`var` is not used in Ekips Prime Pro.",
                "`var` gibt es in Ekips Prime Pro nicht.",
            ),
            K::TypesNotNeeded => t(
                "Ekips Prime Pro has no types, so `: …` is not needed.",
                "Ekips Prime Pro hat keine Typen, `: …` wird nicht gebraucht.",
            ),
            K::InvalidAssignTarget => t(
                "You can only assign to a variable or a property.",
                "Zuweisen kann man nur an eine Variable oder Eigenschaft.",
            ),
            K::UnexpectedElse => t(
                "`else` must directly follow an `if` block.",
                "`else` muss direkt nach einem `if`-Block kommen.",
            ),
            K::MissingCloseBrace => t(
                "This `{` is never closed with `}`.",
                "Diese `{` wird nie mit `}` geschlossen.",
            ),
            K::UnmatchedCloseBrace => t(
                "This `}` does not close anything.",
                "Diese `}` schließt nichts.",
            ),
            K::AwaitOutsideAsync => t(
                "`await` only works inside an `async` function.",
                "`await` funktioniert nur in einer `async`-Funktion.",
            ),
            K::OutsideLoop(word) => {
                if de {
                    format!("`{word}` gibt es nur innerhalb einer Schleife.")
                } else {
                    format!("`{word}` only works inside a loop.")
                }
            }
            K::ReturnOutsideFunction => t(
                "`return` only works inside a function.",
                "`return` gibt es nur innerhalb einer Funktion.",
            ),
            K::MissingMain => t(
                "The program has no `main` function, so the robot would do nothing.",
                "Das Programm hat keine `main`-Funktion, der Roboter würde nichts tun.",
            ),
            K::MainNotAsync => t(
                "The `main` function must be `async`.",
                "Die `main`-Funktion muss `async` sein.",
            ),
            K::UnknownName { name, .. } => {
                if de {
                    format!("Den Namen `{name}` kenne ich nicht.")
                } else {
                    format!("I don't know the name `{name}`.")
                }
            }
            K::UnknownMember { owner, member, .. } => match owner {
                MemberOwner::Name(n) => {
                    if de {
                        format!("`{n}` hat kein `{member}`.")
                    } else {
                        format!("`{n}` has no `{member}`.")
                    }
                }
                MemberOwner::Value(ty) => {
                    let what = capitalize(&ty.describe(lang));
                    if de {
                        format!("{what} hat kein `.{member}`.")
                    } else {
                        format!("{what} has no `.{member}`.")
                    }
                }
            },
            K::DuplicateDeclaration(name) => {
                if de {
                    format!("`{name}` gibt es hier schon.")
                } else {
                    format!("`{name}` already exists here.")
                }
            }
            K::ShadowsBuiltin(name) => {
                if de {
                    format!("`{name}` ist schon ein eingebauter Name.")
                } else {
                    format!("`{name}` is already a built-in name.")
                }
            }
            K::UsedBeforeDeclaration(name) => {
                if de {
                    format!("`{name}` wird benutzt, bevor es erstellt wird.")
                } else {
                    format!("`{name}` is used before it is created.")
                }
            }
            K::AssignToConst(name) => {
                if de {
                    format!("`{name}` ist eine Konstante (`const`) und kann nicht geändert werden.")
                } else {
                    format!("`{name}` is a constant (`const`) and can not be changed.")
                }
            }
            K::AssignToFunction(name) => {
                if de {
                    format!("`{name}` ist eine Funktion und kann nicht geändert werden.")
                } else {
                    format!("`{name}` is a function and can not be changed.")
                }
            }
            K::ReadOnly(name) => {
                if de {
                    format!("`{name}` ist eingebaut und kann nur gelesen werden.")
                } else {
                    format!("`{name}` is built in and can only be read.")
                }
            }
            K::MustCall(name) => {
                if de {
                    format!("`{name}` ist ein Befehl und muss mit Klammern aufgerufen werden.")
                } else {
                    format!("`{name}` is a command and must be called with brackets.")
                }
            }
            K::NotAValue(name) => {
                if de {
                    format!("`{name}` kann man nicht alleine benutzen.")
                } else {
                    format!("`{name}` can not be used on its own.")
                }
            }
            K::MissingNew(name) => {
                if de {
                    format!("Ein `{name}` wird mit `new` erstellt.")
                } else {
                    format!("A `{name}` is created with `new`.")
                }
            }
            K::NotAClass(name) => match name {
                Some(n) => {
                    if de {
                        format!("`{n}` kann man nicht mit `new` erstellen.")
                    } else {
                        format!("`{n}` can not be created with `new`.")
                    }
                }
                None => t(
                    "This can not be created with `new`.",
                    "Das kann man nicht mit `new` erstellen.",
                ),
            },
            K::NotCallable(name) => match name {
                Some(n) => {
                    if de {
                        format!("`{n}` ist kein Befehl und kann nicht aufgerufen werden.")
                    } else {
                        format!("`{n}` is not a command and can not be called.")
                    }
                }
                None => t(
                    "This is not a function and can not be called.",
                    "Das ist keine Funktion und kann nicht aufgerufen werden.",
                ),
            },
            K::WrongArgCount {
                name,
                min,
                max,
                found,
            } => {
                let wanted = match (max, de) {
                    (Some(max), _) if max == min => format!("{min}"),
                    (Some(max), false) => format!("{min} to {max}"),
                    (Some(max), true) => format!("{min} bis {max}"),
                    (None, false) => format!("at least {min}"),
                    (None, true) => format!("mindestens {min}"),
                };
                if de {
                    format!("`{name}` braucht {wanted} Argument(e), bekommt aber {found}.")
                } else {
                    format!("`{name}` needs {wanted} argument(s), but gets {found}.")
                }
            }
            K::WrongArgType {
                name,
                param,
                expected,
                found,
            } => {
                let e = describe_expect(*expected, lang);
                let f = found.describe(lang);
                if de {
                    format!("Bei `{name}` muss `{param}` {e} sein, ist aber {f}.")
                } else {
                    format!("`{name}` needs {e} for `{param}`, but gets {f}.")
                }
            }
            K::UnitMismatch { op, left, right } => {
                let (l, r) = (
                    capitalize(describe_dim(*left, lang)),
                    describe_dim(*right, lang),
                );
                if de {
                    format!("{l} und {r} kann man nicht mit `{op}` verbinden.")
                } else {
                    format!("{l} and {r} can not be combined with `{op}`.")
                }
            }
            K::UnitProduct { left, right } => {
                let (l, r) = (describe_dim(*left, lang), describe_dim(*right, lang));
                if de {
                    format!("Zwei Werte mit Einheit kann man nicht malnehmen ({l} mal {r}).")
                } else {
                    format!("Two values with units can not be multiplied ({l} times {r}).")
                }
            }
            K::UnitQuotient { left, right } => {
                let (l, r) = (
                    capitalize(describe_dim(*left, lang)),
                    describe_dim(*right, lang),
                );
                if de {
                    format!("{l} kann man nicht durch {r} teilen.")
                } else {
                    format!("{l} can not be divided by {r}.")
                }
            }
            K::UnitAssign { name, old, new } => {
                let (o, n) = (describe_dim(*old, lang), describe_dim(*new, lang));
                match (name, de) {
                    (Some(v), true) => format!("`{v}` ist {o}, soll aber {n} werden."),
                    (Some(v), false) => format!("`{v}` holds {o}, but gets {n}."),
                    (None, true) => format!("Das hier ist {o}, soll aber {n} werden."),
                    (None, false) => format!("This holds {o}, but gets {n}."),
                }
            }
            K::UnitIncrement(dim) => {
                let d = describe_dim(*dim, lang);
                if de {
                    format!("`++` und `--` gibt es nur für Zahlen ohne Einheit, das hier ist {d}.")
                } else {
                    format!("`++` and `--` only work for plain numbers, but this is {d}.")
                }
            }
            K::WrongArgUnit {
                name,
                param,
                expected,
                found,
                ..
            } => {
                let (e, f) = (describe_dim(*expected, lang), describe_dim(*found, lang));
                if de {
                    format!("Bei `{name}` muss `{param}` {e} sein, ist aber {f}.")
                } else {
                    format!("`{name}` needs {e} for `{param}`, but gets {f}.")
                }
            }
            K::TooBig(limit) => match (limit, de) {
                (Limit::Variables, false) => {
                    "This function has too many variables (more than 255).".into()
                }
                (Limit::Variables, true) => {
                    "Diese Funktion hat zu viele Variablen (mehr als 255).".into()
                }
                (Limit::Code, false) => "This function is too long.".into(),
                (Limit::Code, true) => "Diese Funktion ist zu lang.".into(),
                (Limit::Program, false) => "The program is too big.".into(),
                (Limit::Program, true) => "Das Programm ist zu groß.".into(),
                (Limit::Arguments, false) => {
                    "This call has too many arguments (more than 255).".into()
                }
                (Limit::Arguments, true) => {
                    "Dieser Aufruf hat zu viele Argumente (mehr als 255).".into()
                }
                (Limit::ListItems, false) => "This list has too many items.".into(),
                (Limit::ListItems, true) => "Diese Liste hat zu viele Elemente.".into(),
            },
            K::UnusedPromise { name, .. } => match name {
                Some(n) => {
                    if de {
                        format!("Das Programm wartet nicht, bis `{n}(…)` fertig ist.")
                    } else {
                        format!("The program does not wait until `{n}(…)` is finished.")
                    }
                }
                None => t(
                    "The program does not wait until this is finished.",
                    "Das Programm wartet nicht, bis das hier fertig ist.",
                ),
            },
        }
    }

    /// An optional tip on how to fix the problem.
    pub fn hint(&self, lang: Lang) -> Option<String> {
        use ErrorKind as K;
        let de = lang == Lang::De;
        let t = |en: &str, de_text: &str| Some(if de { de_text } else { en }.to_string());
        match &self.kind {
            K::UnexpectedChar(c) => match c {
                '#' => t("Comments start with `//`.", "Kommentare beginnen mit `//`."),
                '&' => t("Use `&&` for \"and\".", "Benutze `&&` für \"und\"."),
                '|' => t("Use `||` for \"or\".", "Benutze `||` für \"oder\"."),
                '`' => t(
                    "Template strings are not supported yet. Join texts with `+`.",
                    "Template-Strings gibt es noch nicht. Verbinde Texte mit `+`.",
                ),
                _ => None,
            },
            K::UnknownUnit(_) => t(
                "Known units: mm, cm, deg, rot, s, ms, %",
                "Bekannte Einheiten: mm, cm, deg, rot, s, ms, %",
            ),
            K::Expected {
                expected: Expected::Token("("),
                ..
            } => t(
                "Conditions and arguments go in round brackets, e.g. `if (x > 3) { … }`.",
                "Bedingungen und Argumente stehen in runden Klammern, z. B. `if (x > 3) { … }`.",
            ),
            K::Expected {
                expected: Expected::Token("{"),
                ..
            } => t(
                "Put the code of the block between `{` and `}`.",
                "Schreibe den Code des Blocks zwischen `{` und `}`.",
            ),
            K::MissingSemicolon(_) => t(
                "End the statement with `;` or start a new line.",
                "Beende den Befehl mit `;` oder beginne eine neue Zeile.",
            ),
            K::UnknownWord { word, suggestion } => match suggestion {
                Some(s) => {
                    if de {
                        Some(format!("Meintest du `{s}`?"))
                    } else {
                        Some(format!("Did you mean `{s}`?"))
                    }
                }
                None => {
                    if de {
                        Some(format!(
                            "Falls `{word}` eine Funktion ist, rufe sie mit Klammern auf: `{word}()`."
                        ))
                    } else {
                        Some(format!(
                            "If `{word}` is a function, call it with brackets: `{word}()`."
                        ))
                    }
                }
            },
            K::VarKeyword => t(
                "Use `let` (can change later) or `const` (never changes).",
                "Benutze `let` (kann sich ändern) oder `const` (ändert sich nie).",
            ),
            K::TypesNotNeeded => t(
                "Just leave out the `:` and the type.",
                "Lass den `:` und den Typ einfach weg.",
            ),
            K::MissingCloseBrace => t(
                "Add a `}` at the end of the block.",
                "Füge am Ende des Blocks eine `}` ein.",
            ),
            K::UnmatchedCloseBrace => t(
                "Maybe there is one `}` too many.",
                "Vielleicht ist hier eine `}` zu viel.",
            ),
            K::AwaitOutsideAsync => t(
                "Write `async` in front of `function`.",
                "Schreibe `async` vor `function`.",
            ),
            K::MissingMain | K::MainNotAsync => t(
                "Write your program into `async function main() { … }`.",
                "Schreibe dein Programm in `async function main() { … }`.",
            ),
            K::UnknownName { name, suggestion } => match suggestion {
                Some(s) => {
                    if de {
                        Some(format!("Meintest du `{s}`?"))
                    } else {
                        Some(format!("Did you mean `{s}`?"))
                    }
                }
                None => {
                    if de {
                        Some(format!(
                            "Erstelle `{name}` zuerst, z. B. mit `const {name} = …;`. Achte auf Groß- und Kleinschreibung."
                        ))
                    } else {
                        Some(format!(
                            "Create `{name}` first, e.g. with `const {name} = …;`. Check upper and lower case."
                        ))
                    }
                }
            },
            K::UnknownMember { suggestion, .. } => match suggestion {
                Some(s) => {
                    if de {
                        Some(format!("Meintest du `{s}`?"))
                    } else {
                        Some(format!("Did you mean `{s}`?"))
                    }
                }
                None => {
                    if de {
                        Some("In der Referenz steht alles, was es gibt: docs/de/stdlib.md".into())
                    } else {
                        Some("The reference lists everything that exists: docs/en/stdlib.md".into())
                    }
                }
            },
            K::DuplicateDeclaration(name) => {
                if de {
                    Some(format!(
                        "Gib einem der beiden einen anderen Namen. Um `{name}` zu ändern, schreibe `{name} = …;` ohne `let`."
                    ))
                } else {
                    Some(format!(
                        "Give one of them a different name. To change `{name}`, write `{name} = …;` without `let`."
                    ))
                }
            }
            K::ShadowsBuiltin(name) => {
                if de {
                    Some(format!(
                        "Wähle einen anderen Namen, z. B. `my{}`.",
                        capitalize(name)
                    ))
                } else {
                    Some(format!(
                        "Choose another name, e.g. `my{}`.",
                        capitalize(name)
                    ))
                }
            }
            K::UsedBeforeDeclaration(_) => t(
                "Move the line that creates it further up.",
                "Schiebe die Zeile, die es erstellt, weiter nach oben.",
            ),
            K::AssignToConst(name) => {
                if de {
                    Some(format!(
                        "Wenn sich `{name}` ändern soll, schreibe bei der Erstellung `let` statt `const`."
                    ))
                } else {
                    Some(format!(
                        "If `{name}` should change, write `let` instead of `const` where it is created."
                    ))
                }
            }
            K::AssignToFunction(_) => t(
                "Create a variable for the value: `let x = …;`.",
                "Erstelle eine Variable für den Wert: `let x = …;`.",
            ),
            K::ReadOnly(_) => t(
                "Built-in values only show what the robot measures. Store a value in your own variable: `let x = …;`. Some can be set to zero with a `reset…()` command.",
                "Eingebaute Werte zeigen nur, was der Roboter misst. Speichere einen Wert in deiner eigenen Variable: `let x = …;`. Manche kann man mit einem `reset…()`-Befehl auf null setzen.",
            ),
            K::MustCall(name) => {
                if de {
                    Some(format!("Schreibe `{name}()`."))
                } else {
                    Some(format!("Write `{name}()`."))
                }
            }
            K::NotAValue(name) => {
                if de {
                    Some(format!(
                        "Benutze ein Mitglied davon, z. B. `{name}.…`, oder erstelle ein Gerät mit `new`."
                    ))
                } else {
                    Some(format!(
                        "Use one of its members, e.g. `{name}.…`, or create a device with `new`."
                    ))
                }
            }
            K::MissingNew(name) => {
                if de {
                    Some(format!("Schreibe `new {name}(…)`."))
                } else {
                    Some(format!("Write `new {name}(…)`."))
                }
            }
            K::NotAClass(_) => t(
                "With `new` you create devices: `Motor`, `ColorSensor`, `DistanceSensor`, `ForceSensor`, `DriveBase`.",
                "Mit `new` erstellst du Geräte: `Motor`, `ColorSensor`, `DistanceSensor`, `ForceSensor`, `DriveBase`.",
            ),
            K::NotCallable(_) => t(
                "If it is a value, leave out the brackets `()`.",
                "Wenn es ein Wert ist, lass die Klammern `()` weg.",
            ),
            K::WrongArgCount { max, found, .. } => {
                if max.is_some_and(|m| *found > m) {
                    t(
                        "Remove the extra arguments.",
                        "Entferne die überzähligen Argumente.",
                    )
                } else {
                    t(
                        "Add the missing arguments between the brackets, separated by `,`.",
                        "Ergänze die fehlenden Argumente in den Klammern, getrennt durch `,`.",
                    )
                }
            }
            K::UnitMismatch { op, left, right } => {
                if *left == Dim::Number || *right == Dim::Number {
                    let other = if *left == Dim::Number { *right } else { *left };
                    unit_missing_hint(other, lang)
                } else {
                    let ex = example_dim(*left);
                    if de {
                        Some(format!(
                            "Beide Seiten brauchen die gleiche Art von Einheit, z. B. `{ex} {op} {ex}`."
                        ))
                    } else {
                        Some(format!(
                            "Both sides need the same kind of unit, e.g. `{ex} {op} {ex}`."
                        ))
                    }
                }
            }
            K::UnitProduct { .. } => t(
                "One side must be a plain number, e.g. `2 * 90deg`.",
                "Eine Seite muss eine Zahl ohne Einheit sein, z. B. `2 * 90deg`.",
            ),
            K::UnitQuotient { .. } => t(
                "You can divide by a plain number (`20cm / 2` is `10cm`) or by the same kind (`20cm / 5cm` is `4`).",
                "Teilen kann man durch eine Zahl ohne Einheit (`20cm / 2` ist `10cm`) oder durch die gleiche Art (`20cm / 5cm` ist `4`).",
            ),
            K::UnitAssign { old, new, .. } => {
                if *new == Dim::Number {
                    unit_missing_hint(*old, lang)
                } else {
                    t(
                        "Use a new variable for the other kind of value.",
                        "Benutze eine neue Variable für die andere Art von Wert.",
                    )
                }
            }
            K::UnitIncrement(dim) => {
                let one = match dim {
                    Dim::Length => "1cm",
                    Dim::Angle => "1deg",
                    Dim::Time => "1s",
                    Dim::Percent => "1%",
                    Dim::Number => "1",
                };
                if de {
                    Some(format!("Schreibe stattdessen `x += {one}`."))
                } else {
                    Some(format!("Write `x += {one}` instead."))
                }
            }
            K::WrongArgUnit {
                expected,
                found,
                alternative,
                ..
            } => match alternative {
                Some(alt) => {
                    let f = describe_dim(*found, lang);
                    if de {
                        Some(format!("Meintest du `{alt}(…)`? Dort passt {f}."))
                    } else {
                        Some(format!("Did you mean `{alt}(…)`? It takes {f}."))
                    }
                }
                None if *found == Dim::Number => unit_missing_hint(*expected, lang),
                None => {
                    let (e, ex) = (describe_dim(*expected, lang), example_dim(*expected));
                    if de {
                        Some(format!("Hier passt {e}, z. B. `{ex}`."))
                    } else {
                        Some(format!("Use {e}, e.g. `{ex}`."))
                    }
                }
            },
            K::TooBig(_) => t(
                "Split it into several smaller functions.",
                "Teile es in mehrere kleinere Funktionen auf.",
            ),
            K::UnusedPromise { in_async: true, .. } => t(
                "Write `await` in front to wait until it is finished. To do it at the same time as something else, store it (`const driving = …;`) or put it into `Promise.all([…])`.",
                "Schreibe `await` davor, um zu warten, bis es fertig ist. Soll es gleichzeitig mit etwas anderem passieren, speichere es (`const driving = …;`) oder schreibe es in `Promise.all([…])`.",
            ),
            K::UnusedPromise {
                in_async: false, ..
            } => t(
                "Make the function `async` and write `await` in front, or store the Promise in a variable.",
                "Mache die Funktion `async` und schreibe `await` davor, oder speichere das Promise in einer Variable.",
            ),
            K::WrongArgType { expected, .. } => {
                let example = match expected {
                    Expect::Is(Ty::Port) => Some("Port.A"),
                    Expect::Is(Ty::Color) => Some("Color.Red"),
                    Expect::Is(Ty::List) => Some("[a, b]"),
                    Expect::Is(Ty::Function) => Some("() => …"),
                    Expect::Is(Ty::Object(crate::types::Class::Motor)) => Some("new Motor(Port.A)"),
                    _ => None,
                };
                example.map(|ex| {
                    if de {
                        format!("Zum Beispiel: `{ex}`")
                    } else {
                        format!("For example: `{ex}`")
                    }
                })
            }
            _ => None,
        }
    }

    /// Formats the problem with the affected source line, like a compiler would.
    ///
    /// ```text
    /// error: I don't understand the word `cosnt` here.
    ///  --> mission.ekips:5:3
    ///   |
    /// 5 |   cosnt speed = 50%;
    ///   |   ^^^^^
    ///   = help: Did you mean `const`?
    /// ```
    pub fn render(&self, source: &str, path: &str, lang: Lang) -> String {
        let index = LineIndex::new(source);
        let pos = index.line_col(self.span.start);
        let line_text = index.line_text(pos.line);
        let gutter = pos.line.to_string().len();
        let pad = " ".repeat(gutter);

        // Caret length: characters of the span on this line, at least one.
        let end = index.line_col(self.span.end);
        let end_col = if end.line == pos.line {
            end.col
        } else {
            line_text.chars().count() + 1
        };
        let carets = end_col.saturating_sub(pos.col).max(1);

        let (label, help) = match (self.severity(), lang) {
            (Severity::Error, Lang::En) => ("error", "help"),
            (Severity::Error, Lang::De) => ("Fehler", "Hilfe"),
            (Severity::Warning, Lang::En) => ("warning", "help"),
            (Severity::Warning, Lang::De) => ("Warnung", "Hilfe"),
        };

        let mut out = String::new();
        let _ = writeln!(out, "{label}: {}", self.message(lang));
        let _ = writeln!(out, "{pad}--> {path}:{}:{}", pos.line, pos.col);
        let _ = writeln!(out, "{pad} |");
        let _ = writeln!(out, "{} | {line_text}", pos.line);
        let _ = writeln!(
            out,
            "{pad} | {}{}",
            " ".repeat(pos.col - 1),
            "^".repeat(carets)
        );
        if let Some(hint) = self.hint(lang) {
            let _ = writeln!(out, "{pad} = {help}: {hint}");
        }
        if let Some(related) = self.related {
            let r = index.line_col(related.start);
            let note = match lang {
                Lang::En => format!("note: see {path}:{}:{}", r.line, r.col),
                Lang::De => format!("Hinweis: siehe {path}:{}:{}", r.line, r.col),
            };
            let _ = writeln!(out, "{pad} = {note}");
        }
        out
    }
}
