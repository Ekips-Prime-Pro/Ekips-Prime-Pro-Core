//! Bytecode container format for Ekips Prime Pro programs.
//!
//! Every compiled program starts with a fixed 16-byte [`Header`]. The
//! firmware checks the header before running anything, so an outdated
//! hub can tell the user to update instead of crashing on unknown code.
//!
//! **Status:** only the header exists so far. Instructions follow
//! together with the VM.
//!
//! Header layout (all integers little-endian):
//!
//! | Offset | Size | Field            |
//! |-------:|-----:|------------------|
//! | 0      | 4    | magic `b"EKPP"`  |
//! | 4      | 2    | format version   |
//! | 6      | 2    | flags (reserved) |
//! | 8      | 4    | code length      |
//! | 12     | 4    | constants length |

#![no_std]

mod native;

pub use native::Native;

/// The physical kind of a number.
///
/// Units are checked by the compiler and normalised to one base unit per
/// kind, so the VM only needs to know the kind (for printing and for the
/// result kind of arithmetic):
///
/// | Kind | Base unit | Written as |
/// |---|---|---|
/// | [`Dim::Number`] | – | `3`, `2.5` |
/// | [`Dim::Length`] | millimetre | `mm`, `cm` |
/// | [`Dim::Angle`] | degree | `deg`, `°`, `rot` |
/// | [`Dim::Time`] | millisecond | `ms`, `s` |
/// | [`Dim::Percent`] | percent | `%` |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Dim {
    /// A plain number without unit
    Number = 0,
    /// A length (base unit: mm)
    Length = 1,
    /// An angle (base unit: degree)
    Angle = 2,
    /// A duration (base unit: ms)
    Time = 3,
    /// A percentage, e.g. a speed
    Percent = 4,
}

impl Dim {
    /// All kinds, in the order of their encoding.
    pub const ALL: [Dim; 5] = [
        Dim::Number,
        Dim::Length,
        Dim::Angle,
        Dim::Time,
        Dim::Percent,
    ];

    /// Decodes a kind from its byte.
    pub fn from_u8(b: u8) -> Option<Dim> {
        Dim::ALL.get(usize::from(b)).copied()
    }
}

/// Magic bytes at the start of every program.
pub const MAGIC: [u8; 4] = *b"EKPP";

/// Current bytecode format version. Bump on every incompatible change.
pub const FORMAT_VERSION: u16 = 1;

/// Size of the encoded header in bytes.
pub const HEADER_LEN: usize = 16;

/// Fixed-size header at the start of a compiled program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Bytecode format version the program was compiled for.
    pub version: u16,
    /// Reserved for future use, must be zero for version 1.
    pub flags: u16,
    /// Length of the instruction section in bytes.
    pub code_len: u32,
    /// Length of the constant section in bytes.
    pub const_len: u32,
}

/// Reasons why a header cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    /// Fewer than [`HEADER_LEN`] bytes were given.
    TooShort,
    /// The data does not start with [`MAGIC`]; it is not a program.
    BadMagic,
    /// The program was compiled for a different format version.
    UnsupportedVersion {
        /// Version found in the program.
        found: u16,
        /// Version this build understands.
        supported: u16,
    },
}

impl Header {
    /// Creates a header for the current [`FORMAT_VERSION`].
    pub fn new(code_len: u32, const_len: u32) -> Self {
        Header {
            version: FORMAT_VERSION,
            flags: 0,
            code_len,
            const_len,
        }
    }

    /// Encodes the header into its 16-byte representation.
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..4].copy_from_slice(&MAGIC);
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..12].copy_from_slice(&self.code_len.to_le_bytes());
        out[12..16].copy_from_slice(&self.const_len.to_le_bytes());
        out
    }

    /// Decodes and validates a header from the start of `bytes`.
    pub fn decode(bytes: &[u8]) -> Result<Header, HeaderError> {
        let bytes: &[u8; HEADER_LEN] = bytes
            .get(..HEADER_LEN)
            .and_then(|b| b.try_into().ok())
            .ok_or(HeaderError::TooShort)?;
        if bytes[0..4] != MAGIC {
            return Err(HeaderError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != FORMAT_VERSION {
            return Err(HeaderError::UnsupportedVersion {
                found: version,
                supported: FORMAT_VERSION,
            });
        }
        Ok(Header {
            version,
            flags: u16::from_le_bytes([bytes[6], bytes[7]]),
            code_len: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            const_len: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_round_trip() {
        for dim in Dim::ALL {
            assert_eq!(Dim::from_u8(dim as u8), Some(dim));
        }
        assert_eq!(Dim::from_u8(5), None);
    }

    #[test]
    fn header_round_trip() {
        let header = Header::new(1234, 56);
        let bytes = header.encode();
        assert_eq!(&bytes[0..4], b"EKPP");
        assert_eq!(Header::decode(&bytes), Ok(header));
    }

    #[test]
    fn rejects_short_input() {
        assert_eq!(Header::decode(b"EKPP"), Err(HeaderError::TooShort));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = Header::new(0, 0).encode();
        bytes[0] = b'X';
        assert_eq!(Header::decode(&bytes), Err(HeaderError::BadMagic));
    }

    #[test]
    fn rejects_other_version() {
        let mut bytes = Header::new(0, 0).encode();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            Header::decode(&bytes),
            Err(HeaderError::UnsupportedVersion {
                found: 99,
                supported: FORMAT_VERSION
            })
        );
    }
}
