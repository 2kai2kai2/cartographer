//! For values in `game/in_game/gfx/map/map_objects/*.txt` */

use pdx_parser_core::TextDeserialize;

#[derive(TextDeserialize)]
pub struct MapObject {
    pub name: String,
    pub clamp_to_water_level: bool,
    pub render_under_water: bool,
    pub generated_content: bool,
    pub layer: String,
    pub instances: Option<Vec<MapObjectInstance>>,
}

#[derive(TextDeserialize)]
pub struct MapObjectInstance {
    pub id: String,
    pub position: [f64; 3],
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
}
