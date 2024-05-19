use wasm_bindgen::prelude::*;

use ab_glyph::FontRef;
use anyhow::Result;
use stats_image::make_final_image;

mod bad_parser;
mod eu4_date;
mod eu4_map;
mod map_parsers;
mod new_parser;
mod stats_image;

fn main() -> Result<()> {
    println!("Loading save...");
    let save_file = map_parsers::read_cp1252("../mp_Portugal1567_05_11.eu4")?;
    let save = bad_parser::SaveGame::bad_parser(&save_file)?;

    println!("Loading map definitions...");
    let def_csv_file = map_parsers::read_cp1252("../resources/vanilla/definition.csv")?;
    let def_csv = map_parsers::read_definition_csv(&def_csv_file)?;
    let climate_txt = map_parsers::read_cp1252("../resources/vanilla/climate.txt")?;
    let wasteland = map_parsers::read_wasteland_provinces(&climate_txt)?;
    let default_map = map_parsers::read_cp1252("../resources/vanilla/default.map")?;
    let water = map_parsers::read_water_provinces(&default_map)?;

    println!("Drawing map...");
    let color_map = eu4_map::generate_map_colors_config(&def_csv, &water, &wasteland, &save)?;
    let raw_img = image::open("../resources/vanilla/provinces.bmp")?.to_rgba8();
    let base_map = eu4_map::make_base_map(&raw_img, &color_map);

    println!("Drawing borders...");
    let borders_config = eu4_map::generate_player_borders_config(&save);
    let base_map = eu4_map::apply_borders(&base_map, &borders_config);

    println!("Drawing stats...");
    let flag_images = map_parsers::FlagImages::load_from_filesystem()?;
    let garamond = FontRef::try_from_slice(include_bytes!("../../resources/GARA.TTF"))?;

    let base_img = image::open("../resources/finalTemplate.png")?.to_rgba8();
    let stats_image_assets = stats_image::StatsImageIconAssets::load_from_filesystem()?;
    let res = make_final_image(
        &base_img,
        &base_map,
        &flag_images,
        &garamond,
        &stats_image_assets,
        &save,
    )?;
    println!("final done");

    res.save("final_img.png")?;

    return Ok(());
}
