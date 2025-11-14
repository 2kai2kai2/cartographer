use std::io::{Cursor, Read};

use anyhow::{anyhow, Result};
use pdx_parser_core::{
    modern_header::{ModernHeader, SaveFormat},
    text_deserialize::{self, TextDeserializer, TextError},
    text_lexer::TextToken,
};

#[derive(clap::Args)]
#[command()]
pub struct ViewArgs {
    /// The location of the file to parse and search
    pub file: std::path::PathBuf,
    /// A series of keys
    ///
    /// - If path element is `$` it matches all value-type object items (not KV)
    /// - If path element is `*` it matches all KVs
    /// - If path element is `*some_text_here` it matches all KV with `some_text_here` as the key
    /// - Otherwise matches the first KV with the key.
    ///
    /// ## Examples
    /// - `country` `*` prints each country's data in Stellaris
    pub path: Vec<String>,
    #[arg(long)]
    pub cp1252: bool,
    /// Skip the de-commenting step
    #[arg(long)]
    pub no_comments: bool,
}

enum PathItem<'a> {
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
}

/// stream should start before an item, where the (potential) key is known to match
/// even if we don't know yet if it is a KV or not
///
/// `rest_path` should exclude current key match
fn traverse_match_kv<'de, 'a>(
    mut stream: TextDeserializer<'de>,
    rest_path: &[PathItem<'a>],
) -> Result<TextDeserializer<'de>, TextError> {
    if rest_path.is_empty() {
        let key = match stream.peek_token().ok_or(TextError::EOF)? {
            TextToken::Equal => return Err(TextError::EOF),
            TextToken::OpenBracket => {
                let text_deserialize::SkipValue = stream.parse()?;
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    return Ok(stream); // it's not a KV
                };
                "{}".to_string()
            }
            TextToken::CloseBracket => return Ok(stream),
            scalar => {
                stream.eat_token();
                let Ok(()) = stream.parse_token(TextToken::Equal) else {
                    return Ok(stream); // it's not a KV
                };
                scalar.to_string()
            }
        };

        match stream.peek_token().ok_or(TextError::EOF)? {
            TextToken::Equal => return Err(TextError::EOF),
            TextToken::OpenBracket => {
                let text_deserialize::SkipValue = stream.parse()?;
                println!("{key} = {{}}");
            }
            TextToken::CloseBracket => return Ok(stream),
            scalar => {
                stream.eat_token();
                println!("{key} = {scalar}");
            }
        };
        return Ok(stream);
    }

    // skip key since we already know it matches
    let text_deserialize::SkipValue = stream.parse()?;
    let Ok(()) = stream.parse_token(TextToken::Equal) else {
        return Ok(stream); // it's not a KV
    };
    return match stream.peek_token().ok_or(TextError::EOF)? {
        TextToken::Equal => Err(TextError::EOF),
        TextToken::OpenBracket => traverse_matches(stream, rest_path),
        TextToken::CloseBracket => Err(TextError::UnexpectedToken),
        _scalar => Ok(stream),
    };
}

fn traverse_matches<'de, 'a>(
    mut stream: TextDeserializer<'de>,
    path: &[PathItem<'a>],
) -> Result<TextDeserializer<'de>, TextError> {
    let [next, rest @ ..] = path else {
        let text_deserialize::SkipValue = stream.parse()?;
        return Ok(stream);
    };

    stream.parse_token(TextToken::OpenBracket)?;

    match next {
        PathItem::Values => loop {
            match stream.peek_token().ok_or(TextError::EOF)? {
                TextToken::Equal => return Err(TextError::UnexpectedToken),
                TextToken::OpenBracket => {
                    if rest.is_empty() {
                        // leaf
                        let text_deserialize::SkipValue = stream.parse()?;
                        if let Ok(()) = stream.parse_token(TextToken::Equal) {
                            let text_deserialize::SkipValue = stream.parse()?;
                        } else {
                            println!("{{}}");
                        }
                    } else {
                        // this could be a KV, but for values we assume it's probably not
                        // TODO: figure out an efficient way to be sure

                        stream = traverse_matches(stream, rest)?;
                        // correct if actually was a KV
                        if let Ok(()) = stream.parse_token(TextToken::Equal) {
                            let text_deserialize::SkipValue = stream.parse()?;
                        }
                    }
                }
                TextToken::CloseBracket => {
                    stream.eat_token();
                    break;
                }
                scalar => {
                    stream.eat_token();
                    if let Ok(()) = stream.parse_token(TextToken::Equal) {
                        // it's a KV, skip
                        let text_deserialize::SkipValue = stream.parse()?;
                    } else if rest.is_empty() {
                        println!("{scalar}");
                    }
                }
            }
        },
        PathItem::AllKVs => loop {
            match stream.peek_token().ok_or(TextError::EOF)? {
                TextToken::Equal => return Err(TextError::UnexpectedToken),
                TextToken::CloseBracket => {
                    stream.eat_token();
                    break;
                }
                _ => {
                    stream = traverse_match_kv(stream, rest)?;
                }
            }
        },
        PathItem::MatchingKVs(target_key) => loop {
            match stream.peek_token().ok_or(TextError::EOF)? {
                TextToken::Equal => return Err(TextError::UnexpectedToken),
                TextToken::OpenBracket => {
                    let text_deserialize::SkipValue = stream.parse()?;
                    if let Ok(()) = stream.parse_token(TextToken::Equal) {
                        // it's a KV, skip
                        let text_deserialize::SkipValue = stream.parse()?;
                    }
                }
                TextToken::CloseBracket => {
                    stream.eat_token();
                    break;
                }
                scalar => {
                    if scalar.to_string() == *target_key {
                        stream = traverse_match_kv(stream, rest)?;
                    } else {
                        let text_deserialize::SkipValue = stream.parse()?;
                        if let Ok(()) = stream.parse_token(TextToken::Equal) {
                            // it's a KV, skip
                            let text_deserialize::SkipValue = stream.parse()?;
                        }
                    }
                }
            }
        },
    }
    return Ok(stream);
}

pub fn view_main(args: ViewArgs) -> Result<()> {
    // TODO: eventually might want to add an interactive mode
    let mut file = std::fs::File::open(args.file)?;
    eprintln!("opened file");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let bytes = bytes;

    let text: String;
    let text = if bytes.starts_with(b"SAV0") {
        let Some((gamestate, header)) = ModernHeader::take(&bytes) else {
            return Err(anyhow!(
                "Found modern save header format, but failed to parse it."
            ));
        };
        eprintln!("Parsed modern header format, {:?}", header.save_format);
        match header.save_format {
            SaveFormat::SplitCompressedBinary
            | SaveFormat::UnifiedCompressedBinary
            | SaveFormat::UncompressedBinary => {
                return Err(anyhow!("We currently do not support binary save formats."))
            }
            SaveFormat::SplitCompressedText => {
                return Err(anyhow!("We currently do not support split save formats."))
            }
            SaveFormat::UnifiedCompressedText => {
                return Err(anyhow!(
                    "We currently do not support compressed save formats."
                ))
            }
            SaveFormat::UncompressedText => str::from_utf8(gamestate)?,
        }
    } else if args.cp1252 {
        text = crate::utils::from_cp1252(Cursor::new(bytes))?;
        &text
    } else {
        str::from_utf8(&bytes)?
    };
    let text = format!("{{{text}}}");

    let mut prev_input: String = args
        .path
        .iter()
        .map(|item| format!("{item} "))
        .collect::<String>()
        .trim()
        .to_string();

    let mut path: Vec<PathItem> = args
        .path
        .iter()
        .map(String::as_str)
        .map(PathItem::from_str)
        .collect();

    loop {
        traverse_matches(TextDeserializer::from_str(&text), &path)?;

        prev_input = inquire::Text::new("")
            .with_initial_value(&prev_input)
            .prompt()?;
        path = prev_input
            .split_ascii_whitespace()
            .map(PathItem::from_str)
            .collect();
    }
}
