use std::io::Cursor;

use ab_glyph::FontRef;
use base64::Engine;
use country_history::WarHistoryEvent;
use eu4_parser_core::{raw_parser::RawEU4Object, EU4Date, Month};
use map_history::{ColorMapManager, SerializedColorMapManager};
use map_parsers::from_cp1252;
use save_parser::SaveGame;
use stats_image::StatsImageDefaultAssets;
use wasm_bindgen::prelude::*;
use webgl::webgl_draw_map;

use crate::map_parsers::MapAssets;

mod country_history;
mod eu4_map;
mod map_history;
mod map_parsers;
mod save_parser;
mod stats_image;
mod webgl;

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
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
    let save = if array.starts_with("EU4txt".as_bytes()) {
        log!("Detected uncompressed save file");
        from_cp1252(array).map_err(map_error)?
    } else if array.starts_with("PK\x03\x04".as_bytes()) {
        log!("Detected compressed file");
        decompress_eu4txt(array).map_err(map_error)?
    } else {
        return Err(JsError::new("Could not determine the EU4 save format").into());
    };
    let (_, save) = RawEU4Object::parse_object_inner(&save)
        .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 1)").into())?;
    return SaveGame::new_parser(&save)
        .map(|save| serde_wasm_bindgen::to_value(&save).unwrap())
        .ok_or(js_sys::Error::new("Failed to parse save file (at step 2)").into());
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

#[wasm_bindgen]
pub async fn generate_map_history(save_file: &[u8], base_url: &str) -> Result<String, JsValue> {
    let save = if save_file.starts_with("EU4txt".as_bytes()) {
        log!("Detected uncompressed save file");
        from_cp1252(save_file).map_err(map_error)?
    } else if save_file.starts_with("PK\x03\x04".as_bytes()) {
        log!("Detected compressed file");
        decompress_eu4txt(save_file).map_err(map_error)?
    } else {
        return Err(JsError::new("Could not determine the EU4 save format").into());
    };
    let (_, save) = RawEU4Object::parse_object_inner(&save)
        .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 1)").into())?;

    log!("Loading assets...");
    let url_map_assets = format!("{base_url}/../resources/vanilla");
    let assets = MapAssets::load(&url_map_assets).await.map_err(map_error)?;

    let province_history = map_history::make_combined_events(&save);
    let country_history = country_history::make_combined_events(&save);
    let war_history = WarHistoryEvent::make_war_events(&save)
        .map_err::<JsValue, _>(|_| JsError::new("Failed to parse war events").into())?;
    let save = SaveGame::new_parser(&save)
        .ok_or::<JsValue>(JsError::new("Failed to parse save file (at step 2)").into())?;
    let history = ColorMapManager::new(
        &assets,
        &province_history,
        &country_history,
        &war_history,
        &save,
        EU4Date::new(1444, Month::NOV, 11).unwrap(),
        save.date,
    );

    return serde_json::to_string(&SerializedColorMapManager::encode(&history))
        .map_err(|err| JsError::new(&err.to_string()).into());
}

#[wasm_bindgen]
pub async fn do_webgl(history: &str, base_url: &str) -> Result<JsValue, JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    log!("Loading assets...");
    let url_map_assets = format!("{base_url}/../resources/vanilla");
    let assets = MapAssets::load(&url_map_assets).await.map_err(map_error)?;

    let history = serde_json::from_str::<SerializedColorMapManager>(history)
        .map_err::<JsValue, _>(|err| JsError::new(&err.to_string()).into())?
        .decode(&assets)
        .map_err::<JsValue, _>(|err| JsError::new(&err.to_string()).into())?;

    let mut current_date = history.start_date;
    let mut current_frame = history
        .get_date(&current_date)
        .ok_or::<JsValue>(JsError::new("Could not get the map state at 1444.11.11").into())?;
    let callback = webgl_draw_map(canvas, assets)?;
    log!("Made callback");

    // if no date is specified, it just goes to the next one
    // if it is specified, we will try to resolve it, but this may be slower.
    return Ok(
        Closure::new(move |date: Option<String>| -> Result<String, JsValue> {
            if let Some(date) = date {
                let Ok(date) = date.parse::<EU4Date>() else {
                    return Err(JsError::new("Invalid date.").into());
                };

                current_date = date;
                let Some(frame) = history.get_date(&date) else {
                    return Err(JsError::new("Unable to resolve the map state at this date. It may be outside the game's timespan.").into());
                };

                current_frame = frame;
                log!("{current_date}");
            } else {
                if current_date > history.end_date {
                    return Ok(current_date.to_string());
                }

                history.apply_diffs(&current_date, &mut current_frame);
            }

            callback(&current_frame.0, &current_frame.1);
            let ret = current_date.to_string();
            current_date = current_date.tomorrow();
            return Ok(ret);
        })
        .into_js_value(),
    );
}
