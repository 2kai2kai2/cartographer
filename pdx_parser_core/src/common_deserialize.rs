use crate::{
    BinDeserialize, BinDeserializer, TextDeserialize, TextDeserializer, bin_deserialize::BinError,
    bin_lexer::BinToken, text_deserialize::TextError, text_lexer::TextToken,
};

/// Extracts no data, just exists to implement [`BinDeserialize`] or [`TextDeserialize`] and skip the next value
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
                    stream = SkipValue::take_text(stream)?.1;
                    if let Some(TextToken::Equal) = stream.peek_token() {
                        stream.eat_token();
                        stream = SkipValue::take_text(stream)?.1;
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
