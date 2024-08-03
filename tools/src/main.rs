use decancer::cure;
use image::{GenericImage, GenericImageView};
use map::{parse_wasteland_provinces, parse_water_provinces};
use utils::stdin_line;

use crate::utils::read_cp1252;
use anyhow::Result;
use std::{
    fs::File,
    io::{stdout, Read, Write},
};

mod history;
mod map;
mod utils;

/// Returns a vector of tags
fn load_flagfiles(documents_dir: &str, destination_dir: &str) -> Result<Vec<String>> {
    let flagfiles_txt = read_cp1252(&format!("{documents_dir}/gfx/flags/flagfiles.txt"))?;
    let mut flagfiles_tags: Vec<String> = flagfiles_txt
        .split_ascii_whitespace()
        .skip(1)
        .map(|item| item.strip_suffix(".tga").unwrap().to_string())
        .collect();
    while flagfiles_tags.last().map_or(false, |tag| {
        tag.len() != 3 || tag.chars().any(|c| !c.is_ascii_uppercase())
    }) {
        flagfiles_tags.pop().unwrap();
    }
    let flag_image_lines = flagfiles_tags.len().div_ceil(16);

    File::create(format!("{destination_dir}/flagfiles.txt"))?.write(
        flagfiles_tags
            .iter()
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

    return Ok(flagfiles_tags);
}

/// Returns a hashmap `tag -> name`
pub fn load_tag_names(steam_dir: &str, tags: &Vec<String>) -> Result<Vec<(String, Vec<String>)>> {
    fn parse_line<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
        let line = line.strip_prefix(" ")?;
        let (key, line) = line.split_once(':')?;
        let (_, value) = line.split_once('"')?;
        let value = value.strip_suffix('"')?;
        return Some((key, value));
    }
    fn generate_name_variants(name: &str) -> Result<Vec<String>> {
        let mut names = vec![name.to_string()];

        let normalized = cure(
            name,
            decancer::Options::default()
                .retain_capitalization()
                .ascii_only(),
        )?
        .to_string();
        if normalized != name {
            names.push(normalized);
        }

        return Ok(names);
    }
    let mut items = std::fs::read_dir(format!("{steam_dir}/localisation"))?
        .filter_map(|file| Some(file.ok()?.file_name().to_str()?.to_string()))
        .filter(|filename| filename.ends_with("_l_english.yml"))
        .flat_map(|filename| {
            let mut file = File::open(format!("{steam_dir}/localisation/{filename}"))
                .expect("Failed to open file");
            let text = {
                let mut text = String::new();
                file.read_to_string(&mut text).expect("Failed to read file");
                text
            };
            return text
                .lines()
                .filter_map(parse_line)
                .filter(|(k, _)| tags.contains(&k.to_string()))
                .map(|(k, v)| (k.to_string(), generate_name_variants(v).unwrap()))
                .collect::<Vec<_>>();
        })
        .collect::<Vec<_>>();
    items.sort();
    return Ok(items);
}

fn main() -> Result<()> {
    fn trim_cli(c: char) -> bool {
        return c.is_ascii_whitespace() || c == '\'' || c == '"' || c == '?';
    }

    print!("Target name: ");
    stdout().flush()?;
    let target_name = stdin_line()?;
    let target_name = target_name.trim_matches(trim_cli);

    let destination_web = format!("../cartographer_web/resources/{target_name}");
    let destination_bot = format!("../cartographer_bot/assets/{target_name}");

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
        format!("{destination_web}/definition.csv"),
    )?;
    let definition_csv = read_cp1252(&format!("{destination_web}/definition.csv")).unwrap();
    let definition_csv = map::read_definition_csv(&definition_csv).unwrap();

    // convert provinces.bmp to provinces.png
    let provinces_img = image::open(format!("{steam_dir}/map/provinces.bmp")).unwrap();
    provinces_img
        .save_with_format(
            format!("{destination_web}/provinces.png"),
            image::ImageFormat::Png,
        )
        .unwrap();

    // read water tiles from default.map
    let default_map = read_cp1252(&format!("{steam_dir}/map/default.map"))?;
    let water_provinces = parse_water_provinces(&default_map)?;
    File::create(format!("{destination_web}/water.txt"))?.write(
        water_provinces
            .iter()
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;

    // read impassible terrain from climate.txt and write to wasteland.txt
    let climate_txt = read_cp1252(&format!("{steam_dir}/map/climate.txt"))?;
    let wasteland_provinces = parse_wasteland_provinces(&climate_txt)?;
    map::calculate_wasteland_adjacencies(
        &wasteland_provinces,
        &water_provinces,
        &definition_csv,
        &provinces_img,
        &destination_web,
    );

    // Read country history for capitals
    let country_history = history::CountryHistory::read_all_countries(steam_dir)?;
    let positions_txt = read_cp1252(&format!("{steam_dir}/map/positions.txt"))?;
    let city_positions = map::parse_province_city_positions(&positions_txt)?;
    let mut capitals_txt = File::create(format!("{destination_bot}/capitals.txt"))?;
    for (tag, country) in country_history {
        let Some((x, y)) = city_positions.get(&country.capital) else {
            continue;
        };
        writeln!(&mut capitals_txt, "{tag};{x};{y}")?;
    }

    // ====
    let tags = load_flagfiles(documents_dir, &destination_web)?;

    let country_names = load_tag_names(steam_dir, &tags)?;
    let country_names: Vec<u8> = country_names
        .iter()
        .flat_map(|(tag, name)| format!("{tag};{}\n", name.join(";")).into_bytes())
        .collect();
    File::create(format!("{destination_web}/tags.txt"))
        .unwrap()
        .write(&country_names)
        .unwrap();

    return Ok(());
}
