//! The instruction set.
//!
//! The VM is a stack machine. Every instruction is one opcode byte followed
//! by its operands (little-endian). Jump targets are byte offsets relative
//! to the start of the current function's code.

macro_rules! opcodes {
    ($($(#[$doc:meta])* $name:ident = $byte:literal, [$($operand:ident),*],)*) => {
        /// An instruction opcode.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Op {
            $($(#[$doc])* $name = $byte,)*
        }

        impl Op {
            /// All opcodes.
            pub const ALL: &'static [Op] = &[$(Op::$name,)*];

            /// Decodes an opcode byte.
            pub fn from_u8(b: u8) -> Option<Op> {
                match b {
                    $($byte => Some(Op::$name),)*
                    _ => None,
                }
            }

            /// Mnemonic for disassembly.
            pub fn name(self) -> &'static str {
                match self {
                    $(Op::$name => stringify!($name),)*
                }
            }

            /// Operand sizes in bytes, in order.
            pub fn operands(self) -> &'static [Operand] {
                match self {
                    $(Op::$name => &[$(Operand::$operand),*],)*
                }
            }
        }
    };
}

/// The size of an operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// One byte
    U8,
    /// Two bytes, little-endian
    U16,
}

impl Operand {
    /// Size in bytes.
    pub fn size(self) -> usize {
        match self {
            Operand::U8 => 1,
            Operand::U16 => 2,
        }
    }
}

opcodes! {
    /// Does nothing.
    Nop = 0x00, [],
    /// Pushes constant number `idx`.
    Const = 0x01, [U16],
    /// Pushes `undefined`.
    Undefined = 0x02, [],
    /// Pushes `null`.
    Null = 0x03, [],
    /// Pushes `true`.
    True = 0x04, [],
    /// Pushes `false`.
    False = 0x05, [],
    /// Removes the top value.
    Pop = 0x06, [],
    /// Duplicates the top value: `a` → `a a`.
    Dup = 0x07, [],
    /// Duplicates the top two values: `a b` → `a b a b`.
    Dup2 = 0x08, [],

    /// Pushes local slot `n`.
    LoadLocal = 0x10, [U8],
    /// Pops into local slot `n`.
    StoreLocal = 0x11, [U8],
    /// Pushes global `n`.
    LoadGlobal = 0x12, [U16],
    /// Pops into global `n`.
    StoreGlobal = 0x13, [U16],
    /// Pops a value and puts a new cell holding it into local slot `n`
    /// (for variables captured by inner functions).
    NewCell = 0x14, [U8],
    /// Pushes the value in the cell in local slot `n`.
    LoadCell = 0x15, [U8],
    /// Pops into the cell in local slot `n`.
    StoreCell = 0x16, [U8],
    /// Pushes the value of captured variable `n` of the running closure.
    LoadUpval = 0x17, [U8],
    /// Pops into captured variable `n` of the running closure.
    StoreUpval = 0x18, [U8],

    /// `a b` → `a + b` (numbers or texts)
    Add = 0x20, [],
    /// `a b` → `a - b`
    Sub = 0x21, [],
    /// `a b` → `a * b`
    Mul = 0x22, [],
    /// `a b` → `a / b`
    Div = 0x23, [],
    /// `a b` → `a % b`
    Rem = 0x24, [],
    /// `a` → `-a`
    Neg = 0x25, [],
    /// `a` → `!a`
    Not = 0x26, [],
    /// `a b` → `a == b` (strict)
    Eq = 0x27, [],
    /// `a b` → `a != b` (strict)
    NotEq = 0x28, [],
    /// `a b` → `a < b`
    Lt = 0x29, [],
    /// `a b` → `a <= b`
    Le = 0x2A, [],
    /// `a b` → `a > b`
    Gt = 0x2B, [],
    /// `a b` → `a >= b`
    Ge = 0x2C, [],
    /// `a` → `+a` (must be a number)
    ToNumber = 0x2D, [],

    /// Jumps to offset `target`.
    Jump = 0x30, [U16],
    /// Pops a value; jumps if it is falsy.
    JumpIfFalse = 0x31, [U16],
    /// If the top value is truthy, jumps and keeps it; otherwise pops it (for `||`).
    JumpIfTrueKeep = 0x32, [U16],
    /// If the top value is falsy, jumps and keeps it; otherwise pops it (for `&&`).
    JumpIfFalseKeep = 0x33, [U16],

    /// Pushes a closure of function `idx`, capturing variables as listed
    /// in the function table.
    Closure = 0x40, [U16],
    /// `f a1 … an` → result. Calling an `async` function starts a new task
    /// and pushes its Promise.
    Call = 0x41, [U8],
    /// `a1 … an` → result of native function `id` (see [`crate::Native`]).
    CallNative = 0x42, [U16, U8],
    /// `receiver a1 … an` → result. The receiver is a value of unknown
    /// type; constant `idx` is a method set listing the natives that could
    /// match. The VM picks the one whose receiver kind fits.
    CallMethod = 0x43, [U16, U8],
    /// Pops the return value and leaves the function.
    Return = 0x44, [],

    /// Pops `n` values and pushes a list of them.
    MakeList = 0x48, [U16],
    /// `list index` → `list[index]` (`undefined` if out of range)
    GetIndex = 0x49, [],
    /// `list index value` → `value`, setting `list[index] = value`
    SetIndex = 0x4A, [],

    /// Pops a value. If it is a pending Promise, the task waits until it is
    /// done; then (or right away) the result is pushed.
    Await = 0x50, [],
    /// Marks the start of source line `n` (for runtime error messages).
    Line = 0x51, [U16],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes_round_trip() {
        for &op in Op::ALL {
            assert_eq!(Op::from_u8(op as u8), Some(op));
        }
        assert_eq!(Op::from_u8(0xFF), None);
        assert_eq!(Op::CallNative.operands(), &[Operand::U16, Operand::U8]);
    }
}
