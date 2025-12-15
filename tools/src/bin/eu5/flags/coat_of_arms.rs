use std::{cell::Cell, collections::HashMap};

use anyhow::Context;
use image::{RgbaImage, imageops};
use imageproc::definitions::HasBlack;
use pdx_parser_core::{
    TextDeserializer,
    common_deserialize::{self, NumericColor},
};
use tools::ModdableDir;

use crate::flags::{
    expression_parser::{VariableGet, VariableResolver},
    parser::{
        ColorPalette, ColorReference, ColorValue, RawCOAComponent, RawCOAInstance, RawCOASub,
        RawCoatOfArms, RawColoredEmblem, RawTexturedEmblem,
    },
};

pub struct Assets {
    root_dir: ModdableDir,
    palette: ColorPalette,
    colored_emblems: Cell<HashMap<String, RgbaImage>>,
    patterns: Cell<HashMap<String, RgbaImage>>,
    textured_emblems: Cell<HashMap<String, RgbaImage>>,
}
impl Assets {
    /// Will immediately load the palette, while images are loaded on demand
    pub fn new(root_dir: ModdableDir) -> anyhow::Result<Self> {
        let palette =
            root_dir.moddable_read_utf8("game/main_menu/common/named_colors/01_coa.txt")?;
        let palette: ColorPalette = TextDeserializer::from_str(&palette)
            .parse()
            .context("While parsing palette")?;
        return Ok(Assets {
            root_dir,
            palette,
            colored_emblems: Cell::new(HashMap::new()),
            patterns: Cell::new(HashMap::new()),
            textured_emblems: Cell::new(HashMap::new()),
        });
    }
    pub fn palette<'a>(&'a self) -> &'a ColorPalette {
        return &self.palette;
    }
    pub fn resolve_color_rgb(&self, color: &ColorValue) -> Option<image::Rgb<u8>> {
        return match color {
            ColorValue::Named(name) => self.get_rgb(&name),
            ColorValue::Numeric(NumericColor::Rgb(common_deserialize::Rgb(color))) => {
                Some(image::Rgb(*color))
            }
            ColorValue::Numeric(NumericColor::Hsv360(hsv)) => Some(image::Rgb(hsv.to_rgb().0)),
        };
    }
    /// Gets the color from the palette, if it exists. Returns `image::Rgb` rather than `common_deserialize::Rgb`
    pub fn get_rgb(&self, name: &str) -> Option<image::Rgb<u8>> {
        return self.palette.get_rgb(name).map(|rgb| image::Rgb(rgb.0));
    }
    /// In the files,
    /// - blue hue(240) -> color1
    /// - green hue(150) -> color2
    /// - magenta hue(330) -> color3
    ///
    /// though sometimes the values are slightly different, as well as blending
    pub fn get_colored_emblem(&self, name: &str) -> anyhow::Result<RgbaImage> {
        let mut colored_emblems = self.colored_emblems.take();
        if let Some(image) = colored_emblems.get(name) {
            return Ok(image.clone());
        }
        let dds = self
            .root_dir
            .moddable_open_file(&format!(
                "game/main_menu/gfx/coat_of_arms/colored_emblems/{name}"
            ))
            .with_context(|| format!("While opening colored emblem {name} from disk"))?;
        let dds = ddsfile::Dds::read(dds)
            .with_context(|| format!("While reading DDS-format colored emblem {name} from disk"))?;
        let image = image_dds::image_from_dds(&dds, 0).with_context(|| {
            format!("While decoding DDS-format colored emblem {name} from disk")
        })?;

        colored_emblems.insert(name.to_string(), image.clone());
        self.colored_emblems.set(colored_emblems);

        return Ok(image);
    }
    pub fn get_pattern(&self, name: &str) -> anyhow::Result<RgbaImage> {
        let mut patterns = self.patterns.take();
        if let Some(image) = patterns.get(name) {
            return Ok(image.clone());
        }
        let dds = self
            .root_dir
            .moddable_open_file(&format!("game/main_menu/gfx/coat_of_arms/patterns/{name}"))
            .with_context(|| format!("While opening pattern {name} from disk"))?;
        let dds = ddsfile::Dds::read(dds)
            .with_context(|| format!("While reading DDS-format pattern {name} from disk"))?;
        let image = image_dds::image_from_dds(&dds, 0)
            .with_context(|| format!("While decoding DDS-format pattern {name} from disk"))?;
        patterns.insert(name.to_string(), image.clone());
        self.patterns.set(patterns);

        return Ok(image);
    }
    /// In the files,
    /// - red (255, 0, 0) -> color1
    /// - yellow (255, 255, 0) -> color2
    /// - white (255, 255, 255) -> color3 (idk why)
    /// - magenta (255, 0, 255) -> ???
    pub fn get_textured_emblem(&self, name: &str) -> anyhow::Result<RgbaImage> {
        let mut textured_emblems = self.textured_emblems.take();
        if let Some(image) = textured_emblems.get(name) {
            return Ok(image.clone());
        }
        let dds = self
            .root_dir
            .moddable_open_file(&format!(
                "game/main_menu/gfx/coat_of_arms/textured_emblems/{name}"
            ))
            .with_context(|| format!("While opening textured emblem {name} from disk"))?;
        let dds = ddsfile::Dds::read(dds).with_context(|| {
            format!("While reading DDS-format textured emblem {name} from disk")
        })?;
        let image = image_dds::image_from_dds(&dds, 0).with_context(|| {
            format!("While decoding DDS-format textured emblem {name} from disk")
        })?;
        textured_emblems.insert(name.to_string(), image.clone());
        self.textured_emblems.set(textured_emblems);

        return Ok(image);
    }
}

#[derive(Debug)]
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
    fn colorize_pattern(colors: &[image::Rgb<u8>], pattern: &mut RgbaImage) {
        let color1 = colors.get(0).unwrap_or(&image::Rgb([0, 0, 0]));
        let color2 = colors.get(1).unwrap_or(&image::Rgb([0, 0, 0]));
        let color3 = colors.get(2).unwrap_or(&image::Rgb([0, 0, 0]));
        imageproc::map::map_colors_mut(pattern, |color| {
            // my guess at how this is calculated, should be accurate at least when not blending
            let proportion3 = color.0[2] as u32;
            let proportion2 = (color.0[1] as u32).saturating_sub(proportion3);
            let proportion1 = (color.0[0] as u32)
                .saturating_sub(proportion2)
                .saturating_sub(proportion3);
            assert!(proportion1 + proportion2 + proportion3 <= 255);
            let red = color1.0[0] as u32 * proportion1 / 255
                + color2.0[0] as u32 * proportion2 / 255
                + color3.0[0] as u32 * proportion3 / 255;
            let green = color1.0[1] as u32 * proportion1 / 255
                + color2.0[1] as u32 * proportion2 / 255
                + color3.0[1] as u32 * proportion3 / 255;
            let blue = color1.0[2] as u32 * proportion1 / 255
                + color2.0[2] as u32 * proportion2 / 255
                + color3.0[2] as u32 * proportion3 / 255;
            let color = image::Rgba([red as u8, green as u8, blue as u8, color.0[3]]);
            color
        });
    }
    /// Flags are rendered at 512 x 768 pixels, then scaled down when used.
    ///
    /// For top-level render, `override_colors` is empty.
    pub fn render(
        &self,
        cache: &Assets,
        override_colors: &[ColorValue],
        all_coa: &HashMap<String, CoatOfArms>,
    ) -> anyhow::Result<RgbaImage> {
        let mut color_values = Vec::new();
        let mut colors = Vec::new();
        let mut it_override_colors = override_colors.iter();
        let mut it_self_colors = self.colors.iter();
        while let Some(color) = it_override_colors.next().or(it_self_colors.next()) {
            color_values.push(color.clone());
            let color = cache
                .resolve_color_rgb(color)
                .ok_or_else(|| anyhow::anyhow!("Failed to resolve color {color:?}"))?;
            colors.push(color);
        }

        let mut img = match self.pattern {
            Some(ref pattern) => {
                let mut pattern = cache.get_pattern(pattern)?;
                Self::colorize_pattern(&colors, &mut pattern);
                pattern
            }
            None => RgbaImage::new(384, 256),
        };

        for component in &self.components {
            match component {
                COAComponent::ColoredEmblem(colored_emblem) => {
                    colored_emblem.render(cache, &colors, &mut img)?;
                }
                COAComponent::TexturedEmblem(textured_emblem) => {
                    textured_emblem.render(cache, &mut img)?;
                }
                COAComponent::Sub(sub) => {
                    sub.render(cache, &color_values, all_coa, &mut img)?;
                }
            }
        }
        Ok(img)
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ColoredEmblem {
    pub texture: String,
    pub color1: Option<ColorReference>,
    pub color2: Option<ColorReference>,
    pub color3: Option<ColorReference>,
    pub instance: Vec<Instance>,
    pub mask: Vec<u32>,
}
impl ColoredEmblem {
    pub fn from_parsed(
        raw: RawColoredEmblem,
        parent_variables: &VariableResolver,
    ) -> anyhow::Result<Self> {
        let texture = raw.texture;
        let color1 = raw.color1;
        let color2 = raw.color2;
        let color3 = raw.color3;
        let instance = raw
            .instance
            .into_iter()
            .map(|instance| Instance::from_parsed(instance, parent_variables))
            .collect::<anyhow::Result<_>>()
            .with_context(|| format!("In colored_emblem with texture {texture}"))?;
        let mask = raw.mask;

        return Ok(ColoredEmblem {
            texture,
            color1,
            color2,
            color3,
            instance,
            mask,
        });
    }

    fn colorize_emblem(colors: &[image::Rgb<u8>; 3], emblem: &mut RgbaImage) {
        let [color1, color2, color3] = colors;
        imageproc::map::map_colors_mut(emblem, |color| {
            // my guess at how this is calculated, should be accurate at least when not blending
            let [r, g, b, a] = color.0;
            let rgb = common_deserialize::Rgb([r, g, b]);

            let hue = rgb.to_hsv360().h.clamp(150.0, 330.0);
            if !hue.is_finite() {
                return image::Rgba::black();
            }
            let proportions = if hue < 240.0 {
                // interpolate between color1 and color2
                let distance = (240.0 - hue) / 90.0;
                [1.0 - distance, distance, 0.0]
            } else {
                // interpolate between color1 and color3
                let distance = (hue - 240.0) / 90.0;
                [1.0 - distance, 0.0, distance]
            };
            assert!(proportions.iter().sum::<f64>() <= 1.0);

            let red = color1.0[0] as f64 * proportions[0]
                + color2.0[0] as f64 * proportions[1]
                + color3.0[0] as f64 * proportions[2];
            let green = color1.0[1] as f64 * proportions[0]
                + color2.0[1] as f64 * proportions[1]
                + color3.0[1] as f64 * proportions[2];
            let blue = color1.0[2] as f64 * proportions[0]
                + color2.0[2] as f64 * proportions[1]
                + color3.0[2] as f64 * proportions[2];
            let color = image::Rgba([red as u8, green as u8, blue as u8, a]);
            color
        });
    }

    fn render(
        &self,
        cache: &Assets,
        parent_colors: &[image::Rgb<u8>],
        img: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> anyhow::Result<()> {
        let color1 = self
            .color1
            .as_ref()
            .map(|color_ref| {
                color_ref
                    .resolve_rgb(parent_colors, cache.palette())
                    .unwrap_or(image::Rgb::black())
            })
            .unwrap_or(image::Rgb::black());
        let color2 = self
            .color2
            .as_ref()
            .map(|color_ref| {
                color_ref
                    .resolve_rgb(parent_colors, cache.palette())
                    .unwrap_or(image::Rgb::black())
            })
            .unwrap_or(image::Rgb::black());
        let color3 = self
            .color3
            .as_ref()
            .map(|color_ref| {
                color_ref
                    .resolve_rgb(parent_colors, cache.palette())
                    .unwrap_or(image::Rgb::black())
            })
            .unwrap_or(image::Rgb::black());

        let mut emblem = cache.get_colored_emblem(&self.texture)?;
        Self::colorize_emblem(&[color1, color2, color3], &mut emblem);

        if self.instance.is_empty() {
            if emblem.dimensions() != img.dimensions() {
                emblem = image::imageops::resize(
                    &emblem,
                    img.width(),
                    img.height(),
                    image::imageops::Triangle,
                );
            }
            image::imageops::overlay(img, &emblem, 0, 0);
            return Ok(());
        }
        for instance in &self.instance {
            instance.overlay_instance(img, &emblem);
        }
        return Ok(());
    }
}

#[derive(Debug)]
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
    fn render(
        &self,
        cache: &Assets,
        img: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> anyhow::Result<()> {
        let texture = cache.get_textured_emblem(&self.texture)?;
        if self.instance.is_empty() {
            let texture = if texture.dimensions() != img.dimensions() {
                image::imageops::resize(
                    &texture,
                    img.width(),
                    img.height(),
                    image::imageops::Triangle,
                )
            } else {
                texture
            };
            imageops::overlay(img, &texture, 0, 0);
            return Ok(());
        }

        for instance in &self.instance {
            instance.overlay_instance(img, &texture);
        }
        return Ok(());
    }
}

#[derive(Debug)]
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
    fn render(
        &self,
        cache: &Assets,
        parent_colors: &[ColorValue],
        all_coa: &HashMap<String, CoatOfArms>,
        img: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> anyhow::Result<()> {
        let override_colors: Vec<ColorValue> = self
            .colors
            .iter()
            .map(|color_ref| color_ref.resolve_color_value(&parent_colors))
            .collect::<anyhow::Result<_>>()?;

        let sub_coa = all_coa.get(&self.parent).ok_or(anyhow::anyhow!(
            "Sub {} was not found in coat of arms",
            self.parent
        ))?;
        let sub = sub_coa.render(cache, &override_colors, all_coa)?;

        if self.instance.is_empty() {
            image::imageops::overlay(img, &sub, 0, 0);
            return Ok(());
        }

        for instance in &self.instance {
            instance.overlay_instance(img, &sub);
        }
        return Ok(());
    }
}

#[derive(Debug)]
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
    /// Using the position/scale of this instance, overlays the given component on the image.
    ///
    /// TODO: rotation
    pub fn overlay_instance(&self, img: &mut RgbaImage, component: &RgbaImage) {
        let new_width = self.scale[0] * img.width() as f64;
        let new_height = self.scale[1] * img.height() as f64;
        let x = self.position[0] * img.width() as f64 - new_width / 2.0;
        let y = self.position[1] * img.height() as f64 - new_height / 2.0;
        let component = imageops::resize(
            component,
            new_width as u32,
            new_height as u32,
            imageops::FilterType::Triangle,
        );
        imageops::overlay(img, &component, x as i64, y as i64);
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
