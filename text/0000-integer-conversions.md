- Feature Name: integer_casts
- Start Date: 2015-07-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Use traits `Into`/`From` for lossless conversions between built-in integer types. Implement new
traits for checked and wrapping casts for potentially lossy integer conversions analogous to checked
and wrapping arithmetic operations.

# Motivation

It is useful to separate lossless integer conversions requiring minimal attention from a programmer
from potentially lossy conversions requiring more thought and some analysis. Currenly all integer
conversions are performed with operator `as` which acts as a Swiss knife and can convert anything to
anything without any precautions thus requiring extra attention from a programmer.

Besides, overflow checks proved to be useful for arithmetic operations, they can also be useful for
casts, but operator `as` doesn't perform such checks and just truncates (wraps) the results.
The proposed new methods allow to separate potentially lossy conversions into intentionally wrapping
casts and casts where truncation should be considered an error.

# Detailed design

## Implement `Into`/`From` for integer types

Assume T and U are built-in integer types, then `Into<U>` is implemented for `T` and `From<T>` is
implemented for `U` iff the conversion from `T` to `U` is always lossless.
A good visualization (without `usize` and `isize`) can be found [here][1].

Implementations for `usize` and `isize` are platform dependent by design. If code is ported
to a new platform and some of `into()` conversions are not lossless anymore, they have to be
reviewed and replaced with checked casts. The porting effort shouldn't be large and potentially
caught mistakes can easily outweight it, for example, porting Rust codebase from 64-bit Windows to
32-bit Linux took minimal amount of time (see the Implementation section).

## Introduce new trait `IntegerCast` into `std::num` (and `core::num`)

```
trait IntegerCast<Target> {
    fn cast(self) -> Target;
    fn wrapping_cast(self) -> Target;
    fn overflowing_cast(self) -> (Target, bool);
    fn saturating_cast(self) -> Target;
}
```

The methods are analogous to methods for arithmetic operations like `add`/`wrapping_add`/
`overflowing_add`/`saturating_add`.
- `cast()` is equivalent to `as` but panics when the conversion is lossy and debug assertions are on.
- `wrapping_cast()` is completely equivalent to `as`, it wraps (=truncates) the value.
- `overflowing_cast()` is equivalent to `as` but also supplies overflow flag as a second result.
- `saturating_cast()` clamps the value into the range of the target type.

Statistically, `cast()` is the most common of these four methods, `wrapping_cast()` is less common
and usually related to hashes, random numbers or serialization, and the other methods are rare and
highly specialized.

`IntegerCast` is implemented for all pairs of built-in integer types including pairs with lossless
conversion (this is needed for portability, some conversions can be lossless on one platform and
potentially lossy on other).

## Make `usize` default type parameter for `Index`, `IndexMut`, `Shl` and `Shr`

These traits don't currently have default type parameters and setting them will make type inference in index position possible in cases like:
```
let a: u16 = 10;
let b = c[a.into()];
```

## Bonus: Conversions between integers and raw pointers
With flat memory model and no validity guarantees raw pointers are essentially weird integers and
they are relatively often converted to normal integers (usually `usize`) and back with operator
`as`. However, operator `as` can truncate a pointer even to `i8` without blinking an eye, so it
would be nice to have a dedicated checked conversion method instead of it - the checking in this case is
purely compile time. As a bonus, the conversion method doesn't usually require specifying the target
`usize`.

```
// in core::ptr
trait AsInteger<Target = usize> {
    fn as_integer(self) -> Target;
}

impl<T> AsInteger<$Target> for *const T {
    fn as_integer(self) -> $Target{ self as $Target }
}
impl<T> AsInteger<$Target> for *mut T {
    fn as_integer(self) -> $Target{ self as $Target }
}

// in core::num
trait AsPtr {
    fn as_ptr<T>() -> *const T;
    fn as_mut_ptr<T>() -> *mut T;
}

impl AsPtr for $Source {
    fn as_ptr<T>() -> *const T { $Source as *const T }
    fn as_mut_ptr<T>() -> *mut T { $Source as *mut T }
}
```

where `$Target` and `$Source` belong to the set `{usize, isize, underlying_type(usize),
underlying_type(isize)}`. `underlying_type(T)` here is a fixed-size type equivalent of `T`, for
example `underlying_type(usize) == u64` on 64-bit platforms.  
`as_ptr` and `as_mut_ptr` can arguably be inherent methods.

## Implementation
An experiment implementing similar but somewhat different design and evaluating its practical
impact is described [here][2].

# Drawbacks

This design is not fully ergonomic without type inference fallback based on default type parameters
(sometimes `into()` and `cast()` need redundant type hints) and without type ascription (there's no
way to give a type hint for the target type of `Into` inline). The first problem is currently being
addressed in pending pull request, its solution will reduce the need in type hints to the minimum.
The second problem will hopefully be resolved too in the near future.

# Alternatives

1. Do nothing. No one will use the new methods anyway because the built-in alternative - `as` - is
so short and convenient and doesn't require any imports and even works in constant expressions,
and overflows never happen in code written by a reasonable programmer.

2. Use a separate trait for lossless conversions instead of `Into`, e.g.

    ```
    pub trait Widen<Target>: Sized {
        fn widen(self) -> Target;
    }
    ```

    It would still make sense to implement `Into` for lossless integer conversions, because they are
totally reasonable conversions and `Into` is by definition a trait for, well, reasonable
conversions. In this case a separate trait `Widen` would feel like a duplication.
The trait `Widen` will have to live in the prelude, like `Into`, otherwise it will be rarely
used, because the alternative (`as`) doesn't require importing anything (something similar already
happens with `ptr::null` vs `0 as *const T`). Adding new names to the prelude may be considered a
drawback.

3. Core language solution for lossless conversions, e.g. new operator `as^` or `lossless_as` or
unary plus. This is much more intrusive and doesn't probably pull its weight.
It would still make sense to implement `Into` for lossless integer conversions, because they are
reasonable conversions. There's a non-zero chance that `Into` will get its own language sugar
somewhere in the remote future.

4. Make lossless integer conversions implicit at language level. This alternative is not pursued.
In the relevant thread on internals many people spoke against this alternative and it had no
consensus. Moreover, originally the absence of these conversions is [by design][4] and not just an
omission.

5. Methods `as()`/`wrapping_as()/...` may look better but `as` can't be used as a method name.
Theoretically `as` can be made a context dependent keyword, then the names will become available.

6. `IntegerCast` can be splitted into several traits - `IntegerCast/WrappingIntegerCast/...`, but
there's not much sense in multiplying entities - `IntegerCast` is ought to be implemented for a
limited set of types and all its methods always go in group.

7. Sign conversions with fixed target type described in [the experiment][2] are subsumed by `IntegerCast` in this
design, but they can probably be useful by themselves. They would also have to be provided in
several variants - `as_signed()/as_signed_wrapping()/...`.

8. Some alternative names: `as` -> `to`, `cast` -> `conv`. Bikeshedding is welcome.

# Unresolved questions

None so far

[1]: https://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/1141/45
[2]: https://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/1141/70
[3]: https://internals.rust-lang.org/t/implicit-widening-polymorphic-indexing-and-similar-ideas/1141
[4]: http://graydon2.dreamwidth.org/2015/07/03/
