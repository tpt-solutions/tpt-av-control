def edit(path, old, new, count=1):
    src = open(path, encoding="utf8").read()
    assert old in src, "NOT FOUND in %s: %r" % (path, old[:80])
    src = src.replace(old, new, count)
    open(path, "w", encoding="utf8", newline="\n").write(src)


# 3. OscAddressMatcher: bound worst-case match cost with a step budget.
edit(
    "tpt-av-control-osc/src/address.rs",
    """/// Recursion cap so hostile patterns can't exhaust the stack.
const MAX_PATTERN_LEN: usize = 1024;""",
    """/// Recursion cap so hostile patterns can't exhaust the stack.
const MAX_PATTERN_LEN: usize = 1024;

/// Total match-step budget per `matches()` call. Backtracking through
/// nested `*`/`//`/`{}` groups is cut off once the budget is spent, which
/// bounds worst-case match cost for hostile pattern/address pairs.
const MAX_MATCH_STEPS: u32 = 10_000;""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """    /// Checks whether `address` matches the pattern.
    pub fn matches(&self, address: &str) -> bool {
        if self.pattern.len() > MAX_PATTERN_LEN {
            return false;
        }
        match_parts(self.pattern.as_bytes(), 0, address.as_bytes(), 0)
    }""",
    """    /// Checks whether `address` matches the pattern.
    ///
    /// Matching is bounded: patterns longer than 1024 bytes never match,
    /// and backtracking is cut off after a fixed step budget (bounded-false
    /// on hostile inputs, never a hang).
    pub fn matches(&self, address: &str) -> bool {
        if self.pattern.len() > MAX_PATTERN_LEN {
            return false;
        }
        let mut budget = MAX_MATCH_STEPS;
        match_parts(self.pattern.as_bytes(), 0, address.as_bytes(), 0, &mut budget)
    }""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """/// Matches `pattern[pi..]` against `address[ai..]` (byte-wise; all OSC
/// pattern characters are ASCII).
fn match_parts(pattern: &[u8], mut pi: usize, address: &[u8], mut ai: usize) -> bool {
    // Iterative fast path over literal characters.
    while pi < pattern.len() {""",
    """/// Matches `pattern[pi..]` against `address[ai..]`, spending from
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
    while pi < pattern.len() {""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """                    .any(|k| match_parts(pattern, pi + 1, address, k));
            }""",
    """                    .any(|k| match_parts(pattern, pi + 1, address, k, budget));
            }""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """                    .any(|k| match_parts(pattern, pi + 2, address, k));
            }""",
    """                    .any(|k| match_parts(pattern, pi + 2, address, k, budget));
            }""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """            b'{' => {
                // '{' consumes exactly one character run, so a failed
                // alternation fails the whole match.
                return match_alternation(pattern, pi, address, ai).unwrap_or(false);
            }""",
    """            b'{' => {
                // '{' consumes exactly one character run, so a failed
                // alternation fails the whole match.
                return match_alternation(pattern, pi, address, ai, budget).unwrap_or(false);
            }""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """/// Attempts a `{a,b,c}` alternation at `pattern[pi]`. Returns `Some(matched)`
/// if the class is well-formed.
fn match_alternation(pattern: &[u8], pi: usize, address: &[u8], ai: usize) -> Option<bool> {""",
    """/// Attempts a `{a,b,c}` alternation at `pattern[pi]`. Returns `Some(matched)`
/// if the class is well-formed.
fn match_alternation(
    pattern: &[u8],
    pi: usize,
    address: &[u8],
    ai: usize,
    budget: &mut u32,
) -> Option<bool> {""",
)
edit(
    "tpt-av-control-osc/src/address.rs",
    """    for (s, e) in alternatives {
        if address[ai..].starts_with(&pattern[s..e])
            && match_parts(pattern, after_class, address, ai + (e - s))
        {
            return Some(true);
        }
    }
    Some(false)
}""",
    """    for (s, e) in alternatives {
        if address[ai..].starts_with(&pattern[s..e])
            && match_parts(pattern, after_class, address, ai + (e - s), budget)
        {
            return Some(true);
        }
    }
    Some(false)
}""",
)

# 4. Fixture: bound-check writes defensively.
edit(
    "tpt-av-control-dmx/src/fixture.rs",
    """    fn write(&self, universe: &mut DmxUniverse, offset: usize, value: u8) {
        universe.set_channel(self.start_address + offset as u16, value);
    }""",
    """    fn write(&self, universe: &mut DmxUniverse, offset: usize, value: u8) {
        // Defensive re-check: `start_address` is a public field, so writes
        // must never panic or overflow the universe even if it was mutated
        // after patching.
        let index = usize::from(self.start_address) + offset;
        if index < crate::dmx::DMX_CHANNELS {
            universe.set_channel(index as u16, value);
        }
    }""",
)

# 5. webrtc: checked arithmetic in decode.
edit(
    "tpt-av-control-webrtc/src/envelope.rs",
    """                    0x04 => {
                        let len = read_u32(cursor)? as usize;
                        let text = data.get(cursor + 4..cursor + 4 + len).ok_or_else(|| {
                            ControlError::InvalidData("string value truncated".into())
                        })?;
                        ParameterValue::String(String::from_utf8_lossy(text).into_owned())
                    }""",
    """                    0x04 => {
                        let len = read_u32(cursor)? as usize;
                        let start = cursor.checked_add(4).ok_or_else(|| {
                            ControlError::InvalidData("string offset overflow".into())
                        })?;
                        let end = start.checked_add(len).ok_or_else(|| {
                            ControlError::InvalidData("string length overflow".into())
                        })?;
                        let text = data
                            .get(start..end)
                            .ok_or_else(|| {
                                ControlError::InvalidData("string value truncated".into())
                            })?;
                        ParameterValue::String(String::from_utf8_lossy(text).into_owned())
                    }""",
)
edit(
    "tpt-av-control-webrtc/src/envelope.rs",
    """            Some(&0x03) => {
                let len = read_u32(1)? as usize;
                let text = data
                    .get(5..5 + len)
                    .ok_or_else(|| ControlError::InvalidData("text truncated".into()))?;
                Ok(ControlEnvelope::Text(String::from_utf8_lossy(text).into_owned()))
            }""",
    """            Some(&0x03) => {
                let len = read_u32(1)? as usize;
                let end = 5usize.checked_add(len).ok_or_else(|| {
                    ControlError::InvalidData("text length overflow".into())
                })?;
                let text = data
                    .get(5..end)
                    .ok_or_else(|| ControlError::InvalidData("text truncated".into()))?;
                Ok(ControlEnvelope::Text(String::from_utf8_lossy(text).into_owned()))
            }""",
)

print("remaining phase 7 fixes applied")
