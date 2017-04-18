- Feature Name: use-in-impl-and-match
- Start Date: 2017-04-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow the usage of `use` inside `impl` blocks and `match` blocks.

# Motivation
[motivation]: #motivation

Some of us want to scope `use declaration` aliases as close to the code that
uses them.

This also increases ergonomics and readability ever so slightly because there
would be less scrolling since the declaration would be closer to the use site.

# Detailed design
[design]: #detailed-design

Allow `use declarations` to be inside of `implementations`. The bindings that
the `use declaration` creates would be scoped to the implementation.

Also `use declarations` to be inside of `match expressions` where `match arms`
are allowed. The bindings that the `use declaration` creates would be
scoped to all `match arms` and other `use declarations`

These `use declarations` would allow the `pub` modifier. It would do nothing,
just like it currently does in block expressions.

## Example

This example uses `use` in both the `impl` and the `match`.

Note that this `impl` could be written without the full match, but it's close
enough to actual examples.

```rust
struct ReverseCompare(i32);

impl std::cmp::PartialOrd for ReverseCompare {
    use std::cmp::Ordering

    fn partial_cmp(&self, other: &ReverseCompare) -> Ordering {
        match (self.0).partial_cmp(other.0) {
            use Ordering::*;

            None => unreachable!(),
            Some(Greater) => Some(Lesser),
            Some(Less) => Some(Greater),
            Some(Eq) => Some(Eq)
        }
    }
}
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This is a continuation of current Rust concepts. No new termionlogy needs to
be taught. That said, if the "use in match" part is accepted, it would be
useful to coin a new term for the two things that can show up where `match
arms` currently are in `match expressions`.

_The Rust Reference_ would need to be updated. Specifically the
`Use Declarations` section would need to be say that it can also be used in
the new locations. Relatedly, the `Use Delcarations` section should also be
updated to say the scope of the bindings it creates. Right now only the
`Block Expression` section discusses the scope of `use declarations` which
feels like the wrong place.

_The Rust Grammar Reference_ would need to be updated in the `Implementations`
and `Match Expressions` sections. Although there's currently no
`Implementations` section at all right now.

Looking at the new _The Rust Programming Language_, there's no discussion on
where `use declarations` are valid currently, so unless that gets added,
there's no reason to mention the changes in here explicitly. Except perhaps
that the allowance where `match arms` could be mentioned in the place where
`match expressions` are discussed.

_Rust by Example_'s "The use declaration" section could have examples added
for `use declarations` in `implementations` and `match expressions`.

# Drawbacks
[drawbacks]: #drawbacks

For `use in match`, this means that we can allow things other than
pattern => expr that are separated by semicolons and not commas.

# Alternatives
[alternatives]: #alternatives

Only allow `use` in one of `impl` or `match`. Or do nothing. This is
purely an ergonomics improvement and doesn't make anything impossible
possible.

The idea of using `_::Greater` to elide the enum name in match arms would
reduce the added ergonomics of `use declarations` in `match expressions`.

Make `pub use` a hard error in `implementations` and `match expressions`. The
allowance already exists for block expressions where the `pub` is ignored.
Macro authors have suggested that allowing it there like that makes it
easier to write macros with use declarations.

# Unresolved questions
[unresolved]: #unresolved-questions

None
