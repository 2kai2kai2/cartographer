use std::collections::HashMap;

use image::RgbaImage;

use crate::{
    stellaris::flags::{FlagFrames, FlagParts},
    Fetcher,
};

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

    /// `game_mod_dir` is, for example, "vanilla"
    pub async fn load(fetcher: &impl Fetcher, game_mod_dir: &str) -> anyhow::Result<MapAssets> {
        let url_colors = format!("stellaris/{game_mod_dir}/colors.csv");
        let colors = fetcher.get_utf8(&url_colors).await?;
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
    pub(crate) flag_parts: FlagParts,
    pub(crate) flag_frames: FlagFrames,
}
impl StatsImageAssets {
    pub async fn load(
        fetcher: &impl Fetcher,
        game_mod_dir: &str,
    ) -> anyhow::Result<StatsImageAssets> {
        let url_screen_bg = "stellaris/screen_bg.png";
        let url_resource_icons = "stellaris/resource_icons.png";
        let url_flag_parts_png = format!("stellaris/{game_mod_dir}/flag_parts.png");
        let url_flag_parts_txt = format!("stellaris/{game_mod_dir}/flag_parts.txt");
        let url_flag_frames_png = format!("stellaris/{game_mod_dir}/flag_frames.png");

        let screen_bg = fetcher.get_image(&url_screen_bg).await?.to_rgba8();
        let resource_icons = fetcher.get_image(&url_resource_icons).await?.to_rgba8();
        let flag_parts_png = fetcher.get_image(&url_flag_parts_png).await?.to_rgba8();
        let flag_parts_txt = fetcher.get_utf8(&url_flag_parts_txt).await?;
        let flag_parts = FlagParts::new(
            flag_parts_png,
            flag_parts_txt.lines().map(str::to_string).collect(),
        );
        let flag_frames_png = fetcher.get_image(&url_flag_frames_png).await?.to_rgba8();
        let flag_frames = FlagFrames::new(flag_frames_png);

        return Ok(StatsImageAssets {
            screen_bg,
            resource_icons,
            flag_parts,
            flag_frames,
        });
    }
}
