use ab_glyph::Font;
use imageproc::drawing;

/// Displays a number in thousands, or millions if over a million.
/// Expects input to be positive.
pub fn display_num_thousands(num: f64) -> String {
    return match num {
        0.0..10000.0 => format!("{:.2}k", num / 1000.0),
        10000.0..100000.0 => format!("{:.1}k", num / 1000.0),
        100000.0..1000000.0 => format!("{:.0}k", num / 1000.0),
        1000000.0..10000000.0 => format!("{:.2}M", num / 1000000.0),
        10000000.0..100000000.0 => format!("{:.1}M", num / 1000000.0),
        100000000.0.. => format!("{:.0}M", num / 1000000.0),
        _ => "ERROR".to_string(),
    };
}

/// Expects input to be positive.
pub fn display_num(num: f64) -> String {
    return match num {
        0.0..1000.0 => format!("{num:.0}"),
        1000.0..10000.0 => format!("{:.2}k", num / 1000.0),
        10000.0..100000.0 => format!("{:.1}k", num / 1000.0),
        100000.0..1000000.0 => format!("{:.0}k", num / 1000.0),
        1000000.0..10000000.0 => format!("{:.2}M", num / 1000000.0),
        10000000.0..100000000.0 => format!("{:.1}M", num / 1000000.0),
        100000000.0.. => format!("{:.0}M", num / 1000000.0),
        _ => "ERROR".to_string(),
    };
}

/// Assumes whitespace is only a single space between words
pub fn text_wrap(text: &str, font: &impl Font, scale: f32, width: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();

    for part in text.split_ascii_whitespace() {
        let prospective = if line.is_empty() {
            part.to_string()
        } else {
            format!("{line} {part}")
        };
        if drawing::text_size(scale, font, &prospective).0 > width {
            out.push(line);
            line = part.to_string();
        } else {
            line = prospective;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    return out;
}

/// Internally is a sorted vec. Ensures every key is unique.
///
/// If all keys match their index, lookup is O(1). Otherwise, we do a binary search.
///
/// Not optimized for insertion, only lookup.
pub struct MaybeIndexedMap<V> {
    inner: Vec<(usize, V)>,
    /// Whether all keys match their index.
    is_indexed: bool,
}
#[allow(unused)]
impl<V> MaybeIndexedMap<V> {
    /// Creates a new, empty `MaybeIndexedMap`.
    /// Typically it is preferable to create all at once with `collect()` instead,
    /// as the `MaybeIndexedMap` is not optimized for insertion.
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            is_indexed: true,
        }
    }
    pub fn get(&self, key: usize) -> Option<&V> {
        if self.is_indexed {
            self.inner.get(key).map(|(_, v)| v)
        } else {
            // try indexing first
            if let Some((key_at_idx, value)) = self.inner.get(key)
                && *key_at_idx == key
            {
                return Some(value);
            }

            self.inner
                .binary_search_by_key(&key, |(k, _)| *k)
                .ok()
                .map(|i| &self.inner[i].1)
        }
    }
    pub fn get_mut(&mut self, key: usize) -> Option<&mut V> {
        if self.is_indexed {
            self.inner.get_mut(key).map(|(_, v)| v)
        } else {
            // try indexing first
            if let Some((key_at_idx, _)) = self.inner.get(key)
                && *key_at_idx == key
            {
                return Some(&mut self.inner[key].1);
            }

            self.inner
                .binary_search_by_key(&key, |(k, _)| *k)
                .ok()
                .map(|i| &mut self.inner[i].1)
        }
    }
    pub fn size(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn contains_key(&self, key: usize) -> bool {
        if self.is_indexed {
            self.inner.len() > key
        } else {
            self.inner.binary_search_by_key(&key, |(k, _)| *k).is_ok()
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (usize, &V)> {
        self.inner.iter().map(|(k, v)| (*k, v))
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.iter().map(|(_, v)| v)
    }
}
impl<V> Default for MaybeIndexedMap<V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<V> IntoIterator for MaybeIndexedMap<V> {
    type Item = (usize, V);
    type IntoIter = std::vec::IntoIter<(usize, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
impl<V> FromIterator<(usize, V)> for MaybeIndexedMap<V> {
    fn from_iter<T: IntoIterator<Item = (usize, V)>>(iter: T) -> Self {
        let mut inner: Vec<_> = iter.into_iter().collect();
        inner.sort_by_key(|(k, _)| *k);
        inner.dedup_by_key(|(k, _)| *k);
        let is_indexed = Self::compute_is_indexed(&inner);
        Self { inner, is_indexed }
    }
}
impl<V> Extend<(usize, V)> for MaybeIndexedMap<V> {
    fn extend<T: IntoIterator<Item = (usize, V)>>(&mut self, iter: T) {
        let mut tmp: Vec<_> = iter.into_iter().collect();
        std::mem::swap(&mut self.inner, &mut tmp);
        self.inner.extend(tmp);
        self.inner.sort_by_key(|(k, _)| *k);
        self.inner.dedup_by_key(|(k, _)| *k);
        self.is_indexed = Self::compute_is_indexed(&self.inner);
    }
}

impl<V> MaybeIndexedMap<V> {
    /// Updates `is_indexed` based on the current state of the map.
    fn compute_is_indexed(inner: &[(usize, V)]) -> bool {
        return inner.iter().enumerate().all(|(i, (k, _))| i == *k);
    }
}
