mod flags;
mod map;

use crate::map::{HexRgb, adjacencies};
use anyhow::{Context, Result};
use clap::Parser;
use image::{DynamicImage, GenericImageView, imageops};
use pdx_parser_core::TextDeserializer;
use std::{
    collections::HashMap,
    fs::File,
    io::{Write, stdout},
    path::PathBuf,
    str::FromStr,
};
use tools::{ModdableDir, stdin_line};

#[derive(Parser)]
pub struct Eu5Args {
    steam_dir: String,
    #[arg(short, long, default_value_t = String::from("vanilla"))]
    target: String,
    #[arg(short, long)]
    mod_dir: Option<String>,
}

pub fn main() -> Result<()> {
    let args = Eu5Args::parse();
    fn trim_cli(c: char) -> bool {
        return c.is_ascii_whitespace() || c == '\'' || c == '"' || c == '?';
    }

    let target_name = args.target;
    let destination_web = format!(
        "{}/../../../../../cartographer_web/resources/eu5/{target_name}",
        file!()
    );
    let destination_bot = format!(
        "{}/../../../../../cartographer_bot/assets/eu5/{target_name}",
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

    let named_locations =
        map::parse_named_locations(&dir).context("While reading named locations")?;
    let named_locations_txt: String = named_locations
        .iter()
        .map(|(k, _)| format!("{k}\n"))
        .collect();
    File::create(format!("{destination_web}/locations.txt"))?
        .write_all(named_locations_txt.as_bytes())?;
    let named_locations_map: HashMap<_, _> = named_locations.iter().cloned().collect();
    let named_locations_color_indices: HashMap<_, _> = named_locations
        .iter()
        .enumerate()
        .map(|(idx, (_, v))| (v.clone(), idx))
        .collect();

    let mut water: Vec<HexRgb> = std::iter::chain(default_map.lakes, default_map.sea_zones)
        .map(|k| named_locations_map.get(k).unwrap().clone())
        .collect();
    water.sort();

    const DEFAULT_MAP_SIZE: (u32, u32) = (16384, 8192);
    let locations_png = dir_map_data.moddable_read_image(&default_map.provinces)?;
    assert_eq!(locations_png.dimensions(), DEFAULT_MAP_SIZE);
    let locations_png = locations_png.to_rgb8();
    let locations_png = imageproc::map::map_colors(&locations_png, |rgb: image::Rgb<u8>| {
        let rgb = HexRgb(rgb.0);
        if water.binary_search(&rgb).is_ok() {
            return image::Luma([u16::MAX]);
        }
        let idx = *named_locations_color_indices
            .get(&rgb)
            .expect("Expected to find a location corresponding to location image color.");

        return image::Luma([idx as u16]);
    });

    let mut unownable: Vec<u16> =
        std::iter::chain(default_map.impassable_mountains, default_map.non_ownable)
            .map(|k| named_locations_map.get(k).unwrap().clone())
            .map(|v| named_locations_color_indices.get(&v).unwrap().clone() as u16)
            .collect();
    unownable.sort();
    let mut adjacencies: Vec<(u16, Vec<u16>)> = adjacencies(&locations_png)?
        .into_iter()
        .filter(|(k, _)| unownable.binary_search(k).is_ok())
        .map(|(k, mut adjs)| {
            adjs.retain(|adj| !unownable.binary_search(adj).is_ok());
            (k, adjs)
        })
        .collect();
    adjacencies.sort();
    let unownable: String = adjacencies
        .into_iter()
        .map(|(k, adjs)| {
            let adjs: String = adjs.into_iter().map(|adj| format!(";{adj}")).collect();
            format!("{k}{adjs}\n")
        })
        .collect();
    File::create(format!("{destination_web}/unownable.txt"))?.write_all(unownable.as_bytes())?;

    let locations_png = DynamicImage::ImageLuma16(locations_png).resize(
        DEFAULT_MAP_SIZE.0 / 4,
        DEFAULT_MAP_SIZE.1 / 4,
        imageops::FilterType::Nearest,
    ); // downscale to reduce memory and bandwidth usage
    locations_png.save_with_format(
        format!("{destination_web}/locations.png"),
        image::ImageFormat::Png,
    )?;

    let (flags_img, flags) = flags::do_flags(&dir).context("While rendering flags")?;
    flags_img.save_with_format(
        format!("{destination_web}/flags.png"),
        image::ImageFormat::Png,
    )?;
    std::fs::write(format!("{destination_web}/flags.txt"), flags.join("\n"))?;

    return Ok(());
}
