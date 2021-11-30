- Feature Name: `float_next_up_down`
- Start Date: 2021-09-06
- RFC PR: [rust-lang/rfcs#3173](https://github.com/rust-lang/rfcs/pull/3173)
- Rust Issue: [rust-lang/rust#91399](https://github.com/rust-lang/rust/issues/91399)

# Summary
[summary]: #summary

This RFC adds two argumentless methods to `f32`/`f64`, `next_up` and
`next_down`. These functions are specified in the IEEE 754 standard, and provide
the capability to enumerate floating point values in order.


# Motivation
[motivation]: #motivation

Currently it is not possible to answer the question 'which floating point value
comes after `x`' in Rust without intimate knowledge of the IEEE 754 standard.
Answering this question has multiple uses:

 - Simply exploratory or educational purposes. Being able to enumerate values is
   critical for understanding how floating point numbers work, and how they have
   varying precision at different sizes. E.g. one might wonder what sort of
   precision `f32` has at numbers around 10,000. With this feature one could
   simply print `10_000f32.next_up() - 10_000f32` to find out it is
   `0.0009765625`.

 - Testing. If you wish to ensure a property holds for all values in a certain
   range, you need to be able to enumerate them. One might also want to check if
   and how your function fails just outside its supported range.

 - Exclusive ranges. If you want to ensure a variable lies within an exclusive
   range, these functions can help. E.g. to ensure that `x` lies within [0, 1)
   one can write `x.clamp(0.0, 1.0.next_down())`.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Because floating point numbers have finite precision sometimes you might want to
know which floating point number is *right* below or above a number you already
have. For this you can use the methods `next_down` or `next_up` respectively.
Using them repeatedly allows you to iterate over all the values within a range.

The method `x.next_up()` defined on both `f32` and `f64` returns the smallest
number greater than `x`. Similarly, `x.next_down()` returns the greatest number
less than `x`.

If you wanted to test a function for all `f32` floating point values between 1
and 2, you could for example write:
```rust
let mut x = 1.0;
while x <= 2.0 {
    test(x);
    x = x.next_up();
}
```

On another occasion might be interested in how much `f32` and `f64` differ in
their precision for numbers around one million. This is easy to figure out:
```rust
dbg!(1_000_000f32.next_up() - 1_000_000.0);
dbg!(1_000_000f64.next_up() - 1_000_000.0);
```

The answer is:
```rust
1_000_000f32.next_up() - 1_000_000.0 = 0.0625
1_000_000f64.next_up() - 1_000_000.0 = 0.00000000011641532182693481
```

If you want to ensure that a value `s` lies within -1 to 1, excluding the
endpoints, this is easy to do:
```rust
s.clamp((-1.0).next_up(), 1.0.next_down())
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The functions `nextUp` and `nextDown` are defined precisely (and identically)
in the standards IEEE 754-2008 and IEEE 754-2019. This RFC proposes the methods
`f32::next_up`, `f32::next_down`, `f64::next_up`, and `f64::next_down` with the
behavior exactly as specified in those standards.

To be precise, let `tiny` be the smallest representable positive value and
`max` be the largest representable finite positive value of the floating point
type. Then if `x` is an arbitrary value `x.next_up()` is specified as:

 - `x` if `x.is_nan()`,
 - `-max` if `x` is negative infinity,
 - `-0.0` if `x` is `-tiny`,
 - `tiny` if `x` is `0.0` or `-0.0`,
 - positive infinity if `x` is `max` or positive infinity, and
 - the unambiguous and unique minimal finite value `y` such that `x < y` in
   all other cases.

`x.next_down()` is specified as `-(-x).next_up()`.

A reference implementation for `f32` follows, using exclusively integer
arithmetic. The implementation for `f64` is entirely analogous, with the
exception that the constants `0x7fff_ffff` and `0x8000_0001` are replaced by
respectively `0x7fff_ffff_ffff_ffff` and `0x8000_0000_0000_0001`. Using
exclusively integer arithmetic aids stabilization as a `const fn`, reduces
transfers between floating point and integer registers or execution units (which
incur penalties on some processors), and avoids issues with denormal values
potentially flushing to zero during floating point arithmetic operations
on some platforms.

```rust
/// Returns the least number greater than `self`.
///
/// Let `TINY` be the smallest representable positive `f32`. Then,
///  - if `self.is_nan()`, this returns `self`;
///  - if `self` is `NEG_INFINITY`, this returns `-MAX`;
///  - if `self` is `-TINY`, this returns -0.0;
///  - if `self` is -0.0 or +0.0, this returns `TINY`;
///  - if `self` is `MAX` or `INFINITY`, this returns `INFINITY`;
///  - otherwise the unique least value greater than `self` is returned.
///
/// The identity `x.next_up() == -(-x).next_down()` holds for all `x`. When `x`
/// is finite `x == x.next_up().next_down()` also holds.
pub const fn next_up(self) -> Self {
    const TINY_BITS: u32 = 0x1; // Smallest positive f32.
    const CLEAR_SIGN_MASK: u32 = 0x7fff_ffff;

    let bits = self.to_bits();
    if self.is_nan() || bits == Self::INFINITY.to_bits() {
        return self;
    }
    
    let abs = bits & CLEAR_SIGN_MASK;
    let next_bits = if abs == 0 {
        TINY_BITS
    } else if bits == abs {
        bits + 1
    } else {
        bits - 1
    };
    Self::from_bits(next_bits)
}

/// Returns the greatest number less than `self`.
///
/// Let `TINY` be the smallest representable positive `f32`. Then,
///  - if `self.is_nan()`, this returns `self`;
///  - if `self` is `INFINITY`, this returns `MAX`;
///  - if `self` is `TINY`, this returns 0.0;
///  - if `self` is -0.0 or +0.0, this returns `-TINY`;
///  - if `self` is `-MAX` or `NEG_INFINITY`, this returns `NEG_INFINITY`;
///  - otherwise the unique greatest value less than `self` is returned.
///
/// The identity `x.next_down() == -(-x).next_up()` holds for all `x`. When `x`
/// is finite `x == x.next_down().next_up()` also holds.
pub const fn next_down(self) -> Self {
    const NEG_TINY_BITS: u32 = 0x8000_0001; // Smallest (in magnitude) negative f32.
    const CLEAR_SIGN_MASK: u32 = 0x7fff_ffff;

    let bits = self.to_bits();
    if self.is_nan() || bits == Self::NEG_INFINITY.to_bits() {
        return self;
    }
    
    let abs = bits & CLEAR_SIGN_MASK;
    let next_bits = if abs == 0 {
        NEG_TINY_BITS
    } else if bits == abs {
        bits - 1
    } else {
        bits + 1
    };
    Self::from_bits(next_bits)
}
```

# Drawbacks
[drawbacks]: #drawbacks

Two more functions get added to `f32` and `f64`, which may be considered
already cluttered by some.

Additionally, there is a minor pitfall regarding signed zero. Repeatedly calling
`next_up` on a negative number will iterate over all values above it, with the
exception of +0.0, only -0.0 will be visited. Similarly starting at positive
number and iterating downwards will only visit +0.0, not -0.0.

However, if we were to define `(-0.0).next_up() == 0.0` we would lose compliance
with the IEEE 754 standard, and lose the property that `x.next_up() > x` for all
finite `x`. It would also lead to the pitfall that `(0.0).next_down()` would not
be the smallest negative number, but -0.0 instead.

Finally, there is a minor risk of confusion regarding precedence with unary
minus. A user might inadvertently write `-1.0.next_up()` instead of
`(-1.0).next_up()`, giving a value on the wrong side of -1. However, this
potential confusion holds for most methods on `f32`/`f64`, and can be avoided
by the cautious by writing `f32::next_up(-1.0)`.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

To implement the features described in the motivation the user essentially
*needs* the `next_up`/`next_down` methods, or the alternative mentioned just
below. If these are not available the user must either install a third party
library for what is essentially one elementary function, or implement it
themselves using `to_bits` and `from_bits`. This has several issues or pitfalls:

 1. The user might not even be aware that a third party library exists,
    searching the standard library in vain. If they find a third party library
    they might not be able to judge if it is of sufficient quality and with the
    exact semantics they expect.

 2. Even if the user is aware of IEEE 754 representation and chooses to
    implement it themselves, they might not get the edge cases correct. It is
    also a wasted duplicate effort.

 3. The user might misunderstand the meaning of `f32::EPSILON`, thinking that
    adding this to a number results in the next floating point number.
    Alternatively they might misunderstand `f32::MIN_POSITIVE` to be the
    smallest positive `f32`, or believe that `x + f32::MIN_POSITIVE` is a
    correct implementation of `x.next_up()`.
  
 4. The user might give up entirely and simply choose an arbitrary offset, e.g.
    instead of `x.clamp(0, 1.0.next_down())` they end up writing
    `x.clamp(0, 1.0 - 1e-9)`.

The main alternative to these two functions is `nextafter(x, y)` (sometimes
called `nexttoward`). This function was specified in IEEE 754-1985 to "return
the next representable neighbor of `x` in the direction toward `y`". If `x == y`
then `x` is supposed to be returned. Besides error signaling and NaNs, that is
the complete specification.

We did not choose this function for three reasons:

 - The IEEE specification is lacking, and deprecated. Unfortunately IEEE
   754-1985 does not specify how to handle signed zeros at all, and some
   implementations (such as the one in the ISO C standard) deviate from the IEEE
   754 standard by defining `nextafter(x, y)` as `y` when `x == y`.
   Specifications IEEE 754-2008 and IEEE 754-2019 do not mention `nextafter` at
   all.

 - From an informal study by searching for code using `nextafter` or
   `nexttoward` across a variety of languages we found that essentially every
   use case in the wild consisted of `nextafter(x, c)` where `c` is a constant
   effectively equal to negative or positive infinity. That is, the users would
   have been better suited by `x.next_up()` or `x.next_down()`.
 
   Worse still, we also saw a lot of scenarios where `c` was somewhat
   arbitrarily chosen to be bigger/smaller than `x`, which might cause bugs when
   `x` is carelessly changed without updating `c`.

 - The function `next_after` has been deprecated by the libs team in the past
   (see [Prior art](#prior-art)).

The advantage of a potential `x.next_toward(y)` method would be that only a
single method would need to be added to `f32`/`f64`, however we argue that this
simply shifts the burden from documentation bloat to code bloat. Other
advantages are that it might considered more readable by some, and that it is
more familiar to those used to `nextafter` in other languages.

Finally, if we were to take inspiration from Julia and Ruby these two functions
could be called `next_float` and `prev_float`, which are arguably more readable,
albeit slightly more ambiguous as to which direction 'next' is.


# Prior art
[prior-art]: #prior-art

First we must mention that Rust used to have the `next_after` function, which
got deprecated in https://github.com/rust-lang/rust/issues/27752. We quote
@alexcrichton:

> We were somewhat ambivalent if I remember correctly on whether to stabilize or
> deprecate these functions. The functionality is likely needed by someone, but
> the names are unfortunately sub-par wrt the rest of the module.
> [...]
> We realize that the FCP for this issue was pretty short, however, so please
> comment with any objections you might have! We're very willing to backport an
> un-deprecate for the few APIs we have this cycle.

One might consider this a formal un-deprecation request, albeit with a different
name and slightly different API.

Within the Rust ecosystem the crate `float_next_after` solely provides the
`x.next_after(y)` method, and has 30,000 all-time downloads at the moment of
writing. The crate `ieee754` provides the `next` and `prev` methods among a few
others and sits at 244,000 all-time downloads.

As for other languages supporting this feature, the list of prior art is
extensive:

 - C has `nextafter` and `nexttoward`, essentially identical:  
   https://en.cppreference.com/w/c/numeric/math/nextafter

 - C++ follows in C's footsteps:  
   https://en.cppreference.com/w/cpp/numeric/math/nextafter

 - Python has `nextafter`:  
   https://docs.python.org/3/library/math.html#math.nextafter

 - Java has `nextUp`, `nextDown` and `nextAfter`:  
   https://docs.oracle.com/javase/8/docs/api/java/lang/Math.html#nextUp-double-

 - Swift has `nextUp` and `nextDown`:  
   https://developer.apple.com/documentation/swift/double/1847593-nextup

 - Go has `Nextafter`:  
   https://pkg.go.dev/math#Nextafter

 - Julia has `nextfloat` and `prevfloat`:  
   https://docs.julialang.org/en/v1/base/numbers/#Base.nextfloat

 - Ruby has `next_float` and `prev_float`:  
   https://ruby-doc.org/core-3.0.2/Float.html#next_float-method


# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - Which is the better pair of names, `next_up` and `next_down` or `next_float`
   and `prev_float`?

# Future possibilities
[future-possibilities]: #future-possibilities

In the future Rust might consider having an iterator for `f32` / `f64` ranges
that uses `next_up` or `next_down` internally.

The method `ulp` might also be considered, being a more precise implementation
of what is approximated as `x.next_up() - x` in this document. Its
implementation would directly compute the correct [ULP](https://en.wikipedia.org/wiki/Unit_in_the_last_place) by inspecting the exponent
field of the IEEE 754 number.
