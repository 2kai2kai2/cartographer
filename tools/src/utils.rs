use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub fn from_cp1252<T: Read>(buffer: T) -> Result<String, std::io::Error> {
    let mut text = "".to_string();
    DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(buffer)
        .read_to_string(&mut text)?;
    return Ok(text);
}

pub fn read_cp1252(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    return from_cp1252(File::open(path)?);
}

pub fn stdin_line() -> std::io::Result<String> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    return Ok(line);
}

pub fn lines_without_comments<'a>(input: &'a str) -> impl Iterator<Item = &'a str> {
    return input
        .lines()
        .map(|line| line.split('#').next().unwrap_or(line));
}

/// Reads (from `cp1252` encoding rather than utf8) a file,
/// optionally trying a modded version of the file first.
pub fn moddable_read_cp1252(
    relative_path: &str,
    default_root: impl AsRef<Path>,
    mod_root: Option<impl AsRef<Path>>,
) -> Result<String, std::io::Error> {
    if let Some(mod_root) = mod_root {
        let modded_file = mod_root.as_ref().join(relative_path);
        if std::fs::exists(&modded_file)? {
            return from_cp1252(File::open(modded_file)?);
        }
    }
    let default_file = default_root.as_ref().join(relative_path);
    return from_cp1252(File::open(default_file)?);
}

/// Reads a utf8 file, optionally trying a modded version of the file first.
pub fn moddable_read_utf8(
    relative_path: &str,
    default_root: impl AsRef<Path>,
    mod_root: Option<impl AsRef<Path>>,
) -> Result<String, std::io::Error> {
    if let Some(mod_root) = mod_root {
        let modded_file = mod_root.as_ref().join(relative_path);
        if std::fs::exists(&modded_file)? {
            return std::fs::read_to_string(modded_file);
        }
    }
    let default_file = default_root.as_ref().join(relative_path);
    return std::fs::read_to_string(default_file);
}

/// Reads an image, optionally trying a modded version of the file first.
pub fn moddable_read_image(
    relative_path: &str,
    default_root: impl AsRef<Path>,
    mod_root: Option<impl AsRef<Path>>,
) -> Result<image::DynamicImage, image::ImageError> {
    if let Some(mod_root) = mod_root {
        let modded_file = mod_root.as_ref().join(relative_path);
        if std::fs::exists(&modded_file)? {
            return image::open(modded_file);
        }
    }
    let default_file = default_root.as_ref().join(relative_path);
    return image::open(default_file);
}

pub struct MergedDirEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: std::fs::FileType,
}

/// Reads both the default and (optionally) mod directories and merges them with preference for mod directory
pub fn moddable_read_dir(
    relative_path: &str,
    default_root: impl AsRef<Path>,
    mod_root: Option<impl AsRef<Path>>,
) -> Result<Vec<MergedDirEntry>, std::io::Error> {
    let mut out = HashMap::<OsString, MergedDirEntry>::new();
    let default_dir = default_root.as_ref().join(relative_path);
    for entry in std::fs::read_dir(default_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let file_type = entry.file_type()?;

        out.insert(
            entry.file_name(),
            MergedDirEntry {
                name,
                path,
                file_type,
            },
        );
    }

    if let Some(mod_root) = mod_root {
        let mod_dir = mod_root.as_ref().join(relative_path);
        if std::fs::exists(&mod_dir)? {
            for entry in std::fs::read_dir(mod_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                let file_type = entry.file_type()?;

                out.insert(
                    entry.file_name(),
                    MergedDirEntry {
                        name,
                        path,
                        file_type,
                    },
                );
            }
        }
    }

    return Ok(out.into_values().collect());
}
