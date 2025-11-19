//! For parsing the binary version of clausewitz files

use std::fmt::{Display, Write};

/// Binary tokens used in the binary format. They are each represented by 2 bytes.
#[derive(Debug, Clone, Copy)]
pub enum BinToken<'a> {
    Equal,
    OpenBracket,
    CloseBracket,
    I32(i32),
    F32(f32),
    Bool(bool),
    StringQuoted(&'a [u8]),
    U32(u32),
    StringUnquoted(&'a [u8]),
    F64(f64),
    U64(u64),
    I64(i64),
    Other(u16),
}
impl<'a> BinToken<'a> {
    /// `=`
    pub const ID_EQUAL: u16 = 0x0001;
    /// `{`
    pub const ID_OPEN_BRACKET: u16 = 0x0003;
    /// `}`
    pub const ID_CLOSE_BRACKET: u16 = 0x0004;
    pub const ID_I32: u16 = 0x000c;
    pub const ID_F32: u16 = 0x000d;
    pub const ID_BOOL: u16 = 0x000e;
    pub const ID_STRING_QUOTED: u16 = 0x000f;
    pub const ID_U32: u16 = 0x0014;
    pub const ID_STRING_UNQUOTED: u16 = 0x0017;
    pub const ID_F64: u16 = 0x0167;
    pub const ID_U64: u16 = 0x029c;
    pub const ID_I64: u16 = 0x0317;

    /// Checks if the value matches one of the const tokens
    pub fn is_base_token(token: u16) -> bool {
        return matches! {
            token,
            Self::ID_EQUAL
            | Self::ID_OPEN_BRACKET
            | Self::ID_CLOSE_BRACKET
            | Self::ID_I32
            | Self::ID_F32
            | Self::ID_BOOL
            | Self::ID_STRING_QUOTED
            | Self::ID_U32
            | Self::ID_STRING_UNQUOTED
            | Self::ID_F64
            | Self::ID_U64
            | Self::ID_I64
        };
    }

    pub fn is_base_scalar(&self) -> bool {
        return matches!(
            self,
            BinToken::I32(_)
                | BinToken::I64(_)
                | BinToken::U32(_)
                | BinToken::U64(_)
                | BinToken::F32(_)
                | BinToken::F64(_)
                | BinToken::Bool(_)
                | BinToken::StringQuoted(_)
                | BinToken::StringUnquoted(_)
        );
    }
}
impl<'a> Display for BinToken<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            BinToken::Equal => f.write_char('='),
            BinToken::OpenBracket => f.write_char('{'),
            BinToken::CloseBracket => f.write_char('}'),
            BinToken::I32(num) => write!(f, "{num}i32"),
            BinToken::F32(num) => write!(f, "{num}f32"),
            BinToken::Bool(value) => write!(f, "{value}"),
            BinToken::StringQuoted(text) => {
                f.write_fmt(format_args!("\"{}\"", String::from_utf8_lossy(&text)))
            }
            BinToken::U32(num) => write!(f, "{num}u32"),
            BinToken::StringUnquoted(text) => f.write_str(&String::from_utf8_lossy(&text)),
            BinToken::F64(num) => write!(f, "{num}f64"),
            BinToken::U64(num) => write!(f, "{num}u64"),
            BinToken::I64(num) => write!(f, "{num}i64"),
            BinToken::Other(id) => write!(f, "<token {id}>"),
        };
    }
}

#[derive(Clone)]
pub struct BinLexer<'a>(&'a [u8]);
impl<'a> BinLexer<'a> {
    pub fn new(buffer: &'a [u8]) -> BinLexer<'a> {
        return BinLexer(buffer);
    }

    pub fn print_to_string(self) -> String {
        let mut depth = 0;
        let mut out_buf = String::new();

        for token in self {
            if let BinToken::CloseBracket = token {
                depth -= 4;
            }
            out_buf.push_str(&format!("{:depth$}{token}\n", ""));
            if let BinToken::OpenBracket = token {
                depth += 4;
            }
        }

        return out_buf;
    }
}
impl<'a> Iterator for BinLexer<'a> {
    type Item = BinToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (token_id, mut rest) = self.0.split_first_chunk()?;
        let token_id = u16::from_le_bytes(*token_id);

        let token = match token_id {
            BinToken::ID_EQUAL => BinToken::Equal,
            BinToken::ID_OPEN_BRACKET => BinToken::OpenBracket,
            BinToken::ID_CLOSE_BRACKET => BinToken::CloseBracket,
            BinToken::ID_I32 => {
                let (value, new_rest) = rest.split_first_chunk()?;
                rest = new_rest;
                BinToken::I32(i32::from_le_bytes(*value))
            }
            BinToken::ID_F32 => {
                let (value, new_rest) = rest.split_first_chunk()?;
                rest = new_rest;
                BinToken::F32(f32::from_le_bytes(*value))
            }
            BinToken::ID_BOOL => {
                let (value, new_rest) = rest.split_first()?;
                rest = new_rest;
                BinToken::Bool(*value != 0)
            }
            BinToken::ID_STRING_QUOTED => {
                let (len, new_rest) = rest.split_first_chunk::<2>()?;
                let len = u16::from_le_bytes(*len);
                let (value, new_rest) = new_rest.split_at_checked(len as usize)?;
                rest = new_rest;
                BinToken::StringQuoted(value)
            }
            BinToken::ID_U32 => {
                let (value, new_rest) = rest.split_first_chunk()?;
                rest = new_rest;
                BinToken::U32(u32::from_le_bytes(*value))
            }
            BinToken::ID_STRING_UNQUOTED => {
                let (len, new_rest) = rest.split_first_chunk()?;
                let len = u16::from_le_bytes(*len);
                let (value, new_rest) = new_rest.split_at_checked(len as usize)?;
                rest = new_rest;
                BinToken::StringUnquoted(value)
            }
            BinToken::ID_F64 => {
                let (value, new_rest) = rest.split_first_chunk()?;
                rest = new_rest;
                BinToken::F64(f64::from_be_bytes(*value))
            }
            BinToken::ID_U64 => {
                let (value, new_rest) = rest.split_first_chunk()?;
                rest = new_rest;
                BinToken::U64(u64::from_le_bytes(*value))
            }
            BinToken::ID_I64 => {
                let (value, new_rest) = rest.split_first_chunk()?;
                rest = new_rest;
                BinToken::I64(i64::from_le_bytes(*value))
            }
            other => BinToken::Other(other),
        };
        self.0 = rest;
        return Some(token);
    }
}

// pub struct TokenRegistry<'a> {
//     tokens: HashMap<u16, RawPDXValue<'a>>,
// }
// impl<'a> TokenRegistry<'a> {
//     pub fn new() -> TokenRegistry<'a> {
//         return TokenRegistry {
//             tokens: HashMap::new(),
//         };
//     }
//     pub fn check()
// }

fn read(file: &[u8]) {}
