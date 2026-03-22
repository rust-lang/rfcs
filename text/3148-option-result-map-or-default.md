- Feature Name: `option_result_map_or_default`
- Start Date: 2021-07-14
- RFC PR: [rust-lang/rfcs#3148](https://github.com/rust-lang/rfcs/pull/3148)
- Rust Issue: [rust-lang/rust#138099](https://github.com/rust-lang/rust/issues/138099)

# Summary
[summary]: #summary

`Option` has the methods `unwrap`, `unwrap_or`, `unwrap_or_else` and `unwrap_or_default`. It
similarly has `map`, `map_or`, `map_or_else`, however `map_or_default` is missing. The exact same
problem exists for `Result`. This RFC is a proposal to add this method to `Option` and `Result`.

# Motivation
[motivation]: #motivation

As mentioned before, a user might reasonably expect this method to exist, based on the existence of
other `or_default` methods such as `unwrap_or_default`. Furthermore, this is a very common usecase.
Searching for `map_or_else` in `.rs` files in the official Rust repository it is incredibly common
to see instances like these:

```rust
.map_or_else(String::new, ...)
.map_or_else(SmallVec::new, ...)
.map_or_else(Vec::new, ...)
```

In fact, from a manual count at the time of writing, 25 out of the 57 occurrences of `map_or_else`
in the Rust codebase could have been replaced with an equivalent call to `map_or_default`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following implementation would get added to `core::option::Option`:

```rust
impl<T> Option<T> {
    pub fn map_or_default<U: Default, F: FnOnce(T) -> U>(self, f: F) -> U {
        match self {
            Some(t) => f(t),
            None => Default::default(),
        }
    }
}
```

The following implementation would get added to `core::result::Result`:

```rust
impl<T, E> Result<T, E> {
    pub fn map_or_default<U: Default, F: FnOnce(T) -> U>(self, f: F) -> U {
        match self {
            Ok(t) => f(t),
            Err(e) => Default::default(),
        }
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

It adds another method to `Option` and `Result`, which may be considered as cluttered by some.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

 - `.map_or_else(Default::default, ...)` can be written, although it is significantly longer.
 - In case `feature(default_free_fn)` stabilizes a user can write `.map_or_else(default, ...)`
   after `std::default::default`, which is a bit shorter.
   
However, neither alternative solves the discrepancy between `unwrap_or_default` existing but 
`map_or_default` not existing.

# Prior art
[prior-art]: #prior-art

We already have `unwrap_or_default`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.
