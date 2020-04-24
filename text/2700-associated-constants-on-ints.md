- Feature Name: `assoc_int_consts`
- Start Date: 2019-05-13
- RFC PR: [rust-lang/rfcs#2700](https://github.com/rust-lang/rfcs/pull/2700)
- Rust Issue: [rust-lang/rust#68490](https://github.com/rust-lang/rust/issues/68490)

# Summary
[summary]: #summary

Add the relevant associated constants to the numeric types in the standard library, and consider a
timeline for the deprecation of the corresponding (and originally intended to be temporary)
primitive numeric modules and associated functions.

# Motivation
[motivation]: #motivation

All programming languages with bounded integers provide numeric constants for their maximum and
minimum extents. In Rust, [these constants were
stabilized](https://github.com/rust-lang/rust/pull/23549) in the eleventh hour before Rust 1.0
(literally the day before the branching of 1.0-beta on April 1, 2015), with some
known-to-be-undesirable properties. In particular, associated consts were yet to be implemented
(these landed, amusingly, one month after 1.0-beta and two weeks before 1.0-stable), and so each of
the twelve numeric types were given their own top-level modules in the standard library, whose
contents are exclusively these constants (all related non-constants being defined in inherent impls
directly on each type). However, in the even-eleventh-er hour before 1.0-beta, it was realized that
this solution did not work for anyone seeking to reference these constants when working with types
such as `c_int`, which are defined as type aliases and can thus access inherent impls but not
modules that merely happen to be named the same as the original type; as a result, [an emergency
PR](https://github.com/rust-lang/rust/pull/23947) also added redundant `max_value` and `min_value`
inherent functions as a last-second workaround. The PR itself notes how distasteful this remedy is:

> It's unfortunate to freeze these as methods, but when we can provide inherent associated constants
> these methods can be deprecated. [aturon, Apr 1, 2015]

Meanwhile, the author of the associated consts patch
[despairs](https://github.com/rust-lang/rust/pull/23606#issuecomment-88541583) of just barely
missing the deadline:

> @nikomatsakis The original motivation for trying to get this in before the beta was to get rid of
> all the functions that deal with constants in Int/Float, and then to get rid of all the modules
> like std::i64 that just hold constants as well. We could have dodged most of the issues (ICEs and
> generic code design) by using inherent impls instead of associating the constants with traits. But
> since [#23549](https://github.com/rust-lang/rust/pull/23549) came in a bit earlier and stabilized
> a bunch more of those constants before the beta, whereas this hasn't landed yet, blegh.
> [quantheory, Apr 1, 2015]

Anticipating the situation, an [issue](https://github.com/rust-lang/rfcs/issues/1099) was filed in
the RFCs repo regarding moving the contents of these modules into associated consts:

> I think it's a minor enough breaking change to move the constants and deprecate the modules u8,
> u16, etc. Not so sure about removing these modules entirely, I'd appreciate that, but it'll break
> all the code use-ing them. [petrochenkov, Apr 29, 2015]

Finally, so obvious was this solution that [the original RFC for associated
items](https://github.com/nox/rust-rfcs/blob/master/text/0195-associated-items.md#expressiveness)
used the numeric constants as the only motivating example for the feature of associated consts:

> For example, today's Rust includes a variety of numeric traits, including Float, which must
> currently expose constants as static functions [...] Associated constants would allow the consts
> to live directly on the traits

Despite the obvious intent, 1.0 came and went and there were plenty of other things to occupy
everyone's attention. Now, two days shy of Rust's fourth anniversary, let's re-examine the
situation. We propose to deprecate all of the aforementioned functions and constants in favor of
associated constants defined on the appropriate types, and to additionally deprecate all constants
living directly in the `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`, `u64`,
`u128`, `usize`, `f32` and `f64` modules in `std`. Advantages of this:

1. Consistency with the rest of the language. As demonstrated by the above quotes, associated consts
have been the natural way to express these concepts in Rust since before associated consts were even
implemented; this approach satisfies the principle of least surprise.

2. Documentation. On the front page of the [standard library API
docs](https://doc.rust-lang.org/std/index.html), 12 of the 60 modules in the standard library (20%)
are the aforementioned numeric modules which exist only to namespace two constants each. This
number will increase as new numeric primitives are added to Rust, as already seen with
`i128` and `u128`. Although deprecated modules cannot be easily removed from std, they can be
removed from the documentation, making the stdlib API docs less cluttered and easier to navigate.

3. Beginner ease. For a beginner, finding two identical ways to achieve something immediately raises
the question of "why", to which the answer here is ultimately uninteresting (and mildly
embarrassing). Even then the question of "which one to use" remains unanswered; neither current
approach is more idiomatic than the other. As noted, deprecated items can be removed from the
documentation, thereby decreasing the likelihood of head-scratching and incredulous sidelong
glances from people new to Rust.

4. Removal of ambiguity between primitive types and their identically-named modules. Currently
if you import an integer module and access constants in the module and methods on the type,
one has no apparent indication as to what comes from where:
```rust
use std::u32;
assert_eq!(u32::MAX, u32::max_value());
```
The fact that this sort of shadowing of primitive types works in the first place is surprising
even to experienced Rust programmers; the fact that such a pattern is seemingly encouraged by
the standard library is even more of a surprise. By making this change we would be able to
remove all modules in the standard library whose names shadow integral types.

5. Removal of a frustrating papercut. Even experienced Rust programmers are prone to trip over
this and curse at having to be reminded of a bizarre and jarring artifact of Rust 1.0.
By removing these artifacts we can make the experience of using Rust more universally pleasant.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

1. Add the following associated constants to the relevant types in standard library, with their definitions taken from the corresponding legacy module-level constants:
    - i8::{MAX, MIN}
    - i16::{MAX, MIN}
    - i32::{MAX, MIN}
    - i64::{MAX, MIN}
    - i128::{MAX, MIN}
    - isize::{MAX, MIN}
    - u8::{MAX, MIN}
    - u16::{MAX, MIN}
    - u32::{MAX, MIN}
    - u64::{MAX, MIN}
    - u128::{MAX, MIN}
    - usize::{MAX, MIN}
    - f32::{DIGITS, EPSILON, INFINITY, MANTISSA_DIGITS, MAX, MAX_10_EXP, MAX_EXP, MIN, MIN_10_EXP, MIN_EXP, MIN_POSITIVE, NAN, NEG_INFINITY, RADIX}
    - f64::{DIGITS, EPSILON, INFINITY, MANTISSA_DIGITS, MAX, MAX_10_EXP, MAX_EXP, MIN, MIN_10_EXP, MIN_EXP, MIN_POSITIVE, NAN, NEG_INFINITY, RADIX}

2. Redefine the following module-level constants in terms of the associated constants added in step 1:
    - std::i8::{[MIN](https://doc.rust-lang.org/std/i8/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/i8/constant.MAX.html)}
    - std::i16::{[MIN](https://doc.rust-lang.org/std/i16/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/i16/constant.MAX.html)}
    - std::i32::{[MIN](https://doc.rust-lang.org/std/i32/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/i32/constant.MAX.html)}
    - std::i64::{[MIN](https://doc.rust-lang.org/std/i64/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/i64/constant.MAX.html)}
    - std::i128::{[MIN](https://doc.rust-lang.org/std/i128/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/i128/constant.MAX.html)}
    - std::isize::{[MIN](https://doc.rust-lang.org/std/isize/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/isize/constant.MAX.html)}
    - std::u8::{[MIN](https://doc.rust-lang.org/std/u8/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/u8/constant.MAX.html)}
    - std::u16::{[MIN](https://doc.rust-lang.org/std/u16/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/u16/constant.MAX.html)}
    - std::u32::{[MIN](https://doc.rust-lang.org/std/u32/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/u32/constant.MAX.html)}
    - std::u64::{[MIN](https://doc.rust-lang.org/std/u64/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/u64/constant.MAX.html)}
    - std::u128::{[MIN](https://doc.rust-lang.org/std/u128/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/u128/constant.MAX.html)}
    - std::usize::{[MIN](https://doc.rust-lang.org/std/usize/constant.MIN.html), [MAX](https://doc.rust-lang.org/std/usize/constant.MAX.html)}
    - std::f32::{[DIGITS](https://doc.rust-lang.org/std/f32/constant.DIGITS.html), [EPSILON](https://doc.rust-lang.org/std/f32/constant.EPSILON.html), [INFINITY](https://doc.rust-lang.org/std/f32/constant.INFINITY.html), [MANTISSA_DIGITS](https://doc.rust-lang.org/std/f32/constant.MANTISSA_DIGITS.html), [MAX](https://doc.rust-lang.org/std/f32/constant.MAX.html), [MAX_10_EXP](https://doc.rust-lang.org/std/f32/constant.MAX_10_EXP.html), [MAX_EXP](https://doc.rust-lang.org/std/f32/constant.MAX_EXP.html), [MIN](https://doc.rust-lang.org/std/f32/constant.MIN.html), [MIN_10_EXP](https://doc.rust-lang.org/std/f32/constant.MIN_10_EXP.html), [MIN_EXP](https://doc.rust-lang.org/std/f32/constant.MIN_EXP.html), [MIN_POSITIVE](https://doc.rust-lang.org/std/f32/constant.MIN_POSITIVE.html), [NAN](https://doc.rust-lang.org/std/f32/constant.NAN.html), [NEG_INFINITY](https://doc.rust-lang.org/std/f32/constant.NEG_INFINITY.html), [RADIX](https://doc.rust-lang.org/std/f32/constant.RADIX.html)}
    - std::f64::{[DIGITS](https://doc.rust-lang.org/std/f64/constant.DIGITS.html), [EPSILON](https://doc.rust-lang.org/std/f64/constant.EPSILON.html), [INFINITY](https://doc.rust-lang.org/std/f64/constant.INFINITY.html), [MANTISSA_DIGITS](https://doc.rust-lang.org/std/f64/constant.MANTISSA_DIGITS.html), [MAX](https://doc.rust-lang.org/std/f64/constant.MAX.html), [MAX_10_EXP](https://doc.rust-lang.org/std/f64/constant.MAX_10_EXP.html), [MAX_EXP](https://doc.rust-lang.org/std/f64/constant.MAX_EXP.html), [MIN](https://doc.rust-lang.org/std/f64/constant.MIN.html), [MIN_10_EXP](https://doc.rust-lang.org/std/f64/constant.MIN_10_EXP.html), [MIN_EXP](https://doc.rust-lang.org/std/f64/constant.MIN_EXP.html), [MIN_POSITIVE](https://doc.rust-lang.org/std/f64/constant.MIN_POSITIVE.html), [NAN](https://doc.rust-lang.org/std/f64/constant.NAN.html), [NEG_INFINITY](https://doc.rust-lang.org/std/f64/constant.NEG_INFINITY.html), [RADIX](https://doc.rust-lang.org/std/f64/constant.RADIX.html)}

3. At a future point to be determined (see "Unresolved questions" below), deprecate the items listed in step 2. Additionally, deprecate the following associated functions:
    - i8::{[min_value](https://doc.rust-lang.org/std/primitive.i8.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.i8.html#method.max_value)}
    - i16::{[min_value](https://doc.rust-lang.org/std/primitive.i16.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.i16.html#method.max_value)}
    - i32::{[min_value](https://doc.rust-lang.org/std/primitive.i32.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.i32.html#method.max_value)}
    - i64::{[min_value](https://doc.rust-lang.org/std/primitive.i64.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.i64.html#method.max_value)}
    - i128::{[min_value](https://doc.rust-lang.org/std/primitive.i128.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.i128.html#method.max_value)}
    - isize::{[min_value](https://doc.rust-lang.org/std/primitive.isize.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.isize.html#method.max_value)}
    - u8::{[min_value](https://doc.rust-lang.org/std/primitive.u8.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.u8.html#method.max_value)}
    - u16::{[min_value](https://doc.rust-lang.org/std/primitive.u16.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.u16.html#method.max_value)}
    - u32::{[min_value](https://doc.rust-lang.org/std/primitive.u32.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.u32.html#method.max_value)}
    - u64::{[min_value](https://doc.rust-lang.org/std/primitive.u64.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.u64.html#method.max_value)}
    - u128::{[min_value](https://doc.rust-lang.org/std/primitive.u128.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.u128.html#method.max_value)}
    - usize::{[min_value](https://doc.rust-lang.org/std/primitive.usize.html#method.min_value), [max_value](https://doc.rust-lang.org/std/primitive.usize.html#method.max_value)}

4. Following step 3, the following modules will be made hidden from the front page of the stdlib documentation, as they no longer contain any non-deprecated items: `std::{i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize}` (note that this does not apply to either of `std::{f32, f64}`; see the Alternatives section below)

# Drawbacks
[drawbacks]: #drawbacks

1. Deprecation warnings, although these can be easily addressed.
2. Because associated items cannot be directly imported, code of the form `use std::i32::MAX; foo(MAX, MAX);`
   will most likely be changed to `foo(i32::MAX, i32::MAX)`, which may be marginally more verbose.
   However, given how many `MAX` and `MIN` constants there are in the stdlib,
   it is easy to argue that such unprefixed constants in the wild would be confusing,
   and ought to be avoided in the first place. In any case, users desperate for such behavior
   will be trivially capable of replacing `use std::i32::MAX;` with `const MAX: i32 = i32::MAX;`.

# Unresolved questions

How long should we go before issuing a deprecation warning? At the extreme end of the scale we could wait until the next edition of Rust is released, and have the legacy items only issue deprecation warnings when opting in to the new edition; this would limit disruption only to people opting in to a new edition (and, being merely an trivially-addressed deprecation, would constitute far less of a disruption than any ordinary edition-related change; any impact of the deprecation would be mere noise in light of the broader edition-related impacts). However long it takes, it is the opinion of the author that deprecation should happen *eventually*, as we should not give the impression that it is the ideal state of things that there should exist three ways of finding the maximum value of an integer type; we expect experienced users to intuitively reach for the new way proposed in this RFC as the "natural" way these constants ought to be implemented, but for the sake of new users it would be a pedagogical wart to allow all three to exist without explicitly calling out the preferred one.

# Alternatives
[alternatives]: #alternatives

- Unlike the twelve integral modules, the two floating-point modules would not themselves be
entirely deprecated by the changes proposed here. This is because the `std::f32` and `std::f64`
modules each contain a `consts` submodule, in which reside constants of a more mathematical bent
(the sort of things other languages might put in a `std::math` module).
It is the author's opinion that special treatment for such "math-oriented constants" (as opposed to
the "machine-oriented constants" addressed by this RFC) is not particularly precedented; e.g. this
separation is not consistent with the existing set of associated functions implemented on `f32`
and `f64`, which consist of a mix of both functions concerned with mathematical operations
(e.g. `f32::atanh`) and functions concerned with machine representation (e.g.
`f32::is_sign_negative`). However, although earlier versions of this RFC proposed deprecating
`std::{f32, f64}::consts` (and thereby `std::{f32, f64}` as well), the current version does not do
so, as this was met with mild resistance (and, in any case, the greatest gains from this RFC will
be its impact on the integral modules).
Ultimately, there is no reason that such a change could not be left to a future RFC if desired.
However, one alternative design would be to turn all the constants in `{f32, f64}` into associated
consts as well, which would leave no more modules in the standard library that shadow primitive
types. A different alternative would be to restrict this RFC only to the integral modules, leaving
f32 and f64 for a future RFC, since the integral modules are the most important aspect of this
RFC and it would be a shame for them to get bogged down by the unrelated concerns of the
floating-point modules.

- Rather than immediately deprecating the existing items in the standard library, we could add
the new associated consts without any corresponding deprecations. The downside of this idea is
that we now have *three* ways of doing the exact same thing, and without deprecation warnings
(and their associated notes) there is little enough to guide users as to which is solution
is the idiomatic one. It is the author's opinion that there is no downside to deprecation
warnings in this case, especially since mitigation of the warning is trivial (as discussed in
the Drawbacks section above).
