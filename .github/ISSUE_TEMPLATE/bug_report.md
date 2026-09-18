---
name: Bug report
about: Report a protocol, parsing, or API defect
labels: bug
---

**Affected crate(s)**

<!-- e.g. tpt-av-control-osc 0.1.0 -->

**What happened**

<!-- Include packet hex dumps where possible — this suite parses untrusted
     network input, so malformed-input reports are especially welcome. -->

**What you expected to happen**

**Minimal reproduction**

```rust
// smallest code that shows the problem
```

**Platform**

- OS:
- Rust version (`rustc -V`):

**Checks**

- [ ] I confirmed the behavior against the relevant protocol spec (OSC 1.0/1.1, M2-101-U, M2-115, ANSI E1.31, Art-Net 4)
- [ ] I checked existing issues for duplicates

For security-sensitive reports (panic/hang/memory issues reachable from
hostile input), please follow [SECURITY.md](../SECURITY.md) instead of
filing a public issue.
