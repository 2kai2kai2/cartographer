use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{anyhow, Context};
use pdx_parser_core::raw_parser::RawPDXObject;

use crate::utils::{lines_without_comments, moddable_read_dir, read_cp1252};

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
    fn get_all_tags(
        steam_dir: impl AsRef<Path>,
        mod_dir: Option<impl AsRef<Path>>,
    ) -> anyhow::Result<HashSet<String>> {
        let steam_common = steam_dir.as_ref().join("common");
        let mod_common = mod_dir
            .as_ref()
            .map(|mod_dir| mod_dir.as_ref().join("common"));
        let mut out: HashSet<String> = HashSet::new();

        let tags_directory = moddable_read_dir("country_tags", &steam_common, mod_common.as_ref())?;

        for tags_file in tags_directory {
            let tags = read_cp1252(&tags_file.path)?;
            let tags: String = lines_without_comments(&tags).collect();
            let (_, tags) = RawPDXObject::parse_object_inner(&tags).ok_or(anyhow!(
                "Failed to parse country_tags file {}",
                &tags_file.name
            ))?;
            for (k, _) in tags.iter_all_KVs() {
                let k = k.as_string();
                if k.len() != 3 || !k.is_ascii() {
                    eprintln!(
                        "WARNING: Skipping line in {} as the tag {k} was not 3 ascii characters long.",
                        &tags_file.path.display()
                    );
                }

                if k == "REB" || k == "PIR" || k == "NAT" || k == "SYN" {
                    continue;
                }

                out.insert(k);
            }
        }

        return Ok(out);
    }
    pub fn read_all_countries(
        steam_dir: impl AsRef<Path>,
        mod_dir: Option<impl AsRef<Path>>,
    ) -> anyhow::Result<HashMap<String, CountryHistory>> {
        let all_tags = CountryHistory::get_all_tags(&steam_dir, mod_dir.as_ref()).context(
            "While checking `country_tags` to determine the set of valid tags in this target.",
        )?;
        let mut out: HashMap<String, CountryHistory> = HashMap::new();

        // defines tags and what they refer to
        let country_history_files =
            moddable_read_dir("history/countries", &steam_dir, mod_dir.as_ref())?;

        for country_file in country_history_files {
            let Some((tag, _)) = country_file.name.split_at_checked(3) else {
                eprintln!("WARNING: country history file name not long enough, skipping.");
                continue;
            };
            if !tag.is_ascii() || tag.to_ascii_uppercase() != tag {
                eprintln!("WARNING: country history tag {tag} invalid, skipping.");
                continue;
            }
            if !all_tags.contains(tag) {
                // should mean modded game that excludes vanilla tags
                continue;
            }

            let country_history = read_cp1252(country_file.path)?;
            let country_history = match CountryHistory::parse_file(&country_history) {
                Ok(country_history) => country_history,
                Err(err) => {
                    // while vanilla always has all out expected fields set, that is not required and some mods do not.
                    // for now just skip the bad ones and hope they're not important
                    eprintln!(
                        "WARNING: failed to parse country history file for {}, skipping it:",
                        country_file.name
                    );
                    for cause in err.chain() {
                        eprintln!("\t{cause}");
                    }
                    continue;
                }
            };
            out.insert(tag.to_string(), country_history);
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
