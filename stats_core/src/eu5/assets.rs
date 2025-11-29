use crate::Fetcher;
use anyhow::Context as _;
use image::{ImageBuffer, Luma, RgbImage, RgbaImage};

/// Assets for eu5 that do not depend on the mod
pub struct CommonAssets {
    /// `eu5/stats_frame.png`
    pub stats_frame: RgbImage,
    pub flag_frame: RgbaImage,
    pub population: RgbaImage,
    pub army_regulars: RgbaImage,
    pub navy_regulars: RgbaImage,
    pub monthly_gold: RgbaImage,
    pub noto_serif_regular: ab_glyph::FontVec,
    pub noto_serif_italic: ab_glyph::FontVec,
}
impl CommonAssets {
    pub async fn load(fetcher: &impl Fetcher) -> anyhow::Result<Self> {
        let url_stats_frame_png = "eu5/stats_frame.png";
        let url_flag_frame_png = "eu5/flag_frame.png";
        let url_population_png = "eu5/population.png";
        let url_army_regulars_png = "eu5/army_regulars.png";
        let url_navy_regulars_png = "eu5/navy_regulars.png";
        let url_monthly_gold_png = "eu5/monthly_gold.png";
        let url_noto_serif_regular = "eu5/NotoSerif-Medium.ttf";
        let url_noto_serif_italic = "eu5/NotoSerif-Italic.ttf";

        let (
            stats_frame,
            flag_frame,
            population,
            army_regulars,
            navy_regulars,
            monthly_gold,
            noto_serif_regular,
            noto_serif_italic,
        ) = futures::try_join!(
            fetcher.get_image(url_stats_frame_png),
            fetcher.get_image(url_flag_frame_png),
            fetcher.get_image(url_population_png),
            fetcher.get_image(url_army_regulars_png),
            fetcher.get_image(url_navy_regulars_png),
            fetcher.get_image(url_monthly_gold_png),
            fetcher.get(url_noto_serif_regular),
            fetcher.get(url_noto_serif_italic),
        )?;

        let stats_frame = stats_frame.to_rgb8();
        let flag_frame = flag_frame.to_rgba8();
        let population = population.to_rgba8();
        let army_regulars = army_regulars.to_rgba8();
        let navy_regulars = navy_regulars.to_rgba8();
        let monthly_gold = monthly_gold.to_rgba8();
        let noto_serif_regular = ab_glyph::FontVec::try_from_vec(noto_serif_regular)
            .context("Failed to parse NotoSerif-Regular.ttf")?;
        let noto_serif_italic = ab_glyph::FontVec::try_from_vec(noto_serif_italic)
            .context("Failed to parse NotoSerif-Italic.ttf")?;

        return Ok(CommonAssets {
            stats_frame,
            flag_frame,
            population,
            army_regulars,
            navy_regulars,
            monthly_gold,
            noto_serif_regular,
            noto_serif_italic,
        });
    }
}

/// Map data, specific to the game mod
pub struct MapAssets {
    /// Generated from `provinces.png` and `definition.csv`, each pixel is a `u16` corresponding to the index in the locations list
    pub base_map: ImageBuffer<Luma<u16>, Vec<u16>>,
    pub locations: Vec<String>,
    // todo: unowned land
}
impl MapAssets {
    pub fn new(
        base_map: ImageBuffer<Luma<u16>, Vec<u16>>,
        locations: &str,
    ) -> anyhow::Result<MapAssets> {
        return Ok(MapAssets {
            base_map,
            locations: locations.lines().map(str::to_string).collect(),
        });
    }

    /// `dir_url` should be, for example, `"vanilla"`
    pub async fn load(fetcher: &impl Fetcher, mod_dir_path: &str) -> anyhow::Result<MapAssets> {
        let url_base_map = format!("eu5/{mod_dir_path}/locations.png");
        let url_locations_txt = format!("eu5/{mod_dir_path}/locations.txt");

        let (base_map, locations_txt) = futures::try_join!(
            fetcher.get_image(&url_base_map),
            fetcher.get_utf8(&url_locations_txt),
        )?;
        return MapAssets::new(base_map.to_luma16(), &locations_txt);
    }
}
