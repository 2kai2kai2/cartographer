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

pub struct ModdableDir {
    pub default: PathBuf,
    pub modded: Option<PathBuf>,
}
impl ModdableDir {
    pub fn new(default: impl AsRef<Path>, modded: Option<impl AsRef<Path>>) -> Self {
        return ModdableDir {
            default: default.as_ref().to_path_buf(),
            modded: modded.map(|modded| modded.as_ref().to_path_buf()),
        };
    }

    pub fn join(&self, relative_path: impl AsRef<Path>) -> ModdableDir {
        return ModdableDir {
            default: self.default.join(&relative_path),
            modded: self
                .modded
                .as_ref()
                .map(|modded| modded.join(relative_path)),
        };
    }

    /// Reads (from `cp1252` encoding rather than utf8) a file,
    /// optionally trying a modded version of the file first.
    pub fn moddable_read_cp1252(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<String, std::io::Error> {
        if let Some(modded) = &self.modded {
            let modded_file = modded.join(&relative_path);
            if std::fs::exists(&modded_file)? {
                return from_cp1252(File::open(modded_file)?);
            }
        }
        let default_file = self.default.join(relative_path);
        return from_cp1252(File::open(default_file)?);
    }

    /// Reads a utf8 file, optionally trying a modded version of the file first.
    pub fn moddable_read_utf8(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<String, std::io::Error> {
        if let Some(modded) = &self.modded {
            let modded_file = modded.join(&relative_path);
            if std::fs::exists(&modded_file)? {
                return std::fs::read_to_string(modded_file);
            }
        }
        let default_file = self.default.join(relative_path);
        return std::fs::read_to_string(default_file);
    }

    /// Reads an image, optionally trying a modded version of the file first.
    pub fn moddable_read_image(
        &self,
        relative_path: &str,
    ) -> Result<image::DynamicImage, image::ImageError> {
        if let Some(modded) = &self.modded {
            let modded_file = modded.join(&relative_path);
            if std::fs::exists(&modded_file)? {
                return image::open(modded_file);
            }
        }
        let default_file = self.default.join(relative_path);
        return image::open(default_file);
    }

    /// Reads both the default and (optionally) mod directories and merges them with preference for mod directory
    pub fn moddable_read_dir(
        &self,
        relative_path: &str,
    ) -> Result<Vec<MergedDirEntry>, std::io::Error> {
        let mut out = HashMap::<OsString, MergedDirEntry>::new();
        let default_dir = self.default.join(relative_path);
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

        if let Some(mod_root) = &self.modded {
            let mod_dir = mod_root.join(relative_path);
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
}

pub struct MergedDirEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: std::fs::FileType,
}
