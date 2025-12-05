use anyhow::Context;

use crate::flags::{
    expression_parser::{VariableGet, VariableResolver},
    parser::{
        ColorReference, ColorValue, RawCOAComponent, RawCOAInstance, RawCOASub, RawCoatOfArms,
        RawColoredEmblem, RawTexturedEmblem,
    },
};

pub struct CoatOfArms {
    pub pattern: Option<String>,
    pub colors: Vec<ColorValue>,
    pub components: Vec<COAComponent>,
}
impl CoatOfArms {
    pub fn from_parsed(
        raw: RawCoatOfArms,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        let variables = parent_variables
            .with_new_scope_from_unresolved(raw.variables)
            .context("While resolving variables for COA")?;

        let pattern = raw.pattern;
        let colors = raw.colors;
        let components = raw
            .components
            .into_iter()
            .map(|component| COAComponent::from_parsed(component, &variables))
            .collect::<anyhow::Result<_>>()?;

        return Ok(CoatOfArms {
            pattern,
            colors,
            components,
        });
    }
}

pub enum COAComponent {
    ColoredEmblem(ColoredEmblem),
    TexturedEmblem(TexturedEmblem),
    Sub(Sub),
}
impl COAComponent {
    pub fn from_parsed(
        raw: RawCOAComponent,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        return match raw {
            RawCOAComponent::ColoredEmblem(raw) => {
                ColoredEmblem::from_parsed(raw, parent_variables).map(COAComponent::ColoredEmblem)
            }
            RawCOAComponent::TexturedEmblem(raw) => {
                TexturedEmblem::from_parsed(raw, parent_variables).map(COAComponent::TexturedEmblem)
            }
            RawCOAComponent::Sub(raw) => {
                Sub::from_parsed(raw, parent_variables).map(COAComponent::Sub)
            }
        };
    }
}

pub struct ColoredEmblem {
    pub texture: String,
    pub colors: Vec<ColorReference>,
    pub instance: Vec<Instance>,
    pub mask: Vec<u32>,
}
impl ColoredEmblem {
    pub fn from_parsed(
        raw: RawColoredEmblem,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        let texture = raw.texture;
        let colors = options_to_full_vec([
            raw.color1, raw.color2, raw.color3, raw.color4, raw.color5, raw.color6,
        ])
        .with_context(|| format!("For colors in colored_emblem with texture {texture}"))?;
        let instance = raw
            .instance
            .into_iter()
            .map(|instance| Instance::from_parsed(instance, parent_variables))
            .collect::<anyhow::Result<_>>()
            .with_context(|| format!("In colored_emblem with texture {texture}"))?;
        let mask = raw.mask;

        return Ok(ColoredEmblem {
            texture,
            colors,
            instance,
            mask,
        });
    }
}

pub struct TexturedEmblem {
    pub texture: String,
    pub instance: Vec<Instance>,
}
impl TexturedEmblem {
    pub fn from_parsed(
        raw: RawTexturedEmblem,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        let texture = raw.texture;
        let instance = raw
            .instance
            .into_iter()
            .map(|instance| Instance::from_parsed(instance, parent_variables))
            .collect::<anyhow::Result<_>>()?;

        return Ok(TexturedEmblem { texture, instance });
    }
}

pub struct Sub {
    pub parent: String,
    pub colors: Vec<ColorReference>,
    pub instance: Vec<Instance>,
    pub mask: Vec<u32>,
}
impl Sub {
    pub fn from_parsed(
        raw: RawCOASub,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        let parent = raw.parent;
        let colors = options_to_full_vec([
            raw.color1, raw.color2, raw.color3, raw.color4, raw.color5, raw.color6,
        ])?;
        let instance = raw
            .instance
            .into_iter()
            .map(|instance| Instance::from_parsed(instance, parent_variables))
            .collect::<anyhow::Result<_>>()?;
        let mask = raw.mask;

        return Ok(Sub {
            parent,
            colors,
            instance,
            mask,
        });
    }
}

pub struct Instance {
    pub position: [f64; 2],
    pub rotation: f64,
    pub scale: [f64; 2],
}
impl Instance {
    pub fn from_parsed(
        raw: RawCOAInstance,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        let [pos_x, pos_y] = raw.position;
        let position = [
            parent_variables.resolve(&pos_x)?,
            parent_variables.resolve(&pos_y)?,
        ];
        let rotation = parent_variables.resolve(&raw.rotation)?;
        let [scale_x, scale_y] = raw.scale;
        let scale = [
            parent_variables.resolve(&scale_x)?,
            parent_variables.resolve(&scale_y)?,
        ];

        Ok(Instance {
            position,
            rotation,
            scale,
        })
    }
}
impl Default for Instance {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
        }
    }
}

fn resolve_color_references(
    colors: impl IntoIterator<Item = Option<ColorReference>>,
    parent_colors: &[ColorValue],
) -> anyhow::Result<Vec<ColorValue>> {
    let mut out = Vec::new();
    for (i, color) in colors.into_iter().enumerate() {
        let Some(color) = color else {
            continue;
        };
        if i != out.len() {
            return Err(anyhow::anyhow!("Color references must not skip indices"));
        }
        match color {
            ColorReference::Color(color) => out.push(color),
            ColorReference::Reference(idx) => {
                let idx = idx.checked_sub(1).ok_or(anyhow::anyhow!(
                    "Color reference is indexed starting at 1, got unexpected 0"
                ))?;
                if idx >= parent_colors.len() {
                    let parent_len = parent_colors.len();
                    return Err(anyhow::anyhow!(
                        "Color reference {idx} is out of bounds (parent colors len is {parent_len})"
                    ));
                }
                out.push(parent_colors[idx].clone());
            }
        }
    }
    return Ok(out);
}

fn options_to_full_vec<T>(
    items: impl IntoIterator<Item = Option<T>>,
) -> Result<Vec<T>, anyhow::Error> {
    let mut out = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        if let Some(item) = item {
            if i != out.len() {
                return Err(anyhow::anyhow!("List not skip indices"));
            }
            out.push(item);
        }
    }
    return Ok(out);
}
