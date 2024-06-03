use std::collections::HashMap;

use eu4_parser_core::{
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
    EU4Date,
};
use image::Rgb;

use crate::{
    eu4_map::{generate_map_colors_config, make_base_map, UNCLAIMED_COLOR},
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

struct ProvinceState {
    owner: String,
}

fn update_map_state(
    current_state: &mut HashMap<u64, ProvinceState>,
    events: &Vec<(u64, ProvinceHistoryEvent)>,
) {
    for (id, event) in events {
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
}

/// all I-frames, as in every frame has its data in full rather than diffed
pub fn all_i_frame_color_maps<'a>(
    assets: &'a MapAssets,
    history: &'a HashMap<EU4Date, Vec<(u64, ProvinceHistoryEvent)>>,
    save: &'a SaveGame,
    start_date: EU4Date,
    end_date: EU4Date,
) -> impl Iterator<Item = (EU4Date, Vec<Rgb<u8>>)> + 'a {
    let tag_colors: HashMap<_, _> = save
        .all_nations
        .iter()
        .map(|(tag, nation)| (tag, Rgb(nation.map_color)))
        .collect();
    let mut prev_map = generate_map_colors_config(
        assets.provinces_len,
        &assets.water,
        &assets.wasteland,
        |_| None,
        |tag| tag_colors.get(&tag).map(Rgb::to_owned),
    );

    return EU4Date::iter_range_inclusive(start_date, end_date).map(move |date| {
        let Some(events) = history.get(&date) else {
            return (date, prev_map.clone());
        };
        for (id, event) in events {
            match event {
                ProvinceHistoryEvent::Owner(tag) => {
                    prev_map[*id as usize] =
                        tag_colors.get(tag).unwrap_or(&UNCLAIMED_COLOR).clone();
                }
                _ => {}
            }
        }
        return (date, prev_map.clone());
    });
}

#[cfg(test)]
mod tests {
    use eu4_parser_core::{raw_parser, EU4Date};
    use image::GenericImage;

    use crate::{
        eu4_map::make_base_map,
        map_history::all_i_frame_color_maps,
        map_parsers::{from_cp1252, MapAssets},
        save_parser::SaveGame,
    };

    use super::ProvinceHistoryEvent;

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

        let start_date = EU4Date {
            year: 1444,
            month: eu4_parser_core::Month::NOV,
            day: 11,
        };
        let end_date = EU4Date {
            year: 1450,
            month: eu4_parser_core::Month::JAN,
            day: 1,
        };
        print!("Generating color maps...");
        let start_time = std::time::Instant::now();
        let color_maps =
            all_i_frame_color_maps(&assets, &combined_history, &save, start_date, end_date);
        println!(
            " ({}ms.)",
            std::time::Instant::now()
                .duration_since(start_time)
                .as_millis()
        );

        let mut img_out = image::RgbImage::new(5632, 2048 * 30);
        for (i, (_, color_map)) in color_maps.step_by(10).take(30).enumerate() {
            img_out
                .copy_from(
                    &make_base_map(&assets.base_map, &color_map),
                    0,
                    i as u32 * 2048,
                )
                .unwrap();
        }
        img_out.save("weeks.png").unwrap();

        return;
        /*print!("Generating frames...");
        let start_time = std::time::Instant::now();
        let frames: Vec<image::Frame> = color_maps
            .iter()
            .map(|(date, color_map)| {
                image::Frame::from_parts(
                    imageproc::map::map_colors(&make_base_map(&assets.base_map, color_map), |p| {
                        p.to_rgba()
                    }),
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(400, 1),
                )
            })
            .collect();
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
        );*/
    }
}
