use image::{GenericImage, GenericImageView};

use crate::utils::read_cp1252;
use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{stdin, stdout, Write},
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

fn read_definition_csv(text: &str) -> Result<HashMap<[u8; 3], u64>> {
    let mut out: HashMap<[u8; 3], u64> = HashMap::new();
    for line in text.lines().skip(1) {
        let parts = line.split(';').collect::<Vec<&str>>();
        let [id, r, g, b, _name, x] = parts.as_slice() else {
            return Err(anyhow!("Invalid csv line {}", line));
        };
        if x != &"x" {
            continue; // the x seems to mark it as used?
        }

        let id: u64 = id.parse()?;
        let r: u8 = r.parse()?;
        let g: u8 = g.parse()?;
        let b: u8 = b.parse()?;

        out.insert([r, g, b], id);
    }

    return Ok(out);
}

/// wasteland.txt
///
/// The format of each line is `[wasteland];[neighbor_a];[neighbor_b];[...]`
///
/// Specifically, the neighboring provinces are all non-wasteland, non-water provinces
/// that occupy adjacent pixels in the provinces image.
fn calculate_wasteland_adjacencies(
    wasteland_provinces: &Vec<u64>,
    water_provinces: &Vec<u64>,
    definition_csv: &HashMap<[u8; 3], u64>,
    provinces: &image::DynamicImage,
    destination_dir: &str,
) {
    let mut neighbors: HashMap<u64, Vec<u64>> = wasteland_provinces
        .iter()
        .copied()
        .map(|w| (w, Vec::new()))
        .collect();
    for (x, y, image::Rgba([r, g, b, _])) in provinces.pixels() {
        let color = [r, g, b];
        let Some(province) = definition_csv.get(&color) else {
            continue;
        };
        let Some(neighbor_vec) = neighbors.get_mut(province) else {
            continue;
        };

        let mut check_neighbor = |x2, y2| {
            let color2 = &provinces.get_pixel(x2, y2).0[0..3];
            if color == color2 {
                return;
            }
            let Some(province2) = definition_csv.get(color2) else {
                return;
            };
            if !neighbor_vec.contains(province2)
                && !water_provinces.contains(province2)
                && !wasteland_provinces.contains(province2)
            {
                neighbor_vec.push(province2.clone());
            }
        };

        if y != 0 {
            check_neighbor(x, y - 1);
        }
        if y + 1 < provinces.width() {
            check_neighbor(x, y + 1);
        }
        if x != 0 {
            check_neighbor(x - 1, y);
        }
        if x + 1 < provinces.width() {
            check_neighbor(x + 1, y);
        }
    }

    let mut neighbors = neighbors.into_iter().collect::<Vec<(u64, Vec<u64>)>>();
    neighbors.sort();
    File::create(format!("{destination_dir}/wasteland.txt"))
        .unwrap()
        .write(
            neighbors
                .into_iter()
                .map(|(p, n)| {
                    std::iter::once(p)
                        .chain(n.into_iter())
                        .map(|i| i.to_string())
                        .collect::<Vec<String>>()
                        .join(";")
                })
                .map(|line| format!("{line}\n",))
                .collect::<String>()
                .as_bytes(),
        )
        .unwrap();
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
    let definition_csv = read_cp1252(&format!("{destination}/definition.csv")).unwrap();
    let definition_csv = read_definition_csv(&definition_csv).unwrap();

    // convert provinces.bmp to provinces.png
    let provinces_img = image::open(format!("{steam_dir}/map/provinces.bmp")).unwrap();
    provinces_img
        .save_with_format(
            format!("{destination}/provinces.png"),
            image::ImageFormat::Png,
        )
        .unwrap();

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
    let water_provinces: Vec<u64> = sea_tiles.chain(lake_tiles).collect();
    File::create(format!("{destination}/water.txt"))?.write(
        water_provinces
            .iter()
            .map(|p| format!("{p}\n"))
            .collect::<String>()
            .as_bytes(),
    )?;

    // read impassible terrain from climate.txt and write to wasteland.txt
    let climate_txt = read_cp1252(&format!("{steam_dir}/map/climate.txt"))?;
    let wasteland_provinces: Vec<u64> = lines_without_comments(&climate_txt)
        .skip_while(|line| !line.trim().starts_with("impassable = {"))
        .skip(1)
        .take_while(|line| !line.contains('}'))
        .flat_map(|line| line.split_ascii_whitespace())
        .map(|item| {
            item.parse::<u64>()
                .expect(&format!("Failed to parse: \"{item}\""))
        })
        .collect();
    calculate_wasteland_adjacencies(
        &wasteland_provinces,
        &water_provinces,
        &definition_csv,
        &provinces_img,
        &destination,
    );

    // ====
    load_flagfiles(documents_dir, &destination)?;

    return Ok(());
}
