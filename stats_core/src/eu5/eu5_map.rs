use std::collections::HashMap;

use anyhow::anyhow;
use image::{ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::definitions::HasBlack;
use pdx_parser_core::{
    eu5_gamestate::{RawCountriesEntry, RawGamestate},
    eu5_meta::RawMeta,
};

pub const UNCLAIMED_COLOR: Rgb<u8> = Rgb([150, 150, 150]);
pub const WATER_COLOR: Rgb<u8> = Rgb([68, 107, 163]);

/// ## Parameters
/// - locations: list of all locations, with idx being the grayscale value in `locations.png`
/// - meta: parsed meta section of the save file
/// - gamestate: parsedgamestate section of the save file
///
/// ## Returns
/// Grayscale value on location img -> owner country color
pub fn generate_map_colors_config(
    locations: &[String],
    meta: &RawMeta,
    gamestate: &RawGamestate,
) -> Result<Vec<Rgb<u8>>, anyhow::Error> {
    let meta_locs_inverse: HashMap<&str, i32> = meta
        .compatibility
        .locations
        .iter()
        .enumerate()
        .map(|(i, loc)| (loc.as_str(), i as i32 + 1))
        .collect::<HashMap<_, _>>();
    return locations
        .iter()
        .enumerate()
        .map(|(grayscale, loc)| {
            let Some(save_idx) = meta_locs_inverse.get(loc.as_str()) else {
                // meta section is missing this location (not sure why this can happen but it does)
                return Ok(Rgb::black());
            };
            let Some(location) = gamestate.locations.locations.get(&save_idx) else {
                // location is in meta but not in gamestate
                return Err(anyhow!(
                    "Failed to find location {loc} (grayscale {grayscale}, save loc idx {save_idx}) in gamestate"
                ));
            };
            let Some(owner) = location.owner else {
                return Ok(UNCLAIMED_COLOR);
            };
            let Some(RawCountriesEntry::Country(owner)) = gamestate.countries.database.get(&owner)
            else {
                return Err(anyhow!(
                    "Failed to find entry for owner {} of location {:?}",
                    owner,
                    locations.get(grayscale),
                ));
            };
            if let Some(color) = owner.color {
                return Ok(Rgb(color.0));
            } else {
                return Err(anyhow!(
                    "Failed to find color for owner {} of location {:?}",
                    owner.definition.as_ref().unwrap_or(&"UNKNOWN".into()),
                    locations.get(grayscale),
                ));
            }
        })
        .collect::<Result<Vec<_>, _>>();
}

pub fn make_base_map(
    bitmap: &ImageBuffer<Luma<u16>, Vec<u16>>,
    color_map: &Vec<Rgb<u8>>,
) -> RgbImage {
    return imageproc::map::map_colors(bitmap, |color| {
        if color.0[0] == u16::MAX {
            return WATER_COLOR;
        }
        color_map
            .get(color.0[0] as usize)
            .unwrap_or(&Rgb::black())
            .clone()
    });
}
