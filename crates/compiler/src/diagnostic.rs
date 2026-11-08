//! Error messages, written for students.
//!
//! Every message exists in English and German. Messages say what is
//! wrong in plain words and, where possible, how to fix it.

use std::fmt::Write as _;

use crate::span::{LineIndex, Span};

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
}

/// One problem with its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// What went wrong
    pub kind: ErrorKind,
    /// Where it went wrong
    pub span: Span,
}

fn code(s: &str) -> String {
    format!("`{s}`")
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
        Diagnostic { kind, span }
    }

    /// Whether this is an error or a warning.
    pub fn severity(&self) -> Severity {
        Severity::Error
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
        out
    }
}
