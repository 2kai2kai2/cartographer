use anyhow::Context;
use pdx_parser_core::{
    common_deserialize::UnbracketedKVs, text_deserialize::TextError, text_lexer::TextToken,
    TextDeserialize, TextDeserializer,
};
use std::collections::HashMap;
use tools::ModdableDir;

/// `game/in_game/map_data/default.map`
#[derive(Debug, TextDeserialize)]
#[no_brackets]
#[allow(dead_code)]
pub struct DefaultMap<'de> {
    /// Location of provinces image (usually `locations.png`)
    pub provinces: &'de str,
    /// Location of rivers image (usually `rivers.png`)
    pub rivers: &'de str,
    /// Heightmap: doesn't seem to be a real filepath?
    pub topology: &'de str,
    /// Location of adjacency csv. (usually `adjacencies.csv`)
    /// Only includes adjacencies like straits that cannot be computed from the locations image.
    pub adjacencies: &'de str,
    /// Location of continent/subcontinent/region/area/province definitions (not the setup directory)
    /// (usually `definitions.txt`)
    pub setup: &'de str,
    /// Location of ports csv (usually `ports.csv`)
    pub ports: &'de str,
    /// Location of location templates file, has initial location terrain/material/etc data (usually `location_templates.txt`)
    pub location_templates: &'de str,

    pub equator_y: f64,
    /// Whether the world wraps around east-west. True for real world-like maps.
    pub wrap_x: bool,

    /// Sound toll name -> location name
    pub sound_toll: HashMap<&'de str, &'de str>,

    /// Locations which are volcanoes
    pub volcanoes: Vec<&'de str>,
    /// Locations in earthquake zones
    pub earthquakes: Vec<&'de str>,
    /// Locations which are sea zones
    pub sea_zones: Vec<&'de str>,
    /// Locations which are lakes
    pub lakes: Vec<&'de str>,
    /// Locations which are impassable mountains
    pub impassable_mountains: Vec<&'de str>,
    /// Locations which are non-ownable, e.g. desert corridors
    pub non_ownable: Vec<&'de str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HexRgb(pub [u8; 3]);
impl<'de> TextDeserialize<'de> for HexRgb {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        match stream.expect_token()? {
            TextToken::UInt(decimal) => {
                // due to silliness in my parser, the hex may be treated as a decimal
                let decimal = decimal.to_string();
                let Ok(hex) = u32::from_str_radix(&decimal, 16) else {
                    return Err(TextError::IntegerOverflow);
                };
                let [_, r, g, b] = hex.to_be_bytes();
                return Ok((HexRgb([r, g, b]), stream));
            }
            TextToken::StringUnquoted(hex) => {
                let Ok(hex) = u32::from_str_radix(hex, 16) else {
                    return Err(TextError::IntegerOverflow);
                };
                let [_, r, g, b] = hex.to_be_bytes();
                return Ok((HexRgb([r, g, b]), stream));
            }
            _ => {
                return Err(TextError::UnexpectedToken);
            }
        }
    }
}
impl std::fmt::Display for HexRgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return write!(f, "#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2]);
        } else {
            return write!(f, "{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2]);
        }
    }
}

/// Parses `game/in_game/map_data/named_locations`
/// which contains the colors of all(?) the locations on the image at [`DefaultMap::provinces`].
pub fn parse_named_locations(dir: &ModdableDir) -> anyhow::Result<Vec<(String, HexRgb)>> {
    let files = dir.moddable_read_dir("game/in_game/map_data/named_locations")?;
    let files: Vec<_> = files
        .into_iter()
        .map(|entry| {
            if !entry.file_type.is_file() {
                return Ok(Vec::new()); // probably shouldn't be here, but ignore for now
            }
            let text = std::fs::read_to_string(&entry.path)?;
            let text = text.strip_prefix("\u{FEFF}").unwrap_or(&text); // remove BOM
            let items: UnbracketedKVs<String, HexRgb> =
                TextDeserializer::from_str(&text).parse().with_context(|| {
                    format!("While parsing named locations in {}", entry.path.display())
                })?;
            return Ok(items.unwrap());
        })
        .collect::<anyhow::Result<_>>()?;
    return Ok(files.into_iter().flatten().collect());
}
