use anyhow::Result;

mod eu4;
mod stellaris;
mod utils;
mod view;

use clap::Parser;

#[derive(Parser)]
enum Cli {
    /// Using game files, generates the required resources/assets for EU4
    Eu4(eu4::Eu4Args),
    /// Using game files, generates the required resources/assets for Stellaris
    Stellaris(stellaris::StellarisArgs),
    /// A tool for parsing and exploring clausewitz-formatted files.
    View(view::ViewArgs),
}

fn main() -> Result<()> {
    let command = Cli::parse();
    return match command {
        Cli::Eu4(eu4_args) => eu4::eu4_main(eu4_args),
        Cli::Stellaris(stellaris_args) => stellaris::stellaris_main(stellaris_args),
        Cli::View(view_args) => view::view_main(view_args),
    };
}
