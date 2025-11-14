//! In-game saving is kinda inconsistent and results in tokens not being lined up properly.
//! However, we can use the wonderful https://pdx.tools/ to melt a binary file,
//! and we will get a 1:1 token mapping, allowing us to recover the tokens.

use crate::utils::stdin_line;

use anyhow::anyhow;
use anyhow::Result;
use pdx_parser_core::bin_lexer::BinLexer;
use pdx_parser_core::bin_lexer::BinToken;
use pdx_parser_core::modern_header::ModernHeader;
use pdx_parser_core::modern_header::SaveFormat;
use pdx_parser_core::text_lexer::TextLexer;
use pdx_parser_core::text_lexer::TextToken;
use std::io::Cursor;
use std::io::{stdout, Read, Write};

#[derive(clap::Args)]
#[command()]
pub struct BinTokensArgs {
    binary_file_path: Option<String>,
    text_file_path: Option<String>,
    out: Option<String>,
}

pub fn bin_tokens_main(args: BinTokensArgs) -> Result<()> {
    fn trim_cli(c: char) -> bool {
        return c.is_ascii_whitespace() || c == '\'' || c == '"' || c == '?';
    }

    let binary_file_name = match args.binary_file_path {
        Some(binary_file_path) => binary_file_path,
        None => {
            print!("Binary file: ");
            stdout().flush()?;
            let binary_file_name = stdin_line()?;
            binary_file_name.trim_matches(trim_cli).to_string()
        }
    };

    let text_file_name = match args.text_file_path {
        Some(text_file_path) => text_file_path,
        None => {
            print!("Text file: ");
            stdout().flush()?;
            let text_file_name = stdin_line()?;
            text_file_name.trim_matches(trim_cli).to_string()
        }
    };

    let out_file_name = match args.out {
        Some(out) => out,
        None => {
            print!("Text file: ");
            stdout().flush()?;
            let out_file_name = stdin_line()?;
            out_file_name.trim_matches(trim_cli).to_string()
        }
    };

    // ====

    let bin_file = std::fs::read(binary_file_name)?;
    let (bin_gamestate, bin_header) = ModernHeader::take(&bin_file).unwrap();
    if !matches!(bin_header.save_format, SaveFormat::UnifiedCompressedBinary) {
        return Err(anyhow!("Expected unified compressed binary"));
    }
    let mut bin_gamestate = zip::ZipArchive::new(Cursor::new(bin_gamestate))?;
    let bin_gamestate = bin_gamestate.by_name("gamestate")?;
    let bin_gamestate: Vec<u8> = bin_gamestate.bytes().collect::<Result<_, _>>()?;
    let bin_gamestate = &bin_gamestate[bin_header.meta.len()..];
    let bin_gamestate = BinLexer::new(&bin_gamestate);

    let text_file = std::fs::read(text_file_name)?;
    let (text_gamestate, text_header) = ModernHeader::take(&text_file).unwrap();
    if !matches!(text_header.save_format, SaveFormat::UncompressedText) {
        return Err(anyhow!("Expected decompressed text"));
    }
    let text_gamestate = str::from_utf8(text_gamestate)?;
    let text_gamestate = TextLexer::new(&text_gamestate);

    // std::fs::write("bin.txt", bin_gamestate.clone().print_to_string())?;
    // std::fs::write("text.txt", text_gamestate.clone().print_to_string())?;

    let mut found_tokens = [None; 1 << 16];
    for (i, (bin, text)) in std::iter::zip(bin_gamestate, text_gamestate).enumerate() {
        match (bin, text) {
            (BinToken::Equal, TextToken::Equal) => {}
            (BinToken::OpenBracket, TextToken::OpenBracket) => {}
            (BinToken::CloseBracket, TextToken::CloseBracket) => {}
            (BinToken::I32(_), TextToken::Int(_)) => {}
            (BinToken::I32(_), TextToken::UInt(_)) => {}
            (BinToken::I32(_), TextToken::Float(_)) => {}
            (BinToken::I32(_), TextToken::Bool(_)) => {}
            (BinToken::I32(_), TextToken::StringQuoted(_)) => {}
            (BinToken::I32(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::F32(_), TextToken::Int(_)) => {}
            (BinToken::F32(_), TextToken::UInt(_)) => {}
            (BinToken::F32(_), TextToken::Float(_)) => {}
            (BinToken::F32(_), TextToken::Bool(_)) => {}
            (BinToken::F32(_), TextToken::StringQuoted(_)) => {}
            (BinToken::F32(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::Bool(_), TextToken::Int(_)) => {}
            (BinToken::Bool(_), TextToken::UInt(_)) => {}
            (BinToken::Bool(_), TextToken::Float(_)) => {}
            (BinToken::Bool(_), TextToken::Bool(_)) => {}
            (BinToken::Bool(_), TextToken::StringQuoted(_)) => {}
            (BinToken::Bool(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::StringQuoted(_), TextToken::Int(_)) => {}
            (BinToken::StringQuoted(_), TextToken::UInt(_)) => {}
            (BinToken::StringQuoted(_), TextToken::Float(_)) => {}
            (BinToken::StringQuoted(_), TextToken::Bool(_)) => {}
            (BinToken::StringQuoted(_), TextToken::StringQuoted(_)) => {}
            (BinToken::StringQuoted(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::U32(_), TextToken::Int(_)) => {}
            (BinToken::U32(_), TextToken::UInt(_)) => {}
            (BinToken::U32(_), TextToken::Float(_)) => {}
            (BinToken::U32(_), TextToken::Bool(_)) => {}
            (BinToken::U32(_), TextToken::StringQuoted(_)) => {}
            (BinToken::U32(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::StringUnquoted(_), TextToken::Int(_)) => {}
            (BinToken::StringUnquoted(_), TextToken::UInt(_)) => {}
            (BinToken::StringUnquoted(_), TextToken::Float(_)) => {}
            (BinToken::StringUnquoted(_), TextToken::Bool(_)) => {}
            (BinToken::StringUnquoted(_), TextToken::StringQuoted(_)) => {}
            (BinToken::StringUnquoted(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::F64(_), TextToken::Int(_)) => {}
            (BinToken::F64(_), TextToken::UInt(_)) => {}
            (BinToken::F64(_), TextToken::Float(_)) => {}
            (BinToken::F64(_), TextToken::Bool(_)) => {}
            (BinToken::F64(_), TextToken::StringQuoted(_)) => {}
            (BinToken::F64(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::U64(_), TextToken::Int(_)) => {}
            (BinToken::U64(_), TextToken::UInt(_)) => {}
            (BinToken::U64(_), TextToken::Float(_)) => {}
            (BinToken::U64(_), TextToken::Bool(_)) => {}
            (BinToken::U64(_), TextToken::StringQuoted(_)) => {}
            (BinToken::U64(_), TextToken::StringUnquoted(_)) => {}
            (BinToken::I64(_), TextToken::Int(_)) => {}
            (BinToken::I64(_), TextToken::UInt(_)) => {}
            (BinToken::I64(_), TextToken::Float(_)) => {}
            (BinToken::I64(_), TextToken::Bool(_)) => {}
            (BinToken::I64(_), TextToken::StringQuoted(_)) => {}
            (BinToken::I64(_), TextToken::StringUnquoted(_)) => {}
            (
                BinToken::Other(id),
                TextToken::StringQuoted(text) | TextToken::StringUnquoted(text),
            ) => {
                if let None = found_tokens[id as usize] {
                    found_tokens[id as usize] = Some(text);
                    println!("Token {id:5}: {text}");
                }
            }
            (bin, text) => return Err(anyhow!("Unmatched at token {i:08}: {bin}, {text}")),
        }
    }

    let out_text: String = found_tokens
        .iter()
        .enumerate()
        .map(|(id, token)| {
            if let Some(token) = token {
                return format!("{id};{token}\n");
            } else {
                return String::new();
            }
        })
        .collect();
    std::fs::write(out_file_name, out_text)?;
    return Ok(());
}
