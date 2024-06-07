use std::collections::HashMap;

use eu4_parser_core::{
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
    EU4Date,
};
use image::Rgb;
use imageproc::definitions::HasBlack;

use crate::{
    country_history::CountryHistoryEvent,
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

pub fn make_combined_events(
    save: &RawEU4Object,
) -> HashMap<EU4Date, Vec<(u64, ProvinceHistoryEvent)>> {
    let province_histories = save
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
    return ProvinceHistoryEvent::combine_events(province_histories);
}

/// all I-frames, as in every frame has its data in full rather than diffed
///
/// Returns `(date, owners, controllers)`
/// If controllers is `[0, 0, 0]` then it is unowned (and shouldn't be shown)
pub fn all_i_frame_color_maps<'a>(
    assets: &'a MapAssets,
    province_history: &'a HashMap<EU4Date, Vec<(u64, ProvinceHistoryEvent)>>,
    country_history: &'a HashMap<EU4Date, Vec<(String, CountryHistoryEvent)>>,
    save: &'a SaveGame,
    start_date: EU4Date,
    end_date: EU4Date,
) -> impl Iterator<Item = (EU4Date, Vec<Rgb<u8>>, Vec<Rgb<u8>>)> + 'a {
    let mut tag_colors: HashMap<_, _> = save
        .all_nations
        .iter()
        .map(|(tag, nation)| (tag, Rgb(nation.map_color)))
        .collect();
    tag_colors.remove(&"---".to_string());
    tag_colors.remove(&"REB".to_string());

    let initial_owners = generate_map_colors_config(
        assets.provinces_len,
        &assets.water,
        &assets.wasteland,
        |_| None,
        |tag| tag_colors.get(&tag).map(Rgb::to_owned),
    );
    let initial_controllers: Vec<Rgb<u8>> = generate_map_colors_config(
        assets.provinces_len,
        &assets.water,
        &assets.wasteland,
        |_| Some("".to_string()),
        |_| Some(Rgb::black()),
    );
    return std::iter::successors(
        Some((start_date, initial_owners, initial_controllers)),
        move |(prev_date, prev_owners, prev_controllers)| {
            let date = prev_date.tomorrow();
            if date > end_date {
                return None;
            }

            let mut owners = prev_owners.clone();
            let mut controllers = prev_controllers.clone();
            if let Some(events) = province_history.get(&date) {
                let mut fake_owners: Vec<(u64, &String)> = Vec::new();
                let mut set_controller: Vec<u64> = Vec::new();
                for (id, event) in events {
                    match event {
                        ProvinceHistoryEvent::Owner(tag) => {
                            owners[*id as usize] =
                                tag_colors.get(tag).unwrap_or(&UNCLAIMED_COLOR).clone();
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
                            controllers[*id as usize] =
                                tag_colors.get(tag).unwrap_or(&Rgb::black()).clone();
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
                                .filter(|color| *color == prev_color)
                                .for_each(|color| *color = new_color.clone());
                            controllers
                                .iter_mut()
                                .filter(|color| *color == prev_color)
                                .for_each(|color| *color = new_color.clone());
                        }
                    }
                }
            }
            return Some((date, owners, controllers));
        },
    );
}
