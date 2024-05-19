use image::{GenericImage, GenericImageView};

use crate::utils::read_cp1252;
use std::{
    fs::File,
    io::{stdin, stdout, Result, Write},
};

mod utils;

fn stdin_line() -> Result<String> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    return Ok(line);
}

fn lines_without_comments<'a>(input: &'a str) -> impl Iterator<Item = &'a str> {
    return input
        .lines()
        .map(|line| line.split('#').next().unwrap_or(line));
}

fn load_flagfiles(documents_dir: &str, destination_dir: &str) -> Result<()> {
    let flagfiles_txt = read_cp1252(&format!("{documents_dir}/gfx/flags/flagfiles.txt"))?;
    let mut flagfiles_tags: Vec<&str> = flagfiles_txt
        .split_ascii_whitespace()
        .skip(1)
        .map(|item| item.strip_suffix(".tga").unwrap())
        .collect();
    while flagfiles_tags.last().map_or(false, |tag| {
        tag.len() != 3 || tag.chars().any(|c| !c.is_ascii_uppercase())
    }) {
        flagfiles_tags.pop().unwrap();
    }
    let flag_image_lines = flagfiles_tags.len().div_ceil(16);

    File::create(format!("{destination_dir}/flagfiles.txt"))?.write(
        flagfiles_tags
            .into_iter()
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;

    // combine flag image files
    let flag_image_files = flag_image_lines.div_ceil(16);
    let mut combined_flag_image =
        image::RgbImage::new(128 * 16, 128 * 16 * flag_image_files as u32);
    for i in 0..flag_image_files {
        let img = image::open(format!("{documents_dir}/gfx/flags/flagfiles_{i}.tga"))
            .unwrap()
            .to_rgb8();
        assert_eq!(img.width(), 128 * 16);
        assert!(
            img.height() == 128 * 16 || (i + 1 >= flag_image_files && img.height() % 128 == 0),
            "Invalid flag image height {:?}",
            img.dimensions(),
        );

        combined_flag_image
            .copy_from(&img, 0, 128 * 16 * i as u32)
            .unwrap();
    }
    combined_flag_image
        .view(0, 0, 128 * 16, flag_image_lines as u32 * 128)
        .to_image()
        .save_with_format(
            format!("{destination_dir}/flagfiles.png"),
            image::ImageFormat::Png,
        )
        .unwrap();

    return Ok(());
}

fn main() -> Result<()> {
    fn trim_cli(c: char) -> bool {
        return c.is_ascii_whitespace() || c == '\'' || c == '"' || c == '?';
    }

    print!("Target name: ");
    stdout().flush()?;
    let target_name = stdin_line()?;
    let target_name = target_name.trim_matches(trim_cli);

    let destination = format!("../resources/{target_name}");

    print!("Steam files directory: ");
    stdout().flush()?;
    let steam_dir = stdin_line()?;
    let steam_dir = steam_dir.trim_matches(trim_cli);

    print!("EU4 documents directory: ");
    stdout().flush()?;
    let documents_dir = stdin_line()?;
    let documents_dir = documents_dir.trim_matches(trim_cli);

    // ====

    // definition.csv is unchanged
    std::fs::copy(
        &format!("{steam_dir}/map/definition.csv"),
        format!("{destination}/definition.csv"),
    )?;

    // read impassible terrain from climate.txt and write to wasteland.txt
    let climate_txt = read_cp1252(&format!("{steam_dir}/map/climate.txt"))?;
    let wasteland_provinces = lines_without_comments(&climate_txt)
        .skip_while(|line| !line.trim().starts_with("impassable = {"))
        .skip(1)
        .take_while(|line| !line.contains('}'))
        .flat_map(|line| line.split_ascii_whitespace())
        .map(|item| {
            item.parse::<u64>()
                .expect(&format!("Failed to parse: \"{item}\""))
        });
    File::create(format!("{destination}/wasteland.txt"))?.write(
        wasteland_provinces
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;

    // read water tiles from default.map
    let default_map = read_cp1252(&format!("{steam_dir}/map/default.map"))?;
    let sea_tiles = lines_without_comments(&default_map)
        .skip_while(|line| !line.trim().starts_with("sea_starts = {"))
        .skip(1)
        .take_while(|line| !line.contains('}'))
        .flat_map(|line| line.split_ascii_whitespace())
        .map(|item| {
            item.parse::<u64>()
                .expect(&format!("Failed to parse: \"{item}\""))
        });
    let lake_tiles = lines_without_comments(&default_map)
        .skip_while(|line| !line.trim().starts_with("lakes = {"))
        .skip(1)
        .take_while(|line| !line.contains('}'))
        .flat_map(|line| line.split_ascii_whitespace())
        .map(|item| {
            item.parse::<u64>()
                .expect(&format!("Failed to parse: \"{item}\""))
        });
    File::create(format!("{destination}/water.txt"))?.write(
        sea_tiles
            .chain(lake_tiles)
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;

    // convert provinces.bmp to provinces.png
    let provinces_img = image::open(format!("{steam_dir}/map/provinces.bmp")).unwrap();
    provinces_img
        .save_with_format(
            format!("{destination}/provinces.png"),
            image::ImageFormat::Png,
        )
        .unwrap();

    // ====
    load_flagfiles(documents_dir, &destination)?;

    return Ok(());
}
