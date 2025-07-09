use std::collections::HashMap;

use image::{ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::definitions::HasBlack;
use pdx_parser_core::stellaris_save_parser::SaveGame;

pub const SPACE_COLOR: Rgb<u8> = Rgb([10, 10, 10]);
pub const HYPERLANE_COLOR: Rgb<u8> = Rgb([40, 100, 150]);

/// Adds stars, blackholes, and stuff to the map.
/// Assumes `0,0` is at the map center.
pub fn draw_systems(mut image: RgbImage, save: &SaveGame) -> RgbImage {
    let width = image.width();
    let height = image.height();
    for system in save.galactic_objects.values() {
        imageproc::drawing::draw_filled_circle_mut(
            &mut image,
            (
                (width as f64 / 2.0 - system.coordinate.x) as i32,
                (system.coordinate.y + height as f64 / 2.0) as i32,
            ),
            1,
            Rgb([255, 255, 255]),
        );
    }
    return image;
}

pub fn draw_hyperlanes(mut image: RgbImage, save: &SaveGame) -> RgbImage {
    let width = image.width();
    let height = image.height();
    for (system_idx, system) in &save.galactic_objects {
        for hyperlane in &system.hyperlanes {
            if hyperlane.to < *system_idx {
                continue; // hyperlanes *should* be bidirectional so skip
            }
            let Some(other_system) = save.galactic_objects.get(&hyperlane.to) else {
                // TODO: add a warning that we're skipping this hyperlane
                // because its destination does not exist.
                continue;
            };
            imageproc::drawing::draw_line_segment_mut(
                &mut image,
                (
                    (width as f64 / 2.0 - system.coordinate.x) as f32,
                    (system.coordinate.y + height as f64 / 2.0) as f32,
                ),
                (
                    (width as f64 / 2.0 - other_system.coordinate.x) as f32,
                    (other_system.coordinate.y + height as f64 / 2.0) as f32,
                ),
                HYPERLANE_COLOR,
            );
        }
    }
    return image;
}
