use std::str::FromStr;

use crate::{eu4_date::EU4Date, stellaris_date::StellarisDate};

#[inline]
fn is_eu4_delimiter(c: char) -> bool {
    c.is_whitespace() || c == '{' || c == '}' || c == '='
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum PDXScalar<D: FromStr> {
    Int(i64),
    Float(f64),
    Date(D),
    Bool(bool),
    Str(String),
}
impl<'a, D: FromStr> From<RawPDXScalar<'a>> for PDXScalar<D> {
    fn from(value: RawPDXScalar<'a>) -> Self {
        if value.0 == "yes" {
            return PDXScalar::Bool(true);
        } else if value.0 == "no" {
            return PDXScalar::Bool(false);
        } else if let Some(quoted) = value.0.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            return PDXScalar::Str(quoted.to_string());
        } else if let Ok(int) = value.0.parse::<i64>() {
            return PDXScalar::Int(int);
        } else if let Ok(float) = value.0.parse::<f64>() {
            return PDXScalar::Float(float);
        } else if let Ok(date) = value.0.parse::<D>() {
            return PDXScalar::Date(date);
        } else {
            return PDXScalar::Str(value.0.to_string());
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawPDXScalar<'a>(pub &'a str);
macro_rules! implement_try_from_raw_pdx_scalar {
    ($t:ty, $e:ty) => {
        impl<'a> TryFrom<&RawPDXScalar<'a>> for $t {
            type Error = $e;

            fn try_from(value: &RawPDXScalar<'a>) -> Result<Self, Self::Error> {
                return value.0.parse();
            }
        }
    };
}

implement_try_from_raw_pdx_scalar!(u8, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(u16, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(u32, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(u64, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(u128, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(i8, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(i16, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(i32, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(i64, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(i128, std::num::ParseIntError);
implement_try_from_raw_pdx_scalar!(f32, std::num::ParseFloatError);
implement_try_from_raw_pdx_scalar!(f64, std::num::ParseFloatError);
implement_try_from_raw_pdx_scalar!(EU4Date, anyhow::Error);
implement_try_from_raw_pdx_scalar!(StellarisDate, anyhow::Error);
impl<'a> TryFrom<RawPDXScalar<'a>> for bool {
    type Error = anyhow::Error;

    fn try_from(value: RawPDXScalar<'a>) -> Result<Self, Self::Error> {
        return match value.0 {
            "yes" => Ok(true),
            "no" => Ok(false),
            _ => Err(anyhow::anyhow!("Invalid boolean value")),
        };
    }
}
impl<'a> From<RawPDXScalar<'a>> for &'a str {
    fn from(value: RawPDXScalar<'a>) -> Self {
        return value
            .0
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value.0);
    }
}

impl<'a> RawPDXScalar<'a> {
    pub fn as_int(&self) -> Option<i64> {
        return self.try_into().ok();
    }

    pub fn as_float(&self) -> Option<f64> {
        return self.try_into().ok();
    }

    pub fn as_date<'b, D: TryFrom<&'b RawPDXScalar<'a>>>(&'b self) -> Option<D> {
        return self.try_into().ok();
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
pub enum RawPDXObjectItem<'a> {
    KV(RawPDXScalar<'a>, RawPDXValue<'a>),
    Value(RawPDXValue<'a>),
}
impl<'a> From<RawPDXValue<'a>> for RawPDXObjectItem<'a> {
    #[inline]
    fn from(value: RawPDXValue<'a>) -> Self {
        return RawPDXObjectItem::Value(value);
    }
}
impl<'a> RawPDXObjectItem<'a> {
    /// Should start on the first character of the value; will not trim whitespace
    pub fn take(input: &'a str) -> Option<(&'a str, RawPDXObjectItem<'a>)> {
        match RawPDXValue::take(input)? {
            (rest, RawPDXValue::Scalar(scalar)) => {
                if let Some((rest, obj)) = rest
                    .strip_prefix('{')
                    .and_then(RawPDXObject::parse_object_inner)
                {
                    // sometimes, they just skip the '=' on a kv pair for some reason
                    // only accept this if there is no whitespace inbetween
                    return Some((
                        rest,
                        RawPDXObjectItem::KV(scalar, RawPDXValue::Object(obj).into()),
                    ));
                }

                let Some(rest) = rest.trim_start().strip_prefix('=') else {
                    // it's just a value
                    return Some((rest, RawPDXValue::Scalar(scalar).into()));
                };

                // after an '='
                let (rest, value) = RawPDXValue::take(rest.trim_start())?;
                return Some((rest, RawPDXObjectItem::KV(scalar, value)));
            }
            (rest, value) => return Some((rest, value.into())),
        };
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawPDXObject<'a>(pub Vec<RawPDXObjectItem<'a>>);
impl<'a> RawPDXObject<'a> {
    /// Will end after a '}' (returns rest starting with the next character) or EOF
    pub fn parse_object_inner(input: &'a str) -> Option<(&'a str, RawPDXObject<'a>)> {
        let mut out: Vec<RawPDXObjectItem<'a>> = Vec::new();
        let mut rest: &'a str = input;

        loop {
            rest = rest.trim_start();
            if rest.len() == 0 {
                return Some((rest, RawPDXObject(out)));
            } else if let Some(rest) = rest.strip_prefix('}') {
                return Some((rest, RawPDXObject(out)));
            }

            let (r, item) = RawPDXObjectItem::take(rest)?;
            rest = r;
            out.push(item);
        }
    }

    /// Iterates through all
    pub fn iter_values(&self) -> impl Iterator<Item = &RawPDXValue<'a>> {
        return self.0.iter().filter_map(|v| match v {
            RawPDXObjectItem::Value(value) => Some(value),
            _ => None,
        });
    }

    pub fn iter_all_KVs(&self) -> impl Iterator<Item = (&RawPDXScalar<'a>, &RawPDXValue<'a>)> {
        return self.0.iter().filter_map(|v| match v {
            RawPDXObjectItem::KV(key, value) => Some((key, value)),
            _ => None,
        });
    }

    /// Gets the first value for the specified key
    pub fn get_first(&self, key: &str) -> Option<&RawPDXValue<'a>> {
        return self
            .iter_all_KVs()
            .find(|(k, _)| k.0 == key)
            .map(|(_, v)| v);
    }

    pub fn get_first_obj(&self, key: &str) -> Option<&RawPDXObject<'a>> {
        return self.iter_all_KVs().find_map(|(k, v)| {
            if k.0 != key {
                None
            } else if let RawPDXValue::Object(obj) = v {
                Some(obj)
            } else {
                None
            }
        });
    }

    pub fn get_first_scalar(&self, key: &str) -> Option<&RawPDXScalar<'a>> {
        return self.iter_all_KVs().find_map(|(k, v)| {
            if k.0 != key {
                None
            } else if let RawPDXValue::Scalar(scalar) = v {
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
    pub fn get_first_as_date<'b, D: TryFrom<&'b RawPDXScalar<'a>>>(
        &'b self,
        key: &str,
    ) -> Option<D> {
        return self.get_first(key)?.as_scalar()?.try_into().ok();
    }
    pub fn get_first_as_bool(&self, key: &str) -> Option<bool> {
        return self.get_first(key)?.as_scalar()?.as_bool();
    }
    pub fn get_first_as_string(&self, key: &str) -> Option<String> {
        return Some(self.get_first(key)?.as_scalar()?.as_string());
    }

    pub fn get_first_at_path<const N: usize>(&self, path: [&str; N]) -> Option<&RawPDXValue<'a>> {
        let mut obj = self;
        for key in path.into_iter().take(N - 1) {
            obj = obj.get_first_obj(key)?;
        }
        return obj.get_first(path.last()?);
    }

    pub fn get_first_scalar_at_path<const N: usize>(
        &self,
        path: [&str; N],
    ) -> Option<&RawPDXScalar<'a>> {
        let mut obj = self;
        for key in path.into_iter().take(N - 1) {
            obj = obj.get_first_obj(key)?;
        }
        return obj.get_first_scalar(path.last()?);
    }

    pub fn get_first_object_at_path<const N: usize>(
        &self,
        path: [&str; N],
    ) -> Option<&RawPDXObject<'a>> {
        let mut obj = self;
        for key in path.into_iter().take(N - 1) {
            obj = obj.get_first_obj(key)?;
        }
        return obj.get_first_obj(path.last()?);
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawPDXValue<'a> {
    Scalar(RawPDXScalar<'a>),
    Object(RawPDXObject<'a>),
}
impl<'a> From<RawPDXScalar<'a>> for RawPDXValue<'a> {
    #[inline]
    fn from(value: RawPDXScalar<'a>) -> Self {
        return RawPDXValue::Scalar(value);
    }
}
impl<'a> From<RawPDXObject<'a>> for RawPDXValue<'a> {
    #[inline]
    fn from(value: RawPDXObject<'a>) -> Self {
        return RawPDXValue::Object(value);
    }
}

impl<'a> RawPDXValue<'a> {
    /// Should start on the first character of the value; will not trim whitespace
    pub fn take(input: &'a str) -> Option<(&'a str, RawPDXValue<'a>)> {
        return match input.chars().next() {
            None | Some('}') | Some('=') => None,
            Some('{') => RawPDXObject::parse_object_inner(input.strip_prefix('{')?)
                .map(|(rest, obj)| (rest, RawPDXValue::Object(obj))),
            Some('"') => {
                let Some(end) = input.strip_prefix('"')?.find('"') else {
                    // means this value was at the very end
                    return None;
                };
                let (part, rest) = input.split_at(end + 2);
                return Some((rest, RawPDXValue::Scalar(RawPDXScalar(part))));
            }
            Some(c) if c.is_whitespace() => None,
            Some(_) => {
                let Some(end) = input.find(is_eu4_delimiter) else {
                    // means this value was at the very end
                    return Some(("", RawPDXValue::Scalar(RawPDXScalar(input))));
                };
                let (part, rest) = input.split_at(end);
                return Some((rest, RawPDXValue::Scalar(RawPDXScalar(part))));
            }
        };
    }

    pub fn as_scalar<'b>(&'b self) -> Option<&'b RawPDXScalar<'a>> {
        if let RawPDXValue::Scalar(scalar) = self {
            return Some(scalar);
        } else {
            return None;
        }
    }

    pub fn as_object<'b>(&'b self) -> Option<&'b RawPDXObject<'a>> {
        if let RawPDXValue::Object(object) = self {
            return Some(object);
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::eu4_date::Month;

    use super::*;

    #[test]
    pub fn test_scalar_type_parsing() {
        assert_eq!(PDXScalar::<EU4Date>::Bool(true), RawPDXScalar("yes").into());
        assert_eq!(PDXScalar::<EU4Date>::Bool(false), RawPDXScalar("no").into());
        assert_eq!(
            PDXScalar::<EU4Date>::Int(12352),
            RawPDXScalar("12352").into()
        );
        assert_eq!(PDXScalar::<EU4Date>::Int(0), RawPDXScalar("0").into());
        assert_eq!(PDXScalar::<EU4Date>::Int(-1), RawPDXScalar("-1").into());
        assert_eq!(
            PDXScalar::<EU4Date>::Float(104.3),
            RawPDXScalar("104.3").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Float(0.0),
            RawPDXScalar("0.000").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Float(-9.1),
            RawPDXScalar("-9.1").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Date(EU4Date {
                year: 1444,
                month: Month::NOV,
                day: 11
            }),
            RawPDXScalar("1444.11.11").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Str("asdf".to_string()),
            RawPDXScalar("asdf").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Str("fd{} 1".to_string()),
            RawPDXScalar(r#""fd{} 1""#).into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Str("--91".to_string()),
            RawPDXScalar("--91").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Str("1444.0.11".to_string()),
            RawPDXScalar("1444.0.11").into()
        );
        assert_eq!(
            PDXScalar::<EU4Date>::Str("1.2.3.4".to_string()),
            RawPDXScalar("1.2.3.4").into()
        );
    }

    #[test]
    pub fn test_scalar_value_strings() {
        assert_eq!(
            RawPDXValue::take("asdf"),
            Some(("", RawPDXValue::Scalar(RawPDXScalar("asdf")))),
        );
        assert_eq!(
            RawPDXValue::take("asdf "),
            Some((" ", RawPDXValue::Scalar(RawPDXScalar("asdf")))),
        );
        assert_eq!(RawPDXValue::take(" asdf"), None);

        assert_eq!(
            RawPDXValue::take(r#""a{}=+s{d f" "#),
            Some((" ", RawPDXValue::Scalar(RawPDXScalar(r#""a{}=+s{d f""#)))),
        );
    }

    #[test]
    pub fn test_scalar_value_non_strings() {
        assert_eq!(
            RawPDXValue::take("123 456"),
            Some((" 456", RawPDXValue::Scalar(RawPDXScalar("123")))),
        );
        assert_eq!(
            RawPDXValue::take("123 {}"),
            Some((" {}", RawPDXValue::Scalar(RawPDXScalar("123")))),
        );
        assert_eq!(RawPDXValue::take(" 45.4"), None);

        assert_eq!(
            RawPDXValue::take("45.1.4- "),
            Some((" ", RawPDXValue::Scalar(RawPDXScalar("45.1.4-")))),
        );
    }

    /// Since scalar values are the simplest, use this as a baseline test
    #[test]
    pub fn test_object_inner_scalar_values() {
        let make_abc_vec = || {
            RawPDXObject(vec![
                RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("a"))),
                RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("b"))),
                RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("c"))),
            ])
        };
        assert_eq!(
            RawPDXObject::parse_object_inner("a b c"),
            Some(("", make_abc_vec())),
        );
        assert_eq!(
            RawPDXObject::parse_object_inner("a b c}"),
            Some(("", make_abc_vec())),
        );
        assert_eq!(
            RawPDXObject::parse_object_inner(" a b c } "),
            Some((" ", make_abc_vec())),
        );
    }

    /// testing the inside of an object with scalar values and scalar-scalar KVs, but no nested objects
    #[test]
    pub fn test_object_inner_scalar_mixed() {
        let make_items_vec = || {
            RawPDXObject(vec![
                RawPDXObjectItem::KV(RawPDXScalar("a"), RawPDXValue::Scalar(RawPDXScalar("a2"))),
                RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("b"))),
                RawPDXObjectItem::KV(
                    RawPDXScalar("c"),
                    RawPDXValue::Scalar(RawPDXScalar(r#""c= {} b""#)),
                ),
            ])
        };
        assert_eq!(
            RawPDXObject::parse_object_inner(r#"a = a2 b c="c= {} b""#),
            Some(("", make_items_vec())),
        );
        assert_eq!(
            RawPDXObject::parse_object_inner(r#"a= a2 b c ="c= {} b" }"#),
            Some(("", make_items_vec())),
        );
    }

    #[test]
    pub fn test_object_inner_objects() {
        assert_eq!(
            RawPDXObject::parse_object_inner("a {b c=d}"),
            Some((
                "",
                RawPDXObject(vec![
                    RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("a"))),
                    RawPDXObjectItem::Value(
                        RawPDXObject(vec![
                            RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("b"))),
                            RawPDXObjectItem::KV(
                                RawPDXScalar("c"),
                                RawPDXValue::Scalar(RawPDXScalar("d"))
                            ),
                        ])
                        .into()
                    ),
                ])
            )),
        );
        assert_eq!(
            RawPDXObject::parse_object_inner(r#"a "a1} " {b= { c={"d1 {" =e}} } } a"#),
            Some((
                " a",
                RawPDXObject(vec![
                    RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar("a"))),
                    RawPDXObjectItem::Value(RawPDXValue::Scalar(RawPDXScalar(r#""a1} ""#))),
                    RawPDXObjectItem::Value(
                        RawPDXObject(vec![RawPDXObjectItem::KV(
                            RawPDXScalar("b"),
                            RawPDXObject(vec![RawPDXObjectItem::KV(
                                RawPDXScalar("c"),
                                RawPDXObject(vec![RawPDXObjectItem::KV(
                                    RawPDXScalar(r#""d1 {""#),
                                    RawPDXValue::Scalar(RawPDXScalar("e"))
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
            RawPDXObject::parse_object_inner("a{b}"),
            RawPDXObject::parse_object_inner("a={b}"),
        );
    }
}
