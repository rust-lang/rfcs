- Start Date: 2020-03-24
- RFC PR #:
- Rust Issue #:

# Summary

Some types have one or more forbidden values. Such values can be sometimes used to perform [optimizations](https://github.com/rust-lang/rust/pull/45225). For example, rustc currently uses a known forbidden value of certain types `A` (like `&T`, `Box<T>`, `NonNull<T>`, `NonZeroI8` and some enum types) to represent the `None` enum variant of `Option<A>`. I propose to give users the ability to enable the same optimizations for types rustc currently doesn't know a forbidden value exists for.

# Motivation

This would increase the number of situations where the compiler can do the aforementioned, genuinely useful optimizations.

# Detailed design

Add the following trait somewhere in the standard library:
```rust
unsafe trait ForbiddenValue<B>
where B: FixedSizeArray<u8>
{
    const FORBIDDEN_VALUE_BYTES: B;
}
```

To implement `ForbiddenValue<B>` for type `T`, the following conditions must be met:
1) Type `T` has a stable layout and representation
2) Type `B` is a fixed-size array of `u8`
3) `std::mem::size_of::<T>() == std::mem::size_of::<B>()`
4) For each value `v` of type `T`, `*(&v as *const T as *const B) != T::FORBIDDEN_VALUE_BYTES`

Then compilers would be allowed to use `T::FORBIDDEN_VALUE_BYTES` to represent a forbidden value of `T` in whatever optimizations they decide to perform.

# Drawbacks

Unless we turn the optimizations themselves into a language feature (which is not what I'm proposing), someone might mistakenly assume that some optimization is always guaranteed to happen, and rely on some value being equal to `T::FORBIDDEN_VALUE_BYTES`.

# Alternatives

This is a simple proposal, but a step further would be to make all standard library types that have a forbidden value that is currently used for optimizations implement `ForbiddenValue`. And an even further step would be have Rust (the language) specify which optimizations are guaranteed to happen.

With const generics, the trait could be better as:
```rust
unsafe trait ForbiddenValue<const SIZE: usize> {
    const FORBIDDEN_BYTES: [u8; SIZE];
}
```

# Unresolved questions

Are there some alignment issues with `ForbiddenValue::FORBIDDEN_VALUE_BYTES`? I don't think so, because the compiler is free to use those bytes however it likes.
