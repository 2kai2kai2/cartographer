use std::collections::HashMap;

use eu4_parser_core::{
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
    EU4Date,
};
use image::Rgb;

use crate::{
    eu4_map::{generate_map_colors_config, make_base_map},
    map_parsers::MapAssets,
    save_parser::SaveGame,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ProvinceHistoryEvent {
    Owner(String),
    Controller(String),
    Religion(String),
    AddCore(String),
    RemoveCore(String),
    AddClaim(String),
    RemoveClaim(String),
    // buildings
    // dev
    // advisors
    // hre
}
impl ProvinceHistoryEvent {
    /// Should be passed the `history` object for a province
    pub fn extract_events<'a>(object: &RawEU4Object<'a>) -> Vec<(EU4Date, ProvinceHistoryEvent)> {
        return object
            .iter_all_KVs()
            .filter_map(|(k, v)| Some((k.as_date()?, v.as_object()?)))
            .flat_map(|(date, obj)| {
                obj.iter_all_KVs()
                    .filter_map(move |(RawEU4Scalar(ev), val)| match (ev, val) {
                        (&"owner", RawEU4Value::Scalar(tag)) => {
                            Some((date, ProvinceHistoryEvent::Owner(tag.as_string())))
                        }
                        (&"controller", RawEU4Value::Object(controller_obj)) => Some((
                            date,
                            ProvinceHistoryEvent::Controller(
                                controller_obj.get_first_as_string("tag")?,
                            ),
                        )),
                        (&"religion", RawEU4Value::Scalar(religion)) => {
                            Some((date, ProvinceHistoryEvent::Religion(religion.as_string())))
                        }
                        (&"add_core", RawEU4Value::Scalar(add_core)) => {
                            Some((date, ProvinceHistoryEvent::AddCore(add_core.as_string())))
                        }
                        (&"remove_core", RawEU4Value::Scalar(remove_core)) => Some((
                            date,
                            ProvinceHistoryEvent::RemoveCore(remove_core.as_string()),
                        )),
                        (&"add_claim", RawEU4Value::Scalar(add_claim)) => {
                            Some((date, ProvinceHistoryEvent::AddClaim(add_claim.as_string())))
                        }
                        (&"remove_claim", RawEU4Value::Scalar(remove_claim)) => Some((
                            date,
                            ProvinceHistoryEvent::RemoveClaim(remove_claim.as_string()),
                        )),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
    }

    pub fn combine_events(
        provinces: Vec<(u64, Vec<(EU4Date, ProvinceHistoryEvent)>)>,
    ) -> HashMap<EU4Date, Vec<(u64, ProvinceHistoryEvent)>> {
        let mut out: HashMap<EU4Date, Vec<(u64, ProvinceHistoryEvent)>> = HashMap::new();
        for (id, events) in provinces {
            for (date, event) in events {
                out.entry(date).or_default().push((id, event));
            }
        }
        return out;
    }
}

pub fn make_map_frames(
    assets: &MapAssets,
    history: HashMap<EU4Date, Vec<(u64, ProvinceHistoryEvent)>>,
    save: &SaveGame,
) -> Vec<image::Frame> {
    struct ProvinceState {
        owner: String,
    }

    let mut frames: Vec<image::Frame> = Vec::new();
    let mut current_state: HashMap<u64, ProvinceState> = HashMap::new();

    let start_date = EU4Date {
        year: 1444,
        month: eu4_parser_core::Month::NOV,
        day: 11,
    };
    let end_date = EU4Date {
        year: 1446,
        month: eu4_parser_core::Month::DEC,
        day: 1,
    };

    for date in EU4Date::iter_range_inclusive(start_date, end_date) {
        let Some(today_events) = history.get(&date) else {
            // increase the time of the previous frame
            if let Some(prev_frame) = frames.pop() {
                let delay = prev_frame.delay().numer_denom_ms();
                frames.push(image::Frame::from_parts(
                    prev_frame.buffer().clone(),
                    prev_frame.left(),
                    prev_frame.top(),
                    image::Delay::from_numer_denom_ms(delay.0 + delay.1 * 1000, delay.1),
                ))
            }
            continue;
        };

        for (id, event) in today_events {
            match event {
                ProvinceHistoryEvent::Owner(owner) => {
                    current_state.insert(
                        *id,
                        ProvinceState {
                            owner: owner.to_string(),
                        },
                    );
                }
                _ => {}
            }
        }

        let tag_colors: HashMap<_, _> = save
            .all_nations
            .iter()
            .map(|(tag, nation)| (tag, Rgb(nation.map_color)))
            .collect();
        let color_map = generate_map_colors_config(
            assets.provinces_len,
            &assets.water,
            &assets.wasteland,
            |id| {
                current_state
                    .get(&id)
                    .map(|province| province.owner.to_string())
            },
            |tag| tag_colors.get(&tag).map(Rgb::clone),
        );
        let img = make_base_map(&assets.base_map, &color_map);
        frames.push(image::Frame::from_parts(
            image::DynamicImage::from(img).to_rgba8(),
            0,
            0,
            image::Delay::from_numer_denom_ms(1000, 1),
        ))
    }

    return frames;
}

#[cfg(test)]
mod tests {
    use eu4_parser_core::raw_parser;

    use crate::{
        map_parsers::{from_cp1252, MapAssets},
        save_parser::SaveGame,
    };

    use super::{make_map_frames, ProvinceHistoryEvent};

    #[test]
    pub fn asdf() {
        print!("Loading...");
        let start_time = std::time::Instant::now();
        let filetext = from_cp1252(
            include_bytes!("/Users/oritak/Downloads/mp_Portugal1567_05_11.eu4").as_slice(),
        )
        .unwrap();
        let (_, raw_parsed) = raw_parser::RawEU4Object::parse_object_inner(&filetext).unwrap();
        let save = SaveGame::new_parser(&filetext).unwrap();

        let province_histories = raw_parsed
            .get_first_obj("provinces")
            .unwrap()
            .iter_all_KVs()
            .filter_map(|(k, v)| {
                Some((
                    k.as_int()?.abs() as u64,
                    ProvinceHistoryEvent::extract_events(v.as_object()?.get_first_obj("history")?),
                ))
            })
            .collect();
        let combined_history = ProvinceHistoryEvent::combine_events(province_histories);

        let assets = MapAssets::new(
            &from_cp1252(include_bytes!("../resources/vanilla/definition.csv").as_slice()).unwrap(),
            &include_str!("../resources/vanilla/wasteland.txt"),
            &include_str!("../resources/vanilla/water.txt"),
            &include_str!("../resources/vanilla/flagfiles.txt"),
            image::load_from_memory(include_bytes!("../resources/vanilla/flagfiles.png"))
                .unwrap()
                .into_rgba8(),
            image::load_from_memory(include_bytes!("../resources/vanilla/provinces.png"))
                .unwrap()
                .into_rgb8(),
        )
        .unwrap();
        println!(
            " ({}ms.)",
            std::time::Instant::now()
                .duration_since(start_time)
                .as_millis()
        );

        print!("Generating frames...");
        let start_time = std::time::Instant::now();
        let frames = make_map_frames(&assets, combined_history, &save);
        println!(
            " ({}ms.)",
            std::time::Instant::now()
                .duration_since(start_time)
                .as_millis()
        );

        print!("Writing frames...");
        let start_time = std::time::Instant::now();
        let mut image = std::fs::File::create("test.gif").unwrap();
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut image);
        encoder.encode_frames(frames).unwrap();
        println!(
            " ({}ms.)",
            std::time::Instant::now()
                .duration_since(start_time)
                .as_millis()
        );
    }
}
