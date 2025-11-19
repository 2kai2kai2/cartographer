use crate::{
    eu5::map::HexRgb,
    utils::{stdin_line, ModdableDir},
};
use image::{imageops, GenericImageView};
use pdx_parser_core::TextDeserializer;

use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{stdout, Write},
    path::PathBuf,
    str::FromStr,
};

mod map;

#[derive(clap::Args)]
#[command()]
pub struct Eu5Args {
    steam_dir: String,
    #[arg(short, long, default_value_t = String::from("vanilla"))]
    target: String,
    #[arg(short, long)]
    mod_dir: Option<String>,
}

pub fn eu5_main(args: Eu5Args) -> Result<()> {
    fn trim_cli(c: char) -> bool {
        return c.is_ascii_whitespace() || c == '\'' || c == '"' || c == '?';
    }

    let target_name = args.target;
    let destination_web = format!(
        "{}/../../../../cartographer_web/resources/eu5/{target_name}",
        file!()
    );
    let destination_bot = format!(
        "{}/../../../../cartographer_bot/assets/eu5/{target_name}",
        file!()
    );

    let steam_dir = PathBuf::from_str(&args.steam_dir.as_str())?;

    let mod_dir = if target_name == "vanilla" {
        if args.mod_dir.is_some() {
            eprintln!("WARNING: mod_dir is ignored for vanilla");
        }
        None
    } else if let Some(mod_dir) = args.mod_dir {
        Some(PathBuf::from_str(&mod_dir)?)
    } else {
        print!("(optional) Mod files directory: ");
        stdout().flush()?;
        let mod_dir = stdin_line()?;
        let mod_dir = mod_dir.trim_matches(trim_cli);
        if mod_dir.trim_ascii().is_empty() {
            None
        } else {
            Some(PathBuf::from_str(mod_dir)?)
        }
    };

    // ====

    std::fs::create_dir_all(&destination_web)?;
    std::fs::create_dir_all(&destination_bot)?;
    let dir = ModdableDir::new(steam_dir, mod_dir);

    let dir_map_data = dir.join("game/in_game/map_data");
    let default_map = dir_map_data.moddable_read_utf8("default.map")?;
    let default_map: map::DefaultMap = TextDeserializer::from_str(&default_map)
        .parse()
        .context("While reading default.map")?;

    const DEFAULT_MAP_SIZE: (u32, u32) = (16384, 8192);
    let locations_png = dir_map_data.moddable_read_image(&default_map.provinces)?;
    assert_eq!(locations_png.dimensions(), DEFAULT_MAP_SIZE);
    let locations_png = locations_png.resize(
        DEFAULT_MAP_SIZE.0 / 4,
        DEFAULT_MAP_SIZE.1 / 4,
        imageops::FilterType::Nearest,
    ); // downscale to reduce memory and bandwidth usage
    locations_png.save_with_format(
        format!("{destination_web}/locations.png"),
        image::ImageFormat::Png,
    )?;

    let mut named_locations =
        map::parse_named_locations(&dir).context("While reading named locations")?;
    named_locations.sort_by(|a, b| a.1.cmp(&b.1));
    let named_locations_txt: String = named_locations
        .iter()
        .map(|(k, v)| format!("{k};{v}\n"))
        .collect();
    File::create(format!("{destination_web}/locations.txt"))?
        .write_all(named_locations_txt.as_bytes())?;

    let named_locations: HashMap<_, _> = named_locations.into_iter().collect();
    let mut water: Vec<HexRgb> = std::iter::chain(default_map.lakes, default_map.sea_zones)
        .map(|k| named_locations.get(k).unwrap().clone())
        .collect();
    water.sort();
    let water: String = water.into_iter().map(|v| format!("{v}\n")).collect();
    File::create(format!("{destination_web}/water.txt"))?.write_all(water.as_bytes())?;

    let mut unownable: Vec<HexRgb> =
        std::iter::chain(default_map.impassable_mountains, default_map.non_ownable)
            .map(|k| named_locations.get(k).unwrap().clone())
            .collect();
    unownable.sort();
    let unownable: String = unownable.into_iter().map(|v| format!("{v}\n")).collect();
    File::create(format!("{destination_web}/unownable.txt"))?.write_all(unownable.as_bytes())?;

    return Ok(());
}
