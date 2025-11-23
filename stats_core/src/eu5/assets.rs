use crate::Fetcher;
use image::{ImageBuffer, Luma};

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
