use std::collections::HashMap;

use eu4_parser_core::{
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
    EU4Date,
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum CountryHistoryEvent {
    ChangedTagFrom(String),
}
impl CountryHistoryEvent {
    /// Should be passed the `history` object for a country
    pub fn extract_events<'a>(object: &RawEU4Object<'a>) -> Vec<(EU4Date, CountryHistoryEvent)> {
        return object
            .iter_all_KVs()
            .filter_map(|(k, v)| Some((k.as_date()?, v.as_object()?)))
            .flat_map(|(date, obj)| {
                obj.iter_all_KVs()
                    .filter_map(move |(RawEU4Scalar(ev), val)| match (ev, val) {
                        (&"changed_tag_from", RawEU4Value::Scalar(tag)) => {
                            Some((date, CountryHistoryEvent::ChangedTagFrom(tag.as_string())))
                        }

                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
    }

    pub fn combine_events(
        countries: Vec<(String, Vec<(EU4Date, CountryHistoryEvent)>)>,
    ) -> HashMap<EU4Date, Vec<(String, CountryHistoryEvent)>> {
        let mut out: HashMap<EU4Date, Vec<(String, CountryHistoryEvent)>> = HashMap::new();
        for (id, events) in countries {
            for (date, event) in events {
                out.entry(date).or_default().push((id.clone(), event));
            }
        }
        return out;
    }
}

pub fn make_combined_events(
    save: &RawEU4Object,
) -> HashMap<EU4Date, Vec<(String, CountryHistoryEvent)>> {
    let province_histories = save
        .get_first_obj("countries")
        .unwrap()
        .iter_all_KVs()
        .filter_map(|(k, v)| {
            Some((
                k.as_string(),
                CountryHistoryEvent::extract_events(v.as_object()?.get_first_obj("history")?),
            ))
        })
        .collect();
    return CountryHistoryEvent::combine_events(province_histories);
}
