use std::cmp::Reverse;

use super::map_parsers::FlagImages;
use crate::Fetcher;
use ab_glyph::Font;
use anyhow::{anyhow, Result};
use image::{GenericImage, GenericImageView, Rgba, RgbaImage};
use imageproc::definitions::HasWhite;
use imageproc::drawing;
use pdx_parser_core::eu4_save_parser::{Nation, SaveGame, WarResult};

pub fn army_display(army: f64) -> String {
    if army >= 1000000.0 {
        return format!("{}M", (army / 10000.0).round() / 100.0);
    } else if army >= 100000.0 {
        return format!("{:.0}k", army / 1000.0);
    } else if army >= 0.0 {
        return format!("{}k", (army / 100.0).round() / 10.0);
    } else {
        return "ERROR".to_string();
    }
}

/// Assumes whitespace is only a single space between words
pub fn text_wrap(text: &str, font: &impl Font, scale: f32, width: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();

    for part in text.split_ascii_whitespace() {
        let prospective = if line.is_empty() {
            part.to_string()
        } else {
            format!("{line} {part}")
        };
        if drawing::text_size(scale, font, &prospective).0 > width {
            out.push(line);
            line = part.to_string();
        } else {
            line = prospective;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    return out;
}

/// For assets that don't vary by mod/version
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
    pub async fn load(dir_url: &str) -> anyhow::Result<StatsImageDefaultAssets> {
        let client = Fetcher::new();

        let url_army_png = format!("{dir_url}/eu4/army.png");
        let url_navy_png = format!("{dir_url}/eu4/navy.png");
        let url_development_png = format!("{dir_url}/eu4/development.png");
        let url_income_png = format!("{dir_url}/eu4/income.png");
        let url_bodycount_attacker_button_png =
            format!("{dir_url}/eu4/bodycount_attacker_button.png");
        let url_bodycount_defender_button_png =
            format!("{dir_url}/eu4/bodycount_defender_button.png");
        let url_star_png = format!("{dir_url}/eu4/star.png");
        let url_icon_peace_png = format!("{dir_url}/eu4/icon_peace.png");
        let url_final_template_png = format!("{dir_url}/eu4/finalTemplate.png");
        let (army, navy, development, income, attacker, defender, star, white_peace, base_template) =
            futures::try_join!(
                client.get_image(&url_army_png, image::ImageFormat::Png),
                client.get_image(&url_navy_png, image::ImageFormat::Png),
                client.get_image(&url_development_png, image::ImageFormat::Png),
                client.get_image(&url_income_png, image::ImageFormat::Png),
                client.get_image(&url_bodycount_attacker_button_png, image::ImageFormat::Png),
                client.get_image(&url_bodycount_defender_button_png, image::ImageFormat::Png),
                client.get_image(&url_star_png, image::ImageFormat::Png),
                client.get_image(&url_icon_peace_png, image::ImageFormat::Png),
                client.get_image(&url_final_template_png, image::ImageFormat::Png),
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

pub fn make_final_image(
    map_image: &RgbaImage,
    flag_images: &FlagImages,
    font: &impl Font,
    default_assets: &StatsImageDefaultAssets,
    save: &SaveGame,
) -> Result<RgbaImage> {
    const BASE_SIZE: (u32, u32) = (5632, 3168);
    const MAP_SIZE: (u32, u32) = (5632, 2048);
    if default_assets.base_template.dimensions() != BASE_SIZE {
        return Err(anyhow!("Base image had the incorrect dimensions"));
    }
    if map_image.dimensions() != MAP_SIZE {
        return Err(anyhow!("Map image had the incorrect dimensions"));
    }
    let mut out = default_assets.base_template.clone();

    out.copy_from(map_image, 0, BASE_SIZE.1 - MAP_SIZE.1)?;

    // ==== PLAYER LIST ====
    let mut player_nations: Vec<(&Nation, &String)> = save
        .player_tags
        .iter()
        .filter_map(|(tag, player)| Some((save.all_nations.get(tag)?, player)))
        .filter(|(nation, _)| nation.development != 0)
        .collect();
    player_nations.sort_by_key(|(nation, _)| Reverse(nation.development));
    for (i, (nation, player)) in player_nations.iter().enumerate().take(16) {
        let x = (38 + 2335 * (i / 8)) as i32;
        let y = (38 + 128 * (i % 8)) as i32;

        // x+0: flag
        out.copy_from(
            &*flag_images
                .get_normal_flag(&nation.tag)
                .ok_or(anyhow!("Couldn't find flag for {}", nation.tag))?,
            x as u32,
            y as u32,
        )?;

        // x+128: player
        let mut player_name = (*player).clone();
        while drawing::text_size(100.0, font, &player_name).0 > 760 - 128 {
            player_name.pop();
        }
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 128 + 8,
            y + 14,
            100.0,
            font,
            &player_name,
        );

        // x+760: Army
        out.copy_from(&default_assets.army, x as u32 + 760, y as u32)?;
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 760 + 128,
            y + 14,
            100.0,
            font,
            &army_display(nation.army),
        );

        // x+1100: Navy
        out.copy_from(&default_assets.navy, x as u32 + 1100, y as u32)?;
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 1100 + 128,
            y + 14,
            100.0,
            font,
            &nation.navy.to_string(),
        );

        // x+1440: Dev
        out.copy_from(&default_assets.development, x as u32 + 1440, y as u32)?;
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 1440 + 128,
            y + 14,
            100.0,
            font,
            &nation.development.to_string(),
        );

        // x+1780: Income/Expense
        const INCOME_COLOR: Rgba<u8> = Rgba([49, 190, 66, 255]);
        const EXPENSE_COLOR: Rgba<u8> = Rgba([247, 16, 16, 255]);
        let cashflow = nation.total_income - nation.total_expense;
        let (cashflow_color, income_img) = if cashflow >= 0.0 {
            (INCOME_COLOR, default_assets.income.view(0, 0, 128, 128))
        } else {
            (EXPENSE_COLOR, default_assets.income.view(128, 0, 128, 128))
        };
        out.copy_from(&*income_img, x as u32 + 1780, y as u32)?;
        drawing::draw_text_mut(
            &mut out,
            cashflow_color,
            x + 1780 + 128,
            y + 14,
            100.0,
            font,
            &format!("{:.0}", cashflow),
        );
        drawing::draw_text_mut(
            &mut out,
            INCOME_COLOR,
            x + 2130,
            y + 7,
            50.0,
            font,
            &format!("+{:.2}", nation.total_income),
        );
        drawing::draw_text_mut(
            &mut out,
            EXPENSE_COLOR,
            x + 2130,
            y + 64 + 7,
            50.0,
            font,
            &format!("-{:.2}", nation.total_expense),
        );
    }

    // ==== WARS ====
    let mut player_wars = save.player_wars.clone();
    let player_tags = save.player_tags.values().cloned().collect();
    player_wars.sort_by(|a, b| {
        a.war_scale(&player_tags)
            .partial_cmp(&b.war_scale(&player_tags))
            .unwrap()
            .reverse()
    });

    for (i, w) in player_wars.iter().take(4).enumerate() {
        let x = 4742;
        let y = (230 + 218 * i) as i32;

        let player_attackers = w
            .attackers
            .iter()
            .filter(|tag| save.tag_player(tag).is_some());
        for (i, attacker) in player_attackers.take(8).enumerate() {
            let flag = flag_images
                .get_normal_flag(&attacker)
                .ok_or(anyhow!("failed to get flag for tag {}", attacker))?;
            let resized =
                image::imageops::resize(&*flag, 64, 64, image::imageops::FilterType::Nearest);
            out.copy_from(
                &resized,
                x as u32 + 3 * (12 + 64) - (i as u32 % 4) * (64 + 12),
                y as u32 + (i as u32 - i as u32 % 4) / 4 * (62 + 12) + 12,
            )?;
        }

        image::imageops::overlay(
            &mut out,
            &default_assets.attacker,
            x as i64 + 290 - 12 - 32,
            y as i64 + 156,
        );
        let attacker_losses_str = format!("Losses: {}", army_display(w.attacker_losses as f64));
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 290 - 12 - 32 - drawing::text_size(36.0, font, &attacker_losses_str).0 as i32,
            y + 152,
            36.0,
            font,
            &attacker_losses_str,
        );

        let player_defenders = w
            .defenders
            .iter()
            .filter(|tag| save.tag_player(tag).is_some());
        for (i, defender) in player_defenders.take(8).enumerate() {
            let flag = flag_images
                .get_normal_flag(&defender)
                .ok_or(anyhow!("failed to get flag for tag {}", defender))?;
            let resized =
                image::imageops::resize(&*flag, 64, 64, image::imageops::FilterType::Nearest);
            out.copy_from(
                &resized,
                x as u32 + (i as u32 % 4) * (64 + 12) + 585,
                y as u32 + (i as u32 - i as u32 % 4) / 4 * (62 + 12) + 12,
            )?;
        }

        image::imageops::overlay(
            &mut out,
            &default_assets.defender,
            x as i64 + 12 + 585,
            y as i64 + 156,
        );
        let defender_losses_str = format!("Losses: {}", army_display(w.defender_losses as f64));
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 12 + 32 + 585,
            y + 152,
            36.0,
            font,
            &defender_losses_str,
        );

        let title_lines = text_wrap(&w.name, font, 36.0, 290);
        for (i, line) in title_lines.into_iter().enumerate() {
            let line_width = drawing::text_size(36.0, font, &line).0;
            drawing::draw_text_mut(
                &mut out,
                Rgba::white(),
                x + 437 - line_width as i32 / 2,
                y + 12 + i as i32 * 40,
                36.0,
                font,
                &line,
            );
        }

        let date_span = format!(
            "{}-{}",
            w.start_date.year,
            w.end_date.unwrap_or(save.date).year
        );
        drawing::draw_text_mut(
            &mut out,
            Rgba::white(),
            x + 437 - drawing::text_size(36.0, font, &date_span).0 as i32 / 2,
            y + 115,
            36.0,
            font,
            &date_span,
        );

        match w.result {
            Some(WarResult::WhitePeace) => {
                image::imageops::overlay(
                    &mut out,
                    &default_assets.white_peace,
                    x as i64 + 437 - 32,
                    y as i64 + 140,
                );
            }
            Some(WarResult::AttackerVictory) => {
                image::imageops::overlay(
                    &mut out,
                    &default_assets.star,
                    x as i64 + 290,
                    y as i64 + 148,
                );
            }
            Some(WarResult::DefenderVictory) => {
                image::imageops::overlay(
                    &mut out,
                    &default_assets.star,
                    x as i64 + 12 + 585 - 48,
                    y as i64 + 148,
                );
            }
            None => {}
        }
    }

    // === DRAW DATE ===
    let date_str = format!("{:#}", save.date);
    let date_str_width = drawing::text_size(100.0, font, &date_str);
    drawing::draw_text_mut(
        &mut out,
        Rgba::white(),
        5177 - date_str_width.0 as i32 / 2,
        72,
        100.0,
        font,
        &date_str,
    );

    return Ok(out);
}
