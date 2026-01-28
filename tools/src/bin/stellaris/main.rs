mod flags;

use anyhow::{Context, Result, anyhow};
use clap::Parser;

#[derive(Parser)]
pub struct StellarisArgs {
    /// The location of the steam game files.
    /// This will typically look something like `<...>/steamapps/common/Stellaris`.
    #[arg(short, long)]
    pub gamefiles: Option<std::path::PathBuf>,
    #[arg(long, default_value = "vanilla")]
    pub target: String,
}

pub fn main() -> Result<()> {
    let args = StellarisArgs::parse();
    let gamefiles = match args.gamefiles {
        Some(gamefiles) => gamefiles,
        None => {
            println!("Enter steam game files location:");
            let Ok(gamefiles) = tools::stdin_line() else {
                return Err(anyhow!("Exited after not recieving gamefiles location."));
            };
            gamefiles.try_into()?
        }
    };

    if args.target.contains('/')
        || args.target.contains('\\')
        || args.target == "."
        || args.target == ".."
    {
        return Err(anyhow!("Disallowed target name."));
    }

    let destination_web = std::path::PathBuf::from_iter([
        "..",
        "cartographer_web",
        "resources",
        "stellaris",
        &args.target,
    ]);
    let destination_bot = std::path::PathBuf::from_iter([
        "..",
        "cartographer_bot",
        "assets",
        "stellaris",
        &args.target,
    ]);

    flags::convert_flag_colors(&gamefiles, &destination_web)
        .context("While collecting flag colors")?;
    flags::pack_flag_imgs(&gamefiles, &destination_web).context("While packing flag icons")?;
    flags::pack_flag_frames(&gamefiles, &destination_web).context("While packing flag frames")?;

    return Ok(());
}
