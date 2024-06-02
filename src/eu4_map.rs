use std::collections::HashMap;

use crate::save_parser::SaveGame;
use image::{ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::definitions::HasBlack;

/// Finds the tag (if any) that owns the majority of the provinces in the vector.
pub fn majority_owner(provinces: &Vec<u64>, save: &SaveGame) -> Option<String> {
    let mut owners: Vec<(String, usize)> = Vec::new();
    for id in provinces {
        let Some(owner) = save.provinces.get(id) else {
            continue;
        };
        if let Some((_, count)) = owners.iter_mut().find(|(tag, _)| tag == owner) {
            *count += 1;
        } else {
            owners.push((owner.to_string(), 1));
        }
    }
    return owners
        .into_iter()
        .find(|(_, count)| *count > provinces.len() / 2)
        .map(|(tag, _)| tag);
}

const WASTELAND_COLOR: Rgb<u8> = Rgb([94, 94, 94]);
const UNCLAIMED_COLOR: Rgb<u8> = Rgb([150, 150, 150]);
const WATER_COLOR: Rgb<u8> = Rgb([68, 107, 163]);
/// Note that if we can't tell where a province belongs, it will show as unclaimed.
pub fn generate_map_colors_config(
    provinces_len: u64,
    water_provinces: &Vec<u64>,
    wasteland_neighbors: &HashMap<u64, Vec<u64>>,
    save: &SaveGame,
) -> HashMap<u16, Rgb<u8>> {
    return (1..provinces_len)
        .map(|id| {
            if water_provinces.contains(&id) {
                return (id as u16, WATER_COLOR);
            } else if let Some(neighbors) = wasteland_neighbors.get(&id) {
                return (
                    id as u16,
                    majority_owner(neighbors, save)
                        .and_then(|owner| save.all_nations.get(&owner))
                        .map_or(WASTELAND_COLOR, |nation| Rgb(nation.map_color)),
                );
            }

            let Some(owner) = save
                .provinces
                .get(&id)
                .and_then(|tag| save.all_nations.get(tag))
            else {
                return (id as u16, UNCLAIMED_COLOR);
            };
            return (id as u16, Rgb(owner.map_color));
        })
        .collect();
}

pub fn make_base_map(
    bitmap: &ImageBuffer<Luma<u16>, Vec<u16>>,
    color_map: &HashMap<u16, Rgb<u8>>,
) -> RgbImage {
    return imageproc::map::map_colors(bitmap, |color| {
        color_map.get(&color.0[0]).unwrap_or(&Rgb::black()).clone()
    });
}

pub fn generate_player_borders_config(save: &SaveGame) -> HashMap<Rgb<u8>, Rgb<u8>> {
    return save
        .all_nations
        .values()
        .filter_map(|nation| {
            let mut overlord = nation;
            while let Some(o) = overlord
                .overlord
                .as_ref()
                .and_then(|overlord_tag| save.all_nations.get(overlord_tag))
            {
                overlord = o;
            }

            if !save.player_tags.contains_key(&overlord.tag) {
                return None;
            }
            return Some((
                Rgb(nation.map_color),
                Rgb([
                    255 - overlord.map_color[0],
                    255 - overlord.map_color[1],
                    255 - overlord.map_color[2],
                ]),
            ));
        })
        .collect();
}

/**
 * color_map is country map color to own/overlord player's inverse color
 */
pub fn apply_borders(map_image: &RgbImage, color_map: &HashMap<Rgb<u8>, Rgb<u8>>) -> RgbImage {
    // TODO: this could probably be optimized
    let matches_owner = |a: &Rgb<u8>, b: &Rgb<u8>| -> bool {
        return a == b || color_map.get(a) == color_map.get(b);
    };
    return imageproc::map::map_pixels(map_image, |x, y, color| {
        let Some(inverse_color) = color_map.get(&color) else {
            return color;
        };
        let is_border = x == 0
            || y == 0
            || x + 1 == map_image.width()
            || y + 1 == map_image.height()
            || !matches_owner(map_image.get_pixel(x - 1, y - 1), &color)
            || !matches_owner(map_image.get_pixel(x - 1, y), &color)
            || !matches_owner(map_image.get_pixel(x - 1, y + 1), &color)
            || !matches_owner(map_image.get_pixel(x, y - 1), &color)
            || !matches_owner(map_image.get_pixel(x, y + 1), &color)
            || !matches_owner(map_image.get_pixel(x + 1, y - 1), &color)
            || !matches_owner(map_image.get_pixel(x + 1, y), &color)
            || !matches_owner(map_image.get_pixel(x + 1, y + 1), &color);
        return if is_border {
            inverse_color.clone()
        } else {
            color
        };
    });
}
