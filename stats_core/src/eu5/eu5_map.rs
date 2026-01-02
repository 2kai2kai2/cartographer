use crate::{eu5::assets::UnownableLocations, utils::MaybeIndexedMap};
use anyhow::{Context, anyhow};
use image::{ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::definitions::HasBlack;
use pdx_parser_core::{
    common_deserialize,
    eu5::{Location, RawCountriesEntry, RawGamestate},
};
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

    let color = compute_country_map_color(owner, gamestate).with_context(|| {
        format!("While determining color for owner (by adjacency) of {loc_name}")
    })?;
    return Ok(Rgb(color.0));
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

            let color = compute_country_map_color(owner, gamestate)
                .with_context(|| format!("While determining color for owner of {loc_name}"))?;
            return Ok(Rgb(color.0));
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

/// Returns the color of the given country, based on the owner of the country.
/// Includes blending with the overlord's color.
///
/// TODO: how are subjects of subjects handled? currently just blending once with top overlord
pub fn compute_country_map_color(
    owner_id: u32,
    gamestate: &RawGamestate,
) -> anyhow::Result<common_deserialize::Rgb> {
    let Some(RawCountriesEntry::Country(owner)) = gamestate.countries.database.get(&owner_id)
    else {
        return Err(anyhow!("Failed to find entry for country {owner_id}"));
    };
    let Some(owner_color) = owner.color else {
        return Err(anyhow!("Failed to find color for country {owner_id}"));
    };
    let mut overlord_id = owner_id;
    while let Some(dep) = gamestate.diplomacy_manager.overlords.get(&overlord_id) {
        if dep.subject_type.as_ref() == "tributary" {
            // TODO: this currently hard codes vanilla subject types, where tributary is the only
            // type that does not blend. We should dynamically determine which subject types to blend on.
            break;
        }
        overlord_id = dep.first;
    }
    if overlord_id == owner_id {
        // no overlord, so just use the owner's color
        return Ok(owner_color);
    }

    let Some(RawCountriesEntry::Country(overlord)) = gamestate.countries.database.get(&overlord_id)
    else {
        return Err(anyhow!(
            "Failed to find entry for overlord country {overlord_id}"
        ));
    };
    let Some(overlord_color) = overlord.color else {
        return Err(anyhow!(
            "Failed to find color for overlord country {overlord_id}"
        ));
    };

    return Ok(blend_subject_color(owner_color, overlord_color));
}

/// The best approximation of the in-game subject blending algorithm I could reproduce using a bunch of data points.
///
/// Hue is a decent approximation, but not perfect.
/// Saturation and value are almost always spot-on, though there are a few outliers.
pub fn blend_subject_color(
    subject: common_deserialize::Rgb,
    overlord: common_deserialize::Rgb,
) -> common_deserialize::Rgb {
    let subject = subject.to_hsv360();
    let overlord = overlord.to_hsv360();

    const HUE_BASE_AMPLITUDE: f64 = 10.0;
    let hue = overlord.h + HUE_BASE_AMPLITUDE / subject.s * (subject.h - overlord.h).sin();
    const SATURATION_SLOPE: f64 = 3.0 / 5.0;
    let saturation = overlord
        .s
        .min(SATURATION_SLOPE * (subject.s - overlord.s) + overlord.s - 0.1);
    const VALUE_SLOPE: f64 = 3.0 / 5.0;
    let value = VALUE_SLOPE * (subject.v - overlord.v) + overlord.v - 0.1;

    let blended = common_deserialize::Hsv360::new(hue, saturation, value);
    return blended.to_rgb();
}
