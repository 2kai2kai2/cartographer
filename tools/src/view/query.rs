use std::io::Write;

use pdx_parser_core::{
    bin_deserialize::BinError,
    bin_lexer::{BinToken, BinTokenLookup},
    common_deserialize::SkipValue,
    text_deserialize::TextError,
    text_lexer::TextToken,
    BinDeserializer, TextDeserializer,
};

use crate::view::deser_helpers::{CountItems, ViewDisplayValueBin, ViewDisplayValueText};

#[derive(Clone, Copy)]
pub enum PathItem<'a> {
    /// `$`: matches all values in the parent object.
    Values,
    /// `*`: matches the values of all KVs in the parent object.
    AllKVs,
    /// `something_here`: matches the value of all KVs in the parent object where the key matches `something_here`
    MatchingKVs(&'a str),
}
impl<'a> PathItem<'a> {
    pub fn from_str(path_item: &'a str) -> PathItem<'a> {
        if path_item == "$" {
            return PathItem::Values;
        } else if path_item == "*" {
            return PathItem::AllKVs;
        } else {
            return PathItem::MatchingKVs(path_item);
        }
    }

    pub fn walk_bin<'de, W: Write>(
        self,
        stream: BinDeserializer<'de>,
        path_rest: &[PathItem<'a>],
        write: &mut W,
        tokens: Option<&impl BinTokenLookup>,
    ) -> Result<BinDeserializer<'de>, BinError> {
        match self {
            PathItem::Values => walk_obj_bin_values(stream, path_rest, write, tokens)
                .map_err(|err| err.context("in `$`")),
            PathItem::AllKVs => walk_obj_bin_kvs_all(stream, path_rest, write, tokens)
                .map_err(|err| err.context("in `*`")),
            PathItem::MatchingKVs(key_curr) => {
                walk_obj_bin_kvs_matching(stream, key_curr, path_rest, write, tokens)
                    .map_err(|err| err.context(format! {"in `{key_curr}`"}))
            }
        }
    }

    pub fn walk_text<'de, W: Write>(
        self,
        stream: TextDeserializer<'de>,
        path_rest: &[PathItem<'a>],
        write: &mut W,
    ) -> Result<TextDeserializer<'de>, TextError> {
        match self {
            PathItem::Values => {
                walk_obj_text_values(stream, path_rest, write).map_err(|err| err.context("in `$`"))
            }
            PathItem::AllKVs => {
                walk_obj_text_kvs_all(stream, path_rest, write).map_err(|err| err.context("in `*`"))
            }
            PathItem::MatchingKVs(key_curr) => {
                walk_obj_text_kvs_matching(stream, key_curr, path_rest, write)
                    .map_err(|err| err.context(format! {"in `{key_curr}`"}))
            }
        }
    }
}

/// Next in stream is a value (either by itself or part of a kv) which,
/// if it is an object, should be walked.
/// Cannot be the last item in the path, so requires it to be split into a `next` and a `rest`
///
/// If successful, the stream will be at the end of the value
fn try_walk_possible_next_obj_bin<'de, 'a, W: Write>(
    mut stream: BinDeserializer<'de>,
    path_next: &PathItem<'a>,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
    tokens: Option<&impl BinTokenLookup>,
) -> Result<BinDeserializer<'de>, BinError> {
    match stream.peek_token().ok_or(BinError::EOF)? {
        token @ (BinToken::ID_EQUAL | BinToken::ID_CLOSE_BRACKET) => {
            return Err(BinError::UnexpectedToken(token))
        }
        BinToken::ID_OPEN_BRACKET => {
            return path_next.walk_bin(stream, path_rest, write, tokens);
        }
        _scalar => {
            let SkipValue = stream.parse()?;
            return Ok(stream);
        }
    }
}

fn walk_obj_bin_values<'de, 'a, W: Write>(
    mut stream: BinDeserializer<'de>,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
    tokens: Option<&impl BinTokenLookup>,
) -> Result<BinDeserializer<'de>, BinError> {
    stream.parse_token(BinToken::ID_OPEN_BRACKET)?;

    loop {
        match (stream.peek_token().ok_or(BinError::EOF)?, path_rest) {
            (BinToken::ID_EQUAL, _) => return Err(BinError::UnexpectedToken(BinToken::ID_EQUAL)),
            (BinToken::ID_CLOSE_BRACKET, _) => {
                stream.eat_token();
                return Ok(stream);
            }
            (BinToken::ID_OPEN_BRACKET, [next, ref rest @ ..]) => {
                // we can't actually check if it's a KV before walking it,
                // but let's just assume
                stream = try_walk_possible_next_obj_bin(stream, next, rest, write, tokens)?;
                let Err(_) = stream.parse_token(BinToken::ID_EQUAL) else {
                    let SkipValue = stream.parse()?; // oops it was a KV
                    continue;
                };
            }
            (_, &[]) => {
                // last path item, can print
                let scalar: ViewDisplayValueBin = stream.parse()?;
                let Err(_) = stream.parse_token(BinToken::ID_EQUAL) else {
                    let SkipValue = stream.parse()?;
                    continue;
                };
                let _ = writeln!(write, "{}", scalar.display_with(tokens));
            }
            (_scalar, [_, ..]) => {
                let SkipValue = stream.parse()?;
                if let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) {
                    let SkipValue = stream.parse()?;
                };
            }
        }
    }
}
fn walk_obj_bin_kvs_all<'de, 'a, W: Write>(
    mut stream: BinDeserializer<'de>,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
    tokens: Option<&impl BinTokenLookup>,
) -> Result<BinDeserializer<'de>, BinError> {
    stream.parse_token(BinToken::ID_OPEN_BRACKET)?;

    loop {
        match (stream.peek_token().ok_or(BinError::EOF)?, path_rest) {
            (BinToken::ID_EQUAL, _) => return Err(BinError::UnexpectedToken(BinToken::ID_EQUAL)),
            (BinToken::ID_CLOSE_BRACKET, _) => {
                stream.eat_token();
                return Ok(stream);
            }
            (_, [next, ref rest @ ..]) => {
                let SkipValue = stream.parse()?;
                let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) else {
                    continue; // it's not a KV
                };
                stream = try_walk_possible_next_obj_bin(stream, next, rest, write, tokens)?
            }
            (_, &[]) => {
                // last path item, can print
                let scalar_key: ViewDisplayValueBin = stream.parse()?;
                let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) else {
                    continue; // it's not a KV
                };
                let value: ViewDisplayValueBin = stream.parse()?;
                let _ = writeln!(
                    write,
                    "{} = {}",
                    scalar_key.display_with(tokens),
                    value.display_with(tokens)
                );
            }
        }
    }
}
fn walk_obj_bin_kvs_matching<'de, 'a, W: Write>(
    mut stream: BinDeserializer<'de>,
    key_curr: &str,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
    tokens: Option<&impl BinTokenLookup>,
) -> Result<BinDeserializer<'de>, BinError> {
    stream.parse_token(BinToken::ID_OPEN_BRACKET)?;

    loop {
        match (stream.peek_token().ok_or(BinError::EOF)?, path_rest) {
            (BinToken::ID_EQUAL, _) => return Err(BinError::UnexpectedToken(BinToken::ID_EQUAL)),
            (BinToken::ID_CLOSE_BRACKET, _) => {
                stream.eat_token();
                return Ok(stream);
            }
            (BinToken::ID_OPEN_BRACKET, _) => {
                let SkipValue = stream.parse()?;
                if let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) {
                    let SkipValue = stream.parse()?;
                };
            }
            (_scalar_key, _) => {
                let key: ViewDisplayValueBin = stream.parse()?;
                let is_match = key.matches_query_item(key_curr, tokens);
                let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) else {
                    continue;
                };
                if let [next, ref rest @ ..] = path_rest {
                    if is_match {
                        stream = try_walk_possible_next_obj_bin(stream, next, rest, write, tokens)?;
                    } else {
                        let SkipValue = stream.parse()?;
                    }
                } else {
                    if is_match {
                        let value: ViewDisplayValueBin = stream.parse()?;
                        let _ = writeln!(
                            write,
                            "{} = {}",
                            key.display_with(tokens),
                            value.display_with(tokens)
                        );
                    } else {
                        let SkipValue = stream.parse()?;
                    }
                }
            }
        }
    }
}

/// Next in stream is a value (either by itself or part of a kv) which,
/// if it is an object, should be walked.
/// Cannot be the last item in the path, so requires it to be split into a `next` and a `rest`
///
/// If successful, the stream will be at the end of the value
fn try_walk_possible_next_obj_text<'de, 'a, W: Write>(
    mut stream: TextDeserializer<'de>,
    path_next: &PathItem<'a>,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
) -> Result<TextDeserializer<'de>, TextError> {
    match stream.peek_token().ok_or(TextError::EOF)? {
        TextToken::Equal | TextToken::CloseBracket => return Err(TextError::UnexpectedToken),
        TextToken::OpenBracket => {
            return path_next.walk_text(stream, path_rest, write);
        }
        _scalar => {
            stream.eat_token();
            return Ok(stream);
        }
    }
}

fn walk_obj_text_values<'de, 'a, W: Write>(
    mut stream: TextDeserializer<'de>,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
) -> Result<TextDeserializer<'de>, TextError> {
    stream.parse_token(TextToken::OpenBracket)?;

    loop {
        match (stream.peek_token().ok_or(TextError::EOF)?, path_rest) {
            (TextToken::Equal, _) => return Err(TextError::UnexpectedToken),
            (TextToken::CloseBracket, _) => {
                stream.eat_token();
                return Ok(stream);
            }
            (TextToken::OpenBracket, [next, ref rest @ ..]) => {
                // we can't actually check if it's a KV before walking it,
                // but let's just assume
                stream = try_walk_possible_next_obj_text(stream, next, rest, write)?;
                let Err(_) = stream.parse_token(TextToken::Equal) else {
                    let SkipValue = stream.parse()?; // oops it was a KV
                    continue;
                };
            }
            (TextToken::OpenBracket, &[]) => {
                // last path item, can print
                let CountItems(count_key) = stream.parse()?;
                let Err(_) = stream.parse_token(TextToken::Equal) else {
                    let SkipValue = stream.parse()?;
                    continue;
                };
                let _ = writeln!(write, "{{{count_key}}}");
            }
            (_scalar, [_, ..]) => {
                stream.eat_token();
                if let Ok(()) = stream.parse_token(TextToken::Equal) {
                    let SkipValue = stream.parse()?;
                };
            }
            (scalar, &[]) => {
                // last path item, can print
                stream.eat_token();
                let Err(_) = stream.parse_token(TextToken::Equal) else {
                    let SkipValue = stream.parse()?;
                    continue;
                };
                let _ = writeln!(write, "{scalar}");
            }
        }
    }
}
fn walk_obj_text_kvs_all<'de, 'a, W: Write>(
    mut stream: TextDeserializer<'de>,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
) -> Result<TextDeserializer<'de>, TextError> {
    stream.parse_token(TextToken::OpenBracket)?;

    loop {
        match (stream.peek_token().ok_or(TextError::EOF)?, path_rest) {
            (TextToken::Equal, _) => return Err(TextError::UnexpectedToken),
            (TextToken::CloseBracket, _) => {
                stream.eat_token();
                return Ok(stream);
            }
            (_, [next, ref rest @ ..]) => {
                let SkipValue = stream.parse()?;
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    continue; // it's not a KV
                };
                stream = try_walk_possible_next_obj_text(stream, next, rest, write)?
            }
            (TextToken::OpenBracket, &[]) => {
                // last path item, can print
                let CountItems(count_key) = stream.parse()?;
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    continue; // it's not a KV
                };
                let value: ViewDisplayValueText = stream.parse()?;
                let _ = writeln!(write, "{{{count_key}}} = {value}");
            }
            (scalar_key, &[]) => {
                // last path item, can print
                stream.eat_token();
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    continue; // it's not a KV
                };
                let value: ViewDisplayValueText = stream.parse()?;
                let _ = writeln!(write, "{scalar_key} = {value}");
            }
        }
    }
}
fn walk_obj_text_kvs_matching<'de, 'a, W: Write>(
    mut stream: TextDeserializer<'de>,
    key_curr: &str,
    path_rest: &[PathItem<'a>],
    write: &'a mut W,
) -> Result<TextDeserializer<'de>, TextError> {
    stream.parse_token(TextToken::OpenBracket)?;

    loop {
        match (stream.peek_token().ok_or(TextError::EOF)?, path_rest) {
            (TextToken::Equal, _) => return Err(TextError::UnexpectedToken),
            (TextToken::CloseBracket, _) => {
                stream.eat_token();
                return Ok(stream);
            }
            (TextToken::OpenBracket, _) => {
                let SkipValue = stream.parse()?;
                if let Ok(()) = stream.parse_token(TextToken::Equal) {
                    let SkipValue = stream.parse()?;
                };
            }
            (scalar_key, _) if scalar_key.to_string() != key_curr => {
                stream.eat_token();
                if let Ok(()) = stream.parse_token(TextToken::Equal) {
                    let SkipValue = stream.parse()?;
                };
            }
            (_scalar_key, [next, ref rest @ ..]) => {
                stream.eat_token();
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    continue; // it's not a KV
                };
                stream = try_walk_possible_next_obj_text(stream, next, rest, write)?
            }
            (scalar_key, &[]) => {
                // last path item, can print
                stream.eat_token();
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    continue; // it's not a KV
                };
                let value: ViewDisplayValueText = stream.parse()?;
                let _ = writeln!(write, "{scalar_key} = {value}");
            }
        }
    }
}
