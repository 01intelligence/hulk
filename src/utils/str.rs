use std::str::pattern::{Pattern, ReverseSearcher};

pub trait StrExt {
    fn may_strip_prefix<'a, P>(&'a self, prefix: P) -> &'a str
    where
        P: Pattern<'a>,
        <P as Pattern<'a>>::Searcher: ReverseSearcher<'a>;

    fn may_strip_suffix<'a, P>(&'a self, suffix: P) -> &'a str
    where
        P: Pattern<'a>,
        <P as Pattern<'a>>::Searcher: ReverseSearcher<'a>;

    fn in_ignore_ascii_case(&self, slice: &[&str]) -> bool;
}

impl<T: AsRef<str>> StrExt for T {
    fn may_strip_prefix<'a, P>(&'a self, prefix: P) -> &'a str
    where
        P: Pattern<'a>,
        <P as Pattern<'a>>::Searcher: ReverseSearcher<'a>,
    {
        self.as_ref().strip_prefix(prefix).unwrap_or(self.as_ref())
    }

    fn may_strip_suffix<'a, P>(&'a self, suffix: P) -> &'a str
    where
        P: Pattern<'a>,
        <P as Pattern<'a>>::Searcher: ReverseSearcher<'a>,
    {
        self.as_ref().strip_suffix(suffix).unwrap_or(self.as_ref())
    }

    fn in_ignore_ascii_case(&self, slice: &[&str]) -> bool {
        slice.iter().any(|s| s.eq_ignore_ascii_case(self.as_ref()))
    }
}
