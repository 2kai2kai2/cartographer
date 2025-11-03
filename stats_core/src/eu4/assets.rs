use std::{collections::HashMap, num::ParseIntError};

use crate::Fetcher;
use anyhow::{anyhow, Result};
use image::{GenericImageView, ImageBuffer, Luma, Rgb, RgbImage, RgbaImage};

/// Stats assets that don't change based on mods
pub struct StatsImageDefaultAssets {
    pub(crate) army: RgbaImage,
    pub(crate) navy: RgbaImage,
    pub(crate) development: RgbaImage,
    pub(crate) income: RgbaImage,
    pub(crate) attacker: RgbaImage,
    pub(crate) defender: RgbaImage,
    pub(crate) star: RgbaImage,
    pub(crate) white_peace: RgbaImage,
    pub(crate) base_template: RgbaImage,
}
impl StatsImageDefaultAssets {
    /// `dir_url` should refer to the path where, `cartographer_web/resources` is made public
    pub async fn load(fetcher: &impl Fetcher) -> anyhow::Result<StatsImageDefaultAssets> {
        let url_army_png = "eu4/army.png";
        let url_navy_png = "eu4/navy.png";
        let url_development_png = "eu4/development.png";
        let url_income_png = "eu4/income.png";
        let url_bodycount_attacker_button_png = "eu4/bodycount_attacker_button.png";
        let url_bodycount_defender_button_png = "eu4/bodycount_defender_button.png";
        let url_star_png = "eu4/star.png";
        let url_icon_peace_png = "eu4/icon_peace.png";
        let url_final_template_png = "eu4/finalTemplate.png";
        let (army, navy, development, income, attacker, defender, star, white_peace, base_template) =
            futures::try_join!(
                fetcher.get_image(&url_army_png),
                fetcher.get_image(&url_navy_png),
                fetcher.get_image(&url_development_png),
                fetcher.get_image(&url_income_png),
                fetcher.get_image(&url_bodycount_attacker_button_png),
                fetcher.get_image(&url_bodycount_defender_button_png),
                fetcher.get_image(&url_star_png),
                fetcher.get_image(&url_icon_peace_png),
                fetcher.get_image(&url_final_template_png),
            )?;

        return Ok(StatsImageDefaultAssets {
            army: army.to_rgba8(),
            navy: navy.to_rgba8(),
            development: development.to_rgba8(),
            income: income.to_rgba8(),
            attacker: attacker.to_rgba8(),
            defender: defender.to_rgba8(),
            star: star.to_rgba8(),
            white_peace: white_peace.to_rgba8(),
            base_template: base_template.to_rgba8(),
        });
    }
}

/// Flags, specific to the game mod
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

/// Map data, specific to the game mod
pub struct MapAssets {
    /// Will include skipped spaces. Equal to the highest province id + 1
    pub provinces_len: u64,
    pub wasteland: HashMap<u64, Vec<u64>>,
    pub water: Vec<u64>,
    pub flags: FlagImages,
    /// Generated from `provinces.png` and `definition.csv`, each pixel is a `u16` corresponding to the province id.
    pub base_map: ImageBuffer<Luma<u16>, Vec<u16>>,
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

    /// `dir_url` should be, for example, `"{}/eu4/vanilla"`
    pub async fn load(fetcher: &impl Fetcher, mod_dir_path: &str) -> anyhow::Result<MapAssets> {
        let url_definition_csv = format!("eu4/{mod_dir_path}/definition.csv");
        let url_wasteland_txt = format!("eu4/{mod_dir_path}/wasteland.txt");
        let url_water_txt = format!("eu4/{mod_dir_path}/water.txt");
        let url_flagfiles_txt = format!("eu4/{mod_dir_path}/flagfiles.txt");
        let url_flagfiles_png = format!("eu4/{mod_dir_path}/flagfiles.png");
        let url_provinces_png = format!("eu4/{mod_dir_path}/provinces.png");

        let (csv_file_text, wasteland, water, flagfiles_txt, flagfiles_png, base_map) = futures::try_join!(
            fetcher.get_cp1252(&url_definition_csv),
            fetcher.get_cp1252(&url_wasteland_txt),
            fetcher.get_cp1252(&url_water_txt),
            fetcher.get_cp1252(&url_flagfiles_txt),
            fetcher.get_image(&url_flagfiles_png),
            fetcher.get_image(&url_provinces_png)
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
