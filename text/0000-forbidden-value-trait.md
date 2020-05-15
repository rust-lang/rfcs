- Start Date: 2020-03-24
- RFC PR #:
- Rust Issue #:

# Summary

Some types have one or more forbidden values. Such values can be sometimes used to perform [optimizations](https://github.com/rust-lang/rust/pull/45225). For example, rustc currently uses a known forbidden value of a certain type `A` (like `&T`, `Box<T>`, `NonNull<T>`, `NonZeroI8` and some enum types) to represent the `None` enum variant of `Option<A>`. I propose to give users the ability to enable the same optimizations for types rustc currently doesn't know a forbidden value exists for.

# Motivation

This would increase the number of situations where the compiler can do the aforementioned, genuinely useful optimizations.

# Detailed design

Add the following trait somewhere in the standard library:
```rust
use std::{array::FixedSizeArray, mem::size_of};

unsafe trait ForbiddenValues {
    type Forbidden: FixedSizeArray<[u8; size_of::<Self>()]>;

    const FORBIDDEN: Self::Forbidden;
}
```

To implement `ForbiddenValues` for type `T`, the following conditions must be met:

1. For each value `t` of type `T`, and for each item `f` in `T::FORBIDDEN`:
```rust
*(&raw const t as *const [u8; size_of::<T>()]) != f
```
2. `T` must not be a type that has forbidden values the compiler already knows about (e.g. `char`)

Then compilers would be allowed to use `T::FORBIDDEN` to represent forbidden values of type `T` in whatever optimizations they decide to perform.

# Alternatives

This is a simple proposal, but a step further would be to make all primitive and standard library types that have forbidden value(s) implement `ForbiddenValues`. And an even further step would be have Rust (the language) specify which optimizations are guaranteed to happen.

Presumably the following syntax should work in the future:
```rust
unsafe trait ForbiddenValues {
    const COUNT: usize;
    const FORBIDDEN_VALUES: [[u8; size_of::<Self>()]; Self::COUNT];
}
```

# Unresolved questions

Are there some alignment issues with `ForbiddenValues::FORBIDDEN`? I don't think so, because the compiler is free to use those bytes however it likes.

Given that producing a forbidden value of a `bool`, `char`, or an enum type is considered undefined behaviour, I suppose it makes sense to ask if it should be considered undefined behaviour to produce a value `t` of type `T` such that for some item `f` in `T::FORBIDDEN`, `*(&raw const t as *const [u8; size_of::<T>()]) == f`.
