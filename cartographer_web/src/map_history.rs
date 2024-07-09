use std::collections::HashMap;

use bitstream_io::{BigEndian, ByteRead, ByteReader};
use eu4_parser_core::{
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
    EU4Date, Month,
};
use image::Rgb;
use imageproc::definitions::HasBlack;
use serde::{Deserialize, Serialize};

use crate::{
    country_history::{CountryHistoryEvent, WarHistoryEvent},
    eu4_map::{generate_map_colors_config, UNCLAIMED_COLOR},
    map_parsers::MapAssets,
    save_parser::SaveGame,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ProvinceHistoryEvent {
    Owner(String),
    FakeOwner(String),
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
                        (&"fake_owner", RawEU4Value::Scalar(tag)) => {
                            Some((date, ProvinceHistoryEvent::FakeOwner(tag.as_string())))
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
        provinces: Vec<(u16, Vec<(EU4Date, ProvinceHistoryEvent)>)>,
    ) -> HashMap<EU4Date, Vec<(u16, ProvinceHistoryEvent)>> {
        let mut out: HashMap<EU4Date, Vec<(u16, ProvinceHistoryEvent)>> = HashMap::new();
        for (id, events) in provinces {
            for (date, event) in events {
                out.entry(date).or_default().push((id, event));
            }
        }
        return out;
    }
}

pub fn make_combined_events(
    save: &RawEU4Object,
) -> HashMap<EU4Date, Vec<(u16, ProvinceHistoryEvent)>> {
    let province_histories = save
        .get_first_obj("provinces")
        .unwrap()
        .iter_all_KVs()
        .filter_map(|(k, v)| {
            Some((
                k.as_int()?.abs() as u16,
                ProvinceHistoryEvent::extract_events(v.as_object()?.get_first_obj("history")?),
            ))
        })
        .collect();
    return ProvinceHistoryEvent::combine_events(province_histories);
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ColorMapEvent {
    Owner(Rgb<u8>),
    Controller(Rgb<u8>),
}
impl ColorMapEvent {
    pub fn apply(target: &mut (Vec<Rgb<u8>>, Vec<Rgb<u8>>), event: &(u16, ColorMapEvent)) {
        match event {
            (id, ColorMapEvent::Owner(color)) => target.0[*id as usize] = *color,
            (id, ColorMapEvent::Controller(color)) => target.1[*id as usize] = *color,
        }
    }

    pub fn apply_many(
        target: &mut (Vec<Rgb<u8>>, Vec<Rgb<u8>>),
        events: &Vec<(u16, ColorMapEvent)>,
    ) {
        events
            .into_iter()
            .for_each(|event| ColorMapEvent::apply(target, event));
    }
}

/// Contains a set of i-frames on a few dates, mostly JAN 1 (the full map of province colors)
///
/// And the diffs for every date that there are any (including the ones with i-frames)
///
/// If controller is `[0, 0, 0]` then controller is same as owner
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ColorMapManager {
    pub start_date: EU4Date,
    pub end_date: EU4Date,
    pub diffs: HashMap<EU4Date, Vec<(u16, ColorMapEvent)>>,
    pub i_frames: HashMap<EU4Date, (Vec<Rgb<u8>>, Vec<Rgb<u8>>)>,
}
impl ColorMapManager {
    pub fn new(
        assets: &MapAssets,
        province_history: &HashMap<EU4Date, Vec<(u16, ProvinceHistoryEvent)>>,
        country_history: &HashMap<EU4Date, Vec<(String, CountryHistoryEvent)>>,
        war_history: &HashMap<EU4Date, Vec<WarHistoryEvent>>,
        save: &SaveGame,
        start_date: EU4Date,
        end_date: EU4Date,
    ) -> ColorMapManager {
        let mut tag_colors: HashMap<_, _> = save
            .all_nations
            .iter()
            .map(|(tag, nation)| (tag, Rgb(nation.map_color)))
            .collect();
        tag_colors.remove(&"---".to_string());
        tag_colors.remove(&"REB".to_string());

        let mut owners = generate_map_colors_config(
            assets.provinces_len,
            &assets.water,
            &assets.wasteland,
            |_| None,
            |_| None,
        );
        let mut controllers = generate_map_colors_config(
            assets.provinces_len,
            &assets.water,
            &assets.wasteland,
            |_| Some("".to_string()),
            |_| Some(Rgb::black()),
        );

        let mut out = ColorMapManager {
            start_date,
            end_date,
            diffs: HashMap::new(),
            i_frames: HashMap::new(),
        };
        out.i_frames
            .insert(start_date, (owners.clone(), controllers.clone()));

        for date in EU4Date::iter_range_inclusive(start_date, end_date) {
            let mut diffs: Vec<(u16, ColorMapEvent)> = Vec::new();
            if let Some(events) = war_history.get(&date) {
                for event in events {
                    match event {
                        WarHistoryEvent::RemoveOccupations(w_owner, w_controller) => {
                            let Some(w_owner) = tag_colors.get(w_owner) else {
                                continue;
                            };
                            let Some(w_controller) = tag_colors.get(w_controller) else {
                                continue;
                            };
                            for (id, owner) in owners.iter().enumerate() {
                                if owner == w_owner && controllers[id] == *w_controller {
                                    controllers[id] = Rgb::black();
                                    diffs
                                        .push((id as u16, ColorMapEvent::Controller(Rgb::black())));
                                }
                            }
                        }
                    }
                }
            }
            if let Some(events) = province_history.get(&date) {
                let mut fake_owners: Vec<(u16, &String)> = Vec::new();
                let mut set_controller: Vec<u16> = Vec::new();
                for (id, event) in events {
                    match event {
                        ProvinceHistoryEvent::Owner(tag) => {
                            let color = tag_colors.get(tag).unwrap_or(&UNCLAIMED_COLOR).clone();
                            if owners[*id as usize] == color {
                                continue;
                            }
                            owners[*id as usize] = color;
                            diffs.push((*id, ColorMapEvent::Owner(color)));
                        }
                        ProvinceHistoryEvent::FakeOwner(tag) => {
                            fake_owners.push((*id, tag));
                        }
                        ProvinceHistoryEvent::Controller(tag) => {
                            if fake_owners.contains(&(*id, tag)) || set_controller.contains(id) {
                                // fake_owner seems to be something used to give cores/province history to formed tags
                                // where the core needs to be older than the tag.
                                // So, we need to ignore controller events with a simulataneous fake_owner since it's a lie.
                                // It seems to always be after fake_owner.
                                //
                                // Similarly, when a country pre-tag-formation gains control of a province, it sometimes
                                // has both the old and new tags. It seems the contemporary tag is always first.
                                continue;
                            }
                            let color = tag_colors.get(tag).unwrap_or(&Rgb::black()).clone();
                            if controllers[*id as usize] == color {
                                set_controller.push(*id);
                                continue;
                            }
                            controllers[*id as usize] = color;
                            diffs.push((*id, ColorMapEvent::Controller(color)));
                            set_controller.push(*id);
                        }
                        _ => {}
                    }
                }
            }
            // country formations don't show an owner change event
            if let Some(events) = country_history.get(&date) {
                for (tag, event) in events {
                    match event {
                        CountryHistoryEvent::ChangedTagFrom(prev_tag) => {
                            let Some(prev_color) = tag_colors.get(prev_tag) else {
                                continue;
                            };
                            let Some(new_color) = tag_colors.get(tag) else {
                                continue;
                            };
                            owners
                                .iter_mut()
                                .enumerate()
                                .filter(|(_id, color)| *color == prev_color)
                                .for_each(|(id, color)| {
                                    *color = *new_color;
                                    diffs.push((id as u16, ColorMapEvent::Owner(*new_color)));
                                });
                            controllers
                                .iter_mut()
                                .enumerate()
                                .filter(|(_id, color)| *color == prev_color)
                                .for_each(|(id, color)| {
                                    *color = *new_color;
                                    diffs.push((id as u16, ColorMapEvent::Controller(*new_color)));
                                });
                        }
                    }
                }
            }
            if date.month == Month::JAN && date.day == 1 {
                out.i_frames
                    .insert(date, (owners.clone(), controllers.clone()));
            }
            if diffs.len() > 0 {
                out.diffs.insert(date, diffs);
            }
        }
        return out;
    }

    /// Gets the color maps for a specified date.
    ///
    /// Generally, diffs should be used, as this method can be slow.
    ///
    /// Returns the color maps for the date, or `None` if the date is before the earliest available date.
    pub fn get_date(&self, date: &EU4Date) -> Option<(Vec<Rgb<u8>>, Vec<Rgb<u8>>)> {
        if let Some(i_frame) = self.i_frames.get(&date) {
            return Some(i_frame.clone());
        }

        let (mut iter_date, mut i_frame) =
            EU4Date::iter_range_inclusive_reversed(date.with_year(date.year - 1), *date)
                .find_map(|d| Some((d, self.i_frames.get(&d)?.clone())))?;
        while iter_date < *date {
            self.apply_diffs(&iter_date, &mut i_frame);
            iter_date = iter_date.tomorrow();
        }
        return Some(i_frame);
    }

    pub fn apply_diffs(&self, date: &EU4Date, color_maps: &mut (Vec<Rgb<u8>>, Vec<Rgb<u8>>)) {
        if let Some(events) = self.diffs.get(date) {
            ColorMapEvent::apply_many(color_maps, events);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SerializedColorMapManager {
    start_date: String,
    end_date: String,
    diffs: HashMap<String, String>,
}
impl SerializedColorMapManager {
    pub fn encode(manager: &ColorMapManager) -> Self {
        return Self {
            start_date: manager.start_date.to_string(),
            end_date: manager.end_date.to_string(),
            diffs: manager
                .diffs
                .iter()
                .map(|(date, events)| {
                    (
                        date.to_string(),
                        base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            events
                                .into_iter()
                                .flat_map(|(id, ev)| {
                                    id.to_be_bytes().into_iter().chain(match ev {
                                        ColorMapEvent::Owner(Rgb(color)) => {
                                            std::iter::once(0u8).chain(color.into_iter().cloned())
                                        }
                                        ColorMapEvent::Controller(Rgb(color)) => {
                                            std::iter::once(1u8).chain(color.into_iter().cloned())
                                        }
                                    })
                                })
                                .collect::<Vec<u8>>(),
                        ),
                    )
                })
                .collect::<HashMap<String, String>>(),
        };
    }
    pub fn decode(&self, assets: &MapAssets) -> anyhow::Result<ColorMapManager> {
        let start_date: EU4Date = self.start_date.parse()?;
        let end_date: EU4Date = self.end_date.parse()?;
        let diffs: HashMap<EU4Date, Vec<(u16, ColorMapEvent)>> = self
            .diffs
            .iter()
            .map(|(date, events)| {
                let events =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, events)?;
                let mut reader = ByteReader::endian(std::io::Cursor::new(events), BigEndian);
                let mut out_events: Vec<(u16, ColorMapEvent)> = Vec::new();
                loop {
                    let Ok(id) = reader.read::<u16>() else {
                        break;
                    };
                    match reader.read() {
                        Ok(0u8) => {
                            let Ok(color) = reader.read() else {
                                break;
                            };
                            out_events.push((id, ColorMapEvent::Owner(Rgb(color))));
                        }
                        Ok(1u8) => {
                            let Ok(color) = reader.read() else {
                                break;
                            };
                            out_events.push((id, ColorMapEvent::Controller(Rgb(color))));
                        }
                        _ => break,
                    }
                }

                return Ok((date.parse()?, out_events));
            })
            .collect::<anyhow::Result<_>>()?;

        let mut color_maps = (
            generate_map_colors_config(
                assets.provinces_len,
                &assets.water,
                &assets.wasteland,
                |_| None,
                |_| None,
            ),
            generate_map_colors_config(
                assets.provinces_len,
                &assets.water,
                &assets.wasteland,
                |_| Some("".to_string()),
                |_| Some(Rgb::black()),
            ),
        );

        let mut i_frames: HashMap<EU4Date, (Vec<Rgb<u8>>, Vec<Rgb<u8>>)> = HashMap::new();
        i_frames.insert(start_date, color_maps.clone());
        for date in EU4Date::iter_range_inclusive(start_date, end_date) {
            if let Some(events) = diffs.get(&date) {
                ColorMapEvent::apply_many(&mut color_maps, events);
            }
            if date.month == Month::JAN && date.day == 1 {
                i_frames.insert(date, color_maps.clone());
            }
        }

        return Ok(ColorMapManager {
            start_date,
            end_date,
            diffs,
            i_frames,
        });
    }
}
