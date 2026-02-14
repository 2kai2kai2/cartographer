use std::collections::HashMap;

use anyhow::anyhow;
use ouroboros::self_referencing;
use pdx_parser_core::helpers::SplitAtFirst;

pub struct CapitalLocations {
    inner: CapitalLocationsInner,
}
impl CapitalLocations {
    pub fn parse_new(text: impl ToString) -> anyhow::Result<CapitalLocations> {
        let text = text.to_string();

        fn parse_line<'a>(
            (line_idx, line): (usize, &'a str),
        ) -> anyhow::Result<(&'a str, (f64, f64))> {
            let mut it = line.split(';');
            let tag = it
                .next()
                .ok_or_else(|| anyhow!("Missing tag on line {line_idx}"))?;
            let x = it
                .next()
                .ok_or_else(|| anyhow!("Missing x on line {line_idx}"))?;
            let y = it
                .next()
                .ok_or_else(|| anyhow!("Missing y on line {line_idx}"))?;

            let x: f64 = x
                .parse()
                .map_err(|_| anyhow!("Failed to parse x location on line {line_idx}"))?;
            let y: f64 = y
                .parse()
                .map_err(|_| anyhow!("Failed to parse y location on line {line_idx}"))?;
            Ok((tag, (x, y)))
        }

        let inner = CapitalLocationsInner::try_new(text, |text| {
            text.lines().enumerate().map(parse_line).collect()
        })?;
        return Ok(CapitalLocations { inner });
    }
    pub fn get(&self, tag: &str) -> Option<(f64, f64)> {
        return self.inner.borrow_map().get(tag).copied();
    }
}
#[self_referencing]
struct CapitalLocationsInner {
    raw: String,
    #[borrows(raw)]
    #[covariant]
    map: HashMap<&'this str, (f64, f64)>,
}

pub struct Tags {
    inner: TagsInner,
}
impl Tags {
    pub fn parse_new(text: impl ToString) -> anyhow::Result<Tags> {
        let text = text.to_string();

        let inner = TagsInner::try_new(text, |text| {
            let mut tag_to_name = HashMap::new();
            let mut name_to_tag = HashMap::new();
            for (line_idx, line) in text.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let (tag, rest) = line
                    .split_once(';')
                    .ok_or_else(|| anyhow!("Invalid line {line_idx}"))?;
                let tag = tag.trim();
                if tag.len() != 3 || !tag.chars().all(|c| c.is_ascii_uppercase()) {
                    return Err(anyhow!("Invalid tag '{tag}'"));
                }
                let mut it = rest.split(';').map(str::trim);
                let first_name = it
                    .next()
                    .unwrap_or_else(|| unreachable!("Split always returns at least one"));
                tag_to_name.insert(tag, first_name);
                name_to_tag.insert(first_name, tag);
                name_to_tag.extend(it.map(|name| (name, tag)));
            }
            Ok(TagsInnerMaps {
                tag_to_name,
                name_to_tag,
            })
        })?;
        return Ok(Tags { inner });
    }
    pub fn get_name_for_tag<'a>(&'a self, tag: &str) -> Option<&'a str> {
        return self.inner.borrow_maps().tag_to_name.get(tag).cloned();
    }
    /// From some tag or name, returns the tag.
    ///
    /// ## Examples
    /// - `"GBR" -> "GBR"`
    /// - `"Great Britain" -> "GBR"`
    pub fn get_tag_for_name<'a>(&'a self, name: &str) -> Option<&'a str> {
        if name.len() == 3
            && let Some((k, _)) = self
                .inner
                .borrow_maps()
                .tag_to_name
                .get_key_value(name.to_uppercase().as_str())
        {
            return Some(k);
        }
        return self.inner.borrow_maps().name_to_tag.get(name).cloned();
    }
}
#[self_referencing]
struct TagsInner {
    raw: String,
    /// `tag -> name` has only the canonical name
    #[borrows(raw)]
    #[covariant]
    maps: TagsInnerMaps<'this>,
}
struct TagsInnerMaps<'a> {
    tag_to_name: HashMap<&'a str, &'a str>,
    name_to_tag: HashMap<&'a str, &'a str>,
}
