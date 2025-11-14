use std::{collections::HashMap, hash::Hash};

use crate::bin_lexer::BinToken;

#[derive(Debug, thiserror::Error)]
pub enum BinError {
    #[error("Unexpected end of file/input")]
    EOF,
    #[error("Unexpected token recieved")]
    UnexpectedToken,
    #[error("Recieved binary token was unknown")]
    UnknownToken,
    #[error("Failed to decode string parse")]
    StringDecode,
    #[error(
        "The found value was not the same length as what was supposed to be strictly a fixed-size array/tuple"
    )]
    UnexpectedLength,
    #[error("A KV was found in what was supposted to be strictly a list of values")]
    UnexpectedKV,
    #[error("An expected field was missing from a struct or similar.")]
    MissingExpectedField,
}

pub struct BinDeserializer<'de> {
    pub(crate) input: &'de [u8],
}
impl<'de> BinDeserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        BinDeserializer { input }
    }

    pub fn peek_token(&mut self) -> Option<u16> {
        let (token, _) = self.input.split_first_chunk::<2>()?;
        return Some(u16::from_le_bytes(*token));
    }

    /// Does nothing if at EOF.
    /// If only one byte is left, still does nothing
    pub fn eat_token(&mut self) {
        let Some((_, rest)) = self.input.split_first_chunk::<2>() else {
            return;
        };
        self.input = rest;
    }

    pub fn next_token(&mut self) -> Option<u16> {
        let (token, rest) = self.input.split_first_chunk::<2>()?;
        self.input = rest;
        return Some(u16::from_le_bytes(*token));
    }

    /// `next_token` except it returns [`Error::EOF`] if not found
    pub fn expect_token(&mut self) -> Result<u16, BinError> {
        return self.next_token().ok_or(BinError::EOF);
    }

    /// Expect next token
    pub fn parse_token(&mut self, token: u16) -> Result<(), BinError> {
        if token == self.expect_token()? {
            return Ok(());
        } else {
            return Err(BinError::UnexpectedToken);
        }
    }

    #[inline]
    pub fn eat_bytes_const<const N: usize>(&mut self) -> Result<(), BinError> {
        let (_, rest) = self.input.split_first_chunk::<N>().ok_or(BinError::EOF)?;
        self.input = rest;
        return Ok(());
    }

    /// Returns [`Error::EOF`] on failure or value on success.
    #[inline]
    pub fn expect_bytes_const<const N: usize>(&mut self) -> Result<&'de [u8; N], BinError> {
        let (value, rest) = self.input.split_first_chunk().ok_or(BinError::EOF)?;
        self.input = rest;
        return Ok(value);
    }

    #[inline]
    pub fn eat_bytes(&mut self, len: usize) -> Result<(), BinError> {
        let (_, rest) = self.input.split_at_checked(len).ok_or(BinError::EOF)?;
        self.input = rest;
        return Ok(());
    }

    /// Returns [`Error::EOF`] on failure or value on success.
    #[inline]
    pub fn expect_bytes(&mut self, len: usize) -> Result<&'de [u8], BinError> {
        let (value, rest) = self.input.split_at_checked(len).ok_or(BinError::EOF)?;
        self.input = rest;
        return Ok(value);
    }
}

pub trait BinDeserialize<'de>: Sized {
    fn take(stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError>;
}

impl<'de> BinDeserialize<'de> for bool {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_BOOL)?;
        let &[value] = stream.expect_bytes_const()?;
        return Ok((value != 0, stream));
    }
}

impl<'de> BinDeserialize<'de> for i32 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_I32)?;
        let value = stream.expect_bytes_const::<{ size_of::<i32>() }>()?;
        let value = i32::from_le_bytes(*value);
        return Ok((value, stream));
    }
}

impl<'de> BinDeserialize<'de> for i64 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_I64)?;
        let value = stream.expect_bytes_const::<{ size_of::<i64>() }>()?;
        let value = i64::from_le_bytes(*value);
        return Ok((value, stream));
    }
}

impl<'de> BinDeserialize<'de> for u32 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_U32)?;
        let value = stream.expect_bytes_const::<{ size_of::<u32>() }>()?;
        let value = u32::from_le_bytes(*value);
        return Ok((value, stream));
    }
}

impl<'de> BinDeserialize<'de> for u64 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_U64)?;
        let value = stream.expect_bytes_const::<{ size_of::<u64>() }>()?;
        let value = u64::from_le_bytes(*value);
        return Ok((value, stream));
    }
}

impl<'de> BinDeserialize<'de> for &'de str {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let (BinToken::ID_STRING_QUOTED | BinToken::ID_STRING_UNQUOTED) = stream.expect_token()?
        else {
            return Err(BinError::UnexpectedToken);
        };
        let len = stream.expect_token()?; // technically not a token but still a u16
        let text = stream.expect_bytes(len as usize)?;
        let Ok(text) = str::from_utf8(text) else {
            return Err(BinError::StringDecode);
        };
        // TODO: non-utf decoding
        return Ok((text, stream));
    }
}

impl<'de, T: BinDeserialize<'de>> BinDeserialize<'de> for Vec<T> {
    /// Strict: will error if a KV or non-matching type is found.
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_OPEN_BRACKET)?;
        let mut out = Vec::new();

        loop {
            if let Some(BinToken::ID_CLOSE_BRACKET) = stream.peek_token() {
                stream.eat_token();
                return Ok((out, stream));
            }
            let (item, rest) = T::take(stream)?;
            out.push(item);
            stream = rest;
        }
    }
}

impl<'de, K: BinDeserialize<'de> + Eq + Hash, V: BinDeserialize<'de>> BinDeserialize<'de>
    for HashMap<K, V>
{
    /// Strict: will error if a non-KV or non-matching type is found.
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_OPEN_BRACKET)?;
        let mut out: Vec<(K, V)> = Vec::new();

        loop {
            if let Some(BinToken::ID_CLOSE_BRACKET) = stream.peek_token() {
                stream.eat_token();
                return Ok((HashMap::from_iter(out), stream));
            }
            let (key, mut rest) = K::take(stream)?;
            rest.parse_token(BinToken::ID_EQUAL)?;
            let (value, rest) = V::take(rest)?;
            out.push((key, value));
            stream = rest;
        }
    }
}

/// Extracts no data, just exists to implement [`BinDeserialize`] and skip the next value
///
/// Note that named types are just treated as a scalar string and an unassociated object,
/// since we currently have no consistent way of knowing which are named types.
/// However, this should't matter when we're skipping it all anyway.
pub struct SkipValue;
impl SkipValue {
    /// Starting after the opening `{`, skips the rest of the current object.
    fn finish_object<'de>(
        mut stream: BinDeserializer<'de>,
    ) -> Result<BinDeserializer<'de>, BinError> {
        // TODO: I think we can make a non-recursive version of this, if it becomes an issue
        loop {
            let peek = stream.peek_token().ok_or(BinError::EOF)?;
            match peek {
                BinToken::ID_CLOSE_BRACKET => {
                    stream.eat_token();
                    return Ok(stream);
                }
                BinToken::ID_EQUAL => return Err(BinError::UnexpectedToken),
                _ => {
                    stream = SkipValue::take(stream)?.1;
                    if let Some(BinToken::ID_EQUAL) = stream.peek_token() {
                        stream.eat_token();
                        stream = SkipValue::take(stream)?.1;
                    }
                }
            }
        }
    }
}
impl<'de> BinDeserialize<'de> for SkipValue {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        match stream.expect_token()? {
            BinToken::ID_OPEN_BRACKET => {
                stream = SkipValue::finish_object(stream)?;
            }
            BinToken::ID_CLOSE_BRACKET | BinToken::ID_EQUAL => {
                return Err(BinError::UnexpectedToken);
            }
            BinToken::ID_I32 => {
                stream.eat_bytes_const::<{ size_of::<i32>() }>()?;
            }
            BinToken::ID_I64 => {
                stream.eat_bytes_const::<{ size_of::<i64>() }>()?;
            }
            BinToken::ID_U32 => {
                stream.eat_bytes_const::<{ size_of::<u32>() }>()?;
            }
            BinToken::ID_U64 => {
                stream.eat_bytes_const::<{ size_of::<u64>() }>()?;
            }
            BinToken::ID_F32 => {
                stream.eat_bytes_const::<{ size_of::<f32>() }>()?;
            }
            BinToken::ID_F64 => {
                stream.eat_bytes_const::<{ size_of::<f64>() }>()?;
            }
            BinToken::ID_BOOL => {
                stream.eat_bytes_const::<{ size_of::<bool>() }>()?;
            }
            BinToken::ID_STRING_QUOTED | BinToken::ID_STRING_UNQUOTED => {
                let len = stream.expect_token()?; // not really a token
                stream.eat_bytes(len as usize)?;
            }
            _ => {
                // is some other token.
                // this is potentially dangerous if it is supposed to be the tag on a named type
                // however that's a problem for the caller.
            }
        }
        return Ok((SkipValue, stream));
    }
}
