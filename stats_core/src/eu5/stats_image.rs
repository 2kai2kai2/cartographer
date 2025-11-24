use image::RgbImage;
use pdx_parser_core::eu5_gamestate::RawGamestate;

use crate::{
    eu5::{assets::MapAssets, eu5_map},
    Fetcher,
};

pub async fn render_stats_image(
    fetcher: &impl Fetcher,
    gamestate: RawGamestate,
) -> anyhow::Result<RgbImage> {
    let MapAssets {
        base_map,
        locations,
    } = MapAssets::load(fetcher, "vanilla").await?;
    let color_map = eu5_map::generate_map_colors_config(&locations, &gamestate)?;
    drop(locations);

    let political_map = eu5_map::make_base_map(&base_map, &color_map);
    drop(base_map);
    drop(color_map);

    // todo: the rest

    return Ok(political_map);
}
