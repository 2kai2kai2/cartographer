use std::{collections::HashMap, hash::Hash};

use crate::bin_lexer::BinToken;

#[derive(Debug, thiserror::Error)]
pub enum BinError {
    #[error("Unexpected end of file/input")]
    EOF,
    #[error("Unexpected token 0x{0:04x} ({name}) recieved", name = BinToken::base_token_repr(*.0).unwrap_or("???"))]
    UnexpectedToken(u16),
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
    #[error("An expected field '{0}' was missing from a struct or similar.")]
    MissingExpectedField(String),
    #[error("{0}")]
    Custom(String),
}
impl BinError {
    pub fn context(self, context: impl AsRef<str>) -> Self {
        return BinError::Custom(format!("{}:\n{self}", context.as_ref()));
    }
}

/// Construct a [`BinError::Custom`] from a format string and arguments.
#[macro_export]
macro_rules! bin_err {
    ($($arg:tt)*) => {
        $crate::bin_deserialize::BinError::Custom(format!($($arg)*))
    };
}

#[derive(Clone)]
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
    ///
    /// ## Returns
    /// - `Ok(())` if the token matches
    /// - `Err(BinError::UnexpectedToken(parsed_token))` if `token != parsed_token`
    /// - `Err(BinError::EOF)` if the end of the stream is reached
    ///
    /// Errors will not affect the stream's state.
    pub fn parse_token(&mut self, token: u16) -> Result<(), BinError> {
        let peek = self.peek_token().ok_or(BinError::EOF)?;
        if token == peek {
            self.eat_token();
            return Ok(());
        } else {
            return Err(BinError::UnexpectedToken(peek));
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

    pub fn parse<T: BinDeserialize<'de>>(&mut self) -> Result<T, BinError> {
        let (value, rest) = T::take(self.clone())?;
        self.input = rest.input;
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

impl<'de> BinDeserialize<'de> for f32 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_F32)?;
        let value = stream.expect_bytes_const::<{ size_of::<f32>() }>()?;
        let value = f32::from_le_bytes(*value);
        return Ok((value, stream));
    }
}

impl<'de> BinDeserialize<'de> for f64 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_F64)?;
        let value = stream.expect_bytes_const::<{ size_of::<f64>() }>()?;
        let value = f64::from_le_bytes(*value);
        return Ok((value, stream));
    }
}

impl<'de> BinDeserialize<'de> for &'de str {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        match stream.expect_token()? {
            BinToken::ID_STRING_QUOTED | BinToken::ID_STRING_UNQUOTED => {}
            token => return Err(BinError::UnexpectedToken(token)),
        }
        let len = stream.expect_token()?; // technically not a token but still a u16
        let text = stream.expect_bytes(len as usize)?;
        let Ok(text) = str::from_utf8(text) else {
            return Err(BinError::StringDecode);
        };
        // TODO: non-utf decoding
        return Ok((text, stream));
    }
}
impl<'de> BinDeserialize<'de> for String {
    #[inline]
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let text: &'de str = stream.parse()?;
        return Ok((text.to_string(), stream));
    }
}
impl<'de> BinDeserialize<'de> for Box<str> {
    #[inline]
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let text: &'de str = stream.parse()?;
        return Ok((text.into(), stream));
    }
}

impl<'de, T: BinDeserialize<'de>> BinDeserialize<'de> for Vec<T> {
    /// Strict: will error if a KV or non-matching type is found.
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream
            .parse_token(BinToken::ID_OPEN_BRACKET)
            .map_err(|err| err.context("While parsing open bracket at start of list"))?;
        let mut out = Vec::new();

        loop {
            if let Some(BinToken::ID_CLOSE_BRACKET) = stream.peek_token() {
                stream.eat_token();
                return Ok((out, stream));
            }
            let (item, rest) = T::take(stream)
                .map_err(|err| err.context(format!("While parsing item #{} in list", out.len())))?;
            out.push(item);
            stream = rest;
        }
    }
}
impl<'de, T: BinDeserialize<'de>> BinDeserialize<'de> for Box<[T]> {
    /// Strict: will error if a KV or non-matching type is found.
    #[inline]
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let out: Vec<T> = stream.parse()?;
        return Ok((out.into(), stream));
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
            let (key, mut rest) =
                K::take(stream).map_err(|err| err.context("While parsing key for HashMap"))?;
            rest.parse_token(BinToken::ID_EQUAL)
                .map_err(|err| err.context("While parsing equal sign for HashMap"))?;
            let (value, rest) =
                V::take(rest).map_err(|err| err.context("While parsing value for HashMap"))?;
            out.push((key, value));
            stream = rest;
        }
    }
}
