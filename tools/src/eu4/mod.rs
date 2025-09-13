use crate::utils::{moddable_read_cp1252, moddable_read_dir, moddable_read_image, stdin_line};
use decancer::cure;
use image::{GenericImage, GenericImageView};
use map::{parse_wasteland_provinces, parse_water_provinces};

use crate::utils::read_cp1252;
use anyhow::Result;
use std::{
    fs::File,
    io::{stdout, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

mod history;
mod map;

#[derive(clap::Args)]
#[command()]
pub struct Eu4Args {}

/// Returns a vector of tags
fn load_flagfiles(
    documents_dir: impl AsRef<Path>,
    destination_dir: impl AsRef<Path>,
) -> Result<Vec<String>> {
    let flagfiles_txt = read_cp1252(documents_dir.as_ref().join("gfx/flags/flagfiles.txt"))?;
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

    File::create(destination_dir.as_ref().join("flagfiles.txt"))?.write(
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
        let img = image::open(
            documents_dir
                .as_ref()
                .join(format!("gfx/flags/flagfiles_{i}.tga")),
        )
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
            destination_dir.as_ref().join("flagfiles.png"),
            image::ImageFormat::Png,
        )
        .unwrap();

    return Ok(flagfiles_tags);
}

/// Returns a hashmap `tag -> name`
pub fn load_tag_names(
    steam_dir: impl AsRef<Path>,
    mod_dir: Option<impl AsRef<Path>>,
    tags: &Vec<String>,
) -> Result<Vec<(String, Vec<String>)>> {
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

    let items = moddable_read_dir("localisation", steam_dir, mod_dir)?;
    let mut items = items
        .into_iter()
        .filter(|entry| entry.name.ends_with("_l_english.yml"))
        .flat_map(|entry| {
            let text = std::fs::read_to_string(entry.path).expect("Failed to open file");
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

pub fn eu4_main(args: Eu4Args) -> Result<()> {
    fn trim_cli(c: char) -> bool {
        return c.is_ascii_whitespace() || c == '\'' || c == '"' || c == '?';
    }

    print!("Target name: ");
    stdout().flush()?;
    let target_name = stdin_line()?;
    let target_name = target_name.trim_matches(trim_cli);

    let destination_web = format!("../cartographer_web/resources/eu4/{target_name}");
    let destination_bot = format!("../cartographer_bot/assets/eu4/{target_name}");

    print!("Steam files directory: ");
    stdout().flush()?;
    let steam_dir = stdin_line()?;
    let steam_dir = steam_dir.trim_matches(trim_cli);
    let steam_dir = PathBuf::from_str(steam_dir)?;

    print!("(optional) Mod files directory: ");
    stdout().flush()?;
    let mod_dir = stdin_line()?;
    let mod_dir = mod_dir.trim_matches(trim_cli);
    let mod_dir = if mod_dir.trim_ascii().is_empty() {
        None
    } else {
        Some(PathBuf::from_str(mod_dir)?)
    };

    print!("EU4 documents directory: ");
    stdout().flush()?;
    let documents_dir = stdin_line()?;
    let documents_dir = documents_dir.trim_matches(trim_cli);
    let documents_dir = PathBuf::from_str(documents_dir)?;

    // ====

    std::fs::create_dir_all(&destination_web)?;
    std::fs::create_dir_all(&destination_bot)?;

    // definition.csv is unchanged
    let definition_csv = moddable_read_cp1252("map/definition.csv", &steam_dir, mod_dir.as_ref())?;
    std::fs::write(format!("{destination_web}/definition.csv"), &definition_csv)?;
    let definition_csv = map::read_definition_csv(&definition_csv).unwrap();

    // convert provinces.bmp to provinces.png
    let provinces_img =
        moddable_read_image("map/provinces.bmp", &steam_dir, mod_dir.as_ref()).unwrap();
    provinces_img
        .save_with_format(
            format!("{destination_web}/provinces.png"),
            image::ImageFormat::Png,
        )
        .unwrap();

    // read water tiles from default.map
    let default_map = moddable_read_cp1252("map/default.map", &steam_dir, mod_dir.as_ref())?;
    let water_provinces = parse_water_provinces(&default_map)?;
    File::create(format!("{destination_web}/water.txt"))?.write(
        water_provinces
            .iter()
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;

    // read impassible terrain from climate.txt and write to wasteland.txt
    let climate_txt = moddable_read_cp1252("map/climate.txt", &steam_dir, mod_dir.as_ref())?;
    let wasteland_provinces = parse_wasteland_provinces(&climate_txt)?;
    map::calculate_wasteland_adjacencies(
        &wasteland_provinces,
        &water_provinces,
        &definition_csv,
        &provinces_img,
        &destination_web,
    );

    // Read country history for capitals
    let country_history =
        history::CountryHistory::read_all_countries(&steam_dir, mod_dir.as_ref())?;
    let positions_txt = moddable_read_cp1252("map/positions.txt", &steam_dir, mod_dir.as_ref())?;
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

    let country_names = load_tag_names(&steam_dir, mod_dir.as_ref(), &tags)?;
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
