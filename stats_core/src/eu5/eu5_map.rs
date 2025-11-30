use crate::{eu5::assets::UnownableLocations, utils::MaybeIndexedMap};
use anyhow::anyhow;
use image::{ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::definitions::HasBlack;
use pdx_parser_core::eu5::{Location, RawCountriesEntry, RawGamestate};
use std::collections::HashMap;

pub const UNCLAIMED_COLOR: Rgb<u8> = Rgb([150, 150, 150]);
pub const WATER_COLOR: Rgb<u8> = Rgb([68, 107, 163]);

/// Unownable locations are colored if one country owns all the adjacent ownable locations.
///
/// ## Parameters
/// - loc_name: name of the unownable location, used for error messages
/// - adj: list of grayscale values of adjacent locations
/// - locations: list of all locations, with idx being the grayscale value in `locations.png`
/// - meta_locs_inverse: location name -> meta location index
/// - gamestate: parsed gamestate section of the save file
fn determine_unownable_color(
    loc_name: &str,
    adj: &[u16],
    grayscale_to_loc: &MaybeIndexedMap<(&str, i32, &Location)>,
    gamestate: &RawGamestate,
) -> Result<Rgb<u8>, anyhow::Error> {
    let [first_adj_grayscale, rest @ ..] = adj else {
        // There are no adjacent ownable locations, so it can never be claimed
        return Ok(UNCLAIMED_COLOR);
    };
    let Some(owner) = grayscale_to_loc
        .get(*first_adj_grayscale as usize)
        .and_then(|(_, _, first_adj_loc)| first_adj_loc.owner)
    else {
        // either is missing from save metadata (so ignore it for now), or has no owner
        return Ok(UNCLAIMED_COLOR);
    };

    let all_same_owner = rest.iter().all(|adj_grayscale| {
        grayscale_to_loc
            .get(*adj_grayscale as usize)
            .and_then(|(_, _, adj)| adj.owner)
            .is_some_and(|adj_owner| adj_owner == owner)
    });
    if !all_same_owner {
        return Ok(UNCLAIMED_COLOR);
    }

    let Some(RawCountriesEntry::Country(owner)) = gamestate.countries.database.get(&owner) else {
        return Err(anyhow!(
            "Failed to find entry for owner (by adjacency) {} of location {loc_name:?}",
            owner,
        ));
    };
    if let Some(color) = owner.color {
        return Ok(Rgb(color.0));
    } else {
        return Err(anyhow!(
            "Failed to find color for owner (by adjacency) {} of location {loc_name:?}",
            owner.definition.as_ref().unwrap_or(&"UNKNOWN".into()),
        ));
    }
}

/// ## Parameters
/// - locations: list of all locations, with idx being the grayscale value in `locations.png`
/// - unownable: list of unowned locations, each with a list of adjacent ownable neighbors
/// - gamestate: parsed gamestate section of the save file
///
/// ## Returns
/// Grayscale value on location img -> owner country color
pub fn generate_map_colors_config(
    locations: &[String],
    unownable: &UnownableLocations,
    gamestate: &RawGamestate,
) -> Result<Vec<Rgb<u8>>, anyhow::Error> {
    // meta_locs_inverse: location name -> meta location index
    let meta_locs_inverse: HashMap<&str, i32> = gamestate
        .metadata
        .compatibility
        .locations
        .iter()
        .enumerate()
        .map(|(i, loc)| (loc.as_str(), i as i32 + 1))
        .collect::<HashMap<_, _>>();
    // Skips anything that is not in the save metadata, errors if anything else is missing
    let grayscale_to_loc: MaybeIndexedMap<(&str, i32, &Location)> = locations
        .iter()
        .enumerate()
        .filter_map(|(i, loc_name)| {
            let loc_name = loc_name.as_str();
            let Some(save_idx) = meta_locs_inverse.get(loc_name) else {
                return None;
            };
            let Some(location) = gamestate.locations.locations.get(&save_idx) else {
                return Some(Err(anyhow!(
                    "Failed to find location {loc_name} in gamestate."
                )));
            };
            let value = (loc_name, *save_idx, location);
            return Some(Ok((i, value)));
        })
        .collect::<anyhow::Result<_>>()?;
    drop(meta_locs_inverse);

    return locations
        .iter()
        .enumerate()
        .map(|(grayscale, _)| {
            let Some((loc_name, _, location)) = grayscale_to_loc.get(grayscale) else {
                // location is not in save metadata, hopefully it's not important
                return Ok(Rgb::black());
            };

            let Some(owner) = location.owner else {
                let Some(adj) = unownable.get(grayscale as u16) else {
                    // location is ownable, just no land-based country owns it
                    return Ok(UNCLAIMED_COLOR);
                };
                return determine_unownable_color(loc_name, adj, &grayscale_to_loc, gamestate);
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
