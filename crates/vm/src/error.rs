//! Errors while a program runs, with messages in English and German.

use core::fmt;

use ekips_bytecode::DecodeError;
use ekips_hub_api::{DeviceKind, HardwareError};

/// Language of messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// English
    #[default]
    En,
    /// German
    De,
}

/// What went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The hardware reported a problem (missing or wrong device, …)
    Hardware(HardwareError),
    /// Something that is not a function was called
    NotCallable,
    /// A value has no command or property with this name
    NoSuchMember,
    /// An operator got values it can not work with (`[1] - 2`)
    BadOperands,
    /// A built-in command got an argument of the wrong type
    BadArgument,
    /// `[ ]` on something that is not a list or text
    NotAList,
    /// Writing to a list position far behind its end
    IndexOutOfRange,
    /// The function of `waitUntil` used `await`
    ConditionWaits,
    /// The function of `waitUntil` took too long (endless loop?)
    ConditionTooLong,
    /// Too many nested function calls (endless recursion?)
    TooDeep,
    /// The program file is broken (wrong instruction or index)
    BadProgram,
}

/// A runtime error with the source line it happened on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeError {
    /// What went wrong
    pub kind: ErrorKind,
    /// Source line (1-based), `0` if unknown
    pub line: u16,
}

impl RuntimeError {
    /// Creates an error.
    pub fn new(kind: ErrorKind, line: u16) -> Self {
        RuntimeError { kind, line }
    }

    /// The message in the given language.
    pub fn message(&self, lang: Lang) -> &'static str {
        use ErrorKind as K;
        let (en, de) = match self.kind {
            K::Hardware(HardwareError::NoDevice(_)) => (
                "Nothing is plugged into this port.",
                "An diesem Anschluss ist nichts eingesteckt.",
            ),
            K::Hardware(HardwareError::WrongDevice { expected, .. }) => match expected {
                DeviceKind::Motor => (
                    "A motor was expected at this port, but something else is plugged in.",
                    "An diesem Anschluss sollte ein Motor sein, aber es steckt etwas anderes.",
                ),
                _ => (
                    "A different sensor is plugged into this port.",
                    "An diesem Anschluss steckt ein anderer Sensor.",
                ),
            },
            K::Hardware(HardwareError::NoDriveBase) => (
                "There is no DriveBase yet. Create one with `new DriveBase(…)`.",
                "Es gibt noch keine DriveBase. Erstelle eine mit `new DriveBase(…)`.",
            ),
            K::NotCallable => (
                "This is not a function and can not be called.",
                "Das ist keine Funktion und kann nicht aufgerufen werden.",
            ),
            K::NoSuchMember => (
                "This value does not have this command or property.",
                "Dieser Wert hat diesen Befehl oder diese Eigenschaft nicht.",
            ),
            K::BadOperands => (
                "This calculation does not work with these values.",
                "Diese Rechnung funktioniert mit diesen Werten nicht.",
            ),
            K::BadArgument => (
                "A command got a value of the wrong kind.",
                "Ein Befehl hat einen Wert der falschen Art bekommen.",
            ),
            K::NotAList => (
                "`[ ]` only works for lists and texts.",
                "`[ ]` funktioniert nur bei Listen und Texten.",
            ),
            K::IndexOutOfRange => (
                "This position is outside the list.",
                "Diese Position liegt außerhalb der Liste.",
            ),
            K::ConditionWaits => (
                "The condition of `waitUntil` must not use `await`.",
                "Die Bedingung von `waitUntil` darf kein `await` benutzen.",
            ),
            K::ConditionTooLong => (
                "The condition of `waitUntil` takes too long. Is there an endless loop?",
                "Die Bedingung von `waitUntil` dauert zu lange. Gibt es eine Endlosschleife?",
            ),
            K::TooDeep => (
                "Too many functions call each other. Does a function call itself forever?",
                "Zu viele Funktionen rufen sich gegenseitig auf. Ruft sich eine Funktion endlos selbst auf?",
            ),
            K::BadProgram => (
                "The program file is broken. Compile it again.",
                "Die Programmdatei ist kaputt. Übersetze sie noch einmal.",
            ),
        };
        match lang {
            Lang::En => en,
            Lang::De => de,
        }
    }
}

impl From<DecodeError> for ErrorKind {
    fn from(_: DecodeError) -> Self {
        ErrorKind::BadProgram
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message(Lang::En))
        } else {
            f.write_str(self.message(Lang::En))
        }
    }
}
