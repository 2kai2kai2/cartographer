use std::io::Cursor;
use std::str::FromStr;

use anyhow::Context;
use base64::Engine;
use eu4::country_history::WarHistoryEvent;
use eu4::map_history::{ColorMapManager, SerializedColorMapManager};
use eu4::webgl::webgl_draw_map;
use pdx_parser_core::{eu4_save_parser, eu5_gamestate, stellaris_save_parser};
use pdx_parser_core::{raw_parser::RawPDXObject, EU4Date, Month};
use stats_core::eu5::EU5ParserStepGamestate;
use stats_core::{from_cp1252, EU4ParserStepText, GameSaveType, StellarisParserStepText};
use wasm_bindgen::prelude::*;

mod eu4;

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

/// Should take in a `UInt8Array`
#[wasm_bindgen]
pub fn parse_save_file(array: &[u8], filename: &str) -> Result<JsValue, JsValue> {
    let perf = web_sys::window().and_then(|window| window.performance());
    let game = GameSaveType::determine_from_filename(filename).ok_or(JsError::new(
        "Could not determine the game type from the filename.",
    ))?;
    match game {
        GameSaveType::EU4 => {
            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_preprocess_start");
            });
            let text = EU4ParserStepText::decode_from(array).map_err(map_error)?;

            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_raw_start");
            });
            let raw = text.parse().map_err(map_error)?;

            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_game_start");
            });
            let save = raw.parse().map_err(map_error)?;
            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_game_end");
            });
            drop(text);

            return Ok(serde_wasm_bindgen::to_value(&(game, save)).map_err(map_error)?);
        }
        GameSaveType::Stellaris => {
            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_preprocess_start");
            });
            let text = StellarisParserStepText::decode_from(array).map_err(map_error)?;

            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_raw_start");
            });
            let raw = text.parse().map_err(map_error)?;

            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_game_start");
            });
            let save = raw.parse().map_err(map_error)?;
            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_game_end");
            });
            drop(text);

            return Ok(serde_wasm_bindgen::to_value(&(game, save)).map_err(map_error)?);
        }
        GameSaveType::EU5 => {
            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_preprocess_start");
            });
            let text = EU5ParserStepGamestate::decompress_from(array).map_err(map_error)?;

            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_start");
            });
            let gamestate = text.parse().map_err(map_error)?;
            perf.as_ref().inspect(|perf| {
                let _ = perf.mark("parse_end");
            });
            return Ok(serde_wasm_bindgen::to_value(&(game, gamestate)).map_err(map_error)?);
        }
    }
}

fn map_error<E: ToString>(err: E) -> JsValue {
    return js_sys::Error::new(&err.to_string()).into();
}

struct WebFetcher {
    base_url: reqwest::Url,
    client: reqwest::Client,
}
impl WebFetcher {
    pub fn new(base_url: reqwest::Url) -> Self {
        return WebFetcher {
            base_url,
            client: reqwest::Client::new(),
        };
    }
}
impl stats_core::Fetcher for WebFetcher {
    async fn get(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        return self
            .client
            .get(self.base_url.join(path)?)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .context("While getting the body of the response.");
    }
}

#[wasm_bindgen]
pub async fn render_stats_image_eu4(save: JsValue) -> Result<JsValue, JsValue> {
    let save: eu4_save_parser::SaveGame = serde_wasm_bindgen::from_value(save)?;
    let window = web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
    let base_url = window.location().origin()? + &window.location().pathname()?;
    // typically `base_url == "https://2kai2kai2.github.io/cartographer"`

    let fetcher = WebFetcher::new(reqwest::Url::from_str(&base_url).map_err(map_error)?);
    log!("Rendering stats image...");
    let final_img = stats_core::eu4::render_stats_image(&fetcher, save)
        .await
        .map_err(map_error)?;

    log!("Converting to png...");
    let mut png_buffer: Vec<u8> = Vec::new();
    final_img
        .write_to(&mut Cursor::new(&mut png_buffer), image::ImageFormat::Png)
        .map_err(map_error)?;
    return Ok(JsValue::from_str(
        &base64::engine::general_purpose::STANDARD.encode(png_buffer),
    ));
}

#[wasm_bindgen]
pub async fn render_stats_image_eu5(gamestate: JsValue) -> Result<JsValue, JsValue> {
    let gamestate: eu5_gamestate::RawGamestate = serde_wasm_bindgen::from_value(gamestate)?;
    let window = web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
    let base_url = window.location().origin()? + &window.location().pathname()?;
    // typically `base_url == "https://2kai2kai2.github.io/cartographer"`

    let fetcher = WebFetcher::new(reqwest::Url::from_str(&base_url).map_err(map_error)?);
    log!("Rendering stats image...");
    let final_img = stats_core::eu5::render_stats_image(&fetcher, gamestate)
        .await
        .map_err(map_error)?;

    log!("Converting to png...");
    let mut png_buffer: Vec<u8> = Vec::new();
    final_img
        .write_to(&mut Cursor::new(&mut png_buffer), image::ImageFormat::Png)
        .map_err(map_error)?;
    return Ok(JsValue::from_str(
        &base64::engine::general_purpose::STANDARD.encode(png_buffer),
    ));
}

#[wasm_bindgen]
pub async fn render_stats_image_stellaris(save: JsValue) -> Result<JsValue, JsValue> {
    let save: stellaris_save_parser::SaveGame = serde_wasm_bindgen::from_value(save)?;
    let window = web_sys::window().ok_or::<JsValue>(JsError::new("Failed to get window").into())?;
    let base_url = window.location().origin()? + &window.location().pathname()?;
    // typically `base_url == "https://2kai2kai2.github.io/cartographer"`

    let fetcher = WebFetcher::new(reqwest::Url::from_str(&base_url).map_err(map_error)?);
    log!("Rendering stats image...");
    let final_img = stats_core::stellaris::render_stats_image_stellaris(&fetcher, save)
        .await
        .map_err(map_error)?;

    log!("Converting to png...");
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
        stats_core::eu4::decompress_eu4txt(save_file).map_err(map_error)?
    } else {
        return Err(JsError::new("Could not determine the EU4 save format").into());
    };
    let (_, save) = RawPDXObject::parse_object_inner(&save)
        .ok_or::<JsValue>(js_sys::Error::new("Failed to parse save file (at step 1)").into())?;

    log!("Loading assets...");
    let fetcher = WebFetcher::new(reqwest::Url::from_str(base_url).map_err(map_error)?);
    let assets = stats_core::eu4::assets::MapAssets::load(&fetcher, "vanilla")
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

    let fetcher = WebFetcher::new(reqwest::Url::from_str(base_url).map_err(map_error)?);
    let assets = stats_core::eu4::assets::MapAssets::load(&fetcher, "vanilla")
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
