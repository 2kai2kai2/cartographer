use std::collections::HashMap;

use image::{ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::definitions::HasBlack;
use pdx_parser_core::stellaris_save_parser::{GalacticObject, SaveGame};

pub const SPACE_COLOR: Rgb<u8> = Rgb([10, 10, 10]);
pub const HYPERLANE_COLOR: Rgb<u8> = Rgb([40, 100, 150]);
pub const MAX_BORDER_RANGE: f64 = 50.0;
pub const ERROR_PINK: Rgb<u8> = Rgb([255, 19, 240]);

/// Returns `(scale, pixel_locations)`.
/// - scale is number of pixels per stellaris map distance unit
///
/// Scale is already applied, and is only returned for use elsewhere.
pub fn systems_to_img_space<'a>(
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

pub(crate) fn draw_political_map(
    mut image: RgbImage,
    save: &SaveGame,
    colors: &HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>,
) -> RgbImage {
    let (scale, locations) = systems_to_img_space(image.dimensions(), save);
    let locations: Vec<_> = locations.collect();

    let cell_size = MAX_BORDER_RANGE * scale * 1.1;
    let num_cols = (image.width() as f64 / cell_size).ceil() as usize;
    let num_rows = (image.height() as f64 / cell_size).ceil() as usize;
    let num_bins = num_rows * num_cols;
    let mut bins = vec![Vec::new(); num_bins];

    for (i, (x, y)) in locations
        .iter()
        .filter(|(x, y)| {
            0.0 < *x && *x < (image.width() as f64) && 0.0 < *y && *y < (image.height() as f64)
        })
        .enumerate()
    {
        let col = *x / cell_size;
        let row = *y / cell_size;
        let cell_idx = (row as usize) * num_cols + (col as usize);
        bins[cell_idx].push(i);
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
                    let dx = (x as f64) - locations[*system_idx].0;
                    let dy = (y as f64) - locations[*system_idx].1;
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
        } else {
            let system = save.galactic_objects.get(nearest_idx).expect(
                "locations is derived from galactic_objects so should have the correct length.",
            );

            let Some(owner) = system.map_owner else {
                return original_color; // unowned
            };
            let Some(owner) = save.all_nations.get(&owner) else {
                // TODO: this only happens if there is a reference to a nonexistent country.
                // Should show warning about this inconsistency in the save.
                return original_color;
            };
            let Some((_, map_color, _)) = colors.get(&owner.flag.color_secondary) else {
                return ERROR_PINK;
            };
            return Rgb(*map_color);
        }
    });

    return image;
}
