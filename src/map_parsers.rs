use anyhow::{anyhow, Result};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use image::{GenericImageView, Rgb};
use std::{collections::HashMap, fs::File, io::Read};

pub fn from_cp1252<T: Read>(buffer: T) -> Result<String, std::io::Error> {
    let mut text = "".to_string();
    DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(buffer)
        .read_to_string(&mut text)?;
    return Ok(text);
}

pub fn read_cp1252(path: &str) -> Result<String, std::io::Error> {
    return from_cp1252(File::open(path)?);
}

pub fn read_definition_csv(text: &String) -> Result<HashMap<Rgb<u8>, u64>> {
    let mut out: HashMap<Rgb<u8>, u64> = HashMap::new();
    for line in text.lines().skip(1) {
        let parts = line.split(';').collect::<Vec<&str>>();
        let [id, r, g, b, _name, x] = parts.as_slice() else {
            return Err(anyhow!("Invalid csv line {}", line));
        };
        if x != &"x" {
            continue; // the x seems to mark it as used?
        }

        let id: u64 = id.parse()?;
        let r: u8 = r.parse()?;
        let g: u8 = g.parse()?;
        let b: u8 = b.parse()?;

        out.insert(Rgb([r, g, b]), id);
    }

    return Ok(out);
}

fn get_province_list(text: &str, key: &str) -> Result<Vec<u64>> {
    return Ok(text
        .lines()
        .skip_while(|line| line.trim() != format!("{key} = {{"))
        .skip(1)
        .take_while(|line| line.trim() != "}")
        .map(|line| match line.split_once('#') {
            Some((valid, _)) => valid,
            None => line,
        })
        .flat_map(|line| line.split_ascii_whitespace())
        .map(|p| p.parse::<u64>())
        .collect::<Result<Vec<u64>, _>>()?);
}

pub fn read_wasteland_provinces(text: &str) -> Result<Vec<u64>> {
    return get_province_list(text, "impassable");
}

pub fn read_water_provinces(text: &str) -> Result<Vec<u64>> {
    return Ok(get_province_list(text, "sea_starts")?
        .into_iter()
        .chain(get_province_list(text, "lakes")?)
        .collect());
}

pub struct FlagImages {
    pub(crate) tags: HashMap<String, usize>,
    pub(crate) images: Vec<image::RgbaImage>,
}
impl FlagImages {
    pub fn read_flagfiles_txt(text: &str) -> Result<HashMap<String, usize>> {
        return text
            .split_ascii_whitespace()
            .skip(1)
            .enumerate()
            .map(|(index, item)| Some((item.strip_suffix(".tga")?.to_string(), index)))
            .collect::<Option<_>>()
            .ok_or(anyhow!("Flagfiles.txt was invalid"));
    }

    pub fn load_from_filesystem() -> Result<FlagImages> {
        let tags =
            FlagImages::read_flagfiles_txt(&read_cp1252("../resources/vanilla/flagfiles.txt")?)?;
        let img_count = tags.len().div_ceil(256);
        let images: Vec<image::RgbaImage> = (0..img_count)
            .map(|num| format!("../resources/vanilla/flagfiles_{}.tga", num))
            .map(|filepath| Ok(image::open(filepath)?.into_rgba8()))
            .collect::<Result<_>>()?;
        return Ok(FlagImages { tags, images });
    }

    pub fn get_normal_flag(&self, tag: &str) -> Option<image::SubImage<&image::RgbaImage>> {
        let index = *self.tags.get(tag)?;
        let img = self.images.get(index / 256)?;

        let x = 128 * ((index as u32 % 256) % 16);
        let y = 128 * ((index as u32 % 256) / 16);
        return Some(img.view(x, y, 128, 128));
    }
}
