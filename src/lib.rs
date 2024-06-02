use std::io::Cursor;

use ab_glyph::FontRef;
use base64::Engine;
use map_parsers::from_cp1252;
use save_parser::SaveGame;
use stats_image::StatsImageDefaultAssets;
use wasm_bindgen::prelude::*;

use crate::map_parsers::MapAssets;

mod eu4_map;
mod map_parsers;
mod save_parser;
mod stats_image;
mod map_history;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

fn decompress_eu4txt(array: &[u8]) -> anyhow::Result<String> {
    let mut cursor = Cursor::new(array);
    let mut unzipper = zip::read::ZipArchive::new(&mut cursor)?;

    let unzipped_meta = unzipper.by_name("meta")?;
    let meta = from_cp1252(unzipped_meta)?;

    let unzipped_gamestate = unzipper.by_name("gamestate")?;
    let gamestate = from_cp1252(unzipped_gamestate)?;
    return Ok(meta + "\n" + &gamestate);
}

/// Should take in a `UInt8Array`
#[wasm_bindgen]
pub fn parse_eu4_save(array: &[u8]) -> Result<JsValue, JsValue> {
    if array.starts_with("EU4txt".as_bytes()) {
        log!("Detected uncompressed save file");
        let text = from_cp1252(array).map_err(map_error)?;

        return SaveGame::new_parser(&text)
            .map(|save| serde_wasm_bindgen::to_value(&save).unwrap())
            .ok_or(js_sys::Error::new("Failed to parse save file").into());
    } else if array.starts_with("PK\x03\x04".as_bytes()) {
        log!("Detected compressed file");
        let text = decompress_eu4txt(array).map_err(map_error)?;

        return SaveGame::new_parser(&text)
            .map(|save| serde_wasm_bindgen::to_value(&save).unwrap())
            .ok_or(js_sys::Error::new("Failed to parse save file").into());
    }
    return Err(JsError::new("Could not determine the EU4 save format").into());
}

fn map_error<E: ToString>(err: E) -> JsValue {
    return js_sys::Error::new(&err.to_string()).into();
}

struct Fetcher(reqwest::Client);
impl Fetcher {
    pub fn new() -> Self {
        return Fetcher(reqwest::Client::new());
    }

    pub async fn get(&self, url: &str) -> anyhow::Result<reqwest::Response> {
        return self.0.get(url).send().await.map_err(anyhow::Error::msg);
    }

    /** Gets and throws an error if the status is an error code */
    pub async fn get_200(&self, url: &str) -> anyhow::Result<reqwest::Response> {
        return self
            .get(url)
            .await?
            .error_for_status()
            .map_err(anyhow::Error::msg);
    }

    pub async fn get_image(
        &self,
        url: &str,
        format: image::ImageFormat,
    ) -> anyhow::Result<image::DynamicImage> {
        let response = self.get_200(url).await.map_err(anyhow::Error::msg)?;
        let bytes = response.bytes().await?;
        return image::load(Cursor::new(bytes), format).map_err(anyhow::Error::msg);
    }

    pub async fn get_with_encoding(&self, url: &str) -> anyhow::Result<String> {
        let response = self.get_200(url).await.map_err(anyhow::Error::msg)?;
        let bytes = response.bytes().await.map_err(anyhow::Error::msg)?;
        return from_cp1252(Cursor::new(bytes)).map_err(anyhow::Error::msg);
    }
}

#[wasm_bindgen]
pub async fn render_stats_image(save: JsValue) -> Result<JsValue, JsValue> {
    let save: SaveGame = serde_wasm_bindgen::from_value(save)?;
    log!("Loading assets...");
    let window = web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
    let base_url = window.location().origin()? + &window.location().pathname()?;

    let url_default_assets = format!("{base_url}/resources");
    let url_map_assets = format!("{base_url}/resources/vanilla");
    let (default_assets, map_assets) = futures::try_join!(
        StatsImageDefaultAssets::load(&url_default_assets),
        MapAssets::load(&url_map_assets),
    )
    .map_err(map_error)?;

    let garamond =
        FontRef::try_from_slice(include_bytes!("../resources/GARA.TTF")).map_err(map_error)?;

    log!("Generating map...");
    let color_map = eu4_map::generate_save_map_colors_config(
        map_assets.provinces_len,
        &map_assets.water,
        &map_assets.wasteland,
        &save,
    );
    let base_map = eu4_map::make_base_map(&map_assets.base_map, &color_map);

    log!("Drawing borders...");
    let borders_config = eu4_map::generate_player_borders_config(&save);
    let map_image = eu4_map::apply_borders(&base_map, &borders_config);

    log!("Drawing stats...");

    let final_img = stats_image::make_final_image(
        &image::DynamicImage::ImageRgb8(map_image).to_rgba8(),
        &map_assets.flags,
        &garamond,
        &default_assets,
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
