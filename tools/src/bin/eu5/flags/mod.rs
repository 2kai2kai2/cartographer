use std::collections::HashMap;

use anyhow::Context;
use image::{DynamicImage, GenericImage, RgbImage};
use pdx_parser_core::TextDeserializer;
use tools::ModdableDir;

use crate::flags::{
    coat_of_arms::{Assets, CoatOfArms},
    expression_parser::VariableScope,
    parser::RawCoatOfArmsFile,
};

mod coat_of_arms;
mod expression_parser;
mod parser;

pub fn do_flags(gamefiles: &ModdableDir) -> anyhow::Result<(RgbImage, Vec<String>)> {
    let coa_dir = gamefiles.moddable_read_dir("game/main_menu/common/coat_of_arms/coat_of_arms")?;

    let mut coats_of_arms = HashMap::new();
    for entry in coa_dir {
        if !entry.file_type.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&entry.path)?;
        let mut deser = TextDeserializer::from_str(&text);
        let coa_file: RawCoatOfArmsFile = deser
            .parse()
            .with_context(|| format!("While parsing COA file {}", entry.name))?;
        let variable_resolver = VariableScope::from_unresolved(coa_file.variables)
            .with_context(|| {
                format!(
                    "While resolving file-level variables in COA file {}",
                    entry.name
                )
            })?
            .into_resolver_root();
        for (name, coa) in coa_file.coat_of_arms {
            let coa = CoatOfArms::from_parsed(coa, &variable_resolver)
                .with_context(|| format!("While rendering COA {name}"))?;
            coats_of_arms.insert(name, coa);
        }
    }

    let mut include_sorted: Vec<_> = coats_of_arms
        .keys()
        .cloned()
        .filter(|key| key.len() == 3 && key.bytes().all(|b| b.is_ascii_uppercase()))
        .collect();
    include_sorted.sort();

    let mut out_img = RgbImage::new(150, include_sorted.len() as u32 * 100);
    let assets = Assets::new(gamefiles.clone()).context("While loading initial assets")?;

    for (i, name) in include_sorted.iter().enumerate() {
        let coa = coats_of_arms
            .get(name)
            .unwrap_or_else(|| unreachable!("Keys came from the map"));
        let rendered = coa
            .render(&assets, &[], &coats_of_arms)
            .with_context(|| format!("While rendering COA for {name} (idx {i})"))?;
        let rendered = DynamicImage::ImageRgba8(rendered).to_rgb8();
        let resized = image::imageops::resize(&rendered, 150, 100, image::imageops::Triangle);
        out_img.copy_from(&resized, 0, i as u32 * 100)?;
    }
    return Ok((out_img, include_sorted));
}
