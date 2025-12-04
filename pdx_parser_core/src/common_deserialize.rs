use serde::{Deserialize, Serialize};

use crate::{
    BinDeserialize, BinDeserializer, TextDeserialize, TextDeserializer, bin_deserialize::BinError,
    bin_lexer::BinToken, text_deserialize::TextError, text_lexer::TextToken,
};

/// Represents a string lookup index in v2 save formats. Generally used in [`crate::StringsResolver`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LookupU16(pub u16);
impl<'de> BinDeserialize<'de> for LookupU16 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_LOOKUP_U16)?;
        let value = stream.expect_bytes_const::<{ size_of::<u16>() }>()?;
        let value = u16::from_le_bytes(*value);
        return Ok((LookupU16(value), stream));
    }
}
impl std::ops::Deref for LookupU16 {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LookupU8(pub u8);
impl<'de> BinDeserialize<'de> for LookupU8 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_LOOKUP_U8)?;
        let value = stream.expect_bytes_const::<{ size_of::<u8>() }>()?;
        let value = u8::from_le_bytes(*value);
        return Ok((LookupU8(value), stream));
    }
}
impl std::ops::Deref for LookupU8 {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extracts no data, just exists to implement [`BinDeserialize`] or [`TextDeserialize`] and skip the next value
///
/// Note that named types are just treated as a scalar string and an unassociated object,
/// since we currently have no consistent way of knowing which are named types.
/// However, this should't matter when we're skipping it all anyway.
#[derive(Debug, Serialize, Deserialize)]
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
                BinToken::ID_EQUAL => return Err(BinError::UnexpectedToken(BinToken::ID_EQUAL)),
                _ => {
                    stream = SkipValue::take(stream)
                        .map_err(|err| err.context("While skipping value or KV key"))?
                        .1;
                    if let Some(BinToken::ID_EQUAL) = stream.peek_token() {
                        stream.eat_token();
                        stream = SkipValue::take(stream)
                            .map_err(|err| err.context("While skipping value or KV key"))?
                            .1;
                    }
                }
            }
        }
    }

    /// Starting after the opening `{`, skips the rest of the current object.
    fn finish_object_text<'de>(
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
                    stream = SkipValue::take_text(stream)
                        .map_err(|err| err.context("While skipping value or KV key"))?
                        .1;
                    if let Some(TextToken::Equal) = stream.peek_token() {
                        stream.eat_token();
                        stream = SkipValue::take_text(stream)
                            .map_err(|err| err.context("While skipping KV value"))?
                            .1;
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
            token @ (BinToken::ID_CLOSE_BRACKET | BinToken::ID_EQUAL) => {
                return Err(BinError::UnexpectedToken(token));
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
            BinToken::ID_LOOKUP_U8 => {
                stream.eat_bytes_const::<{ size_of::<u8>() }>()?;
            }
            BinToken::ID_LOOKUP_U16 => {
                stream.eat_bytes_const::<{ size_of::<u16>() }>()?;
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
impl<'de> TextDeserialize<'de> for SkipValue {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        match stream.expect_token()? {
            TextToken::OpenBracket => {
                stream = SkipValue::finish_object_text(stream)?;
            }
            TextToken::CloseBracket | TextToken::Equal => {
                return Err(TextError::UnexpectedToken);
            }
            _ => {}
        }
        return Ok((SkipValue, stream));
    }
}

/// A newtype for parsing a list of key-value pairs from an input with no outer brackets `{}`.
/// Very similar to `HashMap`.
///
/// Strict: will error if a non-KV or non-matching type is found.
pub struct UnbracketedKVs<'de, K: TextDeserialize<'de>, V: TextDeserialize<'de>> {
    inner: Vec<(K, V)>,
    _phantom: std::marker::PhantomData<&'de ()>,
}
impl<'de, K: TextDeserialize<'de>, V: TextDeserialize<'de>> UnbracketedKVs<'de, K, V> {
    pub fn unwrap(self) -> Vec<(K, V)> {
        return self.inner;
    }
}
impl<'de, K: TextDeserialize<'de>, V: TextDeserialize<'de>> TextDeserialize<'de>
    for UnbracketedKVs<'de, K, V>
{
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        let mut out = Vec::new();
        while let Some(_) = stream.peek_token() {
            let (key, mut rest) = K::take_text(stream).map_err(|err| {
                TextError::Custom(format!(
                    "{err} while parsing key #{} for UnbracketedKVs",
                    out.len()
                ))
            })?;
            rest.parse_token(TextToken::Equal).map_err(|err| {
                TextError::Custom(format!(
                    "{err} while parsing eq #{} for UnbracketedKVs",
                    out.len()
                ))
            })?;
            let (value, rest) = V::take_text(rest).map_err(|err| {
                TextError::Custom(format!(
                    "{err} while parsing value #{} for UnbracketedKVs",
                    out.len()
                ))
            })?;
            out.push((key, value));
            stream = rest;
        }
        return Ok((
            UnbracketedKVs {
                inner: out,
                _phantom: std::marker::PhantomData,
            },
            stream,
        ));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub [u8; 3]);
impl Rgb {
    pub fn unwrap(self) -> [u8; 3] {
        return self.0;
    }
}
impl<'de> BinDeserialize<'de> for Rgb {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(pdx_parser_macros::eu5_token!("rgb"))?;
        stream.parse_token(BinToken::ID_OPEN_BRACKET)?;
        let r: u32 = stream.parse()?;
        let g: u32 = stream.parse()?;
        let b: u32 = stream.parse()?;
        let _ = stream.parse::<u32>(); // TODO: do we want to keep alpha on the occasion it is present?
        stream.parse_token(BinToken::ID_CLOSE_BRACKET)?;

        return Ok((Rgb([r as u8, g as u8, b as u8]), stream));
    }
}
impl<'de> TextDeserialize<'de> for Rgb {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::StringUnquoted("rgb"))?;
        stream.parse_token(TextToken::OpenBracket)?;
        let r: u8 = stream.parse()?;
        let g: u8 = stream.parse()?;
        let b: u8 = stream.parse()?;
        stream.parse_token(TextToken::CloseBracket)?;
        return Ok((Rgb([r, g, b]), stream));
    }
}
