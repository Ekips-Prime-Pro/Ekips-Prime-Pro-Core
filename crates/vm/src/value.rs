//! Runtime values.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt;

use ekips_bytecode::Dim;
use ekips_hub_api::{Color, Port};

use crate::promise::Promise;

/// A heap cell holding a captured variable.
pub type Cell = Rc<RefCell<Value>>;

/// A device created with `new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    /// `new Motor(port)`
    Motor(Port),
    /// `new ColorSensor(port)`
    ColorSensor(Port),
    /// `new DistanceSensor(port)`
    DistanceSensor(Port),
    /// `new ForceSensor(port)`
    ForceSensor(Port),
    /// `new DriveBase(…)` (there is one drive base per hub)
    DriveBase,
}

/// A function value: a function of the program plus its captured variables.
#[derive(Debug)]
pub struct Closure {
    /// Index into the function table
    pub function: u16,
    /// Captured variables, in the order of the function's capture list
    pub upvals: Vec<Cell>,
}

/// A value at runtime.
#[derive(Debug, Clone, Default)]
pub enum Value {
    /// `undefined`
    #[default]
    Undefined,
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// A number in the base unit of its kind
    Num(f32, Dim),
    /// A text
    Str(Rc<str>),
    /// A list (shared, changes are visible through every reference)
    List(Rc<RefCell<Vec<Value>>>),
    /// A function
    Closure(Rc<Closure>),
    /// A Promise
    Promise(Rc<Promise>),
    /// `Port.A` …
    Port(Port),
    /// `Color.Red` …
    Color(Color),
    /// A device
    Device(Device),
    /// A local slot holding a captured variable (never visible to programs)
    Cell(Cell),
}

impl Value {
    /// A plain number.
    pub fn num(v: f32) -> Value {
        Value::Num(v, Dim::Number)
    }

    /// JavaScript truthiness: `false`, `0`, `""`, `null`, `undefined` and
    /// NaN are false, everything else is true.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Bool(b) => *b,
            Value::Num(n, _) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Cell(c) => c.borrow().truthy(),
            _ => true,
        }
    }

    /// Strict equality (`==` and `===` are the same in Ekips Prime Pro).
    pub fn strict_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Num(a, _), Value::Num(b, _)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b),
            (Value::Closure(a), Value::Closure(b)) => Rc::ptr_eq(a, b),
            (Value::Promise(a), Value::Promise(b)) => Rc::ptr_eq(a, b),
            (Value::Port(a), Value::Port(b)) => a == b,
            (Value::Color(a), Value::Color(b)) => a == b,
            (Value::Device(a), Value::Device(b)) => a == b,
            _ => false,
        }
    }

    /// The value as text, as `console.log` prints it.
    pub fn to_text(&self) -> String {
        use core::fmt::Write as _;
        let mut s = String::new();
        let _ = write!(s, "{self}");
        s
    }
}

/// Formats a number without needless digits (`20`, `0.5`).
fn write_number(f: &mut fmt::Formatter<'_>, v: f32) -> fmt::Result {
    if v.is_nan() {
        return f.write_str("NaN");
    }
    if v.is_infinite() {
        return f.write_str(if v > 0.0 { "Infinity" } else { "-Infinity" });
    }
    // Round to 4 decimals so `0.1 + 0.2` prints as `0.3`.
    let rounded = libm::roundf(v * 10_000.0) / 10_000.0;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    write!(f, "{rounded}")
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Undefined => f.write_str("undefined"),
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            // Printed in the units students usually write.
            Value::Num(v, Dim::Number) => write_number(f, *v),
            Value::Num(v, Dim::Length) => {
                write_number(f, v / 10.0)?;
                f.write_str("cm")
            }
            Value::Num(v, Dim::Angle) => {
                write_number(f, *v)?;
                f.write_str("deg")
            }
            Value::Num(v, Dim::Time) => {
                write_number(f, v / 1000.0)?;
                f.write_str("s")
            }
            Value::Num(v, Dim::Percent) => {
                write_number(f, *v)?;
                f.write_str("%")
            }
            Value::Str(s) => f.write_str(s),
            Value::List(items) => {
                f.write_str("[")?;
                for (i, item) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    match item {
                        Value::Str(s) => write!(f, "\"{s}\"")?,
                        other => write!(f, "{other}")?,
                    }
                }
                f.write_str("]")
            }
            Value::Closure(_) => f.write_str("function"),
            Value::Promise(_) => f.write_str("Promise"),
            Value::Port(p) => write!(f, "Port.{}", p.as_char()),
            Value::Color(c) => write!(f, "Color.{}", color_name(*c)),
            Value::Device(d) => match d {
                Device::Motor(p) => write!(f, "Motor(Port.{})", p.as_char()),
                Device::ColorSensor(p) => write!(f, "ColorSensor(Port.{})", p.as_char()),
                Device::DistanceSensor(p) => write!(f, "DistanceSensor(Port.{})", p.as_char()),
                Device::ForceSensor(p) => write!(f, "ForceSensor(Port.{})", p.as_char()),
                Device::DriveBase => f.write_str("DriveBase"),
            },
            Value::Cell(c) => write!(f, "{}", c.borrow()),
        }
    }
}

/// The name of a color as written in programs (`Red`).
pub fn color_name(c: Color) -> &'static str {
    match c {
        Color::None => "None",
        Color::Black => "Black",
        Color::Violet => "Violet",
        Color::Blue => "Blue",
        Color::Azure => "Azure",
        Color::Green => "Green",
        Color::Yellow => "Yellow",
        Color::Red => "Red",
        Color::White => "White",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn display() {
        assert_eq!(Value::Num(200.0, Dim::Length).to_text(), "20cm");
        assert_eq!(Value::Num(2500.0, Dim::Time).to_text(), "2.5s");
        assert_eq!(Value::Num(90.0, Dim::Angle).to_text(), "90deg");
        assert_eq!(Value::Num(50.0, Dim::Percent).to_text(), "50%");
        assert_eq!(Value::num(0.1 + 0.2).to_text(), "0.3");
        assert_eq!(Value::num(-0.0).to_text(), "0");
        let list = Value::List(Rc::new(RefCell::new(vec![
            Value::num(1.0),
            Value::Str("a".into()),
        ])));
        assert_eq!(list.to_text(), "[1, \"a\"]");
        assert_eq!(Value::Color(Color::Red).to_text(), "Color.Red");
    }

    #[test]
    fn truthiness_and_equality() {
        assert!(!Value::num(0.0).truthy());
        assert!(!Value::Str("".into()).truthy());
        assert!(Value::Str("x".into()).truthy());
        assert!(Value::Null.strict_eq(&Value::Null));
        assert!(!Value::Null.strict_eq(&Value::Undefined));
        assert!(Value::num(2.0).strict_eq(&Value::Num(2.0, Dim::Length)));
    }
}
