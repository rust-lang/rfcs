- Feature Name: esc_escape
- Start Date: 2015-12-29
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a byte escape `\e` as shorthand for 0x1B (ESC), similar to GCC's `\e`.

# Motivation
[motivation]: #motivation

ANSI escape codes are a vital part of CLIs. These are used both in stdin (special keys, control commands, etc.) and stdout (colors, text formatting, change in previous text, movement of the cursor, etc.). Adding a byte escape for it will make it easier to read the source code of and write interactive CLIs.

Many compilers do already have this byte escape, GCC is an example (however, `\e` is not a part of ISO C).

# Detailed design
[design]: #detailed-design

Add a byte escape `\e` as shorthand for 0x1B (ESC) to characters, strings, byte characters, and byte strings. Furthermore, add it to the escape iterators.

# Drawbacks
[drawbacks]: #drawbacks

None.

# Alternatives
[alternatives]: #alternatives

## Add a character escape for CSIs

Add a character escape for Control Sequence Introducers (`\x1B[`). This covers the majority of ANSI escape codes and do not require the extra `[` (as `\e` do), but it do not cover _all_ cases (for example, `\e^`, `\e_`, `\eN`, `\eO`, `\eP` and so on).

# Unresolved questions
[unresolved]: #unresolved-questions

None.
