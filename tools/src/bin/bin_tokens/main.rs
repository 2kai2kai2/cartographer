//! In-game saving is kinda inconsistent and results in tokens not being lined up properly.
//! However, we can use the wonderful https://pdx.tools/ to melt a binary file,
//! and we will get a 1:1 token mapping, allowing us to recover the tokens.

use anyhow::{Result, anyhow};
use clap::Parser;
use pdx_parser_core::StringsResolver;
use pdx_parser_core::bin_lexer::{BinLexer, BinToken};
use pdx_parser_core::modern_header::{ModernHeader, SaveFormat};
use pdx_parser_core::text_lexer::{TextLexer, TextToken};
use std::io::{Cursor, Read, Write, stdout};
use std::path::PathBuf;
use tools::stdin_line;

#[derive(Parser)]
#[command()]
pub struct BinTokensArgs {
    binary_file_path: Option<String>,
    text_file_path: Option<String>,
    out: Option<String>,
    /// If set, will print the gamestate section to the specified file
    #[arg(short, long)]
    gamestate: Option<PathBuf>,
    /// If set, will initialize with already known tokens instead of replacing them
    #[arg(short, long)]
    update: bool,
}

struct SillyDeironmanizer<'b>(BinLexer<'b>);
impl<'b> Iterator for SillyDeironmanizer<'b> {
    type Item = <BinLexer<'b> as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.0.next()?;
        if matches!(item, BinToken::Other(0x05c1)) {
            self.0.next(); // = 
            self.0.next(); // true
            return self.0.next();
        }
        return Some(item);
    }
}

fn get_bin_tokens<'b, 't>(
    found_tokens: &mut [Option<String>; 1 << 16],
    bin_lexer: BinLexer<'b>,
    text_lexer: TextLexer<'t>,
) -> anyhow::Result<String> {
    let mut combined_output = String::new();
    let mut indent = 0;

    for (i, (bin, text)) in std::iter::zip(SillyDeironmanizer(bin_lexer), text_lexer).enumerate() {
        match (bin, text) {
            (BinToken::Equal, TextToken::Equal) => {
                combined_output.push_str(format!("{:indent$}=\n", "").as_str());
            }
            (BinToken::OpenBracket, TextToken::OpenBracket) => {
                combined_output.push_str(format!("{:indent$}{{\n", "").as_str());
                indent += 4;
            }
            (BinToken::CloseBracket, TextToken::CloseBracket) => {
                indent = indent.saturating_sub(4);
                combined_output.push_str(format!("{:indent$}}}\n", "").as_str());
            }
            (bin @ BinToken::Other(_), TextToken::Bool(value)) => {
                // Can this happen? I ran into a case where a token seems to correspond with unquoted "yes"
                // for now just skip
                let text = if value { "yes" } else { "no" };
                combined_output.push_str(format!("{:indent$}{bin}/{text}\n", "").as_str());
            }
            (bin, text) if bin.is_base_scalar() && text.is_base_scalar() => {
                let bin = bin.to_string();
                let text = text.to_string();
                if bin == text {
                    combined_output.push_str(format!("{:indent$}{bin}\n", "").as_str());
                } else {
                    combined_output.push_str(format!("{:indent$}{bin}/{text}\n", "").as_str());
                }
            }
            (
                BinToken::Other(id),
                TextToken::StringQuoted(text) | TextToken::StringUnquoted(text),
            ) => {
                combined_output.push_str(format!("{:indent$}<token {id:5}>/{text}\n", "").as_str());
                if let None = found_tokens[id as usize] {
                    found_tokens[id as usize] = Some(text.to_string());
                    println!("Token {id:5}: {text}");
                }
            }
            (bin, text) => return Err(anyhow!("Unmatched at token {i:08}: {bin}, {text}")),
        }
    }

    return Ok(combined_output);
}

pub fn main() -> Result<()> {
    let args = BinTokensArgs::parse();
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
            print!("Output tokens file: ");
            stdout().flush()?;
            let out_file_name = stdin_line()?;
            out_file_name.trim_matches(trim_cli).to_string()
        }
    };

    // ====

    let bin_file = std::fs::read(binary_file_name)?;
    let bin_header = ModernHeader::take(&bin_file).unwrap();
    if !matches!(bin_header.save_format, SaveFormat::UnifiedCompressedBinary) {
        return Err(anyhow!("Expected unified compressed binary"));
    }
    let mut bin_gamestate = zip::ZipArchive::new(Cursor::new(bin_header.gamestate))?;
    let bin_gamestate = bin_gamestate.by_name("gamestate")?;
    let bin_gamestate: Vec<u8> = bin_gamestate.bytes().collect::<Result<_, _>>()?;
    let empty_strings = StringsResolver::default();
    let bin_gamestate = BinLexer::new(&bin_gamestate, &empty_strings);

    let text_file = std::fs::read(text_file_name)?;
    let text_header = ModernHeader::take(&text_file).unwrap();
    if !matches!(text_header.save_format, SaveFormat::UncompressedText) {
        return Err(anyhow!("Expected decompressed text"));
    }
    let text_gamestate = str::from_utf8(text_header.all)?;
    let text_gamestate = TextLexer::new(&text_gamestate);

    // std::fs::write("bin.txt", bin_gamestate.clone().print_to_string())?;
    // std::fs::write("text.txt", text_gamestate.clone().print_to_string())?;

    let mut found_tokens: Box<[Option<String>; 1 << 16]> =
        if args.update && std::fs::exists(&out_file_name)? {
            let tokens_file = std::fs::read_to_string(&out_file_name)?;
            let init_tokens = pdx_parser_core::bin_lexer::TokenRegistryArray::new(tokens_file)?;
            init_tokens.unwrap()
        } else {
            vec![const { None }; 1 << 16]
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| unreachable!("We just allocated a vec to the size"))
        };

    let combined_gamestate = get_bin_tokens(&mut found_tokens, bin_gamestate, text_gamestate)?;
    if let Some(gamestate_out) = &args.gamestate {
        if let Err(_) = std::fs::write(gamestate_out, combined_gamestate) {
            eprintln!("Failed to write gamestate to {gamestate_out:?}");
        };
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
