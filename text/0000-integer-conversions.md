- Feature Name: integer_casts
- Start Date: 2015-07-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Implement new methods for checked and wrapping casts for potentially lossy integer conversions
analogous to checked and wrapping arithmetic operations.

# Motivation

Overflow checks proved to be useful for arithmetic operations, they can also be useful for
casts, but operator `as` doesn't perform such checks and just truncates/wraps the results.
The proposed new methods allow to separate potentially lossy conversions into intentionally wrapping
casts and casts where truncation should be considered an error.

These methods and already implemented `Into`/`From` for lossless integer conversions are supposed to
replace most uses of operator `as` for conversions between integers.

# Detailed design

## Introduce new inherent methods for all the primitive integer types

```
impl i32 {
    fn cast<Target: __UnspecifiedTrait>(self) -> Target;
    fn wrapping_cast<Target: __UnspecifiedTrait>(self) -> Target;
    fn checked_cast<Target: __UnspecifiedTrait>(self) -> Option<Target>;
    fn overflowing_cast<Target: __UnspecifiedTrait>(self) -> (Target, bool);
    fn saturating_cast<Target: __UnspecifiedTrait>(self) -> Target;
}
```

`__UnspecifiedTrait` is a private unstable trait used as an implementation detail. It is guaranteed
that this trait is implemented for all the primitive integer types.

The methods correspond to existing methods for arithmetic operations like `add`/`wrapping_add`/
`checked_add`/`overflowing_add`/`saturating_add`.
- `cast()` is equivalent to `as` but panics when the conversion is lossy and debug assertions are on.
- `wrapping_cast()` is completely equivalent to `as`, it wraps (=truncates) the value.
- `checked_cast()` returns `None` if the conversion is lossy and `Some(self as Target)` otherwise.
- `overflowing_cast()` is equivalent to `as` but also supplies overflow flag as a second result (true
on overflow, false otherwise).
- `saturating_cast()` clamps the value into the range of the target type.

Statistically, `cast()` is the most common of these methods, `wrapping_cast()` is less common
and usually related to hashes, random numbers or serialization, and the other methods are rare and
highly specialized.

The conversion methods are implemented for all pairs of built-in integer types including pairs with
lossless conversions (this is required for portability, some conversions can be lossless on one
platforms and potentially lossy on others).

## Implementation
An experiment implementing similar but somewhat different design and evaluating its practical
impact is described [here][2].

## Why `std` and not an external crate

People will likely not bother depending on external crate for such a simple functionality and
will just use `as` instead, but using `as` is exactly what we would like to avoid.

# Drawbacks

None.

# Alternatives

1. Do nothing. No one will use the new methods anyway because the built-in alternative - `as` - is
so short and convenient and doesn't require any imports and even works in constant expressions,
and overflows never happen in code written by a reasonable programmer.

2. Names `as()`/`wrapping_as()/...` may look better than `cast()`/`wrapping_cast()/...`, but `as`
can't be used as a method name. Theoretically `as` can be made a context dependent keyword, then the
names will become available.

3. Use methods of a trait `IntCast` instead of inherent methods. The library team is unwilling to
expose numeric traits from the standard library even if they are unstable and brought in scope by
the prelude.

4. Diverge from arithmetic operations and always panic in `cast()`, not only with enabled assertions.
This would make `cast()` equivalent to `checked_cast().unwrap()`.

5. Sign conversions with fixed target type described in [the experiment][2] are subsumed by `IntCast`
in this design, but they can probably be useful by themselves. They would also have to be provided
in several variants - `as_signed()/as_signed_wrapping()/...`.

# Unresolved questions

None.

[1]: https://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/1141/45
[2]: https://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/1141/70
[3]: https://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/1141
[4]: http://graydon2.dreamwidth.org/2015/07/03/
