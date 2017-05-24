- Feature Name: Asserts
- Start Date: 2016-06-29
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Asserts would be a series of macros to use in `#[test]`, expanding
on `assert_eq`. This rfc proposes that the following macros be added:

- `assert_gt` (greater than)
- `assert_lt` (less than)
- `assert_ge` (greater than or equal)
- `assert_le` (less than or equal)

# Motivation
[motivation]: #motivation

The goal of this feature is to provide a feature-ful set of asserts
with consistent formatting and messaging for use in `#[test]`. This
proposal is a follow up to a previous rfc that proposed only
`assert_ne` (not equals).

# Detailed design
[design]: #detailed-design

These macros should be added with nearly identical implentation as
`assert_eq`, with changes to the condition and message only:

```rust
macro_rules! assert_eq {
    ($left:expr , $right:expr) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    panic!("assertion failed: `(left == right)` \
                           (left: `{:?}`, right: `{:?}`)", left_val, right_val)
                }
            }
        }
    })
}
```

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

Any addition to the standard library will need to be maintained forever, so it is
worth weighing the maintenance cost of this over the value add. Given that it is so
similar to `assert_eq`, I believe the weight of this drawback is low.

# Alternatives
[alternatives]: #alternatives

Alternatively, users implement this feature themselves, or use a crate.

# Unresolved questions
[unresolved]: #unresolved-questions

It was brought up in the rfc on `assert_ne` that if `assert!` were rewritten as a
syntax extension instead of as a macro as it is now, then it would be possible to
 automatically detect `assert!(x < y)`, `assert!(x == y)`, `assert!(x != y)`.

This, however, would be a longer and more serious change than the proposed addtional
macros.

