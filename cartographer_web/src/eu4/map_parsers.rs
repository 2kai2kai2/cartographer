use anyhow::{anyhow, Result};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use image::{GenericImageView, ImageBuffer, Luma, Rgb, RgbImage, RgbaImage};
use std::{collections::HashMap, io::Read, num::ParseIntError};

use crate::Fetcher;

pub fn from_cp1252<T: Read>(buffer: T) -> Result<String, std::io::Error> {
    let mut text = "".to_string();
    DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(buffer)
        .read_to_string(&mut text)?;
    return Ok(text);
}

pub struct FlagImages {
    tags: HashMap<String, usize>,
    images: image::RgbaImage,
}
impl FlagImages {
    fn read_flagfiles_txt(text: &str) -> HashMap<String, usize> {
        return text
            .split_ascii_whitespace()
            .enumerate()
            .map(|(i, tag)| (tag.to_string(), i))
            .collect();
    }

    pub fn new(flagfiles_txt: &str, flagfiles_png: image::RgbaImage) -> FlagImages {
        return FlagImages {
            tags: FlagImages::read_flagfiles_txt(&flagfiles_txt),
            images: flagfiles_png,
        };
    }

    pub fn get_normal_flag(&self, tag: &str) -> Option<image::SubImage<&image::RgbaImage>> {
        let index = *self.tags.get(tag)?;

        let x = 128 * (index as u32 % 16);
        let y = 128 * (index as u32 / 16);
        return Some(self.images.view(x, y, 128, 128));
    }
}

pub struct MapAssets {
    /// Will include skipped spaces. Equal to the highest province id + 1
    pub(crate) provinces_len: u64,
    pub(crate) wasteland: HashMap<u64, Vec<u64>>,
    pub(crate) water: Vec<u64>,
    pub(crate) flags: FlagImages,
    /// Generated from `provinces.png` and `definition.csv`, each pixel is a `u16` corresponding to the province id.
    pub(crate) base_map: ImageBuffer<Luma<u16>, Vec<u16>>,
}
impl MapAssets {
    pub fn read_definition_csv(text: &str) -> Result<HashMap<Rgb<u8>, u64>> {
        let mut out: HashMap<Rgb<u8>, u64> = HashMap::new();
        for line in text.lines().skip(1) {
            let parts = line.split(';').collect::<Vec<&str>>();
            let [id, r, g, b, _name, x] = parts.as_slice() else {
                return Err(anyhow!("Invalid csv line {}", line));
            };
            if x.trim() != "x" {
                continue; // the x seems to mark it as used?
            }

            let id: u64 = id.parse()?;
            let r: u8 = r.parse()?;
            let g: u8 = g.parse()?;
            let b: u8 = b.parse()?;

            out.insert(Rgb([r, g, b]), id);
        }

        return Ok(out);
    }
    pub fn read_wasteland_provinces(text: &str) -> Result<HashMap<u64, Vec<u64>>, anyhow::Error> {
        return text
            .lines()
            .map(|line| -> anyhow::Result<(u64, Vec<u64>)> {
                let mut parts = line.split(';').map(str::parse::<u64>);
                let Some(Ok(first)) = parts.next() else {
                    return Err(anyhow!("Wasteland definition row is missing a first item"));
                };

                return Ok((first, parts.collect::<Result<_, _>>()?));
            })
            .collect();
    }
    pub fn read_water_provinces(text: &str) -> Result<Vec<u64>, ParseIntError> {
        return text
            .split_ascii_whitespace()
            .map(str::parse::<u64>)
            .collect();
    }

    pub fn new(
        csv_file_text: &str,
        wasteland: &str,
        water: &str,
        flagfiles_txt: &str,
        flagfiles_png: RgbaImage,
        base_map: RgbImage,
    ) -> anyhow::Result<MapAssets> {
        let map_definitions = MapAssets::read_definition_csv(&csv_file_text)?;
        let base_map: ImageBuffer<Luma<u16>, Vec<u16>> =
            imageproc::map::map_colors(&base_map, |color| {
                Luma([*map_definitions.get(&color).unwrap_or(&0) as u16])
            });

        return Ok(MapAssets {
            provinces_len: map_definitions
                .values()
                .max()
                .map(|m| m + 1)
                .unwrap_or_default(),
            wasteland: MapAssets::read_wasteland_provinces(&wasteland)?,
            water: MapAssets::read_water_provinces(&water)?,
            flags: FlagImages::new(&flagfiles_txt, flagfiles_png),
            base_map,
        });
    }

    /// `dir_url` should be, for example, `"{}/vanilla"`
    pub async fn load(dir_url: &str) -> anyhow::Result<MapAssets> {
        let url_definition_csv = format!("{dir_url}/definition.csv");
        let url_wasteland_txt = format!("{dir_url}/wasteland.txt");
        let url_water_txt = format!("{dir_url}/water.txt");
        let url_flagfiles_txt = format!("{dir_url}/flagfiles.txt");
        let url_flagfiles_png = format!("{dir_url}/flagfiles.png");
        let url_provinces_png = format!("{dir_url}/provinces.png");

        let client = Fetcher::new();
        let (csv_file_text, wasteland, water, flagfiles_txt, flagfiles_png, base_map) = futures::try_join!(
            client.get_with_encoding(&url_definition_csv),
            client.get_with_encoding(&url_wasteland_txt),
            client.get_with_encoding(&url_water_txt),
            client.get_with_encoding(&url_flagfiles_txt),
            client.get_image(&url_flagfiles_png, image::ImageFormat::Png),
            client.get_image(&url_provinces_png, image::ImageFormat::Png)
        )?;

        return MapAssets::new(
            &csv_file_text,
            &wasteland,
            &water,
            &flagfiles_txt,
            flagfiles_png.to_rgba8(),
            base_map.to_rgb8(),
        );
    }
}
