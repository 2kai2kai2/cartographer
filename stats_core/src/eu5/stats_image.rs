use anyhow::{anyhow, Result};
use image::{DynamicImage, RgbImage, Rgba};
use imageproc::{definitions::HasWhite as _, drawing};
use pdx_parser_core::eu5_gamestate::{PreviousPlayedItem, RawCountry, RawGamestate};

use crate::{
    eu4::army_display,
    eu5::{
        assets::{CommonAssets, MapAssets},
        eu5_map,
    },
    Fetcher,
};

/// Makes the part that is not the map.
pub fn make_image_top(
    // flag_images: &FlagImages,
    assets: CommonAssets,
    save: &RawGamestate,
) -> Result<RgbImage> {
    const TOP_SIZE: (u32, u32) = (4096, 1024);
    if assets.stats_frame.dimensions() != TOP_SIZE {
        return Err(anyhow!("Template image had the incorrect dimensions"));
    }

    let mut out = DynamicImage::ImageRgb8(assets.stats_frame).into_rgba8();

    // ==== PLAYER LIST ====
    let mut player_nations: Vec<(&str, &RawCountry, &str)> = save
        .previous_played
        .iter()
        .filter_map(|PreviousPlayedItem { idtype, name }| {
            Some((
                save.countries.tags.get(idtype)?.as_ref(),
                save.countries.database.get(idtype)?.as_country()?,
                name.as_ref(),
            ))
        })
        .filter(|(_, nation, _)| nation.last_months_population > 0.0)
        .collect();
    player_nations.sort_by_key(|(_, nation, _)| nation.great_power_rank.unwrap_or(i32::MAX));
    for (i, (tag, nation, player)) in player_nations.into_iter().enumerate().take(16) {
        const TOP_MARGIN: i32 = 96;
        const LEFT_MARGIN: i32 = 43 + 96; // include the border graphic
        let x = LEFT_MARGIN + (2986 / 2) * (i as i32 / 8);
        let y = TOP_MARGIN + 128 * (i as i32 % 8);

        // x+0: flag
        image::imageops::overlay(&mut out, &assets.flag_frame, x as i64, y as i64 + 4);

        // x+170: player
        let mut player_name = (*player).to_string();
        while drawing::text_size(100.0, &assets.noto_serif_regular, &player_name).0 > 760 - 170 {
            player_name.pop();
        }
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 170 + 8,
            y + 14,
            100.0,
            &assets.noto_serif_regular,
            &player_name,
        );

        // x+760: Population
        image::imageops::overlay(&mut out, &assets.population, x as i64 + 760, y as i64 + 14);
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 760 + 128,
            y + 14,
            100.0,
            &assets.noto_serif_regular,
            &army_display(nation.last_months_population * 1000.0),
        );
    }

    // === DRAW DATE ===
    let date_str = format!("{:#}", save.metadata.date);
    let date_str_size = drawing::text_size(100.0, &assets.noto_serif_italic, &date_str);
    drawing::draw_text_mut(
        &mut out,
        Rgba::white(),
        (4096 - 512) - date_str_size.0 as i32 / 2,
        (191 / 2) - 50,
        100.0,
        &assets.noto_serif_italic,
        &date_str,
    );

    return Ok(DynamicImage::ImageRgba8(out).to_rgb8());
}

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

    let common_assets = CommonAssets::load(fetcher).await?;
    let image_top = make_image_top(common_assets, &gamestate)?;
    drop(gamestate);

    let mut image_top = image_top.into_raw();
    image_top.extend(political_map.into_raw());
    let Some(final_img) = RgbImage::from_raw(4096, 1024 + 2048, image_top) else {
        unreachable!("We just rendered these images at this size");
    };
    return Ok(final_img);
}
