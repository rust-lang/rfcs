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
unsafe trait ForbiddenValues<A, B>
where
    A: FixedSizeArray<B>,
    B: FixedSizeArray<u8>,
{
    const FORBIDDEN_VALUES: A;
}
```

To implement `ForbiddenValues<A, B>` for type `T`, the following conditions must be met:
1) Type `T` has a stable layout and representation
2) Type `B` is a fixed-size array of `u8`
3) Type `A` is a fixed-size array of `B`
4) `std::mem::size_of::<T>() == std::mem::size_of::<B>()`
5) For each value `v` of type `T`, and for each item `f` in `T::FORBIDDEN_VALUES`, `*(&v as *const T as *const B) != f`

Then compilers would be allowed to use `T::FORBIDDEN_VALUES` to represent forbidden values of type `T` in whatever optimizations they decide to perform.

Users could then, for example, implement a wrapper for floating point values that are always finite like this:
```rust
#![feature(const_transmute)]

#[repr(transparent)]
struct FastFloat(f32);

unsafe impl ForbiddenValues<[[u8; 4]; 3], [u8; 4]> for FastFloat {
    const FORBIDDEN_VALUES: [[u8; 4]; 3] = unsafe {
        [
            mem::transmute(f32::NAN),
            mem::transmute(f32::INFINITY),
            mem::transmute(f32::NEG_INFINITY),
        ]
    };
}
```

# Drawbacks

Unless we turn the optimizations themselves into a language feature (which is not what I'm proposing), someone might mistakenly assume that some optimization is always guaranteed to happen, and rely on some value being equal to `T::FORBIDDEN_VALUES[0]`. This design doesn't make it easy to implement a large number of forbidden values (like a range of integers for example).

# Alternatives

This is a simple proposal, but a step further would be to make all standard library types that have a forbidden value that is currently used for optimizations implement `ForbiddenValue`. And an even further step would be have Rust (the language) specify which optimizations are guaranteed to happen.

With const generics, the trait could be better as:
```rust
unsafe trait ForbiddenValues<const SIZE: usize, const COUNT: usize> {
    const FORBIDDEN_VALUES: [[u8; SIZE]; COUNT];
}
```

# Unresolved questions

Are there some alignment issues with `ForbiddenValues::FORBIDDEN_VALUES`? I don't think so, because the compiler is free to use those bytes however it likes.

Given that producing a forbidden value of a `bool`, `char`, or an enum type is considered undefined behaviour, I suppose it makes sense to ask if producing a value of type `T` that is equal to some item in `T::FORBIDDEN_VALUES` should be considered undefined behaviour.
