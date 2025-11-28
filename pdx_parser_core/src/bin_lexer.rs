//! For parsing the binary version of clausewitz files

use std::fmt::{Display, Write};

use crate::{BinDeserialize, BinDeserializer, bin_deserialize::BinError};

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
    pub fn is_base_token_id(token: u16) -> bool {
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

    pub fn base_token_repr(token: u16) -> Option<&'static str> {
        return match token {
            BinToken::ID_EQUAL => Some("="),
            BinToken::ID_OPEN_BRACKET => Some("{"),
            BinToken::ID_CLOSE_BRACKET => Some("}"),
            BinToken::ID_I32 => Some("i32"),
            BinToken::ID_F32 => Some("f32"),
            BinToken::ID_BOOL => Some("bool"),
            BinToken::ID_STRING_QUOTED => Some("string_quoted"),
            BinToken::ID_U32 => Some("u32"),
            BinToken::ID_STRING_UNQUOTED => Some("string_unquoted"),
            BinToken::ID_F64 => Some("f64"),
            BinToken::ID_U64 => Some("u64"),
            BinToken::ID_I64 => Some("i64"),
            _ => None,
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
impl<'de> BinDeserialize<'de> for BinToken<'de> {
    fn take(stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let mut as_lexer = BinLexer::new(stream.input);
        let next = as_lexer.next().ok_or(BinError::EOF)?;
        return Ok((next, BinDeserializer::from_bytes(as_lexer.0)));
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

    pub fn print_to_string_with_tokens(self, tokens: &impl BinTokenLookup) -> String {
        let mut depth = 0;
        let mut out_buf = String::new();

        for token in self {
            if let BinToken::CloseBracket = token {
                depth -= 4;
            }
            if let BinToken::Other(id) = token {
                let text = tokens.get_text(id);
                let text = match &text {
                    Some(text) => text.as_str(),
                    None => "???",
                };
                out_buf.push_str(&format!("{:depth$}{token}/{text}\n", ""));
            } else {
                out_buf.push_str(&format!("{:depth$}{token}\n", ""));
            }
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
                BinToken::F32(i32::from_le_bytes(*value) as f32 / 1000.0)
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
                BinToken::F64(i64::from_le_bytes(*value) as f64 / 100_000.0)
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

/// Get the text representation from binary token
pub trait BinTokenLookup {
    fn get_text(&self, token: u16) -> Option<String>;
}
/// Get the binary token from text representation
pub trait BinTokenReverseLookup {
    fn get_token(&self, text: impl AsRef<str>) -> Option<u16>;
}

/// Very fast for [`BinTokenLookup`] but requires a lot of memory.
/// Conversely, very slow for [`BinTokenReverseLookup`] as it has to iterate over the whole array.
pub struct TokenRegistryArray {
    lookup_table: Box<[Option<String>; u16::MAX as usize]>,
}
impl TokenRegistryArray {
    fn new_empty() -> Self {
        let lookup_table = vec![const { None }; u16::MAX as usize]
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!("We just allocated a vec to the size"));
        return TokenRegistryArray { lookup_table };
    }
    pub fn new(tokens_file: impl AsRef<str>) -> anyhow::Result<TokenRegistryArray> {
        let mut out = TokenRegistryArray::new_empty();
        for line in tokens_file.as_ref().lines() {
            let Some((token, text)) = line.split_once(';') else {
                return Err(anyhow::anyhow!("Invalid tokens file format"));
            };
            let token = u16::from_str_radix(token, 10)?;
            out.lookup_table[token as usize] = Some(text.to_string());
        }
        return Ok(out);
    }
}
impl BinTokenLookup for TokenRegistryArray {
    fn get_text(&self, token: u16) -> Option<String> {
        return self.lookup_table.get(token as usize)?.clone();
    }
}
impl BinTokenReverseLookup for TokenRegistryArray {
    fn get_token(&self, text: impl AsRef<str>) -> Option<u16> {
        return self
            .lookup_table
            .iter()
            .enumerate()
            .find_map(|(i, token)| match token {
                Some(token) if token == text.as_ref() => Some(i as u16),
                _ => None,
            });
    }
}
