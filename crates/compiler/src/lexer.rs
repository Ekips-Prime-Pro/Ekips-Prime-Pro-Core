//! Turns source text into tokens.

use crate::ast::Unit;
use crate::diagnostic::{Diagnostic, ErrorKind, Found};
use crate::keywords::Keyword;
use crate::span::Span;

/// Punctuation and operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punct {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `;`
    Semi,
    /// `.`
    Dot,
    /// `:`
    Colon,
    /// `?`
    Question,
    /// `=>`
    Arrow,
    /// `=`
    Assign,
    /// `+=`
    PlusAssign,
    /// `-=`
    MinusAssign,
    /// `*=`
    StarAssign,
    /// `/=`
    SlashAssign,
    /// `%=`
    PercentAssign,
    /// `++`
    PlusPlus,
    /// `--`
    MinusMinus,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `==`
    EqEq,
    /// `===`
    EqEqEq,
    /// `!=`
    NotEq,
    /// `!==`
    NotEqEq,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,
}

/// Operator spellings, longest first so that `===` wins over `==`.
const PUNCTS: &[(&str, Punct)] = &[
    ("===", Punct::EqEqEq),
    ("!==", Punct::NotEqEq),
    ("=>", Punct::Arrow),
    ("==", Punct::EqEq),
    ("!=", Punct::NotEq),
    ("<=", Punct::Le),
    (">=", Punct::Ge),
    ("&&", Punct::AndAnd),
    ("||", Punct::OrOr),
    ("++", Punct::PlusPlus),
    ("--", Punct::MinusMinus),
    ("+=", Punct::PlusAssign),
    ("-=", Punct::MinusAssign),
    ("*=", Punct::StarAssign),
    ("/=", Punct::SlashAssign),
    ("%=", Punct::PercentAssign),
    ("(", Punct::LParen),
    (")", Punct::RParen),
    ("{", Punct::LBrace),
    ("}", Punct::RBrace),
    ("[", Punct::LBracket),
    ("]", Punct::RBracket),
    (",", Punct::Comma),
    (";", Punct::Semi),
    (".", Punct::Dot),
    (":", Punct::Colon),
    ("?", Punct::Question),
    ("=", Punct::Assign),
    ("+", Punct::Plus),
    ("-", Punct::Minus),
    ("*", Punct::Star),
    ("/", Punct::Slash),
    ("%", Punct::Percent),
    ("<", Punct::Lt),
    (">", Punct::Gt),
    ("!", Punct::Bang),
];

/// The kind of a token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A name
    Ident(String),
    /// A reserved word
    Keyword(Keyword),
    /// A number, optionally with a unit suffix (`20cm`)
    Number {
        /// Numeric value
        value: f64,
        /// Unit suffix
        unit: Option<Unit>,
    },
    /// A text in quotes, escapes already resolved
    Str(String),
    /// Punctuation or operator
    Punct(Punct),
    /// End of the file
    Eof,
}

/// A token with its position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is
    pub kind: TokenKind,
    /// Where it is in the source
    pub span: Span,
    /// `true` if a line break comes between the previous token and this one.
    /// Used to make semicolons optional.
    pub newline_before: bool,
    /// The original text of the token
    pub text: String,
}

impl Token {
    /// Describes the token for error messages.
    pub fn found(&self) -> Found {
        match &self.kind {
            TokenKind::Ident(name) => Found::Word(name.clone()),
            TokenKind::Keyword(k) => Found::Word(k.as_str().to_string()),
            TokenKind::Number { .. } => Found::Number(self.text.clone()),
            TokenKind::Str(_) => Found::Text,
            TokenKind::Punct(_) => Found::Symbol(self.text.clone()),
            TokenKind::Eof => Found::FileEnd,
        }
    }

    /// `true` if the token is the given punctuation.
    pub fn is(&self, p: Punct) -> bool {
        self.kind == TokenKind::Punct(p)
    }

    /// `true` if the token is the given keyword.
    pub fn is_kw(&self, k: Keyword) -> bool {
        self.kind == TokenKind::Keyword(k)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
    newline_before: bool,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
            newline_before: self.newline_before,
            text: self.src[start..self.pos].to_string(),
        });
        self.newline_before = false;
    }

    fn error(&mut self, kind: ErrorKind, start: usize) {
        self.diags
            .push(Diagnostic::new(kind, Span::new(start, self.pos)));
    }

    fn run(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            if c == '\n' {
                self.bump();
                self.newline_before = true;
            } else if c.is_whitespace() {
                self.bump();
            } else if c == '/' && self.peek_nth(1) == Some('/') {
                while self.peek().is_some_and(|c| c != '\n') {
                    self.bump();
                }
            } else if c == '/' && self.peek_nth(1) == Some('*') {
                self.block_comment(start);
            } else if c.is_ascii_digit() {
                self.number(start);
            } else if is_ident_start(c) {
                while self.peek().is_some_and(is_ident_char) {
                    self.bump();
                }
                let word = &self.src[start..self.pos];
                let kind = match Keyword::lookup(word) {
                    Some(k) => TokenKind::Keyword(k),
                    None => TokenKind::Ident(word.to_string()),
                };
                self.push(kind, start);
            } else if c == '"' || c == '\'' {
                self.string(start, c);
            } else if let Some(&(text, punct)) = PUNCTS
                .iter()
                .find(|(text, _)| self.src[start..].starts_with(text))
            {
                self.pos += text.len();
                self.push(TokenKind::Punct(punct), start);
            } else {
                self.bump();
                self.error(ErrorKind::UnexpectedChar(c), start);
            }
        }
        let end = self.src.len();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(end, end),
            newline_before: true,
            text: String::new(),
        });
        (self.tokens, self.diags)
    }

    fn block_comment(&mut self, start: usize) {
        self.pos += 2;
        loop {
            match self.bump() {
                Some('*') if self.peek() == Some('/') => {
                    self.bump();
                    return;
                }
                Some('\n') => self.newline_before = true,
                Some(_) => {}
                None => {
                    self.error(ErrorKind::UnterminatedComment, start);
                    return;
                }
            }
        }
    }

    fn number(&mut self, start: usize) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        // Only take the dot if a digit follows ("2.5", not "2." or "2.toString").
        if self.peek() == Some('.') && self.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
        }
        let value: f64 = self.src[start..self.pos].parse().unwrap_or(0.0);

        // Unit suffix written directly after the digits: 20cm, 90deg, 90°, 50%.
        let suffix_start = self.pos;
        let mut unit = None;
        match self.peek() {
            Some(c) if c.is_alphabetic() => {
                while self.peek().is_some_and(is_ident_char) {
                    self.bump();
                }
                let suffix = &self.src[suffix_start..self.pos];
                unit = Unit::from_suffix(suffix);
                if unit.is_none() {
                    let kind = ErrorKind::UnknownUnit(suffix.to_string());
                    self.diags
                        .push(Diagnostic::new(kind, Span::new(suffix_start, self.pos)));
                }
            }
            Some('°') => {
                self.bump();
                unit = Some(Unit::Deg);
            }
            // `50%` is a percentage, but `10%3` and `10 % 3` are a remainder.
            Some('%') => {
                let next = self.peek_nth(1);
                let is_operator = next.is_some_and(|c| is_ident_char(c) || c == '(' || c == '=');
                if !is_operator {
                    self.bump();
                    unit = Some(Unit::Percent);
                }
            }
            _ => {}
        }
        self.push(TokenKind::Number { value, unit }, start);
    }

    fn string(&mut self, start: usize, quote: char) {
        self.bump();
        let mut text = String::new();
        loop {
            match self.peek() {
                Some(c) if c == quote => {
                    self.bump();
                    self.push(TokenKind::Str(text), start);
                    return;
                }
                Some('\\') => {
                    self.bump();
                    match self.bump() {
                        Some('n') => text.push('\n'),
                        Some('t') => text.push('\t'),
                        Some(c) if c != '\n' => text.push(c),
                        _ => {}
                    }
                }
                Some('\n') | None => {
                    self.error(ErrorKind::UnterminatedString, start);
                    // Still produce a token so the parser can continue.
                    self.push(TokenKind::Str(text), start);
                    return;
                }
                Some(c) => {
                    self.bump();
                    text.push(c);
                }
            }
        }
    }
}

/// Splits `source` into tokens. Always ends with an [`TokenKind::Eof`] token.
pub fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer {
        src: source,
        pos: 0,
        tokens: Vec::new(),
        diags: Vec::new(),
        newline_before: false,
    }
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        let (tokens, diags) = lex(src);
        assert!(diags.is_empty(), "unexpected errors: {diags:?}");
        tokens.into_iter().map(|t| t.kind).collect()
    }

    fn ident(s: &str) -> TokenKind {
        TokenKind::Ident(s.into())
    }

    fn p(p: Punct) -> TokenKind {
        TokenKind::Punct(p)
    }

    fn num(value: f64, unit: Option<Unit>) -> TokenKind {
        TokenKind::Number { value, unit }
    }

    #[test]
    fn method_call_with_units() {
        assert_eq!(
            kinds("await robot.drive(20cm, 50%);"),
            vec![
                TokenKind::Keyword(Keyword::Await),
                ident("robot"),
                p(Punct::Dot),
                ident("drive"),
                p(Punct::LParen),
                num(20.0, Some(Unit::Cm)),
                p(Punct::Comma),
                num(50.0, Some(Unit::Percent)),
                p(Punct::RParen),
                p(Punct::Semi),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn all_units() {
        assert_eq!(
            kinds("1mm 2cm 3deg 4° 5rot 1.5s 300ms 7%"),
            vec![
                num(1.0, Some(Unit::Mm)),
                num(2.0, Some(Unit::Cm)),
                num(3.0, Some(Unit::Deg)),
                num(4.0, Some(Unit::Deg)),
                num(5.0, Some(Unit::Rot)),
                num(1.5, Some(Unit::S)),
                num(300.0, Some(Unit::Ms)),
                num(7.0, Some(Unit::Percent)),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn percent_versus_remainder() {
        assert_eq!(
            kinds("10%3"),
            vec![
                num(10.0, None),
                p(Punct::Percent),
                num(3.0, None),
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds("a % b"),
            vec![ident("a"), p(Punct::Percent), ident("b"), TokenKind::Eof]
        );
        assert_eq!(
            kinds("x %= 2"),
            vec![
                ident("x"),
                p(Punct::PercentAssign),
                num(2.0, None),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn unknown_unit() {
        let (tokens, diags) = lex("20km");
        assert_eq!(diags[0].kind, ErrorKind::UnknownUnit("km".into()));
        assert_eq!(tokens[0].kind, num(20.0, None));
    }

    #[test]
    fn longest_operator_wins() {
        assert_eq!(
            kinds("a === b !== c => d++"),
            vec![
                ident("a"),
                p(Punct::EqEqEq),
                ident("b"),
                p(Punct::NotEqEq),
                ident("c"),
                p(Punct::Arrow),
                ident("d"),
                p(Punct::PlusPlus),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn comments_and_newlines() {
        let (tokens, diags) = lex("a // one\n/* two\nthree */ b c");
        assert!(diags.is_empty());
        assert!(!tokens[0].newline_before);
        assert!(tokens[1].newline_before);
        assert!(!tokens[2].newline_before);
        assert_eq!(tokens[1].kind, ident("b"));
    }

    #[test]
    fn strings() {
        assert_eq!(
            kinds(r#"console.log("Hallo\n", 'it\'s')"#),
            vec![
                ident("console"),
                p(Punct::Dot),
                ident("log"),
                p(Punct::LParen),
                TokenKind::Str("Hallo\n".into()),
                p(Punct::Comma),
                TokenKind::Str("it's".into()),
                p(Punct::RParen),
                TokenKind::Eof
            ]
        );
        let (tokens, diags) = lex("\"oops\nx");
        assert_eq!(diags[0].kind, ErrorKind::UnterminatedString);
        assert_eq!(tokens[0].kind, TokenKind::Str("oops".into()));
        assert_eq!(tokens[1].kind, ident("x"));
    }

    #[test]
    fn keywords_are_case_sensitive() {
        assert_eq!(
            kinds("const Const"),
            vec![
                TokenKind::Keyword(Keyword::Const),
                ident("Const"),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn unexpected_characters() {
        let (_, diags) = lex("x # y");
        assert_eq!(diags[0].kind, ErrorKind::UnexpectedChar('#'));
        let (_, diags) = lex("unterminated /* comment");
        assert_eq!(diags[0].kind, ErrorKind::UnterminatedComment);
    }
}
