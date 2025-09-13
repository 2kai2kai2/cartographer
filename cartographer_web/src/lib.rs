use std::io::{Cursor, Read};

use ab_glyph::FontRef;
use base64::Engine;
use eu4::country_history::WarHistoryEvent;
use eu4::map_history::{ColorMapManager, SerializedColorMapManager};
use eu4::map_parsers::from_cp1252;
use eu4::stats_image::StatsImageDefaultAssets;
use eu4::webgl::webgl_draw_map;
use image::Rgb;
use pdx_parser_core::{eu4_save_parser, stellaris_save_parser};
use pdx_parser_core::{raw_parser::RawPDXObject, EU4Date, Month};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod eu4;
mod stellaris;

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

fn decompress_stellaris(array: &[u8]) -> anyhow::Result<String> {
    let mut cursor = Cursor::new(array);
    let mut unzipper = zip::read::ZipArchive::new(&mut cursor)?;

    // let mut unzipped_meta = unzipper.by_name("meta")?;
    // let mut meta = String::new();
    // unzipped_meta.read_to_string(&mut meta)?;

    let mut unzipped_gamestate = unzipper.by_name("gamestate")?;
    let mut gamestate = String::new();
    unzipped_gamestate.read_to_string(&mut gamestate)?;
    return Ok(gamestate);
}

#[derive(Serialize, Deserialize)]
pub enum GameSaveType {
    // /// `.ck3` extension
    // CK3,
    /// `.eu4` extension
    EU4,
    // /// TBD extension
    // EU5,
    /// `.sav` extension
    Stellaris,
    // /// `.v3` extension
    // Victoria3,
}

/// Should take in a `UInt8Array`
#[wasm_bindgen]
pub fn parse_save_file(array: &[u8], filename: &str) -> Result<JsValue, JsValue> {
    // Need to figure out what game this file is for
    let filename_lower = filename.to_ascii_lowercase();
    if filename_lower.ends_with(".eu4") {
        let save = if array.starts_with("EU4txt".as_bytes()) {
            log!("Detected uncompressed eu4 save file");
            from_cp1252(array).map_err(map_error)?
        } else if array.starts_with("PK\x03\x04".as_bytes()) {
            log!("Detected compressed file");
            decompress_eu4txt(array).map_err(map_error)?
        } else {
            return Err(JsError::new(
                "Initial check shows file does not seem to be a valid eu4 format",
            )
            .into());
        };
        let (_, save) = RawPDXObject::parse_object_inner(&save)
            .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 1)").into())?;
        let save = eu4_save_parser::SaveGame::new_parser(&save)
            .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 2)").into())?;
        return serde_wasm_bindgen::to_value(&(GameSaveType::EU4, save)).map_err(map_error);
    } else if filename_lower.ends_with(".sav") {
        // seems like all stellaris saves are compressed
        if !array.starts_with("PK\x03\x04".as_bytes()) {
            return Err(
                JsError::new("Stellaris save was not a proper zip-compressed file.").into(),
            );
        }
        let save = decompress_stellaris(array).map_err(map_error)?;
        let (_, save) = RawPDXObject::parse_object_inner(&save)
            .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 1)").into())?;
        let save = stellaris_save_parser::SaveGame::new_parser(&save).map_err(map_error)?;

        return serde_wasm_bindgen::to_value(&(GameSaveType::Stellaris, save)).map_err(map_error);
    } else {
        // TODO: try to figure it out from context.
        return Err(JsError::new(
            "Could not determine the save format. Did you change the file extension?",
        )
        .into());
    }
}

fn map_error<E: ToString>(err: E) -> JsValue {
    return js_sys::Error::new(&err.to_string()).into();
}

struct Fetcher(reqwest::Client);
impl Fetcher {
    pub fn new() -> Self {
        return Fetcher(reqwest::Client::new());
    }

    pub async fn get(&self, url: &str) -> reqwest::Result<reqwest::Response> {
        return self.0.get(url).send().await;
    }

    /** Gets and throws an error if the status is an error code */
    pub async fn get_200(&self, url: &str) -> reqwest::Result<reqwest::Response> {
        return self.get(url).await?.error_for_status();
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

    pub async fn get_utf8(&self, url: &str) -> reqwest::Result<String> {
        let response = self.get_200(url).await?;
        return response.text().await;
    }
}

#[wasm_bindgen]
pub async fn render_stats_image_eu4(save: JsValue) -> Result<JsValue, JsValue> {
    let save: eu4_save_parser::SaveGame = serde_wasm_bindgen::from_value(save)?;
    log!("Loading assets...");
    let window = web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
    let base_url = window.location().origin()? + &window.location().pathname()?;

    let url_default_assets = base_url.clone();
    log!("Detected game mod is {}", save.game_mod.id());
    let url_map_assets = format!("{base_url}/eu4/{}", save.game_mod.id());
    let (default_assets, map_assets) = futures::try_join!(
        StatsImageDefaultAssets::load(&url_default_assets),
        eu4::map_parsers::MapAssets::load(&url_map_assets),
    )
    .map_err(map_error)?;

    let garamond =
        FontRef::try_from_slice(include_bytes!("../resources/eu4/GARA.TTF")).map_err(map_error)?;

    log!("Generating map...");
    let color_map = eu4_map_core::generate_save_map_colors_config(
        map_assets.provinces_len,
        &map_assets.water,
        &map_assets.wasteland,
        &save,
    );
    let base_map = eu4_map_core::make_base_map(&map_assets.base_map, &color_map);

    log!("Drawing borders...");
    let borders_config = eu4_map_core::generate_player_borders_config(&save);
    let map_image = eu4_map_core::apply_borders(&base_map, &borders_config);

    log!("Drawing stats...");

    let final_img = eu4::stats_image::make_final_image(
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
pub async fn render_stats_image_stellaris(save: JsValue) -> Result<JsValue, JsValue> {
    let save: stellaris_save_parser::SaveGame = serde_wasm_bindgen::from_value(save)?;
    log!("Loading assets...");
    let window = web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
    let base_url = window.location().origin()? + &window.location().pathname()?;

    let url_default_assets = format!("{base_url}/stellaris");
    let url_map_assets = format!("{base_url}/stellaris/vanilla");
    let (map_assets, stats_assets) = futures::try_join!(
        stellaris::asset_loaders::MapAssets::load(&url_map_assets),
        stellaris::asset_loaders::StatsImageAssets::load(&url_default_assets),
    )
    .map_err(map_error)?;

    let jura = FontRef::try_from_slice(stellaris::JURA_MEDIUM_TTF).map_err(map_error)?;

    log!("Generating map...");
    let map_image = image::RgbImage::from_pixel(
        stellaris::STELLARIS_MAP_IMAGE_SIZE,
        stellaris::STELLARIS_MAP_IMAGE_SIZE,
        Rgb([0, 0, 0]),
    );
    let map_image = stellaris_map_core::draw_political_map(map_image, &save, &map_assets.colors);
    let map_image = stellaris_map_core::draw_hyperlanes(map_image, &save);
    let map_image = stellaris_map_core::draw_systems(map_image, &save);

    log!("Drawing stats...");

    let final_img = stellaris::stats_image::make_final_image(
        &map_image,
        &jura,
        &stats_assets,
        &save,
        &map_assets.colors,
    )
    .map_err(map_error)?;

    let mut png_buffer: Vec<u8> = Vec::new();
    final_img
        .write_to(&mut Cursor::new(&mut png_buffer), image::ImageFormat::Png)
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
    let (_, save) = RawPDXObject::parse_object_inner(&save)
        .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 1)").into())?;

    log!("Loading assets...");
    let url_map_assets = format!("{base_url}/../vanilla");
    let assets = eu4::map_parsers::MapAssets::load(&url_map_assets)
        .await
        .map_err(map_error)?;

    let province_history = eu4::map_history::make_combined_events(&save);
    let country_history = eu4::country_history::make_combined_events(&save);
    let war_history = WarHistoryEvent::make_war_events(&save)
        .map_err::<JsValue, _>(|_| JsError::new("Failed to parse war events").into())?;
    let save = eu4_save_parser::SaveGame::new_parser(&save)
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
    let url_map_assets = format!("{base_url}/../vanilla");
    let assets = eu4::map_parsers::MapAssets::load(&url_map_assets)
        .await
        .map_err(map_error)?;

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
                let Some(frame) = history.get_date(&date) else {
                    return Err(JsError::new("Unable to resolve the map state at this date. It may be outside the game's timespan.").into());
                };
                current_date = date;

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
