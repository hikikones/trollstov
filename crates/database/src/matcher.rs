pub(crate) struct Matcher {
    matcher: nucleo_matcher::Matcher,
    atom: nucleo_matcher::pattern::Atom,
    buffer: Vec<char>,
}

impl Matcher {
    pub(crate) fn new() -> Self {
        Self {
            matcher: nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
            atom: Self::create_atom(""),
            buffer: Vec::new(),
        }
    }

    pub(crate) fn update(&mut self, needle: &str) {
        self.atom = Self::create_atom(needle);
    }

    pub(crate) fn score(&mut self, haystack: &str) -> Option<u16> {
        self.atom.score(
            nucleo_matcher::Utf32Str::new(haystack, &mut self.buffer),
            &mut self.matcher,
        )
    }

    pub(crate) fn create_atom(needle: &str) -> nucleo_matcher::pattern::Atom {
        nucleo_matcher::pattern::Atom::new(
            needle,
            nucleo_matcher::pattern::CaseMatching::Smart,
            nucleo_matcher::pattern::Normalization::Smart,
            nucleo_matcher::pattern::AtomKind::Fuzzy,
            true,
        )
    }
}
