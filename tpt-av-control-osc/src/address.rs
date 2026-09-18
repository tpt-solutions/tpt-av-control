//! OSC address pattern matching.
//!
//! Implements the OSC 1.0 pattern characters (`?`, `*`, `[...]` with `!`
//! negation and `a-z` ranges, `{a,b}` alternation) plus the OSC 1.1 `//`
//! recursive-any-parts wildcard.

/// A compiled OSC address pattern (e.g. `/track/*/volume`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OscAddressMatcher {
    pattern: String,
}

/// Recursion cap so hostile patterns can't exhaust the stack.
const MAX_PATTERN_LEN: usize = 1024;

/// Total match-step budget per `matches()` call. Backtracking through
/// nested `*`/`//`/`{}` groups is cut off once the budget is spent, which
/// bounds worst-case match cost for hostile pattern/address pairs.
const MAX_MATCH_STEPS: u32 = 10_000;

impl OscAddressMatcher {
    /// Creates a matcher for the given pattern.
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    /// The pattern this matcher holds.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Checks whether `address` matches the pattern.
    ///
    /// Matching is bounded: patterns longer than 1024 bytes never match,
    /// and backtracking is cut off after a fixed step budget (bounded-false
    /// on hostile inputs, never a hang).
    /// # Examples
    ///
    /// ```
    /// use tpt_av_control_osc::OscAddressMatcher;
    /// let m = OscAddressMatcher::new("/track/*/volume");
    /// assert!(m.matches("/track/1/volume"));
    /// assert!(!m.matches("/track/1/pan"));
    /// ```
    pub fn matches(&self, address: &str) -> bool {
        if self.pattern.len() > MAX_PATTERN_LEN {
            return false;
        }
        let mut budget = MAX_MATCH_STEPS;
        match_parts(
            self.pattern.as_bytes(),
            0,
            address.as_bytes(),
            0,
            &mut budget,
        )
    }
}

impl std::fmt::Display for OscAddressMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.pattern)
    }
}

/// Matches `pattern[pi..]` against `address[ai..]`, spending from
/// `budget` on every step (byte-wise; all OSC pattern characters are ASCII).
fn match_parts(
    pattern: &[u8],
    mut pi: usize,
    address: &[u8],
    mut ai: usize,
    budget: &mut u32,
) -> bool {
    if *budget == 0 {
        return false; // budget exhausted: treat as no-match
    }
    *budget -= 1;
    // Iterative fast path over literal characters.
    while pi < pattern.len() {
        let pc = pattern[pi];
        match pc {
            b'?' => {
                if ai >= address.len() || address[ai] == b'/' {
                    return false;
                }
                pi += 1;
                ai += 1;
            }
            b'*' => {
                // '*' matches zero or more characters within one address
                // part (it never crosses a '/').
                return (ai..=address.len())
                    .take_while(|&k| {
                        // candidate slice address[ai..k] contains no '/'
                        !address[ai..k].contains(&b'/')
                    })
                    .any(|k| match_parts(pattern, pi + 1, address, k, budget));
            }
            b'/' if pattern.get(pi + 1) == Some(&b'/') => {
                // '//' matches zero or more whole parts, including their
                // separating slashes. Candidates: the current position
                // (zero parts), positions right after any following '/',
                // and the end of the address.
                return std::iter::once(ai)
                    .chain(std::iter::once(address.len()))
                    .chain((ai + 1..address.len()).filter(|&k| address[k - 1] == b'/'))
                    .any(|k| match_parts(pattern, pi + 2, address, k, budget));
            }
            b'[' => {
                if ai >= address.len() {
                    return false;
                }
                match match_char_class(pattern, pi, address[ai]) {
                    Some(next_pi) => {
                        pi = next_pi;
                        ai += 1;
                    }
                    None => return false,
                }
            }
            b'{' => {
                // '{' consumes exactly one character run, so a failed
                // alternation fails the whole match.
                return match_alternation(pattern, pi, address, ai, budget).unwrap_or(false);
            }
            _ => {
                if ai >= address.len() || address[ai] != pc {
                    return false;
                }
                pi += 1;
                ai += 1;
            }
        }
    }
    ai == address.len()
}

/// Matches a `[...]` class at `pattern[pi]` against `ch`. On success returns
/// the pattern index just past the closing `]`.
fn match_char_class(pattern: &[u8], pi: usize, ch: u8) -> Option<usize> {
    debug_assert_eq!(pattern[pi], b'[');
    let mut i = pi + 1;
    let negated = pattern.get(i) == Some(&b'!');
    if negated {
        i += 1;
    }
    let mut matched = false;
    while i < pattern.len() {
        match pattern[i] {
            b']' => {
                // ']' as the very first character is literal, but for
                // simplicity we close the class here (matching liblo).
                let result = matched != negated;
                return result.then_some(i + 1);
            }
            b'-' if i + 1 < pattern.len()
                && pattern[i + 1] != b']'
                && i > pi + 1 + usize::from(negated) =>
            {
                // Range: needs a start char before the '-'.
                let start = pattern[i - 1];
                let end = pattern[i + 1];
                if start <= ch && ch <= end {
                    matched = true;
                }
                i += 2;
            }
            c => {
                if c == ch {
                    matched = true;
                }
                i += 1;
            }
        }
    }
    None // unterminated class
}

/// Attempts a `{a,b,c}` alternation at `pattern[pi]`. Returns `Some(matched)`
/// if the class is well-formed.
fn match_alternation(
    pattern: &[u8],
    pi: usize,
    address: &[u8],
    ai: usize,
    budget: &mut u32,
) -> Option<bool> {
    debug_assert_eq!(pattern[pi], b'{');
    let mut alternatives: Vec<(usize, usize)> = Vec::new(); // (start, end) spans
    let mut start = pi + 1;
    let mut i = pi + 1;
    while i < pattern.len() {
        match pattern[i] {
            b',' => {
                alternatives.push((start, i));
                start = i + 1;
            }
            b'}' => {
                alternatives.push((start, i));
                break;
            }
            _ => {}
        }
        i += 1;
    }
    if i == pattern.len() {
        return None; // unterminated alternation
    }
    let after_class = i + 1; // index just past '}'
    for (s, e) in alternatives {
        if address[ai..].starts_with(&pattern[s..e])
            && match_parts(pattern, after_class, address, ai + (e - s), budget)
        {
            return Some(true);
        }
    }
    Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pattern: &str) -> OscAddressMatcher {
        OscAddressMatcher::new(pattern)
    }

    #[test]
    fn literal_addresses() {
        assert!(m("/foo").matches("/foo"));
        assert!(!m("/foo").matches("/bar"));
        assert!(!m("/foo").matches("/foo/bar"));
    }

    #[test]
    fn question_mark() {
        assert!(m("/f?o").matches("/foo"));
        assert!(m("/f?o").matches("/f!o"));
        assert!(!m("/f?o").matches("/fo"));
        assert!(!m("/f?o").matches("/fooo"));
        // '?' must not match '/'.
        assert!(!m("/?/bar").matches("/a/b/bar"));
    }

    #[test]
    fn star_wildcard() {
        assert!(m("/*").matches("/foo"));
        assert!(m("/*").matches("/"));
        assert!(!m("/*").matches("/foo/bar"), "* does not cross '/'");
        assert!(m("/*/*").matches("/foo/bar"));
        assert!(m("/foo*bar").matches("/fooXbar"));
        assert!(m("/foo*bar").matches("/foobar"));
        assert!(m("/foo*").matches("/foo"));
    }

    #[test]
    fn double_star_parts() {
        // OSC 1.1 '//': zero or more whole parts.
        assert!(m("/foo//bar").matches("/foo/bar"));
        assert!(m("/foo//bar").matches("/foo/a/b/bar"));
        assert!(m("//bar").matches("/a/b/bar"));
        assert!(m("/foo//").matches("/foo"));
        assert!(m("/foo//").matches("/foo/a/b"));
    }

    #[test]
    fn char_classes() {
        assert!(m("/[fgh]oo").matches("/foo"));
        assert!(!m("/[fgh]oo").matches("/boo"));
        assert!(m("/[a-z]oo").matches("/koo"));
        assert!(!m("/[a-z]oo").matches("/Koo"));
        assert!(m("/[!a-z]oo").matches("/Koo"));
        assert!(!m("/[!a-z]oo").matches("/koo"));
        assert!(m("/[a-cx-z]oo").matches("/xoo"));
        assert!(!m("/[a-cx-z]oo").matches("/doo"));
    }

    #[test]
    fn alternations() {
        assert!(m("/{foo,bar}").matches("/foo"));
        assert!(m("/{foo,bar}").matches("/bar"));
        assert!(!m("/{foo,bar}").matches("/baz"));
        assert!(m("/track/{1,2}/volume").matches("/track/2/volume"));
        assert!(!m("/track/{1,2}/volume").matches("/track/3/volume"));
    }

    #[test]
    fn combined_patterns() {
        assert!(m("/*/[a-z]??").matches("/x/f00"));
        // '*' does not cross '/', so the pattern only covers one more part.
        assert!(m("/{foo,bar}/[0-9]/*").matches("/foo/7/anything"));
        assert!(!m("/{foo,bar}/[0-9]/*").matches("/foo/7/anything/here"));
        assert!(!m("/{foo,bar}/[0-9]/*").matches("/baz/7/x"));
    }

    #[test]
    fn pattern_matching_table() {
        // Practical conformance table: (pattern, address, expected).
        let cases: &[(&str, &str, bool)] = &[
            ("/f[!bc]nd/sc/s*d", "/frnd/sc/snd", true),
            ("/f[!bc]nd/sc/s*d", "/fcnd/sc/snd", false),
            ("/f[!bc]nd/sc/s*d", "/frnd/sc/sxx", false),
            ("/??nd/sc/snd", "/frnd/sc/snd", true),
            ("/?nd/sc/snd", "/frnd/sc/snd", false),
            ("/?nd/sc/snd", "/frnd/sc/snx", false),
        ];
        for (pattern, address, expected) in cases {
            assert_eq!(
                m(pattern).matches(address),
                *expected,
                "{pattern} vs {address}"
            );
        }
    }
}
