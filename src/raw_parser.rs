use crate::eu4_date::EU4Date;

#[inline]
fn is_eu4_delimiter(c: char) -> bool {
    c.is_whitespace() || c == '{' || c == '}' || c == '='
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum EU4Scalar {
    Int(i64),
    Float(f64),
    Date(EU4Date),
    Bool(bool),
    Str(String),
}
impl<'a> From<RawEU4Scalar<'a>> for EU4Scalar {
    fn from(value: RawEU4Scalar<'a>) -> Self {
        if value.0 == "yes" {
            return EU4Scalar::Bool(true);
        } else if value.0 == "no" {
            return EU4Scalar::Bool(false);
        } else if let Some(quoted) = value.0.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            return EU4Scalar::Str(quoted.to_string());
        } else if let Ok(int) = value.0.parse::<i64>() {
            return EU4Scalar::Int(int);
        } else if let Ok(float) = value.0.parse::<f64>() {
            return EU4Scalar::Float(float);
        } else if let Ok(date) = value.0.parse::<EU4Date>() {
            return EU4Scalar::Date(date);
        } else {
            return EU4Scalar::Str(value.0.to_string());
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawEU4Scalar<'a>(pub &'a str);
impl<'a> RawEU4Scalar<'a> {
    pub fn as_int(&self) -> Option<i64> {
        return self.0.parse().ok();
    }

    pub fn as_float(&self) -> Option<f64> {
        return self.0.parse().ok();
    }

    pub fn as_date(&self) -> Option<EU4Date> {
        return self.0.parse().ok();
    }

    pub fn as_bool(&self) -> Option<bool> {
        return match self.0 {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        };
    }

    pub fn as_string(&self) -> String {
        return self
            .0
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(self.0)
            .to_string();
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawEU4ObjectItem<'a> {
    KV(RawEU4Scalar<'a>, RawEU4Value<'a>),
    Value(RawEU4Value<'a>),
}
impl<'a> From<RawEU4Value<'a>> for RawEU4ObjectItem<'a> {
    #[inline]
    fn from(value: RawEU4Value<'a>) -> Self {
        return RawEU4ObjectItem::Value(value);
    }
}
impl<'a> RawEU4ObjectItem<'a> {
    /// Should start on the first character of the value; will not trim whitespace
    pub fn take(input: &'a str) -> Option<(&'a str, RawEU4ObjectItem<'a>)> {
        match RawEU4Value::take(input)? {
            (rest, RawEU4Value::Scalar(scalar)) => {
                if let Some((rest, obj)) = rest
                    .strip_prefix('{')
                    .and_then(RawEU4Object::parse_object_inner)
                {
                    // sometimes, they just skip the '=' on a kv pair for some reason
                    // only accept this if there is no whitespace inbetween
                    return Some((
                        rest,
                        RawEU4ObjectItem::KV(scalar, RawEU4Value::Object(obj).into()),
                    ));
                }

                let Some(rest) = rest.trim_start().strip_prefix('=') else {
                    // it's just a value
                    return Some((rest, RawEU4Value::Scalar(scalar).into()));
                };

                // after an '='
                let (rest, value) = RawEU4Value::take(rest.trim_start())?;
                return Some((rest, RawEU4ObjectItem::KV(scalar, value)));
            }
            (rest, value) => return Some((rest, value.into())),
        };
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawEU4Object<'a>(Vec<RawEU4ObjectItem<'a>>);
impl<'a> RawEU4Object<'a> {
    /// Will end after a '}' (returns rest starting with the next character) or EOF
    pub fn parse_object_inner(input: &'a str) -> Option<(&'a str, RawEU4Object<'a>)> {
        let mut out: Vec<RawEU4ObjectItem<'a>> = Vec::new();
        let mut rest: &'a str = input;

        loop {
            rest = rest.trim_start();
            if rest.len() == 0 {
                return Some((rest, RawEU4Object(out)));
            } else if let Some(rest) = rest.strip_prefix('}') {
                return Some((rest, RawEU4Object(out)));
            }

            let (r, item) = RawEU4ObjectItem::take(rest)?;
            rest = r;
            out.push(item);
        }
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &RawEU4Value<'a>> {
        return self.0.iter().filter_map(|v| {
            if let RawEU4ObjectItem::Value(value) = v {
                Some(value)
            } else {
                None
            }
        });
    }

    pub fn iter_all_KVs(&self) -> impl Iterator<Item = (&RawEU4Scalar<'a>, &RawEU4Value<'a>)> {
        return self.0.iter().filter_map(|item| {
            if let RawEU4ObjectItem::KV(key, value) = item {
                Some((key, value))
            } else {
                None
            }
        });
    }

    /// Gets the first value for the specified key
    pub fn get_first(&self, key: &str) -> Option<&RawEU4Value<'a>> {
        return self
            .iter_all_KVs()
            .find(|(k, _)| k.0 == key)
            .map(|(_, v)| v);
    }

    pub fn get_first_obj(&self, key: &str) -> Option<&RawEU4Object<'a>> {
        return self.iter_all_KVs().find_map(|(k, v)| {
            if k.0 != key {
                None
            } else if let RawEU4Value::Object(obj) = v {
                Some(obj)
            } else {
                None
            }
        });
    }

    pub fn get_first_scalar(&self, key: &str) -> Option<&RawEU4Scalar<'a>> {
        return self.iter_all_KVs().find_map(|(k, v)| {
            if k.0 != key {
                None
            } else if let RawEU4Value::Scalar(scalar) = v {
                Some(scalar)
            } else {
                None
            }
        });
    }
    pub fn get_first_as_int(&self, key: &str) -> Option<i64> {
        return self.get_first(key)?.as_scalar()?.as_int();
    }
    pub fn get_first_as_float(&self, key: &str) -> Option<f64> {
        return self.get_first(key)?.as_scalar()?.as_float();
    }
    pub fn get_first_as_date(&self, key: &str) -> Option<EU4Date> {
        return self.get_first(key)?.as_scalar()?.as_date();
    }
    pub fn get_first_as_bool(&self, key: &str) -> Option<bool> {
        return self.get_first(key)?.as_scalar()?.as_bool();
    }
    pub fn get_first_as_string(&self, key: &str) -> Option<String> {
        return Some(self.get_first(key)?.as_scalar()?.as_string());
    }

    pub fn get_first_at_path<const N: usize>(&self, path: [&str; N]) -> Option<&RawEU4Value<'a>> {
        let mut obj = self;
        for key in path.into_iter().take(N - 1) {
            obj = obj.get_first_obj(key)?;
        }
        return obj.get_first(path.last()?);
    }

    pub fn get_first_scalar_at_path<const N: usize>(
        &self,
        path: [&str; N],
    ) -> Option<&RawEU4Scalar<'a>> {
        let mut obj = self;
        for key in path.into_iter().take(N - 1) {
            obj = obj.get_first_obj(key)?;
        }
        return obj.get_first_scalar(path.last()?);
    }

    pub fn get_first_object_at_path<const N: usize>(
        &self,
        path: [&str; N],
    ) -> Option<&RawEU4Object<'a>> {
        let mut obj = self;
        for key in path.into_iter().take(N - 1) {
            obj = obj.get_first_obj(key)?;
        }
        return obj.get_first_obj(path.last()?);
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawEU4Value<'a> {
    Scalar(RawEU4Scalar<'a>),
    Object(RawEU4Object<'a>),
}
impl<'a> From<RawEU4Scalar<'a>> for RawEU4Value<'a> {
    #[inline]
    fn from(value: RawEU4Scalar<'a>) -> Self {
        return RawEU4Value::Scalar(value);
    }
}
impl<'a> From<RawEU4Object<'a>> for RawEU4Value<'a> {
    #[inline]
    fn from(value: RawEU4Object<'a>) -> Self {
        return RawEU4Value::Object(value);
    }
}

impl<'a> RawEU4Value<'a> {
    /// Should start on the first character of the value; will not trim whitespace
    pub fn take(input: &'a str) -> Option<(&'a str, RawEU4Value<'a>)> {
        return match input.chars().next() {
            None | Some('}') | Some('=') => None,
            Some('{') => RawEU4Object::parse_object_inner(input.strip_prefix('{')?)
                .map(|(rest, obj)| (rest, RawEU4Value::Object(obj))),
            Some('"') => {
                let Some(end) = input.strip_prefix('"')?.find('"') else {
                    // means this value was at the very end
                    return None;
                };
                let (part, rest) = input.split_at(end + 2);
                return Some((rest, RawEU4Value::Scalar(RawEU4Scalar(part))));
            }
            Some(c) if c.is_whitespace() => None,
            Some(_) => {
                let Some(end) = input.find(is_eu4_delimiter) else {
                    // means this value was at the very end
                    return Some(("", RawEU4Value::Scalar(RawEU4Scalar(input))));
                };
                let (part, rest) = input.split_at(end);
                return Some((rest, RawEU4Value::Scalar(RawEU4Scalar(part))));
            }
        };
    }

    pub fn as_scalar<'b>(&'b self) -> Option<&'b RawEU4Scalar<'a>> {
        if let RawEU4Value::Scalar(scalar) = self {
            return Some(scalar);
        } else {
            return None;
        }
    }

    pub fn as_object<'b>(&'b self) -> Option<&'b RawEU4Object<'a>> {
        if let RawEU4Value::Object(object) = self {
            return Some(object);
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::{eu4_date::Month, map_parsers::from_cp1252};

    use super::*;

    #[test]
    pub fn test_scalar_type_parsing() {
        assert_eq!(EU4Scalar::Bool(true), RawEU4Scalar("yes").into());
        assert_eq!(EU4Scalar::Bool(false), RawEU4Scalar("no").into());
        assert_eq!(EU4Scalar::Int(12352), RawEU4Scalar("12352").into());
        assert_eq!(EU4Scalar::Int(0), RawEU4Scalar("0").into());
        assert_eq!(EU4Scalar::Int(-1), RawEU4Scalar("-1").into());
        assert_eq!(EU4Scalar::Float(104.3), RawEU4Scalar("104.3").into());
        assert_eq!(EU4Scalar::Float(0.0), RawEU4Scalar("0.000").into());
        assert_eq!(EU4Scalar::Float(-9.1), RawEU4Scalar("-9.1").into());
        assert_eq!(
            EU4Scalar::Date(EU4Date {
                year: 1444,
                month: Month::NOV,
                day: 11
            }),
            RawEU4Scalar("1444.11.11").into()
        );
        assert_eq!(
            EU4Scalar::Str("asdf".to_string()),
            RawEU4Scalar("asdf").into()
        );
        assert_eq!(
            EU4Scalar::Str("fd{} 1".to_string()),
            RawEU4Scalar(r#""fd{} 1""#).into()
        );
        assert_eq!(
            EU4Scalar::Str("--91".to_string()),
            RawEU4Scalar("--91").into()
        );
        assert_eq!(
            EU4Scalar::Str("1444.0.11".to_string()),
            RawEU4Scalar("1444.0.11").into()
        );
        assert_eq!(
            EU4Scalar::Str("1.2.3.4".to_string()),
            RawEU4Scalar("1.2.3.4").into()
        );
    }

    #[test]
    pub fn test_scalar_value_strings() {
        assert_eq!(
            RawEU4Value::take("asdf"),
            Some(("", RawEU4Value::Scalar(RawEU4Scalar("asdf")))),
        );
        assert_eq!(
            RawEU4Value::take("asdf "),
            Some((" ", RawEU4Value::Scalar(RawEU4Scalar("asdf")))),
        );
        assert_eq!(RawEU4Value::take(" asdf"), None);

        assert_eq!(
            RawEU4Value::take(r#""a{}=+s{d f" "#),
            Some((" ", RawEU4Value::Scalar(RawEU4Scalar(r#""a{}=+s{d f""#)))),
        );
    }

    #[test]
    pub fn test_scalar_value_non_strings() {
        assert_eq!(
            RawEU4Value::take("123 456"),
            Some((" 456", RawEU4Value::Scalar(RawEU4Scalar("123")))),
        );
        assert_eq!(
            RawEU4Value::take("123 {}"),
            Some((" {}", RawEU4Value::Scalar(RawEU4Scalar("123")))),
        );
        assert_eq!(RawEU4Value::take(" 45.4"), None);

        assert_eq!(
            RawEU4Value::take("45.1.4- "),
            Some((" ", RawEU4Value::Scalar(RawEU4Scalar("45.1.4-")))),
        );
    }

    /// Since scalar values are the simplest, use this as a baseline test
    #[test]
    pub fn test_object_inner_scalar_values() {
        let make_abc_vec = || {
            RawEU4Object(vec![
                RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("a"))),
                RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("b"))),
                RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("c"))),
            ])
        };
        assert_eq!(
            RawEU4Object::parse_object_inner("a b c"),
            Some(("", make_abc_vec())),
        );
        assert_eq!(
            RawEU4Object::parse_object_inner("a b c}"),
            Some(("", make_abc_vec())),
        );
        assert_eq!(
            RawEU4Object::parse_object_inner(" a b c } "),
            Some((" ", make_abc_vec())),
        );
    }

    /// testing the inside of an object with scalar values and scalar-scalar KVs, but no nested objects
    #[test]
    pub fn test_object_inner_scalar_mixed() {
        let make_items_vec = || {
            RawEU4Object(vec![
                RawEU4ObjectItem::KV(RawEU4Scalar("a"), RawEU4Value::Scalar(RawEU4Scalar("a2"))),
                RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("b"))),
                RawEU4ObjectItem::KV(
                    RawEU4Scalar("c"),
                    RawEU4Value::Scalar(RawEU4Scalar(r#""c= {} b""#)),
                ),
            ])
        };
        assert_eq!(
            RawEU4Object::parse_object_inner(r#"a = a2 b c="c= {} b""#),
            Some(("", make_items_vec())),
        );
        assert_eq!(
            RawEU4Object::parse_object_inner(r#"a= a2 b c ="c= {} b" }"#),
            Some(("", make_items_vec())),
        );
    }

    #[test]
    pub fn test_object_inner_objects() {
        assert_eq!(
            RawEU4Object::parse_object_inner("a {b c=d}"),
            Some((
                "",
                RawEU4Object(vec![
                    RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("a"))),
                    RawEU4ObjectItem::Value(
                        RawEU4Object(vec![
                            RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("b"))),
                            RawEU4ObjectItem::KV(
                                RawEU4Scalar("c"),
                                RawEU4Value::Scalar(RawEU4Scalar("d"))
                            ),
                        ])
                        .into()
                    ),
                ])
            )),
        );
        assert_eq!(
            RawEU4Object::parse_object_inner(r#"a "a1} " {b= { c={"d1 {" =e}} } } a"#),
            Some((
                " a",
                RawEU4Object(vec![
                    RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar("a"))),
                    RawEU4ObjectItem::Value(RawEU4Value::Scalar(RawEU4Scalar(r#""a1} ""#))),
                    RawEU4ObjectItem::Value(
                        RawEU4Object(vec![RawEU4ObjectItem::KV(
                            RawEU4Scalar("b"),
                            RawEU4Object(vec![RawEU4ObjectItem::KV(
                                RawEU4Scalar("c"),
                                RawEU4Object(vec![RawEU4ObjectItem::KV(
                                    RawEU4Scalar(r#""d1 {""#),
                                    RawEU4Value::Scalar(RawEU4Scalar("e"))
                                )])
                                .into()
                            )])
                            .into()
                        )])
                        .into()
                    ),
                ])
            )),
        );

        // for some reason, they occasionally skip the '='. Test this:
        assert_eq!(
            RawEU4Object::parse_object_inner("a{b}"),
            RawEU4Object::parse_object_inner("a={b}"),
        );
    }
}
