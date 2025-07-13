use std::collections::HashMap;

use image::{Rgb, RgbImage};
use pdx_parser_core::stellaris_save_parser::{Country, GalacticObject, SaveGame};

const SPACE_COLOR: Rgb<u8> = Rgb([10, 10, 10]);
const HYPERLANE_COLOR: Rgb<u8> = Rgb([40, 100, 150]);
const MAX_BORDER_RANGE: f64 = 50.0;
const ERROR_PINK: Rgb<u8> = Rgb([255, 19, 240]);

/// Returns `(scale, pixel_locations)`.
/// - scale is number of pixels per stellaris map distance unit
///
/// Scale is already applied, and is only returned for use elsewhere.
fn systems_to_img_space<'a>(
    dimensions: (u32, u32),
    save: &'a SaveGame,
) -> (f64, impl Iterator<Item = (f64, f64)> + 'a) {
    let (width, height) = dimensions;
    let image_diameter = std::cmp::min(width, height) as f64;
    let scale = image_diameter / (save.galaxy_radius * 2.0);

    return (
        scale,
        save.galactic_objects.iter().map(move |system| {
            (
                width as f64 / 2.0 - system.coordinate.x * scale,
                system.coordinate.y * scale + height as f64 / 2.0,
            )
        }),
    );
}

/// Adds stars, blackholes, and stuff to the map.
/// Assumes `0,0` is at the map center.
pub fn draw_systems(mut image: RgbImage, save: &SaveGame) -> RgbImage {
    let (scale, it) = systems_to_img_space(image.dimensions(), save);
    for (x, y) in it {
        imageproc::drawing::draw_filled_circle_mut(
            &mut image,
            (x as i32, y as i32),
            std::cmp::max(scale as i32, 1),
            Rgb([255, 255, 255]),
        );
    }
    return image;
}

pub fn draw_hyperlanes(mut image: RgbImage, save: &SaveGame) -> RgbImage {
    let (scale, locations) = systems_to_img_space(image.dimensions(), save);
    let locations: Vec<_> = locations.collect();

    for (system_idx, ((x, y), system)) in
        std::iter::zip(locations.iter().cloned(), &save.galactic_objects).enumerate()
    {
        for hyperlane in &system.hyperlanes {
            if (hyperlane.to as usize) < system_idx {
                continue; // hyperlanes *should* be bidirectional so skip
            }
            let Some((other_x, other_y)) = locations.get(hyperlane.to as usize).cloned() else {
                // TODO: add a warning that we're skipping this hyperlane
                // because its destination does not exist.
                continue;
            };
            imageproc::drawing::draw_line_segment_mut(
                &mut image,
                (x as f32, y as f32),
                (other_x as f32, other_y as f32),
                HYPERLANE_COLOR,
            );
        }
    }
    return image;
}

/// Returns for each galactic object (system) by index
/// - `(owner_id, owner_country, owner_map_color)`
/// - Returns an `Err` if an id/name references a country/color that does not exist
fn make_galactic_object_ownership<'a, 'b>(
    save: &'a SaveGame,
    colors: &'b HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>,
) -> impl Iterator<Item = Result<Option<(u32, &'a Country, [u8; 3])>, ()>> + 'b
where
    'a: 'b,
{
    return save.galactic_objects.iter().map(|system| {
        let Some(owner_id) = system.map_owner else {
            return Ok(None);
        };
        let Some(owner) = save.all_nations.get(&owner_id) else {
            return Err(());
        };
        let Some(map_color) = owner.flag.index_map_color(owner.color_index, colors) else {
            return Err(());
        };
        return Ok(Some((owner_id, owner, map_color)));
    });
}

pub fn draw_political_map<'a>(
    mut image: RgbImage,
    save: &'a SaveGame,
    colors: &HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>,
) -> RgbImage {
    let (scale, locations) = systems_to_img_space(image.dimensions(), save);
    let galactic_object_ownership = make_galactic_object_ownership(save, colors);

    /// Combined data on a system/galactic object
    struct SystemData<'a> {
        id: usize,
        galactic_object: &'a GalacticObject,
        img_location: (f64, f64),
        owner: Result<Option<(u32, &'a Country, [u8; 3])>, ()>,
    }
    let systems: Vec<_> = locations
        .zip(galactic_object_ownership)
        .zip(&save.galactic_objects)
        .enumerate()
        .map(
            |(id, ((img_location, owner), galactic_object))| SystemData {
                id,
                galactic_object,
                img_location,
                owner,
            },
        )
        .collect();
    assert_eq!(save.galactic_objects.len(), systems.len());

    let cell_size = MAX_BORDER_RANGE * scale * 1.1;
    let num_cols = (image.width() as f64 / cell_size).ceil() as usize;
    let num_rows = (image.height() as f64 / cell_size).ceil() as usize;
    let num_bins = num_rows * num_cols;
    let mut bins = vec![Vec::new(); num_bins];

    for SystemData {
        id,
        img_location: (x, y),
        ..
    } in systems.iter().filter(
        |SystemData {
             img_location: (x, y),
             ..
         }| {
            0.0 < *x && *x < (image.width() as f64) && 0.0 < *y && *y < (image.height() as f64)
        },
    ) {
        let col = *x / cell_size;
        let row = *y / cell_size;
        let cell_idx = (row as usize) * num_cols + (col as usize);
        bins[cell_idx].push(*id);
    }

    const MAX_BORDER_RANGE_SQUARED: f64 = MAX_BORDER_RANGE * MAX_BORDER_RANGE;
    imageproc::map::map_pixels_mut(&mut image, |x, y, original_color| {
        let col = (x as f64 / cell_size) as usize;
        let row = (y as f64 / cell_size) as usize;

        let mut nearest_idx = usize::MAX;
        let mut nearest_distance_squared = f64::INFINITY;

        let try_rows: &[usize] = if num_rows == 1 {
            &[row]
        } else if row == 0 {
            &[row, row + 1]
        } else if row + 1 == num_rows {
            &[row - 1, row]
        } else {
            &[row - 1, row, row + 1]
        };
        let try_cols: &[usize] = if num_cols == 1 {
            &[col]
        } else if col == 0 {
            &[col, col + 1]
        } else if col + 1 == num_cols {
            &[col - 1, col]
        } else {
            &[col - 1, col, col + 1]
        };
        for row in try_rows {
            for col in try_cols {
                let cell_idx = row * num_cols + col;
                for system_idx in &bins[cell_idx] {
                    let dx = (x as f64) - systems[*system_idx].img_location.0;
                    let dy = (y as f64) - systems[*system_idx].img_location.1;
                    let distance_squared = dx * dx + dy * dy;
                    if distance_squared < nearest_distance_squared {
                        nearest_idx = *system_idx;
                        nearest_distance_squared = distance_squared;
                    }
                }
            }
        }

        if nearest_distance_squared > MAX_BORDER_RANGE_SQUARED {
            return original_color; // Out of border range, leave unchanged.
        }
        return match &systems[nearest_idx].owner {
            Ok(Some((_, _, map_color))) => Rgb(*map_color),
            Ok(None) => original_color,
            Err(()) => ERROR_PINK,
        };
    });

    return image;
}
