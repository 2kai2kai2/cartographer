use anyhow::{Result, anyhow};
use image::{DynamicImage, RgbImage, Rgba};
use imageproc::{definitions::HasWhite as _, drawing};
use pdx_parser_core::eu5::{PreviousPlayedItem, RawCountry, RawGamestate};

use crate::{
    Fetcher,
    eu5::{
        assets::{CommonAssets, MapAssets},
        eu5_map,
    },
    utils::{display_num, display_num_thousands},
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
    for (i, (_, nation, player)) in player_nations.into_iter().enumerate().take(16) {
        const TOP_MARGIN: i32 = 2;
        const LEFT_MARGIN: i32 = 43 + 24; // include the border graphic
        let x = LEFT_MARGIN + (2986 / 2) * (i as i32 / 8);
        let y = TOP_MARGIN + 128 * (i as i32 % 8);

        let (regular_army, regular_navy, _, _) = nation.military_size(&save)?;

        // x+0: flag
        image::imageops::overlay(&mut out, &assets.flag_frame, x as i64, y as i64 + 4);

        // x+170: player
        let mut player_name = (*player).to_string();
        while drawing::text_size(60.0, &assets.noto_serif_regular, &player_name).0 > 500 - 170 {
            player_name.pop();
        }
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 170 + 8,
            y + 34,
            60.0,
            &assets.noto_serif_regular,
            &player_name,
        );

        // x+500: Population
        image::imageops::overlay(&mut out, &assets.population, x as i64 + 500, y as i64 + 24);
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 500 + 80,
            y + 34,
            60.0,
            &assets.noto_serif_regular,
            &display_num_thousands(nation.last_months_population * 1000.0),
        );

        // x+725: Regular Army
        image::imageops::overlay(
            &mut out,
            &assets.army_regulars,
            x as i64 + 725,
            y as i64 + 24,
        );
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 725 + 80,
            y + 34,
            60.0,
            &assets.noto_serif_regular,
            &display_num_thousands(regular_army as f64),
        );

        // x+950: Regular Navy
        image::imageops::overlay(
            &mut out,
            &assets.navy_regulars,
            x as i64 + 950,
            y as i64 + 24,
        );
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 950 + 80,
            y + 34,
            60.0,
            &assets.noto_serif_regular,
            &display_num(regular_navy as f64),
        );

        // x+1175: Income/Expense
        const INCOME_COLOR: Rgba<u8> = Rgba([84, 195, 96, 255]);
        const EXPENSE_COLOR: Rgba<u8> = Rgba([226, 96, 102, 255]);
        let cashflow = nation.economy.income - nation.economy.expense;
        let (cashflow_color, cashflow_sign) = if cashflow >= 0.0 {
            (INCOME_COLOR, '+')
        } else {
            (EXPENSE_COLOR, '-')
        };
        image::imageops::overlay(
            &mut out,
            &assets.monthly_gold,
            x as i64 + 1175,
            y as i64 + 24,
        );
        drawing::draw_text_mut(
            &mut out,
            cashflow_color,
            x + 1175 + 80,
            y + 34,
            60.0,
            &assets.noto_serif_regular,
            &format!("{cashflow_sign}{}", display_num(cashflow)),
        );
        drawing::draw_text_mut(
            &mut out,
            INCOME_COLOR,
            x + 1175 + 80 + 150,
            y + 34,
            30.0,
            &assets.noto_serif_italic,
            &format!("+{}", display_num(nation.economy.income)),
        );
        drawing::draw_text_mut(
            &mut out,
            EXPENSE_COLOR,
            x + 1175 + 80 + 150,
            y + 34 + 30,
            30.0,
            &assets.noto_serif_italic,
            &format!("-{}", display_num(nation.economy.expense)),
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
        unownable,
    } = MapAssets::load(fetcher, "vanilla").await?;
    let color_map = eu5_map::generate_map_colors_config(&locations, &unownable, &gamestate)?;
    drop(unownable);
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
