use std::{collections::HashMap, hash::Hash, iter::Peekable};

use crate::text_lexer::{TextLexer, TextToken};

#[derive(Debug, thiserror::Error)]
pub enum TextError {
    #[error("Unexpected end of file/input")]
    EOF,
    #[error("Unexpected token recieved")]
    UnexpectedToken,
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
    #[error("An integer overflow (or underflow) has occurred.")]
    IntegerOverflow,
    #[error("{0}")]
    Custom(String),
}
impl TextError {
    pub fn context(self, context: impl AsRef<str>) -> Self {
        return TextError::Custom(format!("{}:\n{self}", context.as_ref()));
    }
}

/// Construct a [`TextError::Custom`] from a format string and arguments.
#[macro_export]
macro_rules! text_err {
    ($($arg:tt)*) => {
        $crate::text_deserialize::TextError::Custom(format!($($arg)*))
    };
}

#[derive(Clone)]
pub struct TextDeserializer<'de> {
    input: Peekable<TextLexer<'de>>,
}
impl<'de> TextDeserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        let input = TextLexer::new(input).peekable();
        TextDeserializer { input }
    }

    pub fn peek_token(&mut self) -> Option<TextToken<'de>> {
        return self.input.peek().copied();
    }

    /// Does nothing if at EOF.
    pub fn eat_token(&mut self) {
        let _ = self.input.next();
    }

    pub fn next_token(&mut self) -> Option<TextToken<'de>> {
        return self.input.next();
    }

    /// `peek_token` except it returns [`Error::EOF`] if not found
    pub fn expect_peek_token(&mut self) -> Result<TextToken<'de>, TextError> {
        return self.peek_token().ok_or(TextError::EOF);
    }

    /// `next_token` except it returns [`Error::EOF`] if not found
    pub fn expect_token(&mut self) -> Result<TextToken<'de>, TextError> {
        return self.next_token().ok_or(TextError::EOF);
    }

    /// Expect next token
    ///
    /// Will not consume any tokens on failure
    pub fn parse_token(&mut self, token: TextToken<'de>) -> Result<(), TextError> {
        if token == self.peek_token().ok_or(TextError::EOF)? {
            self.eat_token();
            return Ok(());
        } else {
            return Err(TextError::UnexpectedToken);
        }
    }

    pub fn parse<T: TextDeserialize<'de>>(&mut self) -> Result<T, TextError> {
        let (value, rest) = T::take_text(self.clone())?;
        self.input = rest.input;
        return Ok(value);
    }
}

pub trait TextDeserialize<'de>: Sized {
    fn take_text(stream: TextDeserializer<'de>)
    -> Result<(Self, TextDeserializer<'de>), TextError>;
}

impl<'de> TextDeserialize<'de> for bool {
    #[inline]
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        return match stream.expect_token()? {
            TextToken::Bool(out) => Ok((out, stream)),
            _ => Err(TextError::UnexpectedToken),
        };
    }
}

macro_rules! deserialize_int_type (
    ($t:ty) => {
        impl<'de> TextDeserialize<'de> for $t {
            fn take_text(
                mut stream: TextDeserializer<'de>,
            ) -> Result<(Self, TextDeserializer<'de>), TextError> {
                return match stream.expect_token()? {
                    TextToken::Int(out) => Ok((
                        out.try_into().map_err(|_| TextError::IntegerOverflow)?,
                        stream,
                    )),
                    TextToken::UInt(out) => Ok((
                        out.try_into().map_err(|_| TextError::IntegerOverflow)?,
                        stream,
                    )),
                    _ => Err(TextError::UnexpectedToken),
                };
            }
        }
    };
);
deserialize_int_type!(i8);
deserialize_int_type!(i16);
deserialize_int_type!(i32);
deserialize_int_type!(i64);
deserialize_int_type!(u8);
deserialize_int_type!(u16);
deserialize_int_type!(u32);
deserialize_int_type!(u64);

impl<'de> TextDeserialize<'de> for f32 {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        return match stream.expect_token()? {
            TextToken::Int(out) => Ok((out as f32, stream)),
            TextToken::UInt(out) => Ok((out as f32, stream)),
            TextToken::Float(out) => Ok((out as f32, stream)),
            _ => Err(TextError::UnexpectedToken),
        };
    }
}

impl<'de> TextDeserialize<'de> for f64 {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        return match stream.expect_token()? {
            TextToken::Int(out) => Ok((out as f64, stream)),
            TextToken::UInt(out) => Ok((out as f64, stream)),
            TextToken::Float(out) => Ok((out, stream)),
            _ => Err(TextError::UnexpectedToken),
        };
    }
}

impl<'de> TextDeserialize<'de> for &'de str {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        return match stream.expect_token()? {
            TextToken::StringQuoted(out) | TextToken::StringUnquoted(out) => Ok((out, stream)),
            _ => Err(TextError::UnexpectedToken),
        };
    }
}

impl<'de> TextDeserialize<'de> for String {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        let text: &'de str = stream.parse()?;
        return Ok((text.to_string(), stream));
    }
}

impl<'de, T: TextDeserialize<'de>> TextDeserialize<'de> for Vec<T> {
    /// Strict: will error if a KV or non-matching type is found.
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;
        let mut out = Vec::new();

        loop {
            if let Some(TextToken::CloseBracket) = stream.peek_token() {
                stream.eat_token();
                return Ok((out, stream));
            }
            let (item, rest) = T::take_text(stream)?;
            out.push(item);
            stream = rest;
        }
    }
}

impl<'de, K: TextDeserialize<'de> + Eq + Hash, V: TextDeserialize<'de>> TextDeserialize<'de>
    for HashMap<K, V>
{
    /// Strict: will error if a non-KV or non-matching type is found.
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;
        let mut out: Vec<(K, V)> = Vec::new();

        loop {
            if let Some(TextToken::CloseBracket) = stream.peek_token() {
                stream.eat_token();
                return Ok((HashMap::from_iter(out), stream));
            }
            let (key, mut rest) = K::take_text(stream)?;
            rest.parse_token(TextToken::Equal)?;
            let (value, rest) = V::take_text(rest)?;
            out.push((key, value));
            stream = rest;
        }
    }
}

pub trait TextDeserializeWith<'de, W>: Sized {
    fn take_with(
        stream: TextDeserializer<'de>,
        with: W,
    ) -> Result<(Self, TextDeserializer<'de>), TextError>;
}
