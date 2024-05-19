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

pub struct FlagImages {
    tags: HashMap<String, usize>,
    images: image::RgbaImage,
}
impl FlagImages {
    fn read_flagfiles_txt(text: &str) -> HashMap<String, usize> {
        return text
            .split_ascii_whitespace()
            .enumerate()
            .map(|(i, tag)| (tag.to_string(), i))
            .collect();
    }

    pub fn new(flagfiles_txt: &str, flagfiles_png: image::RgbaImage) -> FlagImages {
        return FlagImages {
            tags: FlagImages::read_flagfiles_txt(&flagfiles_txt),
            images: flagfiles_png,
        };
    }

    pub fn get_normal_flag(&self, tag: &str) -> Option<image::SubImage<&image::RgbaImage>> {
        let index = *self.tags.get(tag)?;

        let x = 128 * (index as u32 % 16);
        let y = 128 * (index as u32 / 16);
        return Some(self.images.view(x, y, 128, 128));
    }
}
