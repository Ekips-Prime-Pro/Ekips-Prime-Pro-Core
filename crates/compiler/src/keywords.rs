//! Keywords of the language.

/// Reserved keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// `async`
    Async,
    /// `await`
    Await,
    /// `break`
    Break,
    /// `const`
    Const,
    /// `continue`
    Continue,
    /// `else`
    Else,
    /// `false`
    False,
    /// `for`
    For,
    /// `function`
    Function,
    /// `if`
    If,
    /// `let`
    Let,
    /// `new`
    New,
    /// `null`
    Null,
    /// `of`
    Of,
    /// `return`
    Return,
    /// `true`
    True,
    /// `undefined`
    Undefined,
    /// `while`
    While,
}

impl Keyword {
    /// All keywords.
    pub const ALL: [Keyword; 18] = [
        Keyword::Async,
        Keyword::Await,
        Keyword::Break,
        Keyword::Const,
        Keyword::Continue,
        Keyword::Else,
        Keyword::False,
        Keyword::For,
        Keyword::Function,
        Keyword::If,
        Keyword::Let,
        Keyword::New,
        Keyword::Null,
        Keyword::Of,
        Keyword::Return,
        Keyword::True,
        Keyword::Undefined,
        Keyword::While,
    ];

    /// Source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Break => "break",
            Keyword::Const => "const",
            Keyword::Continue => "continue",
            Keyword::Else => "else",
            Keyword::False => "false",
            Keyword::For => "for",
            Keyword::Function => "function",
            Keyword::If => "if",
            Keyword::Let => "let",
            Keyword::New => "new",
            Keyword::Null => "null",
            Keyword::Of => "of",
            Keyword::Return => "return",
            Keyword::True => "true",
            Keyword::Undefined => "undefined",
            Keyword::While => "while",
        }
    }

    /// Looks up a keyword by its spelling (case-sensitive, like TypeScript).
    pub fn lookup(word: &str) -> Option<Keyword> {
        Keyword::ALL.into_iter().find(|k| k.as_str() == word)
    }
}

/// Keywords that can start a statement. Used for "did you mean" hints.
pub const STATEMENT_KEYWORDS: &[&str] = &[
    "async", "await", "break", "const", "continue", "else", "for", "function", "if", "let",
    "return", "while",
];

/// Words students might type instead of a keyword (German words and
/// keywords from other languages), mapped to the right keyword.
pub const TRANSLATIONS: &[(&str, &str)] = &[
    ("funktion", "function"),
    ("Funktion", "function"),
    ("def", "function"),
    ("fn", "function"),
    ("func", "function"),
    ("fun", "function"),
    ("wenn", "if"),
    ("Wenn", "if"),
    ("falls", "if"),
    ("sonst", "else"),
    ("Sonst", "else"),
    ("elif", "else"),
    ("elseif", "else"),
    ("solange", "while"),
    ("Solange", "while"),
    ("für", "for"),
    ("Für", "for"),
    ("konstante", "const"),
    ("konst", "const"),
    ("val", "const"),
    ("variable", "let"),
    ("Variable", "let"),
    ("mut", "let"),
    ("zurück", "return"),
    ("warte", "await"),
    ("abbrechen", "break"),
    ("weiter", "continue"),
    ("True", "true"),
    ("False", "false"),
    ("wahr", "true"),
    ("falsch", "false"),
    ("None", "null"),
    ("nil", "null"),
];
