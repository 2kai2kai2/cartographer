use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashMap};

use crate::{
    raw_parser::{RawPDXExtractError, RawPDXObject, RawPDXScalar, RawPDXValue},
    stellaris_date::StellarisDate,
};
use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mod {
    Vanilla,
}

fn eu4_obj_as_color<'a>(value: &RawPDXObject<'a>) -> Result<[u8; 3]> {
    return value
        .iter_values()
        .map(|item| match item {
            RawPDXValue::Scalar(scalar) => scalar.try_into().map_err(anyhow::Error::from),
            _ => Err(anyhow!("Found non-scalar in")),
        })
        .collect::<Result<Vec<u8>>>()?
        .try_into()
        .or(Err(anyhow!("Object was wrong length for color")));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalacticObjectCoord {
    pub x: f64,
    pub y: f64,
    /// meaning unknown
    pub origin: u64,
    /// meaning unknown
    pub randomized: bool,
}
impl<'b, 'a> TryFrom<&'b RawPDXObject<'a>> for GalacticObjectCoord {
    type Error = anyhow::Error;

    fn try_from(obj: &'b RawPDXObject<'a>) -> std::result::Result<Self, Self::Error> {
        let x = obj.expect_first_scalar("x")?.try_into()?;
        let y = obj.expect_first_scalar("y")?.try_into()?;
        let origin = obj.expect_first_scalar("origin")?.try_into()?;
        let randomized = obj.expect_first_scalar("randomized")?.try_into()?;

        return Ok(GalacticObjectCoord {
            x,
            y,
            origin,
            randomized,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planet {
    // todo: name
    pub planet_class: String,
    /// Determines where in the system it appears
    pub orbit: u32,
    pub planet_size: u32,
    /// Even if the system is 'owned', this may not be set.
    pub owner: Option<u32>,
    /// Even if the system is 'owned', this may not be set.
    pub controller: Option<u32>,
    pub num_sapient_pops: u32,
    // ...
}
impl<'b, 'a> TryFrom<&'b RawPDXObject<'a>> for Planet {
    type Error = anyhow::Error;

    fn try_from(obj: &'b RawPDXObject<'a>) -> std::result::Result<Self, Self::Error> {
        let planet_class = obj.expect_first_scalar("planet_class")?.as_string();
        let orbit = obj.expect_first_scalar("orbit")?.try_into()?;
        let planet_size = obj.expect_first_scalar("planet_size")?.try_into()?;
        let owner = obj
            .get_first_scalar("owner")
            .and_then(|owner| owner.try_into().ok());
        let controller = obj
            .get_first_scalar("controller")
            .and_then(|controller| controller.try_into().ok());
        let num_sapient_pops = obj.expect_first_scalar("num_sapient_pops")?.try_into()?;

        return Ok(Planet {
            planet_class,
            orbit,
            planet_size,
            owner,
            controller,
            num_sapient_pops,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperlane {
    /// Destination system id. Should also have a path back.
    pub to: u32,
    pub length: f64,
    pub bridge: bool,
}
impl<'b, 'a> TryFrom<&'b RawPDXObject<'a>> for Hyperlane {
    type Error = anyhow::Error;

    fn try_from(obj: &'b RawPDXObject<'a>) -> std::result::Result<Self, Self::Error> {
        let to = obj.expect_first_scalar("to")?.try_into()?;
        let length = obj.expect_first_scalar("length")?.try_into()?;
        let bridge = match obj.get_first_scalar("bridge") {
            Some(bridge) => bridge.try_into()?,
            None => false,
        };

        return Ok(Hyperlane { to, length, bridge });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalacticObject {
    pub coordinate: GalacticObjectCoord,
    pub obj_type: String,
    pub name: String,
    /// The computed owner of this system.
    /// In most cases this is relatively simple-- only one country should have control of the system.
    /// However, this can be complicated during occupation or other situations.
    ///
    /// NOTE: will be `None` when returned from `try_from` and will need to be updated based on further data.
    pub map_owner: Option<u32>,
    /// Planet ids, including the sun.
    pub planets: Vec<u32>,
    pub hyperlanes: Vec<Hyperlane>,
    /// Should be a subset of [`GalacticObject::planets`]
    pub colonies: Vec<u32>,
}
impl<'b, 'a> TryFrom<&'b RawPDXObject<'a>> for GalacticObject {
    type Error = anyhow::Error;

    fn try_from(obj: &'b RawPDXObject<'a>) -> std::result::Result<Self, Self::Error> {
        let coordinate = obj.expect_first_obj("coordinate")?.try_into()?;

        let obj_type = obj.expect_first_scalar("type")?.as_string();

        let name = obj
            .expect_first_obj("name")?
            .get_first_as_string("key")
            .unwrap_or("unknown".to_string());

        let planets = obj
            .iter_all_KVs()
            .filter(|(k, _)| k.0 == "planet")
            .map(|(_, v)| Ok(v.expect_scalar()?.try_into()?))
            .collect::<Result<_, anyhow::Error>>()?;
        let hyperlanes = match obj.get_first_obj("hyperlane") {
            Some(hyperlanes) => hyperlanes
                .iter_values()
                .map(|lane| lane.expect_object()?.try_into())
                .collect::<Result<_, _>>()?,
            None => Vec::new(),
        };
        let colonies = match obj.get_first_obj("colonies") {
            Some(colonies) => colonies
                .iter_values()
                .map(|id| Ok(id.expect_scalar()?.try_into()?))
                .collect::<Result<_, anyhow::Error>>()?,
            None => Vec::new(),
        };

        return Ok(GalacticObject {
            coordinate,
            obj_type,
            name,
            map_owner: None,
            planets,
            hyperlanes,
            colonies,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub idx: u32,
    pub name: String,
    // adjective
    /// Known types:
    /// - `default` is normal player+npc countries
    ///
    pub country_type: String,
    pub victory_rank: u32,
    pub victory_score: f32,
    pub tech_power: f32,
    pub fleet_size: u32,
    pub empire_size: u32,
    pub num_sapient_pops: u32,
    pub capital: Option<u32>,
    /// Pair of `(income, expense)`. Both will be positive.
    pub balance: HashMap<String, (f64, f64)>,
    pub map_color: [u8; 3],
    pub nation_color: [u8; 3],
}
impl Country {
    pub fn from_parsed_obj(idx: u32, obj: &RawPDXObject) -> Result<Country> {
        let name = obj
            .expect_first_obj("name")?
            .get_first_as_string("key")
            .unwrap_or("unknown".to_string());

        let victory_rank = obj.expect_first_scalar("victory_rank")?.try_into()?;
        let victory_score = obj.expect_first_scalar("victory_score")?.try_into()?;

        let tech_power = obj.expect_first_scalar("tech_power")?.try_into()?;

        let fleet_size = obj
            .get_first_scalar("fleet_size")
            .map(u32::try_from)
            .transpose()?
            .unwrap_or(0);
        let empire_size = obj
            .get_first_scalar("empire_size")
            .map(u32::try_from)
            .transpose()?
            .unwrap_or(0);
        let num_sapient_pops = obj
            .get_first_scalar("num_sapient_pops")
            .map(u32::try_from)
            .transpose()?
            .unwrap_or(0);

        let capital = obj
            .get_first_scalar("capital")
            .map(u32::try_from)
            .transpose()?;

        let mut balance: HashMap<String, (f64, f64)> = HashMap::new();
        let budget_current_month = obj
            .expect_first_obj("budget")?
            .expect_first_obj("current_month")?;
        for (_, source) in budget_current_month
            .expect_first_obj("income")?
            .iter_all_KVs()
        {
            let source = source.expect_object()?;
            for (resource, amount) in source.iter_all_KVs() {
                let resource = resource.as_string();
                let amount: f64 = amount.expect_scalar()?.try_into()?;
                balance.entry(resource).or_default().0 += amount;
            }
        }
        for (_, source) in budget_current_month
            .expect_first_obj("expenses")?
            .iter_all_KVs()
        {
            let source = source.expect_object()?;
            for (resource, amount) in source.iter_all_KVs() {
                let resource = resource.as_string();
                let amount: f64 = amount.expect_scalar()?.try_into()?;
                balance.entry(resource).or_default().1 += amount;
            }
        }

        let country_type = obj.expect_first_scalar("type")?.as_string();

        let map_color = [
            ((idx * 37) % 256) as u8,
            ((idx * 71) % 256) as u8,
            ((idx * 127) % 256) as u8,
        ];
        let nation_color = [0, 0, 0];

        return Ok(Country {
            idx,
            name,
            country_type,
            victory_rank,
            victory_score,
            tech_power,
            fleet_size,
            empire_size,
            num_sapient_pops,
            capital,
            balance,
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

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct War {
//     pub name: String,
//     pub attackers: Vec<u32>,
//     pub defenders: Vec<u32>,
//     pub attacker_losses: i64,
//     pub defender_losses: i64,
//     pub start_date: StellarisDate,
//     pub end_date: Option<StellarisDate>,
//     pub result: Option<WarResult>,
// }

// impl War {
//     pub fn player_attackers(&self, player_tags: &Vec<u32>) -> Vec<u32> {
//         return self
//             .attackers
//             .iter()
//             .filter(|a| player_tags.contains(a))
//             .cloned()
//             .collect();
//     }
//     pub fn player_defenders(&self, player_tags: &Vec<u32>) -> Vec<u32> {
//         return self
//             .defenders
//             .iter()
//             .filter(|d| player_tags.contains(d))
//             .cloned()
//             .collect();
//     }

//     /** An evaluation of how 'significant' the war probably is  */
//     pub fn war_scale(&self, players: &Vec<u32>) -> i64 {
//         let player_attackers = self.player_attackers(players).len();
//         let player_defenders = self.player_defenders(players).len();
//         let casualties = self.attacker_losses + self.defender_losses;
//         if player_attackers <= 1 || player_defenders <= 1 {
//             return casualties;
//         }

//         return casualties * min(player_attackers, player_defenders) as i64;
//     }

//     pub fn is_player_war(&self, players: &Vec<u32>) -> bool {
//         return self.attackers.iter().any(|a| players.contains(a))
//             && self.defenders.iter().any(|d| players.contains(d));
//     }

//     /// ?????There are nonexistant wars in save files, so keep an option?????
//     pub fn from_parsed_obj(obj: &RawPDXObject) -> Result<Option<War>> {
//         let mut attackers: Vec<String> = Vec::new();
//         let mut defenders: Vec<String> = Vec::new();
//         let mut earliest_date: Option<StellarisDate> = None;
//         let mut latest_date: Option<StellarisDate> = None;
//         for (date, value) in obj
//             .get_first_obj("history")
//             .ok_or(anyhow!("No history in war"))?
//             .iter_all_KVs()
//             .filter_map(|(k, v)| {
//                 Some((
//                     k.as_date()?,
//                     v.as_object().expect("date events should be objects"),
//                 ))
//             })
//         {
//             for (event, value) in value.iter_all_KVs() {
//                 match (event.0, value) {
//                     ("add_attacker", RawPDXValue::Scalar(value)) => {
//                         attackers.push(value.as_string());
//                         match earliest_date {
//                             None => earliest_date = Some(date),
//                             Some(prev_date) if date < prev_date => earliest_date = Some(date),
//                             _ => {}
//                         }
//                     }
//                     ("add_defender", RawPDXValue::Scalar(value)) => {
//                         defenders.push(value.as_string());
//                         match earliest_date {
//                             None => earliest_date = Some(date),
//                             Some(prev_date) if date < prev_date => earliest_date = Some(date),
//                             _ => {}
//                         }
//                     }
//                     ("rem_attacker", RawPDXValue::Scalar(_))
//                     | ("rem_defender", RawPDXValue::Scalar(_)) => match latest_date {
//                         None => latest_date = Some(date),
//                         Some(prev_date) if prev_date < date => latest_date = Some(date),
//                         _ => {}
//                     },
//                     _ => {}
//                 }
//             }
//         }
//         let Some(start_date) = earliest_date else {
//             return Ok(None);
//         };

//         let mut attacker_losses: i64 = 0;
//         let mut defender_losses: i64 = 0;
//         for (key, value) in obj.iter_all_KVs() {
//             if key.0 != "participants" {
//                 continue;
//             }
//             let RawPDXValue::Object(obj) = value else {
//                 // this shouldn't happen?
//                 continue;
//             };

//             let Some(tag) = obj.get_first_scalar("tag") else {
//                 continue;
//             };
//             let Some(losses) = obj.get_first_object_at_path(["losses", "members"]) else {
//                 continue;
//             };
//             let losses: i64 = losses
//                 .iter_values()
//                 .filter_map(RawPDXValue::as_scalar)
//                 .filter_map(RawPDXScalar::as_int)
//                 .sum();
//             if attackers.contains(&tag.as_string()) {
//                 attacker_losses += losses;
//             } else if defenders.contains(&tag.as_string()) {
//                 defender_losses += losses;
//             }
//         }

//         let name = obj
//             .get_first_scalar("name")
//             .ok_or(anyhow!("No name in war"))?
//             .as_string();
//         return Ok(Some(War {
//             name: name.clone(),
//             attackers: todo!(),
//             defenders: todo!(),
//             attacker_losses,
//             defender_losses,
//             start_date,
//             end_date: latest_date,
//             result: match obj.get_first_scalar("outcome") {
//                 Some(RawPDXScalar("1")) => Some(WarResult::WhitePeace),
//                 Some(RawPDXScalar("2")) => Some(WarResult::AttackerVictory),
//                 Some(RawPDXScalar("3")) => Some(WarResult::DefenderVictory),
//                 _ => None,
//             },
//         }));
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    pub all_nations: HashMap<u32, Country>,
    /// `tag: playername`
    pub player_tags: HashMap<u32, String>,
    pub galactic_objects: Vec<GalacticObject>,
    pub dlc: Vec<String>,
    pub date: StellarisDate,
    pub multiplayer: bool,
    pub galaxy_radius: f64,
    // pub player_wars: Vec<War>,
    pub game_mod: Mod,
}

impl SaveGame {
    pub fn player_nations(&self) -> impl Iterator<Item = (&String, &Country)> {
        return self
            .player_tags
            .iter()
            .filter_map(|(tag, player)| Some((player, self.all_nations.get(tag)?)));
    }

    pub fn new_parser(raw_save: &RawPDXObject) -> Result<SaveGame, anyhow::Error> {
        let all_nations = raw_save
            .expect_first_obj("country")?
            .iter_all_KVs()
            .map(|(k, v)| {
                let k: u32 = k.try_into()?;
                let v = Country::from_parsed_obj(k, v.expect_object()?)?;
                return Ok((k, v));
            })
            .collect::<Result<_, anyhow::Error>>()?;
        let player_tags: Vec<&RawPDXObject> = raw_save
            .expect_first_obj("player")?
            .iter_values()
            .map(RawPDXValue::expect_object)
            .collect::<Result<Vec<_>, _>>()?;
        let player_tags: HashMap<u32, String> = player_tags
            .iter()
            .map(|player_obj| {
                let name = player_obj.expect_first_scalar("name")?.as_string();
                let country_idx = player_obj.expect_first_scalar("country")?.try_into()?;
                return Ok((country_idx, name));
            })
            .collect::<Result<_, anyhow::Error>>()?;

        let mut galactic_objects = raw_save
            .expect_first_obj("galactic_object")?
            .iter_all_KVs()
            .enumerate()
            .map(|(i, (k, v))| {
                let k: u32 = k.try_into()?;
                if i != k as usize {
                    return Err(anyhow!(
                        "Expected consistently incrementing galactic object indices."
                    ));
                }
                return v.expect_object()?.try_into();
            })
            .collect::<Result<Vec<GalacticObject>, anyhow::Error>>()?;

        let planets = raw_save
            .expect_first_obj("planet")?
            .iter_all_KVs()
            .enumerate()
            .map(|(i, (k, v))| {
                let k: u32 = k.try_into()?;
                if i != k as usize {
                    return Err(anyhow!(
                        "Expected consistently incrementing planet indices."
                    ));
                }
                return v.expect_object()?.try_into();
            })
            .collect::<Result<Vec<Planet>, anyhow::Error>>()?;

        'systems_loop: for system in &mut galactic_objects {
            // First see if there are colonies. If there are, they decide ownership.
            // TODO: total wars may confuse this method
            if let Some(colony) = system.colonies.first() {
                let Some(colony) = planets.get(*colony as usize) else {
                    // TODO: this suggests inconsistency in the save file
                    // so we probably want to display some warning.
                    continue 'systems_loop;
                };
                let Some(owner) = colony.owner.or(colony.controller) else {
                    // TODO: colonies should have owners, if they do not then something is wrong.
                    // so we probably want to display some warning.
                    continue 'systems_loop;
                };
                system.map_owner = Some(owner);
                continue 'systems_loop;
            }

            // Otherwise, check if any of the planets has owner set
            for planet in &system.planets {
                let Some(planet) = planets.get(*planet as usize) else {
                    // TODO: this suggests inconsistency in the save file
                    // so we probably want to display some warning.
                    continue;
                };
                if let Some(owner) = planet.owner {
                    system.map_owner = Some(owner);
                    continue 'systems_loop;
                }
            }

            // And finally just if controller is set
            for planet in &system.planets {
                let Some(planet) = planets.get(*planet as usize) else {
                    // TODO: this suggests inconsistency in the save file
                    // so we probably want to display some warning.
                    continue;
                };
                if let Some(controller) = planet.controller {
                    system.map_owner = Some(controller);
                    continue 'systems_loop;
                }
            }
        }

        let dlc: Vec<String> = raw_save
            .expect_first_obj("required_dlcs")?
            .iter_values()
            .filter_map(|v| match v {
                RawPDXValue::Scalar(scalar) => Some(scalar.as_string()),
                _ => None,
            })
            .collect();

        let galaxy_radius = raw_save.expect_first_scalar("galaxy_radius")?.try_into()?;
        let date = raw_save.expect_first_scalar("date")?.try_into()?;

        return Ok(SaveGame {
            all_nations,
            player_tags,
            galactic_objects,
            dlc,
            date,
            multiplayer: raw_save
                .get_first_scalar("multi_player")
                .and_then(|mp| mp.as_bool())
                .unwrap_or(false),
            galaxy_radius,
            game_mod: Mod::Vanilla,
        });
    }
}
