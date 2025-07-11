use anyhow::Result;

mod eu4;
mod stellaris;
mod utils;

use clap::Parser;

#[derive(Parser)]
enum Cli {
    Eu4(eu4::Eu4Args),
    Stellaris(stellaris::StellarisArgs),
}

fn main() -> Result<()> {
    let command = Cli::parse();
    return match command {
        Cli::Eu4(eu4_args) => eu4::eu4_main(eu4_args),
        Cli::Stellaris(stellaris_args) => stellaris::stellaris_main(stellaris_args),
    };
}
