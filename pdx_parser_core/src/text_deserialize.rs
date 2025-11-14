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
        let (value, rest) = T::take(self.clone())?;
        self.input = rest.input;
        return Ok(value);
    }
}

pub trait TextDeserialize<'de>: Sized {
    fn take(stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError>;
}

impl<'de> TextDeserialize<'de> for bool {
    #[inline]
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
        return match stream.expect_token()? {
            TextToken::Bool(out) => Ok((out, stream)),
            _ => Err(TextError::UnexpectedToken),
        };
    }
}

impl<'de> TextDeserialize<'de> for i32 {
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
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

impl<'de> TextDeserialize<'de> for i64 {
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
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

impl<'de> TextDeserialize<'de> for u32 {
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
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

impl<'de> TextDeserialize<'de> for u64 {
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
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

impl<'de> TextDeserialize<'de> for &'de str {
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
        return match stream.expect_token()? {
            TextToken::StringQuoted(out) | TextToken::StringUnquoted(out) => Ok((out, stream)),
            _ => Err(TextError::UnexpectedToken),
        };
    }
}

impl<'de, T: TextDeserialize<'de>> TextDeserialize<'de> for Vec<T> {
    /// Strict: will error if a KV or non-matching type is found.
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;
        let mut out = Vec::new();

        loop {
            if let Some(TextToken::CloseBracket) = stream.peek_token() {
                stream.eat_token();
                return Ok((out, stream));
            }
            let (item, rest) = T::take(stream)?;
            out.push(item);
            stream = rest;
        }
    }
}

impl<'de, K: TextDeserialize<'de> + Eq + Hash, V: TextDeserialize<'de>> TextDeserialize<'de>
    for HashMap<K, V>
{
    /// Strict: will error if a non-KV or non-matching type is found.
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;
        let mut out: Vec<(K, V)> = Vec::new();

        loop {
            if let Some(TextToken::CloseBracket) = stream.peek_token() {
                stream.eat_token();
                return Ok((HashMap::from_iter(out), stream));
            }
            let (key, mut rest) = K::take(stream)?;
            rest.parse_token(TextToken::Equal)?;
            let (value, rest) = V::take(rest)?;
            out.push((key, value));
            stream = rest;
        }
    }
}

/// Extracts no data, just exists to implement [`TextDeserialize`] and skip the next value
///
/// Note that named types are just treated as a scalar string and an unassociated object,
/// since we currently have no consistent way of knowing which are named types.
/// However, this should't matter when we're skipping it all anyway.
pub struct SkipValue;
impl SkipValue {
    /// Starting after the opening `{`, skips the rest of the current object.
    fn finish_object<'de>(
        mut stream: TextDeserializer<'de>,
    ) -> Result<TextDeserializer<'de>, TextError> {
        // TODO: I think we can make a non-recursive version of this, if it becomes an issue
        loop {
            let peek = stream.peek_token().ok_or(TextError::EOF)?;
            match peek {
                TextToken::CloseBracket => {
                    stream.eat_token();
                    return Ok(stream);
                }
                TextToken::Equal => return Err(TextError::UnexpectedToken),
                _ => {
                    stream = SkipValue::take(stream)?.1;
                    if let Some(TextToken::Equal) = stream.peek_token() {
                        stream.eat_token();
                        stream = SkipValue::take(stream)?.1;
                    }
                }
            }
        }
    }
}
impl<'de> TextDeserialize<'de> for SkipValue {
    fn take(mut stream: TextDeserializer<'de>) -> Result<(Self, TextDeserializer<'de>), TextError> {
        match stream.expect_token()? {
            TextToken::OpenBracket => {
                stream = SkipValue::finish_object(stream)?;
            }
            TextToken::CloseBracket | TextToken::Equal => {
                return Err(TextError::UnexpectedToken);
            }
            _ => {}
        }
        return Ok((SkipValue, stream));
    }
}

pub trait TextDeserializeWith<'de, W>: Sized {
    fn take_with(
        stream: TextDeserializer<'de>,
        with: W,
    ) -> Result<(Self, TextDeserializer<'de>), TextError>;
}
