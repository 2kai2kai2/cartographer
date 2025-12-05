use std::collections::HashMap;

use pdx_parser_core::{
    Context, TextDeserialize, TextDeserializer, common_deserialize::SkipValue,
    text_deserialize::TextError, text_lexer::TextToken,
};

#[derive(Debug, Clone)]
pub enum VariableValue {
    Literal(f64),
    /// idk if this ever happens
    Variable(String),
    Expression(String),
}
impl<'de> TextDeserialize<'de> for VariableValue {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        match stream.next_token().ok_or(TextError::EOF)? {
            TextToken::StringUnquoted(value) => {
                // for some reason, sometimes the files have a `]` at the end of the number
                let value = value.strip_suffix(']').ok_or(TextError::UnexpectedToken)?;
                return Ok((
                    VariableValue::Literal(value.parse().map_err(|_| TextError::UnexpectedToken)?),
                    stream,
                ));
            }
            TextToken::Int(int) => Ok((VariableValue::Literal(int as f64), stream)),
            TextToken::UInt(uint) => Ok((VariableValue::Literal(uint as f64), stream)),
            TextToken::Float(float) => Ok((VariableValue::Literal(float), stream)),
            TextToken::Variable(var) => Ok((VariableValue::Variable(var.to_string()), stream)),
            TextToken::Expr(expr) => Ok((VariableValue::Expression(expr.to_string()), stream)),
            _ => Err(TextError::UnexpectedToken),
        }
    }
}

pub struct RawCoatOfArmsFile {
    pub variables: Vec<(String, VariableValue)>,
    pub template: HashMap<String, SkipValue>,
    pub coat_of_arms: HashMap<String, RawCoatOfArms>,
}
impl<'de> TextDeserialize<'de> for RawCoatOfArmsFile {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        let mut variables = Vec::new();
        let mut coat_of_arms = HashMap::new();
        let template = None;

        while let Some(peek) = stream.peek_token() {
            match peek {
                TextToken::Variable(var) => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let value: VariableValue = stream
                        .parse()
                        .with_context(|| format!("While parsing value for variable {var}"))?;
                    variables.push((var.to_string(), value));
                }
                TextToken::StringUnquoted("template") => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let SkipValue = stream.parse().context("While parsing value for template")?;
                }
                TextToken::StringUnquoted(text) => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone value, but just in case
                    };
                    let value: RawCoatOfArms = stream
                        .parse()
                        .with_context(|| format!("While parsing coat of arms value for {text}"))?;
                    coat_of_arms.insert(text.to_string(), value);
                }
                key => {
                    // idk, skip
                    let SkipValue = stream
                        .parse()
                        .with_context(|| format!("While skipping unexpected key {key}"))?;
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let SkipValue = stream.parse().with_context(
                        || format! {"While skipping value for unexpected key {key}"},
                    )?;
                }
            }
        }
        return Ok((
            RawCoatOfArmsFile {
                variables,
                template: template.unwrap_or_default(),
                coat_of_arms,
            },
            stream,
        ));
    }
}

pub struct RawCoatOfArms {
    pub variables: HashMap<String, VariableValue>,
    /// Typically the name of a dds file
    pub pattern: Option<String>,
    /// as `color1 = "red" color2 = "blue"`, converted in to a vec
    pub colors: Vec<ColorValue>,
    pub components: Vec<RawCOAComponent>,
}
impl<'de> TextDeserialize<'de> for RawCoatOfArms {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        stream.parse_token(TextToken::OpenBracket)?;

        let mut pattern = None;
        let mut variables = HashMap::new();
        let mut colors = Vec::new();
        let mut components = Vec::new();

        loop {
            let peek = stream
                .peek_token()
                .ok_or(TextError::EOF)
                .context("RawCoatOfArms should be terminated by `}`")?;
            match peek {
                TextToken::CloseBracket => {
                    stream.eat_token();
                    return Ok((
                        RawCoatOfArms {
                            pattern,
                            colors,
                            variables,
                            components,
                        },
                        stream,
                    ));
                }
                TextToken::Variable(var) => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let value: VariableValue = stream
                        .parse()
                        .with_context(|| format!("While parsing value for variable {var}"))?;
                    variables.insert(var.to_string(), value);
                }
                TextToken::StringUnquoted("pattern") => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    pattern = Some(stream.parse()?);
                }
                TextToken::StringUnquoted("colored_emblem") => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let value: RawColoredEmblem = stream
                        .parse()
                        .context("While parsing value for colored_emblem")?;
                    components.push(RawCOAComponent::ColoredEmblem(value));
                }
                TextToken::StringUnquoted("textured_emblem") => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let value: RawTexturedEmblem = stream
                        .parse()
                        .context("While parsing value for textured_emblem")?;
                    components.push(RawCOAComponent::TexturedEmblem(value));
                }
                TextToken::StringUnquoted("sub") => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let value: RawCOASub = stream.parse().context("While parsing value for sub")?;
                    components.push(RawCOAComponent::Sub(value));
                }
                TextToken::StringUnquoted(key) if key.starts_with("color") => {
                    stream.eat_token();
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue; // not sure why we would have a standalone variable value, but just in case
                    };
                    let value: ColorValue = stream
                        .parse()
                        .with_context(|| format!("While parsing value for {key}"))?;
                    colors.push(value);
                }
                key => {
                    let SkipValue = stream
                        .parse()
                        .with_context(|| format!("While skipping unexpected key {key}"))?;
                    let Ok(()) = stream.parse_token(TextToken::Equal) else {
                        continue;
                    };
                    let SkipValue = stream.parse().with_context(|| {
                        format!("While skipping value for unexpected key {key}")
                    })?;
                }
            }
        }
    }
}

pub enum RawCOAComponent {
    ColoredEmblem(RawColoredEmblem),
    TexturedEmblem(RawTexturedEmblem),
    Sub(RawCOASub),
}

#[derive(TextDeserialize)]
pub struct RawColoredEmblem {
    pub texture: String,
    pub color1: Option<ColorReference>,
    pub color2: Option<ColorReference>,
    pub color3: Option<ColorReference>,
    pub color4: Option<ColorReference>,
    pub color5: Option<ColorReference>,
    pub color6: Option<ColorReference>,
    #[multiple]
    pub instance: Vec<RawCOAInstance>,
    #[default(Vec::new())]
    pub mask: Vec<u32>,
}

#[derive(TextDeserialize)]
pub struct RawTexturedEmblem {
    /// Typically the name of a dds file
    pub texture: String,
    #[multiple]
    pub instance: Vec<RawCOAInstance>,
}

#[derive(TextDeserialize)]
pub struct RawCOASub {
    pub parent: String,
    pub color1: Option<ColorReference>,
    pub color2: Option<ColorReference>,
    pub color3: Option<ColorReference>,
    pub color4: Option<ColorReference>,
    pub color5: Option<ColorReference>,
    pub color6: Option<ColorReference>,
    #[multiple]
    pub instance: Vec<RawCOAInstance>,
    #[default(Vec::new())]
    pub mask: Vec<u32>,
}

#[derive(TextDeserialize)]
pub struct RawCOAInstance {
    #[default([VariableValue::Literal(0.0), VariableValue::Literal(0.0)])]
    pub position: [VariableValue; 2],
    #[default(VariableValue::Literal(0.0))]
    pub rotation: VariableValue,
    #[default([VariableValue::Literal(1.0), VariableValue::Literal(1.0)])]
    pub scale: [VariableValue; 2],
}

#[derive(Debug, PartialEq, Clone)]
pub enum ColorValue {
    Named(String),
    HSV360(f64, f64, f64),
}
impl<'de> TextDeserialize<'de> for ColorValue {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        let peek = stream.peek_token().ok_or(TextError::EOF)?;
        match peek {
            TextToken::StringUnquoted("hsv360") => {
                stream.eat_token();
                let Ok(()) = stream.parse_token(TextToken::OpenBracket) else {
                    return Err(TextError::UnexpectedToken);
                };
                let h: f64 = stream.parse()?;
                let s: f64 = stream.parse()?;
                let v: f64 = stream.parse()?;
                let Ok(()) = stream.parse_token(TextToken::CloseBracket) else {
                    return Err(TextError::UnexpectedToken);
                };
                return Ok((ColorValue::HSV360(h, s, v), stream));
            }
            TextToken::StringQuoted(color) | TextToken::StringUnquoted(color) => {
                stream.eat_token();
                return Ok((ColorValue::Named(color.to_string()), stream));
            }
            _ => return Err(TextError::UnexpectedToken),
        }
    }
}

/// Inside things like `colored_emblem` or `sub`, color fields are either a quoted string (color name) or a reference to the coat of arms' colors
pub enum ColorReference {
    /// e.g. `"red"`
    Color(ColorValue),
    /// e.g. `color1` => `Self::Reference(1)`
    Reference(usize),
}
impl<'de> TextDeserialize<'de> for ColorReference {
    fn take_text(
        mut stream: TextDeserializer<'de>,
    ) -> Result<(Self, TextDeserializer<'de>), TextError> {
        let peek = stream.peek_token().ok_or(TextError::EOF)?;
        match peek {
            TextToken::StringUnquoted(color) if color.starts_with("color") => {
                stream.eat_token();
                let color = color
                    .strip_prefix("color")
                    .ok_or(TextError::UnexpectedToken)?;
                let color =
                    usize::from_str_radix(color, 10).map_err(|_| TextError::UnexpectedToken)?;
                return Ok((ColorReference::Reference(color), stream));
            }
            _ => {
                let color: ColorValue = stream.parse()?;
                return Ok((ColorReference::Color(color), stream));
            }
        }
    }
}
