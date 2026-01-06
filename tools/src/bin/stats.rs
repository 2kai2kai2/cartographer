use anyhow::Context as _;
use clap::Parser;

struct LocalFetcher {
    root: std::path::PathBuf,
}
impl LocalFetcher {
    fn new(root: impl Into<std::path::PathBuf>) -> Self {
        let root = root.into();
        return LocalFetcher { root };
    }
}
impl stats_core::Fetcher for LocalFetcher {
    async fn get(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = self.root.join(path);
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

    let fetcher_root =
        option_env!("CARTOGRAPHER_RESOURCES_DIR").unwrap_or("cartographer_web/resources");
    let fetcher = LocalFetcher::new(fetcher_root);
    let file = std::fs::read(args.file)?;
    let save = stats_core::eu5::EU5ParserStepGamestate::decompress_from(&file)?;
    let save = save.parse()?;
    let img = stats_core::eu5::render_stats_image(&fetcher, save).await?;
    img.save(args.output)?;

    return Ok(());
}
