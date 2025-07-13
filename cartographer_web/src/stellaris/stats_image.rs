use super::STELLARIS_MAP_IMAGE_SIZE;
use ab_glyph::Font;
use anyhow::{anyhow, Result};
use image::{GenericImage, Rgb, RgbImage};
use pdx_parser_core::stellaris_save_parser::SaveGame;

/// The background of the in-game menu color, including transparency
const STELLARIS_MENU_BACKGROUND_A: [u8; 4] = [23, 34, 30, 179];
/// The background of the in-game menu color when overlaid on black
const STELLARIS_MENU_BACKGROUND: [u8; 3] = [16, 23, 20];

pub fn make_final_image(
    map_image: &RgbImage,
    font: &impl Font,
    save: &SaveGame,
) -> Result<RgbImage> {
    if map_image.dimensions() != (STELLARIS_MAP_IMAGE_SIZE, STELLARIS_MAP_IMAGE_SIZE) {
        return Err(anyhow!(
            "Expected map image to be {0}x{0}",
            STELLARIS_MAP_IMAGE_SIZE
        ));
    }

    let mut stats_img = RgbImage::from_pixel(2048, 2048, Rgb(STELLARIS_MENU_BACKGROUND));

    let mut img_out = RgbImage::new(STELLARIS_MAP_IMAGE_SIZE + 2048, 2048);
    image::imageops::overlay(&mut img_out, &stats_img, STELLARIS_MAP_IMAGE_SIZE as i64, 0);
    image::imageops::overlay(&mut img_out, map_image, 0, 0);
    return Ok(img_out);
}
