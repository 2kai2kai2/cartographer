use std::collections::HashMap;

use image::RgbaImage;

use crate::Fetcher;

pub struct MapAssets {
    pub(crate) colors: HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>,
}
impl MapAssets {
    fn parse_colors_csv(csv: &str) -> anyhow::Result<HashMap<String, ([u8; 3], [u8; 3], [u8; 3])>> {
        fn do_entry(line: &str) -> anyhow::Result<(String, ([u8; 3], [u8; 3], [u8; 3]))> {
            let parts: Vec<_> = line.split(',').collect();
            let Ok([name, r1, g1, b1, r2, g2, b2, r3, g3, b3]): Result<[&str; 10], _> =
                parts.try_into()
            else {
                return Err(anyhow::anyhow!(
                    "colors.csv line had an unexpected number of elements."
                ));
            };
            return Ok((
                name.to_string(),
                (
                    [r1.parse()?, g1.parse()?, b1.parse()?],
                    [r2.parse()?, g2.parse()?, b2.parse()?],
                    [r3.parse()?, g3.parse()?, b3.parse()?],
                ),
            ));
        }
        return csv.lines().map(do_entry).collect();
    }
    pub async fn load(dir_url: &str) -> anyhow::Result<MapAssets> {
        let client = Fetcher::new();

        let url_colors = format!("{dir_url}/colors.csv");
        let colors = client.get_200(&url_colors).await?.text().await?;
        let colors = MapAssets::parse_colors_csv(&colors)?;

        return Ok(MapAssets { colors });
    }
}

pub struct StatsImageAssets {
    pub(crate) screen_bg: RgbaImage,
    /// Should be 32x32 images stacked vertically, in order:
    /// - Pop
    /// - Energy
    /// - Minerals
    /// - Food
    /// - Consumer Goods
    /// - Alloys
    /// - Unity
    /// - Research
    pub(crate) resource_icons: RgbaImage,
}
impl StatsImageAssets {
    pub async fn load(dir_url: &str) -> anyhow::Result<StatsImageAssets> {
        let client = Fetcher::new();

        let url_screen_bg = format!("{dir_url}/screen_bg.png");
        let url_resource_icons = format!("{dir_url}/resource_icons.png");

        let screen_bg = client
            .get_image(&url_screen_bg, image::ImageFormat::Png)
            .await?
            .to_rgba8();
        let resource_icons = client
            .get_image(&url_resource_icons, image::ImageFormat::Png)
            .await?
            .to_rgba8();

        return Ok(StatsImageAssets {
            screen_bg,
            resource_icons,
        });
    }
}
