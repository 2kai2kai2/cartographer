use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashMap};

use crate::{
    eu4_date::EU4Date,
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mod {
    Vanilla,
}

fn eu4_obj_as_color<'a>(value: &RawEU4Object<'a>) -> Result<[u8; 3]> {
    return value
        .iter_values()
        .map(|item| match item {
            RawEU4Value::Scalar(scalar) => scalar.try_into().map_err(anyhow::Error::from),
            _ => Err(anyhow!("Found non-scalar in")),
        })
        .collect::<Result<Vec<u8>>>()?
        .try_into()
        .or(Err(anyhow!("Object was wrong length for color")));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nation {
    pub tag: String,
    pub other_tags: Vec<String>,
    pub development: usize,
    pub prestige: f64,
    pub stability: i8,
    pub army: f64,
    pub navy: usize,
    pub debt: f64,
    pub treasury: f64,
    pub total_income: f64,
    pub total_expense: f64,
    pub score_place: usize,
    pub capital_id: usize,
    pub overlord: Option<String>,
    pub allies: Vec<String>,
    pub subjects: Vec<String>,
    pub map_color: [u8; 3],
    pub nation_color: [u8; 3],
}
impl Nation {
    pub fn from_parsed_obj(tag: String, obj: &RawEU4Object) -> Result<Nation> {
        let colors = obj
            .get_first_obj("colors")
            .ok_or(anyhow!("Found no colors for a country"))?;
        let map_color = colors
            .get_first_obj("map_color")
            .ok_or(anyhow!("no 'map_color' obj"))?;
        let map_color = eu4_obj_as_color(map_color)?;
        let nation_color = colors
            .get_first_obj("country_color")
            .ok_or(anyhow!("no 'country_color' obj"))?;
        let nation_color = eu4_obj_as_color(nation_color)?;

        // == FINANCIALS ==
        let treasury = obj
            .get_first_as_float("treasury")
            .ok_or(anyhow!("no float 'treasury'"))?;
        let debt = obj
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar("loan"), RawEU4Value::Object(loan)) => {
                    loan.get_first_as_float("amount")
                }
                _ => None,
            })
            .sum();
        let total_income = obj
            .get_first_scalar_at_path(["ledger", "lastmonthincome"])
            .and_then(RawEU4Scalar::as_float)
            .unwrap_or(0.0);
        let total_expense = obj
            .get_first_scalar_at_path(["ledger", "lastmonthexpense"])
            .and_then(RawEU4Scalar::as_float)
            .unwrap_or(0.0);

        // == MILITARY ==
        let army: f64 = obj
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar("army"), RawEU4Value::Object(army_obj)) => Some(army_obj),
                _ => None,
            })
            .flat_map(|army| {
                army.iter_all_KVs().filter_map(|kv| match kv {
                    (RawEU4Scalar("regiment"), RawEU4Value::Object(regiment_obj)) => {
                        Some(regiment_obj.get_first_as_float("strength").unwrap_or(1.0) * 1000.0)
                    }
                    _ => None,
                })
            })
            .sum();
        let navy: usize = obj
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar("navy"), RawEU4Value::Object(army_obj)) => Some(army_obj),
                _ => None,
            })
            .map(|navy| {
                navy.iter_all_KVs()
                    .filter(|(k, _)| **k == RawEU4Scalar("ship"))
                    .count()
            })
            .sum();

        return Ok(Nation {
            tag,
            other_tags: obj
                .iter_all_KVs()
                .filter_map(|pair| match pair {
                    (RawEU4Scalar("previous_country_tags"), RawEU4Value::Scalar(other_tag)) => {
                        Some(other_tag.as_string())
                    }
                    _ => None,
                })
                .collect(),
            development: obj
                .get_first_as_float("raw_development")
                .unwrap_or_default() as usize,
            prestige: obj
                .get_first_as_float("prestige")
                .ok_or(anyhow!("no float 'prestige"))?,
            stability: obj
                .get_first_as_float("stability")
                .ok_or(anyhow!("no float 'stability'"))? as i8,
            army,
            navy,
            debt,
            treasury,
            total_income,
            total_expense,
            score_place: obj
                .get_first_as_int("score_place")
                .ok_or(anyhow!("No int 'score_place'"))? as usize,
            capital_id: obj
                .get_first_as_int("capital")
                .ok_or(anyhow!("No int 'capital'"))? as usize,
            overlord: obj.get_first_as_string("overlord"),
            allies: obj.get_first_obj("allies").map_or(vec![], |allies| {
                allies
                    .iter_values()
                    .filter_map(RawEU4Value::as_scalar)
                    .map(RawEU4Scalar::as_string)
                    .collect()
            }),
            subjects: obj.get_first_obj("subjects").map_or(vec![], |subjects| {
                subjects
                    .iter_values()
                    .filter_map(RawEU4Value::as_scalar)
                    .map(RawEU4Scalar::as_string)
                    .collect()
            }),
            map_color,
            nation_color,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarResult {
    WhitePeace = 1,
    AttackerVictory = 2,
    DefenderVictory = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct War {
    pub name: String,
    pub attackers: Vec<String>,
    pub defenders: Vec<String>,
    pub attacker_losses: i64,
    pub defender_losses: i64,
    pub start_date: EU4Date,
    pub end_date: Option<EU4Date>,
    pub result: Option<WarResult>,
}

impl War {
    pub fn player_attackers(&self, player_tags: &Vec<String>) -> Vec<String> {
        return self
            .attackers
            .iter()
            .filter(|a| player_tags.contains(a))
            .cloned()
            .collect();
    }
    pub fn player_defenders(&self, player_tags: &Vec<String>) -> Vec<String> {
        return self
            .defenders
            .iter()
            .filter(|d| player_tags.contains(d))
            .cloned()
            .collect();
    }

    /** An evaluation of how 'significant' the war probably is  */
    pub fn war_scale(&self, player_tags: &Vec<String>) -> i64 {
        let player_attackers = self.player_attackers(player_tags).len();
        let player_defenders = self.player_defenders(player_tags).len();
        let casualties = self.attacker_losses + self.defender_losses;
        if player_attackers <= 1 || player_defenders <= 1 {
            return casualties;
        }

        return casualties * min(player_attackers, player_defenders) as i64;
    }

    pub fn is_player_war(&self, player_tags: &Vec<String>) -> bool {
        return self.attackers.iter().any(|a| player_tags.contains(a))
            && self.defenders.iter().any(|d| player_tags.contains(d));
    }

    /// There are nonexistant wars in save files, so keep an option
    pub fn from_parsed_obj(obj: &RawEU4Object) -> Result<Option<War>> {
        let mut attackers: Vec<String> = Vec::new();
        let mut defenders: Vec<String> = Vec::new();
        let mut earliest_date: Option<EU4Date> = None;
        let mut latest_date: Option<EU4Date> = None;
        for (date, value) in obj
            .get_first_obj("history")
            .ok_or(anyhow!("No history in war"))?
            .iter_all_KVs()
            .filter_map(|(k, v)| {
                Some((
                    k.as_date()?,
                    v.as_object().expect("date events should be objects"),
                ))
            })
        {
            for (event, value) in value.iter_all_KVs() {
                match (event.0, value) {
                    ("add_attacker", RawEU4Value::Scalar(value)) => {
                        attackers.push(value.as_string());
                        match earliest_date {
                            None => earliest_date = Some(date),
                            Some(prev_date) if date < prev_date => earliest_date = Some(date),
                            _ => {}
                        }
                    }
                    ("add_defender", RawEU4Value::Scalar(value)) => {
                        defenders.push(value.as_string());
                        match earliest_date {
                            None => earliest_date = Some(date),
                            Some(prev_date) if date < prev_date => earliest_date = Some(date),
                            _ => {}
                        }
                    }
                    ("rem_attacker", RawEU4Value::Scalar(_))
                    | ("rem_defender", RawEU4Value::Scalar(_)) => match latest_date {
                        None => latest_date = Some(date),
                        Some(prev_date) if prev_date < date => latest_date = Some(date),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        let Some(start_date) = earliest_date else {
            return Ok(None);
        };

        let mut attacker_losses: i64 = 0;
        let mut defender_losses: i64 = 0;
        for (key, value) in obj.iter_all_KVs() {
            if key.0 != "participants" {
                continue;
            }
            let RawEU4Value::Object(obj) = value else {
                // this shouldn't happen?
                continue;
            };

            let Some(tag) = obj.get_first_scalar("tag") else {
                continue;
            };
            let Some(losses) = obj.get_first_object_at_path(["losses", "members"]) else {
                continue;
            };
            let losses: i64 = losses
                .iter_values()
                .filter_map(RawEU4Value::as_scalar)
                .filter_map(RawEU4Scalar::as_int)
                .sum();
            if attackers.contains(&tag.as_string()) {
                attacker_losses += losses;
            } else if defenders.contains(&tag.as_string()) {
                defender_losses += losses;
            }
        }

        let name = obj
            .get_first_scalar("name")
            .ok_or(anyhow!("No name in war"))?
            .as_string();
        return Ok(Some(War {
            name: name.clone(),
            attackers: attackers.clone(),
            defenders: defenders.clone(),
            attacker_losses,
            defender_losses,
            start_date,
            end_date: latest_date,
            result: match obj.get_first_scalar("outcome") {
                Some(RawEU4Scalar("1")) => Some(WarResult::WhitePeace),
                Some(RawEU4Scalar("2")) => Some(WarResult::AttackerVictory),
                Some(RawEU4Scalar("3")) => Some(WarResult::DefenderVictory),
                _ => None,
            },
        }));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    pub all_nations: HashMap<String, Nation>,
    /** tag: playername */
    pub player_tags: HashMap<String, String>,
    pub provinces: HashMap<u64, String>,
    pub dlc: Vec<String>,
    pub great_powers: Vec<String>,
    pub date: EU4Date,
    pub multiplayer: bool,
    pub age: Option<String>,
    pub hre: Option<String>,
    pub china: Option<String>,
    pub crusade: Option<String>,
    pub player_wars: Vec<War>,
    pub game_mod: Mod,
}

impl SaveGame {
    pub fn player_nations(&self) -> impl Iterator<Item = (&String, &Nation)> {
        return self
            .player_tags
            .iter()
            .filter_map(|(tag, player)| Some((player, self.all_nations.get(tag)?)));
    }

    /** Gets the player of a nation, including former tags */
    pub fn tag_player(&self, tag: &String) -> Option<&String> {
        return self.player_tags.get(tag).or_else(|| {
            for (player, nation) in self.player_nations() {
                if nation.other_tags.contains(tag) {
                    return Some(player);
                }
            }
            return None;
        });
    }

    pub fn new_parser(raw_save: &RawEU4Object) -> Option<SaveGame> {
        let all_nations = raw_save
            .get_first_obj("countries")
            .unwrap()
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar(tag), RawEU4Value::Object(nation)) => Some((
                    tag.to_string(),
                    Nation::from_parsed_obj(tag.to_string(), nation).unwrap(),
                )),
                _ => None,
            })
            .collect();
        let player_tags: Vec<&RawEU4Scalar> = raw_save
            .get_first_obj("players_countries")?
            .iter_values()
            .map(RawEU4Value::as_scalar)
            .collect::<Option<Vec<_>>>()
            .unwrap();
        let player_tags: HashMap<String, String> = player_tags
            .chunks_exact(2)
            .map(|v| match v {
                [player, tag] => Some((tag.as_string(), player.as_string())),
                _ => None,
            })
            .collect::<Option<HashMap<_, _>>>()
            .unwrap();
        let provinces: HashMap<u64, String> = raw_save
            .get_first_obj("provinces")?
            .iter_all_KVs()
            .filter_map(|(k, v)| Some((k, v.as_object()?)))
            .filter_map(|(k, v)| {
                Some((
                    k.as_int()?.abs() as u64,
                    v.get_first_scalar("owner")?.as_string(),
                ))
            })
            .collect();
        let dlc: Vec<String> = raw_save
            .get_first_obj("dlc_enabled")?
            .iter_values()
            .filter_map(|v| match v {
                RawEU4Value::Scalar(scalar) => Some(scalar.as_string()),
                _ => None,
            })
            .collect();
        let great_powers = Vec::new();
        let date = raw_save.get_first_scalar("date");

        return Some(SaveGame {
            all_nations,
            player_tags,
            provinces,
            dlc,
            great_powers,
            date: date.unwrap().as_date().unwrap(),
            multiplayer: raw_save
                .get_first_scalar("multi_player")
                .unwrap()
                .as_bool()
                .unwrap(),
            age: raw_save
                .get_first_scalar("current_age")
                .map(RawEU4Scalar::as_string),
            hre: raw_save
                .get_first_scalar_at_path(["empire", "emperor"])
                .map(RawEU4Scalar::as_string),
            china: raw_save
                .get_first_scalar_at_path(["celestial_empire", "emperor"])
                .map(RawEU4Scalar::as_string),
            crusade: None,
            player_wars: raw_save
                .iter_all_KVs()
                .filter(|(k, _)| k.0 == "active_war" || k.0 == "previous_war")
                .filter_map(|(_, v)| match v {
                    RawEU4Value::Object(o) => Some(o),
                    _ => None,
                })
                .map(War::from_parsed_obj)
                .collect::<Result<Vec<_>>>()
                .expect("oh no invalid wars?")
                .into_iter()
                .filter_map(|a| a)
                .collect(),
            game_mod: Mod::Vanilla,
        });
    }
}
