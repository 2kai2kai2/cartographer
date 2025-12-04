use anyhow::anyhow;
use ouroboros::self_referencing;

#[self_referencing]
struct _StringsResolver {
    raw: Box<[u8]>,
    #[borrows(raw)]
    strings: Box<[&'this str]>,
}
/// Resolves string lookup indices
///
///
/// ([`crate::bin_lexer::BinToken::LookupU16`]/[`crate::common_deserialize::LookupU16`]) to strings.
pub struct StringsResolver {
    inner: _StringsResolver,
}
impl StringsResolver {
    /// Input is the `string_lookup` archive file contents
    pub fn from_raw(raw: Box<[u8]>) -> anyhow::Result<Self> {
        let inner = _StringsResolver::try_new(raw, |raw| -> anyhow::Result<_> {
            let mut strings = Vec::new();
            let stream = raw.as_ref();
            let (_header, mut stream) = stream
                .split_first_chunk::<5>()
                .ok_or(anyhow!("string_lookup header was not found"))?;
            while let Some((len, rest)) = stream.split_first_chunk() {
                let len = u16::from_le_bytes(*len);
                let (string, rest) = rest
                    .split_at_checked(len as usize)
                    .ok_or(anyhow!("Ran into EOF while reading a string"))?;
                let string = str::from_utf8(string)?;
                stream = rest;
                strings.push(string);
            }
            return Ok(strings.into_boxed_slice());
        })?;
        return Ok(StringsResolver { inner });
    }

    pub fn get<'a>(&'a self, id: u16) -> Option<&'a str> {
        return self.inner.borrow_strings().get(id as usize).copied();
    }

    pub fn len(&self) -> usize {
        return self.inner.borrow_strings().len();
    }

    pub fn is_empty(&self) -> bool {
        return self.inner.borrow_strings().is_empty();
    }

    pub fn iter(&self) -> impl Iterator<Item = (u16, &str)> {
        return self
            .inner
            .borrow_strings()
            .iter()
            .enumerate()
            .map(|(i, &s)| (i as u16, s));
    }
}
impl Default for StringsResolver {
    /// Creates a new resolver with an empty string lookup
    fn default() -> Self {
        let inner = _StringsResolver::new(Box::default(), |_| Box::default());
        return StringsResolver { inner };
    }
}
