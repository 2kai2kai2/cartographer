use pdx_parser_core::{
    bin_deserialize::BinError,
    bin_lexer::{BinToken, BinTokenLookup},
    common_deserialize::SkipValue,
    text_deserialize::TextError,
    text_lexer::TextToken,
    BinDeserialize, BinDeserializer, TextDeserialize, TextDeserializer,
};

pub struct CountItems(pub usize);
impl<'de> BinDeserialize<'de> for CountItems {
    fn take(
        mut stream: BinDeserializer<'de>,
    ) -> std::result::Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_OPEN_BRACKET)?;

        let mut out = 0;
        loop {
            match stream.peek_token().ok_or(BinError::EOF)? {
                BinToken::ID_EQUAL => return Err(BinError::UnexpectedToken(BinToken::ID_EQUAL)),
                BinToken::ID_CLOSE_BRACKET => {
                    stream.eat_token();
                    break;
                }
                _ => {
                    let SkipValue = stream.parse()?;
                    out += 1;
                    if let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) {
                        let SkipValue = stream.parse()?;
                    }
                }
            }
        }
        return Ok((CountItems(out), stream));
    }
}
impl<'de> TextDeserialize<'de> for CountItems {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> std::result::Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;

        let mut out = 0;
        loop {
            match stream.expect_peek_token()? {
                TextToken::Equal => return Err(TextError::UnexpectedToken),
                TextToken::CloseBracket => {
                    stream.eat_token();
                    break;
                }
                _ => {
                    let SkipValue = stream.parse()?;
                    out += 1;
                    if let Ok(()) = stream.parse_token(TextToken::Equal) {
                        let SkipValue = stream.parse()?;
                    }
                }
            }
        }

        return Ok((CountItems(out), stream));
    }
}

pub enum ViewDisplayValueBin<'de> {
    /// The number of items in the object
    Object(usize),
    /// Is expected to never be `{`, `=`, or `}`
    Scalar(BinToken<'de>),
}
impl<'de> ViewDisplayValueBin<'de> {
    pub fn matches_query_item(
        &self,
        query_item: &str,
        tokens: Option<&impl BinTokenLookup>,
    ) -> bool {
        match self {
            ViewDisplayValueBin::Object(_) => false,
            ViewDisplayValueBin::Scalar(BinToken::Other(token_u16)) => {
                if let Some(token_text) = tokens
                    .as_ref()
                    .and_then(|tokens| tokens.get_text(*token_u16))
                {
                    token_text == query_item
                } else {
                    false
                }
            }
            ViewDisplayValueBin::Scalar(scalar) => scalar.to_string() == query_item,
        }
    }
}
impl<'de> ViewDisplayValueBin<'de> {
    pub fn display_with(&self, tokens: Option<&impl BinTokenLookup>) -> String {
        match self {
            ViewDisplayValueBin::Object(count) => format!("{{{count}}}"),
            ViewDisplayValueBin::Scalar(BinToken::Other(token_u16)) => {
                if let Some(token) = tokens
                    .as_ref()
                    .and_then(|tokens| tokens.get_text(*token_u16))
                {
                    format!("{token}/<token 0x{token_u16:04x}>")
                } else {
                    format!("<token {token_u16}>")
                }
            }
            ViewDisplayValueBin::Scalar(scalar) => format!("{scalar}"),
        }
    }
    pub fn display_type_with(&self, tokens: Option<&impl BinTokenLookup>) -> String {
        match self {
            ViewDisplayValueBin::Object(_) => format!("{{...}}"),
            ViewDisplayValueBin::Scalar(token) => match token.token_type_repr() {
                Ok(token) => format!("{token}"),
                Err(token_u16) => {
                    if let Some(token) = tokens
                        .as_ref()
                        .and_then(|tokens| tokens.get_text(token_u16))
                    {
                        format!("{token}/<token 0x{token_u16:04x}>")
                    } else {
                        format!("<token {token_u16}>")
                    }
                }
            },
        }
    }
}
impl<'de> BinDeserialize<'de> for ViewDisplayValueBin<'de> {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        match stream.peek_token().ok_or(BinError::EOF)? {
            token @ (BinToken::ID_EQUAL | BinToken::ID_CLOSE_BRACKET) => {
                return Err(BinError::UnexpectedToken(token))
            }
            BinToken::ID_OPEN_BRACKET => {
                let CountItems(count) = stream.parse()?;
                return Ok((ViewDisplayValueBin::Object(count), stream));
            }
            _ => {
                let scalar: BinToken<'de> = stream.parse()?;
                return Ok((ViewDisplayValueBin::Scalar(scalar), stream));
            }
        }
    }
}

pub enum ViewDisplayValueText<'de> {
    /// The number of items in the object
    Object(usize),
    Scalar(TextToken<'de>),
}
impl<'de> ViewDisplayValueText<'de> {
    pub fn display_type(&self) -> &'static str {
        match self {
            ViewDisplayValueText::Object(_) => "{...}",
            ViewDisplayValueText::Scalar(token) => token.token_type_repr(),
        }
    }
}
impl<'de> TextDeserialize<'de> for ViewDisplayValueText<'de> {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        match stream.peek_token().ok_or(TextError::EOF)? {
            TextToken::Equal | TextToken::CloseBracket => return Err(TextError::UnexpectedToken),
            TextToken::OpenBracket => {
                let CountItems(count) = stream.parse()?;
                return Ok((ViewDisplayValueText::Object(count), stream));
            }
            scalar => {
                stream.eat_token();
                return Ok((ViewDisplayValueText::Scalar(scalar), stream));
            }
        }
    }
}
impl<'de> std::fmt::Display for ViewDisplayValueText<'de> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ViewDisplayValueText::*;
        match self {
            Object(count) => write!(f, "{{{count}}}"),
            Scalar(scalar) => write!(f, "{scalar}"),
        }
    }
}
