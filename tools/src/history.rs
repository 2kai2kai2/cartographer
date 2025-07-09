use std::{collections::HashMap, fs::File};

use anyhow::{anyhow, Context};

use crate::utils::{from_cp1252, lines_without_comments};

/// A file in steamfiles `Europa Universalis IV/history/countries`
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CountryHistory {
    pub government: String,
    // add_government_reform
    // government_rank
    pub technology_group: String,
    // unit_type
    pub religion: String,
    pub primary_culture: String,
    // add_accepted_culture
    pub capital: usize,
    // fixed_capital
    // religious factors
    // historical rivals
    // historical events
}
impl CountryHistory {
    pub fn read_all_countries(steam_dir: &str) -> anyhow::Result<HashMap<String, CountryHistory>> {
        let directory = std::fs::read_dir(format!("{steam_dir}/history/countries"))?;
        let mut out: HashMap<String, CountryHistory> = HashMap::new();
        for file in directory {
            let file = file?;
            let file_name = file.file_name();
            let Some(file_name) = file_name.to_str() else {
                return Err(anyhow!("Failed to decode file name"));
            };
            let (tag, _) = file_name.split_at(3);
            if !tag.chars().all(|c| c.is_ascii_uppercase()) {
                return Err(anyhow!("Invalid tag '{tag}' in file name '{file_name}'"));
            }
            if tag == "REB" || tag == "PIR" || tag == "NAT" || tag == "SYN" {
                continue;
            }

            let file_text = from_cp1252(File::open(file.path())?)?;
            let country = CountryHistory::parse_file(&file_text)
                .context(format!("While reading {file_name}"))?;
            out.insert(tag.to_string(), country);
        }
        return Ok(out);
    }
    pub fn parse_file(text: &str) -> anyhow::Result<CountryHistory> {
        let text: String = lines_without_comments(text)
            .collect::<Vec<&str>>()
            .join("\n");

        let (_, obj) = pdx_parser_core::raw_parser::RawPDXObject::parse_object_inner(&text)
            .ok_or(anyhow!("Failed to parse RawPDXObject for history file"))?;

        let government = obj.get_first_as_string("government").ok_or(anyhow!(
            "Field `government` missing in country history file."
        ))?;
        let technology_group = obj.get_first_as_string("technology_group").ok_or(anyhow!(
            "Field `technology_group` missing in country history file."
        ))?;
        let religion = obj
            .get_first_as_string("religion")
            .ok_or(anyhow!("Field `religion` missing in country history file."))?;
        let primary_culture = obj.get_first_as_string("primary_culture").ok_or(anyhow!(
            "Field `primary_culture` missing in country history file."
        ))?;
        let capital = obj
            .get_first_as_int("capital")
            .ok_or(anyhow!("Field `capital` missing in country history file."))?
            as usize;

        return Ok(CountryHistory {
            government,
            technology_group,
            religion,
            primary_culture,
            capital,
        });
    }
}
