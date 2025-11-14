/// Basically implements [`str::split_once`] for arrays.
///
/// However, we still implement for `str` to add `split_at_first_inclusive`
pub trait SplitAtFirst<T> {
    fn split_at_first<'a, F>(&'a self, func: F) -> Option<(&'a Self, &'a Self)>
    where
        F: FnMut(&T) -> bool;
    fn split_at_first_inclusive<'a, F>(&'a self, func: F) -> Option<(&'a Self, &'a Self)>
    where
        F: FnMut(&T) -> bool;
}
impl<T> SplitAtFirst<T> for [T] {
    fn split_at_first<'a, F>(&'a self, mut func: F) -> Option<(&'a [T], &'a [T])>
    where
        F: FnMut(&T) -> bool,
    {
        let (split_idx, _) = self.iter().enumerate().find(|(_, item)| func(*item))?;
        return Some((&self[..split_idx], &self[split_idx + 1..]));
    }
    fn split_at_first_inclusive<'a, F>(&'a self, mut func: F) -> Option<(&'a [T], &'a [T])>
    where
        F: FnMut(&T) -> bool,
    {
        let (split_idx, _) = self.iter().enumerate().find(|(_, item)| func(*item))?;
        return Some((&self[..split_idx], &self[split_idx..]));
    }
}
impl SplitAtFirst<char> for str {
    #[inline]
    fn split_at_first<'a, F>(&'a self, mut func: F) -> Option<(&'a Self, &'a Self)>
    where
        F: FnMut(&char) -> bool,
    {
        return self.split_once(|c| func(&c));
    }

    fn split_at_first_inclusive<'a, F>(&'a self, mut func: F) -> Option<(&'a Self, &'a Self)>
    where
        F: FnMut(&char) -> bool,
    {
        let (split_idx, _) = self.char_indices().find(|(_, item)| func(item))?;
        return Some((&self[..split_idx], &self[split_idx..]));
    }
}
