use anyhow::Context as _;
use clap::Parser;

struct LocalFetcher;
impl stats_core::Fetcher for LocalFetcher {
    async fn get(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = std::path::PathBuf::from("cartographer_web/resources").join(path);
        return std::fs::read(&path)
            .with_context(|| format!("While reading file {}", path.display()));
    }
}

#[derive(Parser)]
struct Args {
    /// The save file to use
    pub file: std::path::PathBuf,
    /// Where to save the image.
    #[arg(short, long, default_value = "stats.png")]
    pub output: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file = std::fs::read(args.file)?;
    let save = stats_core::eu5::EU5ParserStepGamestate::decompress_from(&file)?;
    let save = save.parse()?;
    let img = stats_core::eu5::render_stats_image(&LocalFetcher, save).await?;
    img.save(args.output)?;

    return Ok(());
}
