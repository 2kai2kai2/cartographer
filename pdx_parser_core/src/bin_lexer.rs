//! For parsing the binary version of clausewitz files

use std::fmt::{Display, Write};

use crate::{
    BinDeserialize, BinDeserializer,
    bin_deserialize::BinError,
    common_deserialize::{LookupU8, LookupU16, LookupU24},
    strings_resolver::StringsResolver,
};

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
    LookupU8(LookupU8),
    LookupU16(LookupU16),
    LookupU24(LookupU24),
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
    /// Represents a constant 0.0
    pub const ID_ZERO: u16 = 0x0d47;
    /// Fixed 2 decimal places
    pub const ID_FIXED2_U8: u16 = 0x0d48;
    /// Fixed 2 decimal places
    pub const ID_FIXED2_U16: u16 = 0x0d49;
    /// Fixed 2 decimal places
    pub const ID_FIXED5_U24: u16 = 0x0d4a;
    /// Fixed 5 decimal places
    pub const ID_FIXED5_U32: u16 = 0x0d4b;
    /// Fixed 5 decimal places
    pub const ID_FIXED5_U40: u16 = 0x0d4c;
    /// Fixed 5 decimal places
    pub const ID_FIXED5_U48: u16 = 0x0d4d;
    /// Fixed 5 decimal places
    pub const ID_FIXED5_U56: u16 = 0x0d4e;
    /// Fixed 2 decimal places, negative
    pub const ID_FIXED2_U8_NEG: u16 = 0x0d4f;
    /// Fixed 2 decimal places, negative
    pub const ID_FIXED2_U16_NEG: u16 = 0x0d50;
    /// Fixed 2 decimal places, negative
    pub const ID_FIXED2_U24_NEG: u16 = 0x0d51;
    /// Fixed 5 decimal places, negative
    pub const ID_FIXED5_U32_NEG: u16 = 0x0d52;
    /// Fixed 5 decimal places, negative
    pub const ID_FIXED5_U40_NEG: u16 = 0x0d53;
    /// Fixed 5 decimal places, negative
    pub const ID_FIXED5_U48_NEG: u16 = 0x0d54;
    /// Fixed 5 decimal places, negative
    pub const ID_FIXED5_U56_NEG: u16 = 0x0d55;
    pub const ID_LOOKUP_U8: u16 = 0x0d40;
    /// Seems to be the same use as [`ID_LOOKUP_U8`]
    pub const ID_LOOKUP_U8_ALT: u16 = 0x0d43;
    pub const ID_LOOKUP_U16: u16 = 0x0d3e;
    /// Seems to be the same use as [`ID_LOOKUP_U16`]
    pub const ID_LOOKUP_U16_ALT: u16 = 0x0d44;
    pub const ID_LOOKUP_U24: u16 = 0x0d41;

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
            | Self::ID_ZERO
            | Self::ID_FIXED2_U8
            | Self::ID_FIXED2_U16
            | Self::ID_FIXED5_U24
            | Self::ID_FIXED5_U32
            | Self::ID_FIXED5_U40
            | Self::ID_FIXED5_U48
            | Self::ID_FIXED5_U56
            | Self::ID_FIXED2_U8_NEG
            | Self::ID_FIXED2_U16_NEG
            | Self::ID_FIXED2_U24_NEG
            | Self::ID_FIXED5_U32_NEG
            | Self::ID_FIXED5_U40_NEG
            | Self::ID_FIXED5_U48_NEG
            | Self::ID_FIXED5_U56_NEG
            | Self::ID_LOOKUP_U8
            | Self::ID_LOOKUP_U8_ALT
            | Self::ID_LOOKUP_U16
            | Self::ID_LOOKUP_U16_ALT
            | Self::ID_LOOKUP_U24
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
            BinToken::ID_ZERO => Some("zero"),
            BinToken::ID_FIXED2_U8 => Some("fixed2_u8"),
            BinToken::ID_FIXED2_U16 => Some("fixed2_u16"),
            BinToken::ID_FIXED5_U24 => Some("fixed5_u24"),
            BinToken::ID_FIXED5_U32 => Some("fixed5_u32"),
            BinToken::ID_FIXED5_U40 => Some("fixed5_u40"),
            BinToken::ID_FIXED5_U48 => Some("fixed5_u48"),
            BinToken::ID_FIXED5_U56 => Some("fixed5_u56"),
            BinToken::ID_FIXED2_U8_NEG => Some("fixed2_u8_neg"),
            BinToken::ID_FIXED2_U16_NEG => Some("fixed2_u16_neg"),
            BinToken::ID_FIXED2_U24_NEG => Some("fixed5_u24_neg"),
            BinToken::ID_FIXED5_U32_NEG => Some("fixed5_u32_neg"),
            BinToken::ID_FIXED5_U40_NEG => Some("fixed5_u40_neg"),
            BinToken::ID_FIXED5_U48_NEG => Some("fixed5_u48_neg"),
            BinToken::ID_FIXED5_U56_NEG => Some("fixed5_u56_neg"),
            BinToken::ID_LOOKUP_U8 => Some("lookup_u8"),
            BinToken::ID_LOOKUP_U8_ALT => Some("lookup_u8_alt"),
            BinToken::ID_LOOKUP_U16 => Some("lookup_u16"),
            BinToken::ID_LOOKUP_U16_ALT => Some("lookup_u16_alt"),
            BinToken::ID_LOOKUP_U24 => Some("lookup_u24"),
            _ => None,
        };
    }

    /// Returns the token type as a string, or if it is not a known base type, `Err(token_u16)`
    pub fn token_type_repr(&self) -> Result<&'static str, u16> {
        return match self {
            BinToken::Equal => Ok("="),
            BinToken::OpenBracket => Ok("{"),
            BinToken::CloseBracket => Ok("}"),
            BinToken::I32(_) => Ok("i32"),
            BinToken::F32(_) => Ok("f32"),
            BinToken::Bool(_) => Ok("bool"),
            BinToken::StringQuoted(_) => Ok("string_quoted"),
            BinToken::U32(_) => Ok("u32"),
            BinToken::StringUnquoted(_) => Ok("string_unquoted"),
            BinToken::F64(_) => Ok("f64"),
            BinToken::U64(_) => Ok("u64"),
            BinToken::I64(_) => Ok("i64"),
            BinToken::LookupU8(_) => Ok("lookup_u8"),
            BinToken::LookupU16(_) => Ok("lookup_u16"),
            BinToken::LookupU24(_) => Ok("lookup_u24"),
            BinToken::Other(id) => Err(*id),
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
                | BinToken::LookupU8(_)
                | BinToken::LookupU16(_)
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
            BinToken::LookupU8(num) => write!(f, "{}lookup_u8", num.0),
            BinToken::LookupU16(num) => write!(f, "{}lookup_u16", num.0),
            BinToken::LookupU24(num) => write!(f, "{}lookup_u24", num.0),
            BinToken::Other(id) => write!(f, "<token {id}>"),
        };
    }
}
impl<'de> BinDeserialize<'de> for BinToken<'de> {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let token_id = stream.expect_token()?;
        let mut rest = stream.input;
        let token = match token_id {
            BinToken::ID_EQUAL => BinToken::Equal,
            BinToken::ID_OPEN_BRACKET => BinToken::OpenBracket,
            BinToken::ID_CLOSE_BRACKET => BinToken::CloseBracket,
            BinToken::ID_I32 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::I32(i32::from_le_bytes(*value))
            }
            BinToken::ID_F32 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F32(i32::from_le_bytes(*value) as f32 / 1000.0)
            }
            BinToken::ID_BOOL => {
                let (value, new_rest) = rest.split_first().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::Bool(*value != 0)
            }
            BinToken::ID_STRING_QUOTED => {
                let (len, new_rest) = rest.split_first_chunk::<2>().ok_or(BinError::EOF)?;
                let len = u16::from_le_bytes(*len);
                let (value, new_rest) = new_rest
                    .split_at_checked(len as usize)
                    .ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::StringQuoted(value)
            }
            BinToken::ID_U32 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::U32(u32::from_le_bytes(*value))
            }
            BinToken::ID_STRING_UNQUOTED => {
                let (len, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                let len = u16::from_le_bytes(*len);
                let (value, new_rest) = new_rest
                    .split_at_checked(len as usize)
                    .ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::StringUnquoted(value)
            }
            BinToken::ID_F64 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(i64::from_le_bytes(*value) as f64 / 100_000.0)
            }
            BinToken::ID_U64 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::U64(u64::from_le_bytes(*value))
            }
            BinToken::ID_I64 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::I64(i64::from_le_bytes(*value))
            }
            BinToken::ID_ZERO => BinToken::F64(0.0),
            BinToken::ID_FIXED2_U8 => {
                let (value, new_rest) = rest.split_first().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(*value as f64 / 100_000.0)
            }
            BinToken::ID_FIXED2_U16 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(u16::from_le_bytes(*value) as f64 / 100_000.0)
            }
            BinToken::ID_FIXED5_U24 => {
                let (&[b0, b1, b2], new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(u32::from_le_bytes([b0, b1, b2, 0]) as f64 / 100_000.0)
            }
            BinToken::ID_FIXED5_U32 => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(u32::from_le_bytes(*value) as f64 / 100_000.0)
            }
            BinToken::ID_FIXED5_U40 => {
                let (&[b0, b1, b2, b3, b4], new_rest) =
                    rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(u64::from_le_bytes([b0, b1, b2, b3, b4, 0, 0, 0]) as f64 / 100_000.0)
            }
            BinToken::ID_FIXED5_U48 => {
                let (&[b0, b1, b2, b3, b4, b5], new_rest) =
                    rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(u64::from_le_bytes([b0, b1, b2, b3, b4, b5, 0, 0]) as f64 / 100_000.0)
            }
            BinToken::ID_FIXED5_U56 => {
                let (&[b0, b1, b2, b3, b4, b5, b6], new_rest) =
                    rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(
                    u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, 0]) as f64 / 100_000.0,
                )
            }
            BinToken::ID_FIXED2_U8_NEG => {
                let (value, new_rest) = rest.split_first().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(-(*value as f64 / 100_000.0))
            }
            BinToken::ID_FIXED2_U16_NEG => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(-(u16::from_le_bytes(*value) as f64 / 100_000.0))
            }
            BinToken::ID_FIXED2_U24_NEG => {
                let (&[b0, b1, b2], new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(-(u32::from_le_bytes([b0, b1, b2, 0]) as f64 / 100_000.0))
            }
            BinToken::ID_FIXED5_U32_NEG => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(-(u32::from_le_bytes(*value) as f64 / 100_000.0))
            }
            BinToken::ID_FIXED5_U40_NEG => {
                let (&[b0, b1, b2, b3, b4], new_rest) =
                    rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(
                    -(u64::from_le_bytes([b0, b1, b2, b3, b4, 0, 0, 0]) as f64 / 100_000.0),
                )
            }
            BinToken::ID_FIXED5_U48_NEG => {
                let (&[b0, b1, b2, b3, b4, b5], new_rest) =
                    rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(
                    -(u64::from_le_bytes([b0, b1, b2, b3, b4, b5, 0, 0]) as f64 / 100_000.0),
                )
            }
            BinToken::ID_FIXED5_U56_NEG => {
                let (&[b0, b1, b2, b3, b4, b5, b6], new_rest) =
                    rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::F64(
                    -(u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, 0]) as f64 / 100_000.0),
                )
            }
            BinToken::ID_LOOKUP_U8 | BinToken::ID_LOOKUP_U8_ALT => {
                let (value, new_rest) = rest.split_first().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::LookupU8(LookupU8(*value))
            }
            BinToken::ID_LOOKUP_U16 | BinToken::ID_LOOKUP_U16_ALT => {
                let (value, new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::LookupU16(LookupU16(u16::from_le_bytes(*value)))
            }
            BinToken::ID_LOOKUP_U24 => {
                let (&[b0, b1, b2], new_rest) = rest.split_first_chunk().ok_or(BinError::EOF)?;
                rest = new_rest;
                BinToken::LookupU24(LookupU24(u32::from_le_bytes([b0, b1, b2, 0])))
            }
            other => BinToken::Other(other),
        };
        stream.input = rest;
        return Ok((token, stream));
    }
}

#[derive(Clone)]
pub struct BinLexer<'a>(BinDeserializer<'a>);
impl<'a> BinLexer<'a> {
    pub fn new_deser(stream: BinDeserializer<'a>) -> BinLexer<'a> {
        return BinLexer(stream);
    }

    pub fn new(buffer: &'a [u8], strings: &'a StringsResolver) -> BinLexer<'a> {
        return BinLexer(BinDeserializer::from_bytes(buffer, strings));
    }

    pub fn write_pretty(self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        let mut depth: usize = 0;
        let BinLexer(mut de) = self;
        loop {
            let idx = de.current_index();
            let Ok(token) = de.parse::<BinToken>() else {
                break;
            };
            if let BinToken::CloseBracket = token {
                depth = depth.saturating_sub(4);
            }
            if let BinToken::LookupU16(lookup_id) = token {
                let text = de.strings.get(*lookup_id as usize).unwrap_or("???");
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            } else if let BinToken::LookupU8(lookup_id) = token {
                let text = de.strings.get(*lookup_id as usize).unwrap_or("???");
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            } else if let BinToken::LookupU24(lookup_id) = token {
                let text = de.strings.get(*lookup_id as usize).unwrap_or("???");
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            } else {
                writeln!(writer, "{idx:08x} {:depth$}{token}", "")?;
            }
            if let BinToken::OpenBracket = token {
                depth += 4;
            }
        }
        return Ok(());
    }

    pub fn write_pretty_with_tokens(
        self,
        writer: &mut impl std::io::Write,
        tokens: &impl BinTokenLookup,
    ) -> std::io::Result<()> {
        let mut depth: usize = 0;
        let BinLexer(mut de) = self;
        loop {
            let idx = de.current_index();
            let Ok(token) = de.parse::<BinToken>() else {
                break;
            };
            if let BinToken::CloseBracket = token {
                depth = depth.saturating_sub(4);
            }
            if let BinToken::Other(id) = token {
                let text = tokens.get_text(id);
                let text = match &text {
                    Some(text) => text.as_str(),
                    None => "???",
                };
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            }
            if let BinToken::LookupU24(lookup_id) = token {
                let text = de.strings.get(*lookup_id as usize).unwrap_or("???");
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            } else if let BinToken::LookupU16(lookup_id) = token {
                let text = de.strings.get(*lookup_id as usize).unwrap_or("???");
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            } else if let BinToken::LookupU8(lookup_id) = token {
                let text = de.strings.get(*lookup_id as usize).unwrap_or("???");
                writeln!(writer, "{idx:08x} {:depth$}{token}/{text}", "")?;
            } else {
                writeln!(writer, "{idx:08x} {:depth$}{token}", "")?;
            }
            if let BinToken::OpenBracket = token {
                depth += 4;
            }
        }
        Ok(())
    }
}
impl<'a> Iterator for BinLexer<'a> {
    type Item = BinToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.parse::<BinToken>().ok()
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
    lookup_table: Box<[Option<String>; 1 << 16]>,
}
impl TokenRegistryArray {
    fn new_empty() -> Self {
        let lookup_table = vec![const { None }; 1 << 16]
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
    pub fn unwrap(self) -> Box<[Option<String>; 1 << 16]> {
        return self.lookup_table;
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
