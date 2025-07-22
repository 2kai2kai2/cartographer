use anyhow::{anyhow, Context, Result};
use image::{GenericImageView, RgbaImage};
use pdx_parser_core::raw_parser::{RawPDXObjectItem, RawPDXScalar, RawPDXValue};
use std::fs::File;

pub fn convert_flag_colors(gamefiles: &std::path::Path, target: &std::path::Path) -> Result<()> {
    let source = gamefiles.join("flags/colors.txt");
    let destination = target.join("colors.csv");

    let colors_txt = std::fs::read_to_string(&source)
        .context(format!("While reading in {}", source.display()))?;
    let colors_txt: String = crate::utils::lines_without_comments(&colors_txt).collect();

    let (rest, colors_txt) =
        pdx_parser_core::raw_parser::RawPDXObject::parse_object_inner(&colors_txt)
            .ok_or(anyhow!("Failed to parse `Stellaris/flags/colors.txt`"))?;
    assert!(rest.is_empty());

    let mut colors_out: Vec<(String, [u8; 9])> = Vec::new();

    let colors = colors_txt.expect_first_obj("colors")?;

    for (color_name, colors) in colors.iter_all_KVs() {
        let color_name = color_name.as_string();
        let mut it = colors.expect_object()?.0.iter();

        let mut flag_value = None;
        let mut map_value = None;
        let mut ship_value = None;

        while let Some(key) = it.next() {
            // I think technically this is supposed to be labeled types,
            // but we can parse it as a KV + a value
            let Some(value) = it.next() else {
                return Err(anyhow!("Found uneven number of entries for flag colors."));
            };
            let (key, RawPDXValue::Scalar(RawPDXScalar("rgb"))) = key.expect_kv()? else {
                return Err(anyhow!("Unexpected color data format."));
            };
            let key = key.as_string();
            let value = value.expect_value()?.expect_object()?;
            let &[r, g, b] = &value.0.as_slice() else {
                return Err(anyhow!("Unexpected length for rgb value"));
            };
            let r: u8 = r.try_into()?;
            let g: u8 = g.try_into()?;
            let b: u8 = b.try_into()?;

            match key.as_str() {
                "flag" => {
                    flag_value = Some([r, g, b]);
                }
                "map" => {
                    map_value = Some([r, g, b]);
                }
                "ship" => {
                    ship_value = Some([r, g, b]);
                }
                _ => (),
            }
        }
        let (Some(flag_value), Some(map_value), Some(ship_value)) =
            (flag_value, map_value, ship_value)
        else {
            return Err(anyhow!(
                "Not all expected color value types were present for {color_name}"
            ));
        };
        colors_out.push((
            color_name,
            [
                flag_value[0],
                flag_value[1],
                flag_value[2],
                map_value[0],
                map_value[1],
                map_value[2],
                ship_value[0],
                ship_value[1],
                ship_value[2],
            ],
        ));
    }

    // Now, save to file
    let csv: String = colors_out
        .into_iter()
        .map(|(name, value)| name + "," + value.map(|v| v.to_string()).join(",").as_str() + "\n")
        .collect();
    std::fs::write(&destination, csv)
        .context(format!("While writing to {}", destination.display()))?;

    return Ok(());
}

fn load_dds<P: AsRef<std::path::Path>>(path: P) -> Result<RgbaImage> {
    let image = std::fs::File::open(path)?;
    let image = ddsfile::Dds::read(image)?;
    let image = image_dds::image_from_dds(&image, 0)?;
    return Ok(image);
}

/// Makes `flag_parts.png` and `flag_parts.txt`
pub fn pack_flag_imgs(gamefiles: &std::path::Path, target: &std::path::Path) -> Result<()> {
    let source = gamefiles.join("flags");
    let flags_dir = std::fs::read_dir(source)?;
    let mut imgs = Vec::new();
    let mut names = Vec::new();
    for entry in flags_dir {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let category = entry.file_name();
        let category = category
            .to_str()
            .ok_or(anyhow!("Flag category directory name was not utf8"))?;
        for entry in std::fs::read_dir(entry.path())? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let filename = entry.file_name();
            let filename = filename.to_str().ok_or(anyhow!("File name was not utf8"))?;
            if !filename.ends_with(".dds") {
                continue;
            }

            let image = load_dds(entry.path())?;
            let image = if image.dimensions() != (128, 128) {
                image::imageops::resize(&image, 128, 128, image::imageops::FilterType::Triangle)
            } else {
                image
            };
            imgs.push(image);
            names.push(format!("{category}/{filename}"));
        }
    }

    let png_destination = target.join("flag_parts.png");
    let num_cols = 8;
    let num_rows = imgs.len().div_ceil(num_cols);
    let mut out_img = RgbaImage::new(128 * num_cols as u32, 128 * num_rows as u32);
    for (i, img) in imgs.iter().enumerate() {
        let row = i / num_cols;
        let col = i % num_cols;
        image::imageops::overlay(&mut out_img, img, col as i64 * 128, row as i64 * 128);
    }
    out_img.save(png_destination)?;

    let txt_destination = target.join("flag_parts.txt");
    std::fs::write(txt_destination, &names.join("\n"))?;
    return Ok(());
}

pub fn pack_flag_frames(gamefiles: &std::path::Path, target: &std::path::Path) -> Result<()> {
    let source = gamefiles.join("gfx/interface/flags");

    let default_frame_path = source.join("empire_flag_200_frame.dds");
    let default_frame = load_dds(default_frame_path)?;
    let default_frame =
        image::imageops::resize(&default_frame, 128, 128, image::imageops::Triangle);

    // TODO: when we have more, pack others

    let png_destination = target.join("flag_frames.png");
    default_frame.save(png_destination)?;
    return Ok(());
}
