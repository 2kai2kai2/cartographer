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
    pub save_format: SaveFormat,
    pub meta: &'a [u8],
}
impl<'a> ModernHeader<'a> {
    /// Returns `(gamestate, header)` if successful
    pub fn take(buffer: &'a [u8]) -> Option<(&'a [u8], Self)> {
        let buffer = buffer.strip_prefix(b"SAV0")?;
        let (_, buffer) = buffer.split_first()?;
        let buffer = buffer.strip_prefix(b"0")?;

        // save format
        let (save_format, buffer) = buffer.split_first()?;
        let save_format = (*save_format as char).to_digit(6)?;
        let save_format = SaveFormat::from_u8(save_format.to_u8()?)?;

        // unknown value
        let (_, buffer) = buffer.split_first_chunk::<8>()?;

        // length of meta section (0 when split format)
        let (meta_len, buffer) = buffer.split_first_chunk::<8>()?;
        let meta_len = str::from_utf8(meta_len).ok()?;
        let meta_len = u32::from_str_radix(meta_len, 16).ok()?;

        let (meta, buffer) = buffer.split_at_checked(meta_len as usize)?;

        let out = ModernHeader { save_format, meta };
        return Some((buffer, out));
    }
}
