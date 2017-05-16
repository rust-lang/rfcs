- Feature Name: use-in-trait-and-impl
- Start Date: 2017-04-17
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Allow the usage of `use` inside `trait` and `impl` blocks.

# Motivation
[motivation]: #motivation

Some of us want to scope `use declaration` aliases as close to the code that
uses them.

This also increases ergonomics and readability ever so slightly because there
would be less scrolling since the declaration would be closer to the use site.

# Detailed design
[design]: #detailed-design

Allow `use declarations` to be inside of `traits`. The bindings that
the `use declaration` creates would be scoped to the trait block.

Allow `use declarations` to be inside of `implementations`. The bindings that
the `use declaration` creates would be scoped to the implementation.

These `use declarations` would not allow the `pub` visibility modifier.

## Example - use in impl

This example uses `use` in the implementation block to put Ordering and its
enum variant constructors into scope.

Note that this `impl` could be written without the full match, using only the
not operator, but it's representativge of real usage.

```rust
struct ReverseCompare(i32);

impl std::cmp::PartialOrd for ReverseCompare {
    use std::cmp::Ordering
    use std::cmp::Ordering::*;

    fn partial_cmp(&self, other: &ReverseCompare) -> Ordering {
        match (self.0).partial_cmp(other.0) {

            None => unreachable!(),
            Some(Greater) => Some(Lesser),
            Some(Less) => Some(Greater),
            Some(Eq) => Some(Eq)
        }
    }
}

```

## Grammar Changes

In `trait_item`, add a variant `trait_use`.

Define `trait_use` as

```
trait_use
: use_item
;
```

In `impl_item`, add a variant `impl_use`.

Define `impl_use` as

```
impl_use
: use_item
;
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This is a continuation of current Rust concepts. No new termionlogy needs to
be taught.

_The Rust Reference_ would need to be updated. Specifically the
`Use Declarations` section would need to be say that it can also be used in
the new locations. Relatedly, the `Use Delcarations` section should also be
updated to say the scope of the bindings it creates. Right now only the
`Block Expression` section discusses the scope of `use declarations` which
feels like the wrong place.

_The Rust Grammar Reference_ would need to be updated in the `Traits` and
`Implementations` sections. Although, currently, both of those sections
are completely empty and would have to be written.

Looking at the new _The Rust Programming Language_, there's no discussion on
where `use declarations` are valid currently, so unless that gets added,
there's no reason to mention the changes in here explicitly.

_Rust by Example_'s "The use declaration" section could have examples added
for `use declarations` in `implementations`. Probably also true for `traits`.

# Drawbacks
[drawbacks]: #drawbacks

All other items in a trait or implementation are public by default and don't
even allow a visiblity modifier. This would be the first item that would not
be public.

# Alternatives
[alternatives]: #alternatives

Do nothing. This is purely an ergonomics improvement and doesn't make anything
currently impossible actually possible.

Allow `pub` in use declarations, just like in blocks. A previous version of
this RFC allowed that, but there have been RFCs for attributing meaning to
`pub use` in traits and having it do nothing for now would be a back-compat
hazard.

Add a `use Path in Item/Expr` construct. This could also be done, but there's
no reason not to allow `use` as is in more places.

# Unresolved questions
[unresolved]: #unresolved-questions

None
