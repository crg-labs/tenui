//! Literal in-buffer search with incremental match navigation.
//!
//! The spec has fuzzy *command-palette* search but no in-buffer find; this fills that gap
//! for text and log views. Returns non-overlapping byte ranges; case-insensitive matching
//! is ASCII case-folding (non-ASCII bytes compared exactly).

/// All non-overlapping match ranges (byte offsets) of `needle` in `haystack`.
pub fn find_all(haystack: &str, needle: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        if haystack.is_char_boundary(i) {
            let window = &hb[i..i + nb.len()];
            let matched = if case_sensitive {
                window == nb
            } else {
                window.eq_ignore_ascii_case(nb)
            };
            if matched {
                out.push((i, i + nb.len()));
                i += nb.len(); // non-overlapping
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Incremental search cursor over a set of match ranges (next/prev with wrap-around).
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    matches: Vec<(usize, usize)>,
    current: usize,
}

impl SearchState {
    /// Runs a search and positions on the first match.
    pub fn search(haystack: &str, needle: &str, case_sensitive: bool) -> Self {
        Self {
            matches: find_all(haystack, needle, case_sensitive),
            current: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// All match ranges (e.g. for highlighting).
    pub fn matches(&self) -> &[(usize, usize)] {
        &self.matches
    }

    /// The currently selected match, if any.
    pub fn current(&self) -> Option<(usize, usize)> {
        self.matches.get(self.current).copied()
    }

    /// The 1-based index of the current match (0 when empty), for a "3/12" indicator.
    pub fn current_index(&self) -> usize {
        if self.matches.is_empty() { 0 } else { self.current + 1 }
    }

    /// Advances to the next match (wraps), returning it.
    pub fn next_match(&mut self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        self.current = (self.current + 1) % self.matches.len();
        self.current()
    }

    /// Moves to the previous match (wraps), returning it.
    pub fn prev_match(&mut self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        self.current = (self.current + self.matches.len() - 1) % self.matches.len();
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_all_literal_and_nonoverlapping() {
        assert_eq!(find_all("abcabc", "bc", true), vec![(1, 3), (4, 6)]);
        assert_eq!(find_all("aaaa", "aa", true), vec![(0, 2), (2, 4)]); // non-overlapping
        assert_eq!(find_all("abc", "x", true), vec![]);
        assert_eq!(find_all("abc", "", true), vec![]);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(find_all("Hello HELLO hello", "hello", false).len(), 3);
        assert_eq!(find_all("Hello", "hello", true), vec![]);
    }

    #[test]
    fn test_multibyte_offsets_are_char_boundaries() {
        // "héllo héllo" — ensure matches land on boundaries and don't split 'é'.
        let hay = "héllo héllo";
        let m = find_all(hay, "llo", true);
        assert_eq!(m.len(), 2);
        for (s, e) in m {
            assert!(hay.is_char_boundary(s) && hay.is_char_boundary(e));
        }
    }

    #[test]
    fn test_incremental_navigation_wraps() {
        let mut s = SearchState::search("a.a.a", "a", true);
        assert_eq!(s.len(), 3);
        assert_eq!(s.current(), Some((0, 1)));
        assert_eq!(s.current_index(), 1);
        assert_eq!(s.next_match(), Some((2, 3)));
        assert_eq!(s.next_match(), Some((4, 5)));
        assert_eq!(s.next_match(), Some((0, 1))); // wrap
        assert_eq!(s.prev_match(), Some((4, 5))); // wrap back
    }
}
