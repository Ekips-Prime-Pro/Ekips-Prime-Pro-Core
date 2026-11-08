//! A complete compiled program: function table, constants and code.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use ekips_hub_api::{Color, Port};

use crate::opcode::{Op, Operand};
use crate::{Dim, HEADER_LEN, Header, HeaderError, Native};

/// A constant in the constant pool.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// A number in the base unit of its kind (mm, degree, ms, percent)
    Num(f32, Dim),
    /// A text
    Str(String),
    /// A port
    Port(Port),
    /// A color
    Color(Color),
    /// Natives a dynamic member access (`x.rotate(…)` on a value of unknown
    /// type) could mean; see [`Op::CallMethod`]
    MethodSet(Vec<Native>),
}

/// Where a closure gets a captured variable from when it is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// The cell in local slot `n` of the function that creates the closure
    Local(u8),
    /// Captured variable `n` of the function that creates the closure
    Upval(u8),
}

/// An entry of the function table.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Name for error messages (`main`, `<top>`, `<arrow>`)
    pub name: String,
    /// Offset of the first instruction in the code section
    pub offset: u32,
    /// Length of the function's code in bytes
    pub len: u32,
    /// Number of parameters (the first local slots)
    pub params: u8,
    /// Number of local slots including parameters
    pub locals: u8,
    /// Declared `async`: calling it starts a new task
    pub is_async: bool,
    /// Variables captured when a closure of this function is created
    pub captures: Vec<Capture>,
}

/// A compiled program. Function 0 is the entry point: it runs the
/// top-level code and then `main`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    /// Number of global variables
    pub globals: u16,
    /// Function table
    pub functions: Vec<Function>,
    /// Constant pool
    pub constants: Vec<Constant>,
    /// Code of all functions, one after the other
    pub code: Vec<u8>,
}

/// Reasons why a program can not be loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The header is invalid (wrong magic, wrong version, too short)
    Header(HeaderError),
    /// The data ends too early
    Truncated,
    /// An unknown constant tag, kind, port, color or native id
    BadValue,
    /// A text is not valid UTF-8
    BadText,
    /// A function lies outside the code section, or there is no function
    BadFunction,
}

impl From<HeaderError> for DecodeError {
    fn from(e: HeaderError) -> Self {
        DecodeError::Header(e)
    }
}

// Constant tags
const TAG_NUM: u8 = 0;
const TAG_STR: u8 = 1;
const TAG_PORT: u8 = 2;
const TAG_COLOR: u8 = 3;
const TAG_METHOD_SET: u8 = 4;

fn color_index(color: Color) -> u8 {
    Color::ALL
        .iter()
        .position(|&c| c == color)
        .expect("every color is in Color::ALL") as u8
}

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    put_u16(out, s.len() as u16);
    out.extend_from_slice(s.as_bytes());
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::Truncated)?;
        let slice = self.data.get(self.pos..end).ok_or(DecodeError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn str(&mut self) -> Result<String, DecodeError> {
        let len = usize::from(self.u16()?);
        let bytes = self.bytes(len)?;
        core::str::from_utf8(bytes)
            .map(String::from)
            .map_err(|_| DecodeError::BadText)
    }
}

impl Program {
    /// Encodes the program: header, code section, data section.
    pub fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        put_u16(&mut data, self.globals);
        put_u16(&mut data, self.functions.len() as u16);
        for f in &self.functions {
            put_str(&mut data, &f.name);
            put_u32(&mut data, f.offset);
            put_u32(&mut data, f.len);
            data.push(f.params);
            data.push(f.locals);
            data.push(u8::from(f.is_async));
            data.push(f.captures.len() as u8);
            for c in &f.captures {
                match *c {
                    Capture::Local(n) => data.extend_from_slice(&[0, n]),
                    Capture::Upval(n) => data.extend_from_slice(&[1, n]),
                }
            }
        }
        put_u16(&mut data, self.constants.len() as u16);
        for c in &self.constants {
            match c {
                Constant::Num(v, dim) => {
                    data.push(TAG_NUM);
                    data.extend_from_slice(&v.to_le_bytes());
                    data.push(*dim as u8);
                }
                Constant::Str(s) => {
                    data.push(TAG_STR);
                    put_str(&mut data, s);
                }
                Constant::Port(p) => data.extend_from_slice(&[TAG_PORT, p.index() as u8]),
                Constant::Color(c) => data.extend_from_slice(&[TAG_COLOR, color_index(*c)]),
                Constant::MethodSet(natives) => {
                    data.push(TAG_METHOD_SET);
                    data.push(natives.len() as u8);
                    for &n in natives {
                        put_u16(&mut data, n as u16);
                    }
                }
            }
        }

        let header = Header::new(self.code.len() as u32, data.len() as u32);
        let mut out = Vec::with_capacity(HEADER_LEN + self.code.len() + data.len());
        out.extend_from_slice(&header.encode());
        out.extend_from_slice(&self.code);
        out.extend_from_slice(&data);
        out
    }

    /// Decodes and validates a program.
    pub fn decode(bytes: &[u8]) -> Result<Program, DecodeError> {
        let header = Header::decode(bytes)?;
        let mut r = Reader {
            data: bytes,
            pos: HEADER_LEN,
        };
        let code = r.bytes(header.code_len as usize)?.to_vec();
        let data_end = r.pos + header.const_len as usize;
        if bytes.len() < data_end {
            return Err(DecodeError::Truncated);
        }
        let mut r = Reader {
            data: &bytes[..data_end],
            pos: r.pos,
        };

        let globals = r.u16()?;
        let n_functions = r.u16()?;
        let mut functions = Vec::with_capacity(usize::from(n_functions));
        for _ in 0..n_functions {
            let name = r.str()?;
            let offset = r.u32()?;
            let len = r.u32()?;
            let params = r.u8()?;
            let locals = r.u8()?;
            let is_async = r.u8()? != 0;
            let n_captures = r.u8()?;
            let mut captures = Vec::with_capacity(usize::from(n_captures));
            for _ in 0..n_captures {
                let kind = r.u8()?;
                let index = r.u8()?;
                captures.push(match kind {
                    0 => Capture::Local(index),
                    1 => Capture::Upval(index),
                    _ => return Err(DecodeError::BadValue),
                });
            }
            let end = offset.checked_add(len).ok_or(DecodeError::BadFunction)?;
            if end as usize > code.len() || params > locals {
                return Err(DecodeError::BadFunction);
            }
            functions.push(Function {
                name,
                offset,
                len,
                params,
                locals,
                is_async,
                captures,
            });
        }
        if functions.is_empty() {
            return Err(DecodeError::BadFunction);
        }

        let n_constants = r.u16()?;
        let mut constants = Vec::with_capacity(usize::from(n_constants));
        for _ in 0..n_constants {
            let c = match r.u8()? {
                TAG_NUM => {
                    let b = r.bytes(4)?;
                    let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                    let dim = Dim::from_u8(r.u8()?).ok_or(DecodeError::BadValue)?;
                    Constant::Num(v, dim)
                }
                TAG_STR => Constant::Str(r.str()?),
                TAG_PORT => Constant::Port(
                    *Port::ALL
                        .get(usize::from(r.u8()?))
                        .ok_or(DecodeError::BadValue)?,
                ),
                TAG_COLOR => Constant::Color(
                    *Color::ALL
                        .get(usize::from(r.u8()?))
                        .ok_or(DecodeError::BadValue)?,
                ),
                TAG_METHOD_SET => {
                    let n = r.u8()?;
                    let mut natives = Vec::with_capacity(usize::from(n));
                    for _ in 0..n {
                        natives.push(Native::from_u16(r.u16()?).ok_or(DecodeError::BadValue)?);
                    }
                    Constant::MethodSet(natives)
                }
                _ => return Err(DecodeError::BadValue),
            };
            constants.push(c);
        }
        Ok(Program {
            globals,
            functions,
            constants,
            code,
        })
    }

    /// Human-readable listing of all functions and instructions.
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        for (i, c) in self.constants.iter().enumerate() {
            let _ = writeln!(out, "const {i}: {c:?}");
        }
        for (index, f) in self.functions.iter().enumerate() {
            let _ = writeln!(
                out,
                "fn {index} {}{}(params {}, locals {}, captures {:?}):",
                if f.is_async { "async " } else { "" },
                f.name,
                f.params,
                f.locals,
                f.captures
            );
            let start = f.offset as usize;
            let code = &self.code[start..start + f.len as usize];
            let mut pc = 0;
            while pc < code.len() {
                let Some(op) = Op::from_u8(code[pc]) else {
                    let _ = writeln!(out, "  {pc:4}  <bad opcode {}>", code[pc]);
                    break;
                };
                let _ = write!(out, "  {pc:4}  {}", op.name());
                pc += 1;
                for operand in op.operands() {
                    let value = match operand {
                        Operand::U8 => code.get(pc).map(|&b| u32::from(b)),
                        Operand::U16 => code
                            .get(pc..pc + 2)
                            .map(|b| u32::from(u16::from_le_bytes([b[0], b[1]]))),
                    };
                    match value {
                        Some(v) if op == Op::CallNative && *operand == Operand::U16 => {
                            let name = Native::from_u16(v as u16).map_or("?", Native::name);
                            let _ = write!(out, " {name}");
                        }
                        Some(v) => {
                            let _ = write!(out, " {v}");
                        }
                        None => {
                            let _ = write!(out, " <truncated>");
                        }
                    }
                    pc += operand.size();
                }
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn sample() -> Program {
        Program {
            globals: 2,
            functions: vec![Function {
                name: "<top>".into(),
                offset: 0,
                len: 4,
                params: 0,
                locals: 1,
                is_async: false,
                captures: vec![Capture::Local(0), Capture::Upval(3)],
            }],
            constants: vec![
                Constant::Num(200.0, Dim::Length),
                Constant::Str("Hallo ä".into()),
                Constant::Port(Port::C),
                Constant::Color(Color::Red),
                Constant::MethodSet(vec![Native::MotorRotate]),
            ],
            code: vec![Op::Const as u8, 0, 0, Op::Return as u8],
        }
    }

    #[test]
    fn program_round_trip() {
        let p = sample();
        let bytes = p.encode();
        assert_eq!(&bytes[0..4], b"EKPP");
        assert_eq!(Program::decode(&bytes), Ok(p));
    }

    #[test]
    fn rejects_truncated_and_bad_programs() {
        let bytes = sample().encode();
        for len in [0, 10, HEADER_LEN, bytes.len() - 1] {
            assert!(Program::decode(&bytes[..len]).is_err(), "length {len}");
        }
        let mut bad = sample();
        bad.functions[0].len = 100;
        assert_eq!(
            Program::decode(&bad.encode()),
            Err(DecodeError::BadFunction)
        );
    }

    #[test]
    fn disassembly() {
        let text = sample().disassemble();
        assert!(text.contains("fn 0 <top>"), "{text}");
        assert!(text.contains("0  Const 0"), "{text}");
        assert!(text.contains("3  Return"), "{text}");
    }
}
