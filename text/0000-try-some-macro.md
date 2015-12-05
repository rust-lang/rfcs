- Feature Name: try-some-macro
- Start Date: 2015-12-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `try_some!` macro for `Option` equivalent to `try!` for `Result`.

# Motivation
[motivation]: #motivation

It would simplify some `Option` related functions.

# Detailed design
[design]: #detailed-design

Just simple macro that would be almost the same as `try!` macro:

```rust
macro_rules! try_some {
    ($expr:expr) => (match $expr {
        Some(val) => val,
        None => return None,
    });
}
```

Additionally I think that macro to early return from function that returns `()`
would be useful:

```rust
// …
($expr:expr => return) => (match $expr {
    Some(val) => val,
    None => return,
})
```

Whole functionality is already implemented in [`soma` crate][soma], but I think
it would be useful to have that in `libcore`.

# Drawbacks
[drawbacks]: #drawbacks

None.

# Alternatives
[alternatives]: #alternatives

Left as is.

# Unresolved questions
[unresolved]: #unresolved-questions

What should be macro name? Is `try_some!` ok? Would it be desired in community?
