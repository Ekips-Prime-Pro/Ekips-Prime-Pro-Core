//! The (very small) type model used by semantic analysis.
//!
//! The language has no type annotations. The compiler still infers a
//! rough type for every expression, mainly to check units (`20cm + 3s`),
//! arguments of built-in commands and forgotten `await`s. Anything it can
//! not know (parameters, list elements) is [`Ty::Any`] and never causes an
//! error.

use crate::diagnostic::Lang;
pub use ekips_bytecode::Dim;

/// Built-in classes that can be created with `new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// `Motor`
    Motor,
    /// `ColorSensor`
    ColorSensor,
    /// `DistanceSensor`
    DistanceSensor,
    /// `ForceSensor`
    ForceSensor,
    /// `DriveBase`
    DriveBase,
}

impl Class {
    /// All classes.
    pub const ALL: [Class; 5] = [
        Class::Motor,
        Class::ColorSensor,
        Class::DistanceSensor,
        Class::ForceSensor,
        Class::DriveBase,
    ];

    /// Name as written in programs.
    pub fn name(self) -> &'static str {
        match self {
            Class::Motor => "Motor",
            Class::ColorSensor => "ColorSensor",
            Class::DistanceSensor => "DistanceSensor",
            Class::ForceSensor => "ForceSensor",
            Class::DriveBase => "DriveBase",
        }
    }
}

/// The inferred type of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    /// Unknown – never causes an error
    Any,
    /// A number of the given kind
    Num(Dim),
    /// `true` / `false`
    Bool,
    /// A text
    Text,
    /// `null`, `undefined` or the result of a command that returns nothing
    Nothing,
    /// A list `[…]`
    List,
    /// A function or arrow function
    Function,
    /// A Promise (result of a command that takes time or of an `async` function)
    Promise,
    /// `Port.A` … `Port.F`
    Port,
    /// `Color.Red`, …
    Color,
    /// An instance of a built-in class
    Object(Class),
}

impl Ty {
    /// The kind of number, if this is a number.
    pub fn dim(self) -> Option<Dim> {
        match self {
            Ty::Num(d) => Some(d),
            _ => None,
        }
    }

    /// Describes the type for messages, e.g. "a length".
    pub fn describe(self, lang: Lang) -> String {
        let de = lang == Lang::De;
        let s = match self {
            Ty::Any => return if de { "ein Wert" } else { "a value" }.into(),
            Ty::Num(d) => return describe_dim(d, lang).into(),
            Ty::Bool => {
                if de {
                    "ein Ja/Nein-Wert (`true` oder `false`)"
                } else {
                    "a yes/no value (`true` or `false`)"
                }
            }
            Ty::Text => {
                if de {
                    "ein Text"
                } else {
                    "a text"
                }
            }
            Ty::Nothing => {
                if de {
                    "nichts (`undefined`)"
                } else {
                    "nothing (`undefined`)"
                }
            }
            Ty::List => {
                if de {
                    "eine Liste"
                } else {
                    "a list"
                }
            }
            Ty::Function => {
                if de {
                    "eine Funktion"
                } else {
                    "a function"
                }
            }
            Ty::Promise => {
                if de {
                    "ein Promise"
                } else {
                    "a Promise"
                }
            }
            Ty::Port => {
                if de {
                    "ein Anschluss wie `Port.A`"
                } else {
                    "a port like `Port.A`"
                }
            }
            Ty::Color => {
                if de {
                    "eine Farbe wie `Color.Red`"
                } else {
                    "a color like `Color.Red`"
                }
            }
            Ty::Object(Class::DriveBase) if de => "eine `DriveBase`",
            Ty::Object(c) => {
                return if de {
                    format!("ein `{}`", c.name())
                } else {
                    format!("a `{}`", c.name())
                };
            }
        };
        s.into()
    }
}

/// Describes a kind of number, e.g. "a length" / "eine Länge".
pub fn describe_dim(dim: Dim, lang: Lang) -> &'static str {
    match (dim, lang) {
        (Dim::Number, Lang::En) => "a plain number",
        (Dim::Number, Lang::De) => "eine Zahl ohne Einheit",
        (Dim::Length, Lang::En) => "a length",
        (Dim::Length, Lang::De) => "eine Länge",
        (Dim::Angle, Lang::En) => "an angle",
        (Dim::Angle, Lang::De) => "ein Winkel",
        (Dim::Time, Lang::En) => "a time",
        (Dim::Time, Lang::De) => "eine Zeit",
        (Dim::Percent, Lang::En) => "a percentage",
        (Dim::Percent, Lang::De) => "ein Prozentwert",
    }
}

/// A typical value of a kind, used in hints (`20cm`, `90deg`, …).
pub fn example_dim(dim: Dim) -> &'static str {
    match dim {
        Dim::Number => "3",
        Dim::Length => "20cm",
        Dim::Angle => "90deg",
        Dim::Time => "2s",
        Dim::Percent => "50%",
    }
}
