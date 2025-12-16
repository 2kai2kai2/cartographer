use serde::{Deserialize, Serialize};

use crate::{
    BinDeserialize, BinDeserializer, Context, TextDeserialize, TextDeserializer,
    bin_deserialize::BinError, bin_lexer::BinToken, text_deserialize::TextError,
    text_lexer::TextToken,
};

/// Represents a string lookup index in v2 save formats. Generally used in [`crate::StringsResolver`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LookupU16(pub u16);
impl<'de> BinDeserialize<'de> for LookupU16 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let token = stream.expect_token()?;
        if !matches!(token, BinToken::ID_LOOKUP_U16 | BinToken::ID_LOOKUP_U16_ALT) {
            return Err(BinError::UnexpectedToken(token));
        }
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
        let token = stream.expect_token()?;
        if !matches!(token, BinToken::ID_LOOKUP_U8 | BinToken::ID_LOOKUP_U8_ALT) {
            return Err(BinError::UnexpectedToken(token));
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LookupU24(pub u32);
impl<'de> BinDeserialize<'de> for LookupU24 {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_LOOKUP_U24)?;
        let &[b0, b1, b2] = stream.expect_bytes_const::<3>()?;
        let value = u32::from_le_bytes([b0, b1, b2, 0]);
        return Ok((LookupU24(value), stream));
    }
}
impl std::ops::Deref for LookupU24 {
    type Target = u32;
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
            BinToken::ID_ZERO => {}
            BinToken::ID_FIXED2_U8 | BinToken::ID_FIXED2_U8_NEG => {
                stream.eat_bytes_const::<{ size_of::<u8>() }>()?;
            }
            BinToken::ID_FIXED2_U16 | BinToken::ID_FIXED2_U16_NEG => {
                stream.eat_bytes_const::<{ size_of::<u16>() }>()?;
            }
            BinToken::ID_FIXED5_U24 | BinToken::ID_FIXED2_U24_NEG => {
                stream.eat_bytes_const::<3>()?;
            }
            BinToken::ID_FIXED5_U32 | BinToken::ID_FIXED5_U32_NEG => {
                stream.eat_bytes_const::<{ size_of::<u32>() }>()?;
            }
            BinToken::ID_FIXED5_U40 | BinToken::ID_FIXED5_U40_NEG => {
                stream.eat_bytes_const::<5>()?;
            }
            BinToken::ID_FIXED5_U48 | BinToken::ID_FIXED5_U48_NEG => {
                stream.eat_bytes_const::<6>()?;
            }
            BinToken::ID_FIXED5_U56 | BinToken::ID_FIXED5_U56_NEG => {
                stream.eat_bytes_const::<7>()?;
            }
            BinToken::ID_BOOL => {
                stream.eat_bytes_const::<{ size_of::<bool>() }>()?;
            }
            BinToken::ID_STRING_QUOTED | BinToken::ID_STRING_UNQUOTED => {
                let len = stream.expect_token()?; // not really a token
                stream.eat_bytes(len as usize)?;
            }
            BinToken::ID_LOOKUP_U8 | BinToken::ID_LOOKUP_U8_ALT => {
                stream.eat_bytes_const::<{ size_of::<u8>() }>()?;
            }
            BinToken::ID_LOOKUP_U16 | BinToken::ID_LOOKUP_U16_ALT => {
                stream.eat_bytes_const::<{ size_of::<u16>() }>()?;
            }
            BinToken::ID_LOOKUP_U24 => {
                stream.eat_bytes_const::<3>()?;
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

/// A newtype for parsing a list of key-value pairs.
/// Very similar to `HashMap`, but just stored as a list of key-value pairs.
/// Also maintains order.
///
/// Strict: will error if a non-KV or non-matching type is found.
pub struct KVs<K, V> {
    inner: Vec<(K, V)>,
}
impl<K, V> KVs<K, V> {
    pub fn unwrap(self) -> Vec<(K, V)> {
        return self.inner;
    }
}
impl<'de, K: TextDeserialize<'de>, V: TextDeserialize<'de>> TextDeserialize<'de> for KVs<K, V> {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;
        let mut out = Vec::new();
        loop {
            if let Some(TextToken::CloseBracket) = stream.peek_token() {
                stream.eat_token();
                return Ok((KVs { inner: out }, stream));
            }
            let (key, mut rest) = K::take_text(stream)
                .with_context(|| format!("While parsing key #{} for KVs", out.len()))?;
            rest.parse_token(TextToken::Equal)
                .with_context(|| format!("While parsing eq #{} for KVs", out.len()))?;
            let (value, rest) = V::take_text(rest)
                .with_context(|| format!("While parsing value #{} for KVs", out.len()))?;
            out.push((key, value));
            stream = rest;
        }
    }
}
impl<'de, K: BinDeserialize<'de>, V: BinDeserialize<'de>> BinDeserialize<'de> for KVs<K, V> {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_OPEN_BRACKET)?;
        let mut out = Vec::new();
        loop {
            if let Some(BinToken::ID_CLOSE_BRACKET) = stream.peek_token() {
                stream.eat_token();
                return Ok((KVs { inner: out }, stream));
            }
            let (key, mut rest) = K::take(stream)
                .with_context(|| format!("While parsing key #{} for KVs", out.len()))?;
            rest.parse_token(BinToken::ID_EQUAL)
                .with_context(|| format!("While parsing eq #{} for KVs", out.len()))?;
            let (value, rest) = V::take(rest)
                .with_context(|| format!("While parsing value #{} for KVs", out.len()))?;
            out.push((key, value));
            stream = rest;
        }
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
            let (key, mut rest) = K::take_text(stream)
                .with_context(|| format!("While parsing key #{} for UnbracketedKVs", out.len()))?;
            rest.parse_token(TextToken::Equal)
                .with_context(|| format!("While parsing eq #{} for UnbracketedKVs", out.len()))?;
            let (value, rest) = V::take_text(rest).with_context(|| {
                format!("While parsing value #{} for UnbracketedKVs", out.len())
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
    pub fn to_hsv360(&self) -> Hsv360 {
        let r = self.0[0] as f64 / 255.0;
        let g = self.0[1] as f64 / 255.0;
        let b = self.0[2] as f64 / 255.0;
        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let chroma = (max - min).abs(); // abs just in case
        let hue = if chroma == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / chroma) % 360.0
        } else if max == g {
            60.0 * ((b - r) / chroma + 2.0) % 360.0
        } else {
            60.0 * ((r - g) / chroma + 4.0) % 360.0
        };
        let saturation = if max == 0.0 { 0.0 } else { chroma / max };
        return Hsv360::new(hue, saturation, max);
    }
}
impl BinDeserialize<'_> for Rgb {
    fn take(mut stream: BinDeserializer<'_>) -> Result<(Self, BinDeserializer<'_>), BinError> {
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
impl TextDeserialize<'_> for Rgb {
    fn take_text(
        mut stream: TextDeserializer<'_>,
    ) -> Result<(Self, TextDeserializer<'_>), TextError> {
        stream.parse_token(TextToken::StringUnquoted("rgb"))?;
        stream.parse_token(TextToken::OpenBracket)?;
        let r: u8 = stream.parse()?;
        let g: u8 = stream.parse()?;
        let b: u8 = stream.parse()?;
        stream.parse_token(TextToken::CloseBracket)?;
        return Ok((Rgb([r, g, b]), stream));
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Hsv360 {
    /// hue 0-360
    pub h: f64,
    /// saturation 0-1 (game files are 0-100, this is converted to 0-1)
    pub s: f64,
    /// value 0-1 (game files are 0-100, this is converted to 0-1)
    pub v: f64,
}
impl Hsv360 {
    /// Creates a new HSV360 with the given values, constrained to 0-360 and 0-1
    pub fn new(h: f64, s: f64, v: f64) -> Self {
        return Self { h, s, v }.constrained();
    }
    /// Constrains hue to 0-360 (cyclically) and clamps saturation and value to 0-1
    pub fn constrained(&self) -> Self {
        return Self {
            h: self.h.rem_euclid(360.0),
            s: self.s.clamp(0.0, 1.0),
            v: self.v.clamp(0.0, 1.0),
        };
    }
    pub fn to_rgb(&self) -> Rgb {
        let hsv = self.constrained();
        let h = hsv.h / 60.0;
        let s = hsv.s;
        let v = hsv.v;
        let chroma = s * v;
        let x = chroma * (1.0 - f64::abs(h % 2.0 - 1.0));
        let min = v - chroma;

        let (r, g, b) = match h {
            0.0..1.0 => (chroma, x, 0.0),
            1.0..2.0 => (x, chroma, 0.0),
            2.0..3.0 => (0.0, chroma, x),
            3.0..4.0 => (0.0, x, chroma),
            4.0..5.0 => (x, 0.0, chroma),
            5.0..=6.0 => (chroma, 0.0, x),
            v if v.is_nan() => (0.0, 0.0, 0.0), // fallback to black
            _ => unreachable!(),
        };
        let r = ((r + min) * 255.0) as u8;
        let g = ((g + min) * 255.0) as u8;
        let b = ((b + min) * 255.0) as u8;
        return Rgb([r, g, b]);
    }
}
impl TextDeserialize<'_> for Hsv360 {
    fn take_text(
        mut stream: TextDeserializer<'_>,
    ) -> Result<(Self, TextDeserializer<'_>), TextError> {
        stream.parse_token(TextToken::StringUnquoted("hsv360"))?;
        stream.parse_token(TextToken::OpenBracket)?;
        let h: f64 = stream.parse()?;
        let s: f64 = stream.parse()?;
        let s = s / 100.0;
        let v: f64 = stream.parse()?;
        let v = v / 100.0;
        stream.parse_token(TextToken::CloseBracket)?;
        return Ok((Hsv360 { h, s, v }, stream));
    }
}

/// A color, without needing to reference a palette
#[derive(Debug, PartialEq, Clone)]
pub enum NumericColor {
    Rgb(Rgb),
    Hsv360(Hsv360),
}
impl NumericColor {
    pub fn to_rgb(&self) -> Rgb {
        match self {
            NumericColor::Rgb(rgb) => rgb.clone(),
            NumericColor::Hsv360(hsv) => hsv.to_rgb(),
        }
    }
}
impl TextDeserialize<'_> for NumericColor {
    fn take_text(
        mut stream: TextDeserializer<'_>,
    ) -> Result<(Self, TextDeserializer<'_>), TextError> {
        let peek = stream.peek_token().ok_or(TextError::EOF)?;
        match peek {
            TextToken::StringUnquoted("rgb") => {
                let rgb: Rgb = stream.parse().context("While parsing rgb value")?;
                return Ok((NumericColor::Rgb(rgb), stream));
            }
            TextToken::StringUnquoted("hsv360") => {
                let hsv: Hsv360 = stream.parse().context("While parsing hsv value")?;
                return Ok((NumericColor::Hsv360(hsv), stream));
            }
            _ => {
                return Err(TextError::UnexpectedToken);
            }
        }
    }
}
