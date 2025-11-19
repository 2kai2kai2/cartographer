use anyhow::Result;

mod bin_tokens;
mod eu4;
mod eu5;
mod stellaris;
mod utils;
mod view;

use clap::Parser;

#[derive(Parser)]
enum Cli {
    /// Determines the meaning of binary token ids by comparing binary and text save files
    BinTokens(bin_tokens::BinTokensArgs),
    /// Using game files, generates the required resources/assets for EU4
    Eu4(eu4::Eu4Args),
    /// Using game files, generates the required resources/assets for EU5
    Eu5(eu5::Eu5Args),
    /// Using game files, generates the required resources/assets for Stellaris
    Stellaris(stellaris::StellarisArgs),
    /// A tool for parsing and exploring clausewitz-formatted files.
    View(view::ViewArgs),
}

fn main() -> Result<()> {
    let command = Cli::parse();
    return match command {
        Cli::BinTokens(bin_tokens_args) => bin_tokens::bin_tokens_main(bin_tokens_args),
        Cli::Eu4(eu4_args) => eu4::eu4_main(eu4_args),
        Cli::Eu5(eu5_args) => eu5::eu5_main(eu5_args),
        Cli::Stellaris(stellaris_args) => stellaris::stellaris_main(stellaris_args),
        Cli::View(view_args) => view::view_main(view_args),
    };
}
