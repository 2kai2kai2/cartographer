use std::collections::HashMap;

use anyhow::{Result, anyhow};
use image::{GenericImageView, Pixel, Rgba, RgbaImage, SubImage};
use imageproc::{definitions::HasBlack, pixelops::interpolate, point::Point};
use pdx_parser_core::stellaris_save_parser::CountryFlag;

pub struct FlagParts {
    img: RgbaImage,
    list: Vec<String>,
}
impl FlagParts {
    pub fn new(img: RgbaImage, list: Vec<String>) -> FlagParts {
        return FlagParts { img, list };
    }
    pub fn get<'a>(&'a self, category: &str, file: &str) -> Option<SubImage<&'a RgbaImage>> {
        let (idx, _) = self
            .list
            .iter()
            .enumerate()
            .find(|(_, name)| **name == format!("{category}/{file}"))?;
        let view = self
            .img
            .view((idx as u32 % 8) * 128, (idx as u32 / 8) * 128, 128, 128);
        return Some(view);
    }
}

pub struct FlagFrames {
    img: RgbaImage,
}
impl FlagFrames {
    pub fn new(img: RgbaImage) -> FlagFrames {
        return FlagFrames { img };
    }
    pub fn default_frame<'a>(&'a self) -> SubImage<&'a RgbaImage> {
        return self.img.view(0, 0, 128, 128);
    }
}

fn render_flag_raw(
    flag: &CountryFlag,
    flag_parts: &FlagParts,
    colors: &HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>,
) -> Result<RgbaImage> {
    let mut img = flag_parts
        .get(&flag.background_category, &flag.background_file)
        .ok_or(anyhow!(
            "Failed to find background {}/{}",
            flag.background_category,
            flag.background_file
        ))?
        .to_image();
    let icon_img = flag_parts
        .get(&flag.icon_category, &flag.icon_file)
        .ok_or(anyhow!(
            "Failed to find icon {}/{}",
            flag.icon_category,
            flag.icon_file
        ))?;
    let icon_img = image::imageops::resize(&*icon_img, 96, 96, image::imageops::Gaussian);
    let primary_color = colors
        .get(&flag.colors[0])
        .ok_or(anyhow!("Unknown primary color"))?
        .0;
    let secondary_color = colors
        .get(&flag.colors[1])
        .ok_or(anyhow!("Unknown secondary color"))?
        .0;
    imageproc::map::map_colors_mut(&mut img, |Rgba(rgba)| {
        let mut color = Rgba([
            primary_color[0],
            primary_color[1],
            primary_color[2],
            rgba[0],
        ]);
        color.blend(&Rgba([
            secondary_color[0],
            secondary_color[1],
            secondary_color[2],
            rgba[1],
        ]));
        color
    });
    image::imageops::overlay(&mut img, &icon_img, 16, 16);
    return Ok(img);
}

fn flag_mask() -> RgbaImage {
    let mut mask = RgbaImage::new(128, 128);
    imageproc::drawing::draw_antialiased_polygon_mut(
        &mut mask,
        &[
            Point::new(64, 4),
            Point::new(12, 36),
            Point::new(12, 92),
            Point::new(64, 124),
            Point::new(116, 92),
            Point::new(116, 36),
        ],
        Rgba::black(),
        interpolate,
    );
    return mask;
}

pub fn render_flag(
    flag: &CountryFlag,
    flag_parts: &FlagParts,
    flag_frames: &FlagFrames,
    colors: &HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>,
) -> Result<RgbaImage> {
    let img = render_flag_raw(flag, flag_parts, colors)?;
    let mask = flag_mask();
    let mut img = imageproc::map::map_colors2(&img, &mask, |Rgba(a), Rgba(b)| {
        Rgba([a[0], a[1], a[2], b[3]])
    });
    image::imageops::overlay(&mut img, &*flag_frames.default_frame(), 0, 0);
    // TODO: some country types have different flag frames
    return Ok(img);
}
