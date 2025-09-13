use anyhow::anyhow;
use image::GenericImageView;
use pdx_parser_core::raw_parser::{RawPDXScalar, RawPDXValue};
use std::{collections::HashMap, fs::File, io::Write};

use crate::utils::lines_without_comments;

pub fn read_definition_csv(text: &str) -> anyhow::Result<HashMap<[u8; 3], u64>> {
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
pub fn calculate_wasteland_adjacencies(
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
        if y + 1 < provinces.height() {
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

/// takes in the text of the file `default.map`
///
/// Returns provinces in `sea_starts` and `lakes`
pub fn parse_water_provinces(default_map: &str) -> anyhow::Result<Vec<u64>> {
    let default_map: String = lines_without_comments(default_map)
        .collect::<Vec<&str>>()
        .join("\n");
    let Some((_, parsed)) =
        pdx_parser_core::raw_parser::RawPDXObject::parse_object_inner(&default_map)
    else {
        return Err(anyhow!("Failed to parse text of default.map"));
    };
    let Some(sea_starts) = parsed.get_first_obj("sea_starts") else {
        return Err(anyhow!("Did not find key `sea_starts` in default.map"));
    };
    let Some(lakes) = parsed.get_first_obj("lakes") else {
        return Err(anyhow!("Did not find key `lakes` in default.map"));
    };
    let sea_starts = sea_starts
        .iter_values()
        .filter_map(RawPDXValue::as_scalar)
        .filter_map(RawPDXScalar::as_int)
        .map(|v| v as u64);
    let lakes = lakes
        .iter_values()
        .filter_map(RawPDXValue::as_scalar)
        .filter_map(RawPDXScalar::as_int)
        .map(|v| v as u64);
    return Ok(sea_starts.chain(lakes).collect());
}

/// takes in the text of the file `climate.txt`
pub fn parse_wasteland_provinces(climate_txt: &str) -> anyhow::Result<Vec<u64>> {
    let climate_txt: String = lines_without_comments(climate_txt)
        .collect::<Vec<&str>>()
        .join("\n");
    let Some((_, parsed)) =
        pdx_parser_core::raw_parser::RawPDXObject::parse_object_inner(&climate_txt)
    else {
        return Err(anyhow!("Failed to parse text of climate.txt"));
    };
    let Some(impassable) = parsed.get_first_obj("impassable") else {
        return Err(anyhow!("Did not find key `impassable` in default.map"));
    };
    let impassable = impassable
        .iter_values()
        .filter_map(RawPDXValue::as_scalar)
        .filter_map(RawPDXScalar::as_int)
        .map(|v| v as u64);
    return Ok(impassable.collect());
}

pub fn parse_province_city_positions(
    positions_txt: &str,
) -> anyhow::Result<HashMap<usize, (f64, f64)>> {
    let positions_txt: String = lines_without_comments(positions_txt)
        .collect::<Vec<&str>>()
        .join("\n");
    let Some((_, parsed)) =
        pdx_parser_core::raw_parser::RawPDXObject::parse_object_inner(&positions_txt)
    else {
        return Err(anyhow!("Failed to parse text of positions.txt"));
    };
    return parsed
        .iter_all_KVs()
        .map(|(k, v)| {
            let province_id = k.as_int().ok_or(anyhow!("Failed to parse province id"))? as usize;

            let mut positions = v
                .as_object()
                .and_then(|v| v.get_first_obj("position"))
                .ok_or(anyhow!(
                    "Failed to get positions for province id {province_id}"
                ))?
                .iter_values();
            let x = positions
                .next()
                .and_then(RawPDXValue::as_scalar)
                .and_then(RawPDXScalar::as_float)
                .ok_or(anyhow!(
                    "Failed to parse x position for province id {province_id}"
                ))?;
            let y = positions
                .next()
                .and_then(RawPDXValue::as_scalar)
                .and_then(RawPDXScalar::as_float)
                .ok_or(anyhow!(
                    "Failed to parse x position for province id {province_id}"
                ))?;
            return Ok((province_id, (x, y)));
        })
        .collect();
}
