use crate::stellaris::asset_loaders::StatsImageAssets;

use super::STELLARIS_MAP_IMAGE_SIZE;
use ab_glyph::Font;
use anyhow::{anyhow, Result};
use image::{imageops, GenericImageView, RgbImage, Rgba, RgbaImage};
use imageproc::{
    definitions::{HasBlack, HasWhite},
    drawing,
    point::Point,
    rect::Rect,
};
use pdx_parser_core::stellaris_save_parser::SaveGame;

const LINE_COLOR: Rgba<u8> = Rgba([49, 83, 71, 255]);
const END_COLOR: Rgba<u8> = Rgba([180, 202, 195, 255]);

fn signed_display_number(num: f64) -> String {
    let abs = num.abs();
    if abs < 10.0 {
        return format!("{num:+.1}");
    } else if abs < 1_000.0 {
        return format!("{num:+.0}");
    } else if abs < 10_000.0 {
        return format!("{:+.1}k", num / 1_000.0);
    } else if abs < 1_000_000.0 {
        return format!("{:+.0}k", num / 1_000.0);
    } else if abs < 10_000_000.0 {
        return format!("{:+.1}M", num / 1_000_000.0);
    } else {
        return format!("{:+.0}M", num / 1_000_000.0);
    }
}
fn unsigned_display_number(num: f64) -> String {
    let abs = num.abs();
    if abs < 10.0 {
        return format!("{num:.1}");
    } else if abs < 1_000.0 {
        return format!("{num:.0}");
    } else if abs < 10_000.0 {
        return format!("{:.1}k", num / 1_000.0);
    } else if abs < 1_000_000.0 {
        return format!("{:.0}k", num / 1_000.0);
    } else if abs < 10_000_000.0 {
        return format!("{:.1}M", num / 1_000_000.0);
    } else {
        return format!("{:.0}M", num / 1_000_000.0);
    }
}

/// Based on `Stellaris/gfx/interface/dividers/divider_300.dds`
///
/// Hard-coded because I can
///
/// ## Params
/// - `x` and `y` are the top-left
/// - `left_len` is the length before the diagonal bit
/// - `right_len` is the length after the diagonal bit
///
///
/// TODO: glow effect
fn draw_stellaris_divider(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    left_len: u32,
    right_len: u32,
    width: i32,
) {
    let diagonal_size = 5 * width;

    let point0 = (x, y + diagonal_size + width);
    let point1 = (x + left_len as i32, y + diagonal_size + width);
    let point2 = (x + left_len as i32 + diagonal_size + width, y);
    let point3 = (
        x + left_len as i32 + diagonal_size + right_len as i32 + width,
        y,
    );

    drawing::draw_antialiased_polygon_mut(
        img,
        &[
            Point::new(point0.0, point0.1),
            Point::new(point1.0, point1.1),
            Point::new(point2.0 - width / 2, point2.1),
            Point::new(point3.0, point3.1),
            Point::new(point3.0, point3.1 + width),
            Point::new(point2.0, point2.1 + width),
            Point::new(point1.0 + width / 2, point1.1 + width),
            Point::new(point0.0, point0.1 + width),
        ],
        LINE_COLOR,
        imageproc::pixelops::interpolate,
    );

    // Glowing ends
    drawing::draw_filled_rect_mut(
        img,
        Rect::at(point0.0, point0.1).of_size(width as u32, width as u32),
        END_COLOR,
    );
    drawing::draw_filled_rect_mut(
        img,
        Rect::at(point3.0 - width, point3.1).of_size(width as u32, width as u32),
        END_COLOR,
    );

    // diagonal 2
    drawing::draw_antialiased_polygon_mut(
        img,
        &[
            Point::new(point1.0 + width * 3, point1.1 + width),
            Point::new(point2.0 + width, point2.1 + width + width * 2),
            Point::new(point2.0 + width + width, point2.1 + width + width * 2),
            Point::new(point1.0 + width * 3 + width, point1.1 + width),
        ],
        LINE_COLOR,
        imageproc::pixelops::interpolate,
    );
}

pub fn make_final_image(
    map_image: &RgbImage,
    font: &impl Font,
    assets: &StatsImageAssets,
    save: &SaveGame,
) -> Result<RgbImage> {
    if map_image.dimensions() != (STELLARIS_MAP_IMAGE_SIZE, STELLARIS_MAP_IMAGE_SIZE) {
        return Err(anyhow!(
            "Expected map image to be {0}x{0}",
            STELLARIS_MAP_IMAGE_SIZE
        ));
    }

    let mut stats_img = RgbaImage::from_pixel(2048, 2048, Rgba::black());
    let screen_bg = imageops::resize(&assets.screen_bg, 562 * 3, 641 * 3, imageops::Triangle);
    const MARGIN_X: i64 = (2048 - 562 * 3) / 2;
    const MARGIN_Y: i64 = (2048 - 641 * 3) / 2;
    imageops::overlay(&mut stats_img, &screen_bg, MARGIN_X, MARGIN_Y);
    drawing::draw_text_mut(
        &mut stats_img,
        Rgba::white(),
        MARGIN_X as i32 + 40,
        MARGIN_Y as i32 + 40,
        80.0,
        font,
        save.date.to_string().as_str(),
    );
    draw_stellaris_divider(
        &mut stats_img,
        MARGIN_X as i32 + 40,
        MARGIN_Y as i32 + 80 + 40,
        500,
        300,
        3,
    );
    let mut countries: Vec<_> = save.player_nations().collect();
    countries.sort_by_key(|(_, c)| c.victory_rank);

    let mut stats_img = drawing::Blend(stats_img);
    for (i, (player_name, country)) in countries.into_iter().take(12).enumerate() {
        const PLAYER_BOX_WIDTH: i64 = 800;
        const PLAYER_BOX_HEIGHT: i64 = 200;
        const PLAYER_BOX_MARGIN: i64 = 24;
        let base_x = MARGIN_X
            + 7
            + PLAYER_BOX_MARGIN
            + (PLAYER_BOX_WIDTH + PLAYER_BOX_MARGIN) * (i as i64 / 6);
        let base_y = MARGIN_Y + 80 + 40 + 40 + (200 + PLAYER_BOX_MARGIN) * (i as i64 % 6);

        // box
        drawing::draw_filled_rect_mut(
            &mut stats_img,
            Rect::at(base_x as i32, base_y as i32)
                .of_size(PLAYER_BOX_WIDTH as u32, PLAYER_BOX_HEIGHT as u32),
            Rgba([0, 0, 0, 102]),
        );
        drawing::draw_hollow_rect_mut(
            &mut stats_img,
            Rect::at(base_x as i32, base_y as i32)
                .of_size(PLAYER_BOX_WIDTH as u32, PLAYER_BOX_HEIGHT as u32),
            LINE_COLOR,
        );
        stats_img
            .0
            .put_pixel(base_x as u32, base_y as u32, END_COLOR);
        stats_img.0.put_pixel(
            base_x as u32,
            (base_y + PLAYER_BOX_HEIGHT) as u32,
            END_COLOR,
        );
        stats_img
            .0
            .put_pixel((base_x + PLAYER_BOX_WIDTH) as u32, base_y as u32, END_COLOR);
        stats_img.0.put_pixel(
            (base_x + PLAYER_BOX_WIDTH) as u32,
            (base_y + PLAYER_BOX_HEIGHT) as u32,
            END_COLOR,
        );

        // flag
        // TODO
        drawing::draw_text_mut(
            &mut stats_img,
            Rgba::white(),
            (base_x + PLAYER_BOX_HEIGHT) as i32,
            (base_y + 16) as i32,
            48.0,
            font,
            &player_name,
        );
        drawing::draw_text_mut(
            &mut stats_img,
            Rgba::white(),
            (base_x + PLAYER_BOX_HEIGHT) as i32,
            (base_y + 16 + 48) as i32,
            30.0,
            font,
            "My Country Name",
        );

        const ICON_SIZE: i64 = 32;

        fn draw_balance_nums(
            img: &mut RgbaImage,
            x: i32,
            y: i32,
            font: &impl Font,
            icon: image::SubImage<&RgbaImage>,
            amount: (f64, f64),
        ) {
            let balance_color = if amount.0 < amount.1 {
                Rgba([255, 40, 40, 255])
            } else {
                Rgba([0, 255, 0, 255])
            };

            imageops::overlay(img, &*icon, x as i64, y as i64);

            let balance_text_scale = ICON_SIZE as f32 * 0.75;
            let balance_text = signed_display_number(amount.0 - amount.1);
            let balance_text_size = drawing::text_size(balance_text_scale, font, &balance_text);
            drawing::draw_text_mut(
                img,
                balance_color,
                x + ICON_SIZE as i32,
                y + (ICON_SIZE as f32 * 0.125) as i32,
                balance_text_scale,
                font,
                &balance_text,
            );
            drawing::draw_text_mut(
                img,
                Rgba([0, 255, 0, 255]),
                x + ICON_SIZE as i32 + 8 + balance_text_size.0 as i32,
                y,
                ICON_SIZE as f32 / 2.0,
                font,
                &signed_display_number(amount.0),
            );
            drawing::draw_text_mut(
                img,
                Rgba([255, 40, 40, 255]),
                x + ICON_SIZE as i32 + 8 + balance_text_size.0 as i32,
                y + 16,
                ICON_SIZE as f32 / 2.0,
                font,
                &signed_display_number(-amount.1),
            );
        }

        let population = country.num_sapient_pops;
        imageops::overlay(
            &mut stats_img.0,
            &*assets
                .resource_icons
                .view(0, 0, ICON_SIZE as u32, ICON_SIZE as u32),
            base_x + PLAYER_BOX_HEIGHT,
            base_y + 16 + 32 + 50,
        );
        drawing::draw_text_mut(
            &mut stats_img.0,
            Rgba::white(),
            (base_x + PLAYER_BOX_HEIGHT + ICON_SIZE) as i32,
            (base_y + 16 + 32 + 50) as i32 + (ICON_SIZE as f32 * 0.125) as i32,
            ICON_SIZE as f32 * 0.75,
            font,
            &unsigned_display_number(population as f64),
        );

        let energy = country.get_resource_balance("energy");
        draw_balance_nums(
            &mut stats_img.0,
            (base_x + PLAYER_BOX_HEIGHT + (ICON_SIZE + 112)) as i32,
            (base_y + 16 + 32 + 50) as i32,
            font,
            assets
                .resource_icons
                .view(0, ICON_SIZE as u32, ICON_SIZE as u32, ICON_SIZE as u32),
            energy,
        );

        let minerals = country.get_resource_balance("minerals");
        draw_balance_nums(
            &mut stats_img.0,
            (base_x + PLAYER_BOX_HEIGHT + (ICON_SIZE + 112) * 2) as i32,
            (base_y + 16 + 32 + 50) as i32,
            font,
            assets
                .resource_icons
                .view(0, ICON_SIZE as u32 * 2, ICON_SIZE as u32, ICON_SIZE as u32),
            minerals,
        );

        let food = country.get_resource_balance("food");
        draw_balance_nums(
            &mut stats_img.0,
            (base_x + PLAYER_BOX_HEIGHT + (ICON_SIZE + 112) * 3) as i32,
            (base_y + 16 + 32 + 50) as i32,
            font,
            assets
                .resource_icons
                .view(0, ICON_SIZE as u32 * 3, ICON_SIZE as u32, ICON_SIZE as u32),
            food,
        );

        let consumer_goods = country.get_resource_balance("consumer_goods");
        draw_balance_nums(
            &mut stats_img.0,
            (base_x + PLAYER_BOX_HEIGHT) as i32,
            (base_y + 16 + 32 + 50 + 48) as i32,
            font,
            assets
                .resource_icons
                .view(0, ICON_SIZE as u32 * 4, ICON_SIZE as u32, ICON_SIZE as u32),
            consumer_goods,
        );

        let alloys = country.get_resource_balance("alloys");
        draw_balance_nums(
            &mut stats_img.0,
            (base_x + PLAYER_BOX_HEIGHT + (ICON_SIZE + 112)) as i32,
            (base_y + 16 + 32 + 50 + 48) as i32,
            font,
            assets
                .resource_icons
                .view(0, ICON_SIZE as u32 * 5, ICON_SIZE as u32, ICON_SIZE as u32),
            alloys,
        );

        let unity = country.get_resource_balance("unity");
        imageops::overlay(
            &mut stats_img.0,
            &*assets.resource_icons.view(
                0,
                ICON_SIZE as u32 * 6,
                ICON_SIZE as u32,
                ICON_SIZE as u32,
            ),
            base_x + PLAYER_BOX_HEIGHT + (ICON_SIZE + 112) * 2,
            base_y + 16 + 32 + 50 + 48,
        );
        drawing::draw_text_mut(
            &mut stats_img.0,
            Rgba::white(),
            (base_x + PLAYER_BOX_HEIGHT + ICON_SIZE + (ICON_SIZE + 112) * 2) as i32,
            (base_y + 16 + 32 + 50 + 48) as i32 + (ICON_SIZE as f32 * 0.125) as i32,
            ICON_SIZE as f32 * 0.75,
            font,
            &signed_display_number(unity.0 - unity.1),
        );

        let physics_research = country.get_resource_balance("physics_research");
        let society_research = country.get_resource_balance("society_research");
        let engineering_research = country.get_resource_balance("engineering_research");
        let research = (
            physics_research.0 + society_research.0 + engineering_research.0,
            physics_research.1 + society_research.1 + engineering_research.1,
        );
        imageops::overlay(
            &mut stats_img.0,
            &*assets.resource_icons.view(
                0,
                ICON_SIZE as u32 * 7,
                ICON_SIZE as u32,
                ICON_SIZE as u32,
            ),
            base_x + PLAYER_BOX_HEIGHT + (ICON_SIZE + 112) * 3,
            base_y + 16 + 32 + 50 + 48,
        );
        drawing::draw_text_mut(
            &mut stats_img.0,
            Rgba::white(),
            (base_x + PLAYER_BOX_HEIGHT + ICON_SIZE + (ICON_SIZE + 112) * 3) as i32,
            (base_y + 16 + 32 + 50 + 48) as i32 + (ICON_SIZE as f32 * 0.125) as i32,
            ICON_SIZE as f32 * 0.75,
            font,
            &signed_display_number(research.0 - research.1),
        );
    }

    let mut img_out = RgbaImage::new(STELLARIS_MAP_IMAGE_SIZE + 2048, 2048);
    imageops::overlay(
        &mut img_out,
        &stats_img.0,
        STELLARIS_MAP_IMAGE_SIZE as i64,
        0,
    );
    imageops::overlay(
        &mut img_out,
        &image::DynamicImage::ImageRgb8(map_image.to_owned()).to_rgba8(),
        0,
        0,
    );
    let img_out = image::DynamicImage::ImageRgba8(img_out).to_rgb8();
    return Ok(img_out);
}
