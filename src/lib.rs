use std::{collections::HashMap, io::Cursor};

use ab_glyph::FontRef;
use bad_parser::SaveGame;
use base64::Engine;
use image::{Rgb, RgbImage, RgbaImage};
use map_parsers::{from_cp1252, read_definition_csv, FlagImages};
use stats_image::StatsImageIconAssets;
use wasm_bindgen::prelude::*;

mod bad_parser;
mod eu4_date;
mod eu4_map;
mod map_parsers;
mod new_parser;
mod stats_image;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub fn parse_eu4_save(text: &str) -> Result<JsValue, JsValue> {
    return SaveGame::bad_parser(text)
        .map(|save| serde_wasm_bindgen::to_value(&save).unwrap())
        .map_err(map_error);
}

fn map_error<E: ToString>(err: E) -> JsValue {
    return js_sys::Error::new(&err.to_string()).into();
}

struct AllAssets {
    icons: StatsImageIconAssets,
    map_definitions: HashMap<Rgb<u8>, u64>,
    wasteland: Vec<u64>,
    water: Vec<u64>,
    flags: FlagImages,
    base_template: RgbaImage,
    base_map: RgbImage,
}

struct Fetcher(reqwest::Client);
impl Fetcher {
    pub fn new() -> Self {
        return Fetcher(reqwest::Client::new());
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, JsValue> {
        return self.0.get(url).send().await.map_err(map_error);
    }

    /** Gets and throws an error if the status is an error code */
    pub async fn get_200(&self, url: &str) -> Result<reqwest::Response, JsValue> {
        return self.get(url).await?.error_for_status().map_err(map_error);
    }

    pub async fn get_image(
        &self,
        url: &str,
        format: image::ImageFormat,
    ) -> Result<image::DynamicImage, JsValue> {
        let response = self.get_200(url).await?;
        let bytes = response.bytes().await.map_err(map_error)?;
        return image::load(Cursor::new(bytes), format).map_err(map_error);
    }

    pub async fn get_with_encoding(&self, url: &str) -> Result<String, JsValue> {
        let response = self.get_200(url).await?;
        let bytes = response.bytes().await.map_err(map_error)?;
        return from_cp1252(Cursor::new(bytes)).map_err(map_error);
    }
}

impl AllAssets {
    async fn load() -> Result<AllAssets, JsValue> {
        let client = Fetcher::new();
        let window =
            web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
        let base_url = window.location().origin()? + &window.location().pathname()?;

        let army = client
            .get_image(
                &format!("{base_url}/resources/army.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let navy = client
            .get_image(
                &format!("{base_url}/resources/navy.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let development = client
            .get_image(
                &format!("{base_url}/resources/development.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let income = client
            .get_image(
                &format!("{base_url}/resources/income.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let attacker = client
            .get_image(
                &format!("{base_url}/resources/bodycount_attacker_button.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let defender = client
            .get_image(
                &format!("{base_url}/resources/bodycount_defender_button.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let star = client
            .get_image(
                &format!("{base_url}/resources/star.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let white_peace = client
            .get_image(
                &format!("{base_url}/resources/icon_peace.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();

        let csv_file_text = client
            .get_with_encoding(&format!("{base_url}/resources/vanilla/definition.csv"))
            .await?;
        let wasteland = client
            .get_with_encoding(&format!("{base_url}/resources/vanilla/wasteland.txt"))
            .await?
            .split_ascii_whitespace()
            .map(|p| p.parse::<u64>())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(map_error)?;
        let water = client
            .get_with_encoding(&format!("{base_url}/resources/vanilla/water.txt"))
            .await?
            .split_ascii_whitespace()
            .map(|p| p.parse::<u64>())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(map_error)?;

        let flagfiles_txt = client
            .get_with_encoding(&format!("{base_url}/resources/vanilla/flagfiles.txt"))
            .await?;
        let flagfiles_png = client
            .get_image(
                &format!("{base_url}/resources/vanilla/flagfiles.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();
        let flags = FlagImages::new(&flagfiles_txt, flagfiles_png);

        let base_template = client
            .get_image(
                &format!("{base_url}/resources/finalTemplate.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgba8();

        let base_map = client
            .get_image(
                &format!("{base_url}/resources/vanilla/provinces.png"),
                image::ImageFormat::Png,
            )
            .await?
            .to_rgb8();

        return Ok(AllAssets {
            icons: StatsImageIconAssets {
                army,
                navy,
                development,
                income,
                attacker,
                defender,
                star,
                white_peace,
            },
            map_definitions: read_definition_csv(&csv_file_text).map_err(map_error)?,
            wasteland,
            water,
            flags,
            base_template,
            base_map,
        });
    }
}

#[wasm_bindgen]
pub async fn render_stats_image(save: JsValue) -> Result<JsValue, JsValue> {
    let save: SaveGame = serde_wasm_bindgen::from_value(save)?;
    log!("Loading assets...");
    let assets = AllAssets::load().await?;
    let garamond =
        FontRef::try_from_slice(include_bytes!("../resources/GARA.TTF")).map_err(map_error)?;

    log!("Generating map...");
    let color_map = eu4_map::generate_map_colors_config(
        &assets.map_definitions,
        &assets.water,
        &assets.wasteland,
        &save,
    )
    .map_err(map_error)?;
    let base_map = eu4_map::make_base_map(&assets.base_map, &color_map);

    log!("Drawing borders...");
    let borders_config = eu4_map::generate_player_borders_config(&save);
    let map_image = eu4_map::apply_borders(&base_map, &borders_config);

    log!("Drawing stats...");

    let final_img = stats_image::make_final_image(
        &assets.base_template,
        &image::DynamicImage::ImageRgb8(map_image).to_rgba8(),
        &assets.flags,
        &garamond,
        &assets.icons,
        &save,
    )
    .map_err(map_error)?;

    let img = image::DynamicImage::ImageRgba8(final_img);

    let mut png_buffer: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_buffer), image::ImageFormat::Png)
        .map_err(map_error)?;
    return Ok(JsValue::from_str(
        &base64::engine::general_purpose::STANDARD.encode(png_buffer),
    ));
}
