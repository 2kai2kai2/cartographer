mod deser_helpers;
mod query;

use anyhow::{Context as _, Result, anyhow};
use clap::Parser;
use pdx_parser_core::{
    BinDeserializer, StringsResolver,
    bin_lexer::{BinToken, BinTokenLookup, TokenRegistryArray},
    modern_header::{ModernHeader, SaveFormat},
    text_deserialize::TextDeserializer,
};
use query::PathItem;
use std::{
    collections::HashSet,
    io::{Cursor, Read, Write},
};

#[derive(Parser)]
pub struct ViewArgs {
    /// The location of the file to parse and search
    pub file: std::path::PathBuf,
    // /// A series of keys
    // ///
    // /// - If path element is `$` it matches all value-type object items (not KV)
    // /// - If path element is `*` it matches all KVs
    // /// - If path element is `*some_text_here` it matches all KV with `some_text_here` as the key
    // /// - Otherwise matches the first KV with the key.
    // ///
    // /// ## Examples
    // /// - `country` `*` prints each country's data in Stellaris
    // pub path: Vec<String>,
    #[arg(long)]
    pub cp1252: bool,
    /// Skip the de-commenting step
    #[arg(long)]
    pub no_comments: bool,
    /// The location of the tokens file. Ignored if the file is not a binary save.
    /// If it matches one of the game names (e.g. "eu5"), the default tokens file will be used for the game.
    /// Otherwise, the file at the provided path will be used.
    #[arg(short, long)]
    pub tokens: Option<String>,
}

/// Will only print the same line once
struct UniqueWriter {
    buf: Vec<u8>,
    set: HashSet<Vec<u8>>,
}
impl UniqueWriter {
    pub fn new() -> UniqueWriter {
        return UniqueWriter {
            buf: Vec::new(),
            set: HashSet::new(),
        };
    }
    fn try_print_line(&mut self, line: &[u8]) -> std::io::Result<usize> {
        if !self.set.insert(line.to_vec()) {
            return Ok(line.len());
        }
        return std::io::stdout().write(line);
    }
}
impl Write for UniqueWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lines = buf.split_inclusive(|b| *b == b'\n');
        let Some(first) = lines.next() else {
            // write was empty
            return Ok(buf.len());
        };

        if matches!(first.last(), Some(b'\n')) {
            self.buf.extend(first);
            let mut temp_buf = Vec::new();
            std::mem::swap(&mut self.buf, &mut temp_buf);
            self.try_print_line(&temp_buf)?;
            std::mem::swap(&mut self.buf, &mut temp_buf);
            self.buf.clear();
        } else {
            self.buf.extend(first);
            assert!(matches!(lines.next(), None));
            return Ok(buf.len());
        }
        assert!(self.buf.is_empty());

        while let Some(line) = lines.next() {
            if matches!(line.last(), Some(b'\n')) {
                self.try_print_line(line)?;
            } else {
                // save partial line
                assert!(matches!(lines.next(), None));
                self.buf.extend(line);
            }
        }

        return Ok(buf.len());
    }

    fn flush(&mut self) -> std::io::Result<()> {
        return std::io::stdout().flush();
    }
}

/// Should include the outer brackets
enum LoadedFile {
    Text(String),
    Binary(Vec<u8>, StringsResolver),
}
impl LoadedFile {
    fn from_modern_save(header: ModernHeader) -> Result<Self> {
        match header.save_format {
            SaveFormat::SplitCompressedBinary | SaveFormat::UncompressedBinary => {
                return Err(anyhow!("We currently do not support binary save formats."));
            }
            SaveFormat::SplitCompressedText => {
                return Err(anyhow!("We currently do not support split save formats."));
            }
            SaveFormat::UnifiedCompressedBinary => {
                let mut zip_file = zip::ZipArchive::new(Cursor::new(header.gamestate))?;

                let string_lookup = match header.save_format_version {
                    1 => StringsResolver::default(),
                    2 => {
                        let mut zip_string_lookup = zip_file.by_name("string_lookup")?;
                        let mut string_lookup =
                            Vec::with_capacity(zip_string_lookup.size() as usize);
                        zip_string_lookup.read_to_end(&mut string_lookup)?;
                        drop(zip_string_lookup);
                        StringsResolver::from_raw(string_lookup.into_boxed_slice())?
                    }
                    _ => {
                        return Err(anyhow!(
                            "Invalid save format version {}",
                            header.save_format_version
                        ));
                    }
                };

                let mut zip_gamestate = zip_file.by_name("gamestate")?;
                let mut gamestate = Vec::with_capacity(4 + zip_gamestate.size() as usize);
                gamestate.extend(BinToken::ID_OPEN_BRACKET.to_le_bytes());
                zip_gamestate.read_to_end(&mut gamestate)?;
                gamestate.extend(BinToken::ID_CLOSE_BRACKET.to_le_bytes());

                return Ok(Self::Binary(gamestate, string_lookup));
            }
            SaveFormat::UnifiedCompressedText => {
                return Err(anyhow!(
                    "We currently do not support compressed save formats."
                ));
            }
            SaveFormat::UncompressedText => {
                let gamestate = str::from_utf8(header.all)?; // to include meta in gamestate
                return Ok(Self::Text(format!("{{{gamestate}}}")));
            }
        }
    }
    pub fn from_file(mut file: std::fs::File, cp1252: bool) -> Result<Self> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let bytes = bytes;

        if bytes.starts_with(b"SAV0") {
            let header = ModernHeader::take(&bytes)
                .context("Found modern save header format, but failed to parse it.")?;
            return Self::from_modern_save(header);
        }

        let text = if cp1252 {
            &tools::from_cp1252(Cursor::new(bytes))?
        } else {
            str::from_utf8(&bytes)?
        };
        let text = format!("{{{text}}}");

        return Ok(LoadedFile::Text(text));
    }
}

fn run_text_view(text: &str) -> Result<()> {
    let mut prev_input = String::new();

    loop {
        prev_input = inquire::Text::new("")
            .with_initial_value(&prev_input)
            .prompt()?;
        let path: Vec<PathItem<'_>> = prev_input
            .split_ascii_whitespace()
            .map(PathItem::from_str)
            .collect();

        if let [first, rest @ ..] = path.as_slice() {
            first.walk_text(
                TextDeserializer::from_str(&text),
                rest,
                &mut UniqueWriter::new(),
            )?;
        }
    }
}

fn run_bin_view(
    bin: &[u8],
    string_lookup: &StringsResolver,
    tokens: Option<&impl BinTokenLookup>,
) -> Result<()> {
    // let mut asdfbin = std::fs::File::create("asdf.bin")?;
    // asdfbin.write_all(bin)?;
    // let mut asdfbin = std::io::BufWriter::new(asdfbin);
    // match tokens {
    //     Some(tokens) => pdx_parser_core::bin_lexer::BinLexer::new(bin)
    //         .write_pretty_with_tokens(&mut asdfbin, tokens)?,
    //     None => pdx_parser_core::bin_lexer::BinLexer::new(bin).write_pretty(&mut asdfbin)?,
    // }

    let mut prev_input = String::new();

    loop {
        prev_input = inquire::Text::new("")
            .with_initial_value(&prev_input)
            .prompt()?;
        let path: Vec<_> = prev_input
            .split_ascii_whitespace()
            .map(PathItem::from_str)
            .collect();

        if let [first, rest @ ..] = path.as_slice() {
            let bin_rest = first.walk_bin(
                BinDeserializer::from_bytes(&bin, string_lookup),
                rest,
                &mut UniqueWriter::new(),
                tokens,
            )?;
            if bin_rest.input.len() != 0 {
                eprintln!(
                    "WARNING: Binary deserializer did not consume all input, {} bytes left.",
                    bin_rest.input.len()
                );
            }
        }
    }
}

pub fn main() -> Result<()> {
    let args = ViewArgs::parse();
    let bin_tokens = args
        .tokens
        .map(|tokens_loc| match tokens_loc.as_str() {
            "eu5" => std::path::PathBuf::from("cartographer_web/resources/eu5/tokens.txt"),
            other => std::path::PathBuf::from(other),
        })
        .map(|tokens_loc| {
            let token_file = std::fs::read_to_string(tokens_loc)?;
            TokenRegistryArray::new(token_file)
        })
        .transpose()?;

    let file = std::fs::File::open(args.file)?;
    let loaded = LoadedFile::from_file(file, args.cp1252)?;
    return match loaded {
        LoadedFile::Text(text) => run_text_view(&text),
        LoadedFile::Binary(bin, string_lookup) => {
            run_bin_view(&bin, &string_lookup, bin_tokens.as_ref())
        }
    };
}
