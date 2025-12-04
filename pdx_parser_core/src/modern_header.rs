use anyhow::{Context, anyhow};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum SaveFormat {
    SplitCompressedBinary = 5,
    SplitCompressedText = 4,
    UnifiedCompressedBinary = 3,
    UnifiedCompressedText = 2,
    UncompressedBinary = 1,
    UncompressedText = 0,
}

pub struct ModernHeader<'a> {
    pub save_format_version: u8,
    pub save_format: SaveFormat,
    /// Both the meta and gamestate
    pub all: &'a [u8],
    pub meta: &'a [u8],
    pub gamestate: &'a [u8],
}
impl<'a> ModernHeader<'a> {
    /// Returns `(gamestate, header)` if successful
    pub fn take(buffer: &'a [u8]) -> anyhow::Result<Self> {
        let buffer = buffer
            .strip_prefix(b"SAV0")
            .ok_or(anyhow!("Missing SAV0 prefix"))?;
        let (version, buffer) = buffer.split_first().ok_or(anyhow!(
            "Unexpected EOF while parsing save format version in header"
        ))?;
        let version = (*version as char)
            .to_digit(10)
            .ok_or(anyhow!("Invalid save format version"))? as u8;
        let buffer = buffer
            .strip_prefix(b"0")
            .ok_or(anyhow!("Unexpected EOF while parsing header"))?;

        // save format
        let (save_format, buffer) = buffer.split_first().ok_or(anyhow!(
            "Reached EOF while parsing save format type in header"
        ))?;
        let save_format = (*save_format as char)
            .to_digit(6)
            .with_context(|| format!("Invalid save format id '{}'", *save_format as char))?;
        let save_format = save_format
            .to_u8()
            .unwrap_or_else(|| unreachable!("We just parsed this"));
        let save_format = SaveFormat::from_u8(save_format)
            .unwrap_or_else(|| unreachable!("We parsed with radix 6"));

        // unknown value
        let (_, buffer) = buffer
            .split_first_chunk::<8>()
            .ok_or(anyhow!("Unexpected EOF while parsing header"))?;

        // length of meta section (0 when split format)
        let (meta_len, buffer) = buffer.split_first_chunk::<8>().ok_or(anyhow!(
            "Unexpected EOF while parsing meta length in header"
        ))?;
        let meta_len = str::from_utf8(meta_len).context("While parsing meta length in header")?;
        let meta_len =
            u32::from_str_radix(meta_len, 16).context("While parsing meta length in header")?;

        let buffer = if version == 2 {
            let (_, buffer) = buffer
                .split_first_chunk::<8>()
                .ok_or(anyhow!("Unexpected EOF while parsing header"))?;
            buffer
        } else {
            buffer
        };

        let buffer = if let Some(buffer) = buffer.strip_prefix(b"\n") {
            buffer
        } else if let Some(buffer) = buffer.strip_prefix(b"\r\n") {
            buffer
        } else {
            return Err(anyhow!("Expected newline at end of header"));
        };

        let (meta, gamestate) = buffer
            .split_at_checked(meta_len as usize)
            .ok_or(anyhow!("Unexpected EOF in the metadata buffer."))?;

        return Ok(ModernHeader {
            save_format_version: version,
            save_format,
            all: buffer,
            meta,
            gamestate,
        });
    }
}
