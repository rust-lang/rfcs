- Feature Name: exhaustive_match_output
- Start Date: 2022-10-31
- RFC PR: [rust-lang/rfcs#3340](https://github.com/rust-lang/rfcs/pull/3340)
- Rust Issue: [rust-lang/rust#103818](https://github.com/rust-lang/rust/issues/103818)

# Summary
[summary]: #summary

Add an attribute which is only applicable to match statements called `#[exhaustive_output]` which will verify that
the match statement has an arm for every variant of the enum type which it returns. This attribute will not be
applicable to match statements that don't return an enum type.

# Motivation
[motivation]: #motivation

Oftentimes it is necessary to convert unrestricted values into enum variants. Unrestricted values can include integers, strings, UUIDs, even other much larger enums.
When adding a new variant to the output enum, it's possible the author forgot to update the match statement which converts from unrestricted values, thus leaving
the new enum variant unusable. This attribute would prevent this scenario, by alerting the author that they didn't update the input match statement.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## exhaustive_output

The `#[exhaustive_output]` attribute will cause a compile time warning to be emitted if the match statement beneath it doesn't have an arm for all possible output variants.
You might want this when parsing input from integers, strings, or UUIDs and mapping that input to enum variants.

### Example

```rust
pub enum Kind {
    Cat,
    Dog,
    Lizard,
}

impl From<u32> for Kind {
    fn from(o: u32) -> Kind {
        #[exhaustive_output] // <-- Warning emitted, Kind::Lizard is not covered.
        match o {
            1 => Kind::Cat,
            2 => Kind::Dog,
            _ => panic!("Unknown kind");
        }
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new warning will be added to rustc with diagnostics like

```
non-exhaustive output in match statement that requires exhaustive output

note: enum variant <enum_name>::<enum_variant> is not returned by any arm in this match statement. Please add an arm that returns <enum_name>::<enum_variant>.
```

This warning can be changed to deny with a module or crate attribute like
`#![deny(exhaustive_output_missing)]`

The warning will be triggered when a match statement tagged with the attribute is found and that match statement does not have arms for all possible variants\* of the
enum which it returns.

\* Please note this is **not** "all possible values", it is "all possible variants". This means that if an enum variant is a tuple or struct variant, it is not necessary
to exhaustively populate the fields of the variant. This lint will only trigger if a variant is missing from the possible outputs of the match statement.

Applying `#[exhaustive_output]` to anything other than a `match` statement will produce an error.

# Drawbacks
[drawbacks]: #drawbacks

This is yet another attribute for the language, and it's possible it will be difficult to teach exactly how it works and in what situations you should use it. Maybe this can be fixed by
making the proposed educational material easier to read and understand.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

One alternative is to rely on the dead code lint, which will inform you when a variant hasn't been constructed. However it will fail to catch this if the 
variant is constructed elsewhere, or the enum is public.

Other alternatives would be to try and implement something like this outside of rustc. Let's consider those options.

### Use the `strum_macros` crate to automatically generate these types of match statements

This is actually a pretty good idea, though it's not compatible with some representations of enum values, such as UUIDs and more complex data structures.
Additionally there may be significant differences between how the data is represented in user input, and how you want to represent it in your enum. Finally,
maybe you just don't want to add a dependency for this.

### Author your own macro which combines "defining the enum" and "pairing the enum with its external id"

This is much more robust than the aforementioned, though it is a good deal more labor and you might only reach for it if this option occurs to you.

# Prior art
[prior-art]: #prior-art

The author is not aware of any prior art on this, but will happily add some to the RFC if that changes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

This attribute as written must be used

- With a match statement
- That returns an enum type

It might be possible to expand this to match statements which return trait objects in the future, though the trait would have to be private as you couldn't truly meet
this restriction for a public trait.
