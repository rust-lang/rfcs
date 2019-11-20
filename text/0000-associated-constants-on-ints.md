- Feature Name: assoc-int-consts
- Start Date: 2019-05-13
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add the relevant associated constants to the numeric types in the standard library, and consider a timeline for the deprecation of the corresponding (and originally intended to be temporary) primitive numeric modules and associated functions.

# Motivation
[motivation]: #motivation

All programming languages with bounded integers provide numeric constants for their maximum and minimum extents. In Rust, [these constants were stabilized](https://github.com/rust-lang/rust/pull/23549) in the eleventh hour before Rust 1.0 (literally the day before the branching of 1.0-beta on April 1, 2015), with some known-to-be-undesirable properties. In particular, associated consts were yet to be implemented (these landed, amusingly, one month after 1.0-beta and two weeks before 1.0-stable), and so each of the twelve numeric types were given their own top-level modules in the standard library, whose contents are exclusively these constants (all related non-constants being defined in inherent impls directly on each type). However, in the even-eleventh-er hour before 1.0-beta, it was realized that this solution did not work for anyone seeking to reference these constants when working with types such as `c_int`, which are defined as type aliases and can thus access inherent impls but not modules that merely happen to be named the same as the original type; as a result, [an emergency PR](https://github.com/rust-lang/rust/pull/23947) also added redundant `max_value` and `min_value` inherent functions as a last-second workaround. The PR itself notes how distasteful this remedy is:

> It's unfortunate to freeze these as methods, but when we can provide inherent associated constants these methods can be deprecated. [aturon, Apr 1, 2015]

Meanwhile, the author of the associated consts patch [despairs](https://github.com/rust-lang/rust/pull/23606#issuecomment-88541583) of just barely missing the deadline:

> @nikomatsakis The original motivation for trying to get this in before the beta was to get rid of all the functions that deal with constants in Int/Float, and then to get rid of all the modules like std::i64 that just hold constants as well. We could have dodged most of the issues (ICEs and generic code design) by using inherent impls instead of associating the constants with traits. But since [#23549](https://github.com/rust-lang/rust/pull/23549) came in a bit earlier and stabilized a bunch more of those constants before the beta, whereas this hasn't landed yet, blegh. [quantheory, Apr 1, 2015]

Anticipating the situation, an [issue](https://github.com/rust-lang/rfcs/issues/1099) was filed in the RFCs repo regarding moving the contents of these modules into associated consts:

> I think it's a minor enough breaking change to move the constants and deprecate the modules u8, u16, etc. Not so sure about removing these modules entirely, I'd appreciate that, but it'll break all the code use-ing them. [petrochenkov, Apr 29, 2015]

Finally, so obvious was this solution that [the original RFC for associated items](https://github.com/nox/rust-rfcs/blob/master/text/0195-associated-items.md#expressiveness) used the numeric constants as the only motivating example for the feature of associated consts:

> For example, today's Rust includes a variety of numeric traits, including Float, which must currently expose constants as static functions [...] Associated constants would allow the consts to live directly on the traits

Despite the obvious intent, 1.0 came and went and there were plenty of other things to occupy everyone's attention. Now, two days shy of Rust's fourth anniversary, let's re-examine the situation. We propose to deprecate all of the aforementioned functions and constants in favor of associated constants defined on the appropriate types, and to additionally deprecate all constants living directly in the `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `f32` and `f64` modules in `std`. But leaving `std::f64::consts` and `std::f32::consts` as they are. Advantages of this:

1. Consistency with the rest of the language. As demonstrated by the above quotes, associated consts have been the natural way to express these concepts in Rust since before associated consts were even implemented; this approach satisfies the principle of least surprise.

2. Documentation. On the front page of the [standard library API docs](https://doc.rust-lang.org/std/index.html), 14 of the 56 modules in the standard library (25%) are the aforementioned numeric modules whose only purpose is to namespace these constants. This number will continue to rise as new numeric primitives are added to Rust, as already seen with `i128` and `u128`. Although deprecated modules cannot be easily removed from std, they can be removed from the documentation, making the stdlib API docs less cluttered and easier to navigate.

3. Beginner ease. For a beginner, finding two identical ways to achieve something immediately raises the question of "why", to which the answer here is ultimately uninteresting (and even then, the question of "which one to use" remains unanswered; neither current approach is idiomatic). As noted, deprecated items can be removed from the documentation, thereby decreasing the likelihood of head-scratching and incredulous sidelong glances from people new to Rust.

4. Remove ambiguity between primitive type and module with same name. Currently if you import an
integer module and access constants in the module and methods on the type, it's very unclear what
comes from where:
    ```rust
    use std::u32;
    assert_eq!(u32::MAX, u32::max_value());
    ```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Deprecate the following items in the standard library:

[i8::min_value](https://doc.rust-lang.org/std/primitive.i8.html#method.min_value)

[i8::max_value](https://doc.rust-lang.org/std/primitive.i8.html#method.max_value)

[std::i8::MIN](https://doc.rust-lang.org/std/i8/constant.MIN.html)

[std::i8::MAX](https://doc.rust-lang.org/std/i8/constant.MIN.html)

[i16::min_value](https://doc.rust-lang.org/std/primitive.i16.html#method.min_value)

[i16::max_value](https://doc.rust-lang.org/std/primitive.i16.html#method.max_value)

[std::i16::MIN](https://doc.rust-lang.org/std/i16/constant.MIN.html)

[std::i16::MAX](https://doc.rust-lang.org/std/i16/constant.MIN.html)

[i32::min_value](https://doc.rust-lang.org/std/primitive.i32.html#method.min_value)

[i32::max_value](https://doc.rust-lang.org/std/primitive.i32.html#method.max_value)

[std::i32::MIN](https://doc.rust-lang.org/std/i32/constant.MIN.html)

[std::i32::MAX](https://doc.rust-lang.org/std/i32/constant.MIN.html)

[i64::min_value](https://doc.rust-lang.org/std/primitive.i64.html#method.min_value)

[i64::max_value](https://doc.rust-lang.org/std/primitive.i64.html#method.max_value)

[std::i64::MIN](https://doc.rust-lang.org/std/i64/constant.MIN.html)

[std::i64::MAX](https://doc.rust-lang.org/std/i64/constant.MIN.html)

[i128::min_value](https://doc.rust-lang.org/std/primitive.i128.html#method.min_value)

[i128::max_value](https://doc.rust-lang.org/std/primitive.i128.html#method.max_value)

[std::i128::MIN](https://doc.rust-lang.org/std/i128/constant.MIN.html)

[std::i128::MAX](https://doc.rust-lang.org/std/i128/constant.MIN.html)

[isize::min_value](https://doc.rust-lang.org/std/primitive.isize.html#method.min_value)

[isize::max_value](https://doc.rust-lang.org/std/primitive.isize.html#method.max_value)

[std::isize::MIN](https://doc.rust-lang.org/std/isize/constant.MIN.html)

[std::isize::MAX](https://doc.rust-lang.org/std/isize/constant.MIN.html)

[u8::min_value](https://doc.rust-lang.org/std/primitive.u8.html#method.min_value)

[u8::max_value](https://doc.rust-lang.org/std/primitive.u8.html#method.max_value)

[std::u8::MIN](https://doc.rust-lang.org/std/u8/constant.MIN.html)

[std::u8::MAX](https://doc.rust-lang.org/std/u8/constant.MIN.html)

[u16::min_value](https://doc.rust-lang.org/std/primitive.u16.html#method.min_value)

[u16::max_value](https://doc.rust-lang.org/std/primitive.u16.html#method.max_value)

[std::u16::MIN](https://doc.rust-lang.org/std/u16/constant.MIN.html)

[std::u16::MAX](https://doc.rust-lang.org/std/u16/constant.MIN.html)

[u32::min_value](https://doc.rust-lang.org/std/primitive.u32.html#method.min_value)

[u32::max_value](https://doc.rust-lang.org/std/primitive.u32.html#method.max_value)

[std::u32::MIN](https://doc.rust-lang.org/std/u32/constant.MIN.html)

[std::u32::MAX](https://doc.rust-lang.org/std/u32/constant.MIN.html)

[u64::min_value](https://doc.rust-lang.org/std/primitive.u64.html#method.min_value)

[u64::max_value](https://doc.rust-lang.org/std/primitive.u64.html#method.max_value)

[std::u64::MIN](https://doc.rust-lang.org/std/u64/constant.MIN.html)

[std::u64::MAX](https://doc.rust-lang.org/std/u64/constant.MIN.html)

[u128::min_value](https://doc.rust-lang.org/std/primitive.u128.html#method.min_value)

[u128::max_value](https://doc.rust-lang.org/std/primitive.u128.html#method.max_value)

[std::u128::MIN](https://doc.rust-lang.org/std/u128/constant.MIN.html)

[std::u128::MAX](https://doc.rust-lang.org/std/u128/constant.MIN.html)

[usize::min_value](https://doc.rust-lang.org/std/primitive.usize.html#method.min_value)

[usize::max_value](https://doc.rust-lang.org/std/primitive.usize.html#method.max_value)

[std::usize::MIN](https://doc.rust-lang.org/std/usize/constant.MIN.html)

[std::usize::MAX](https://doc.rust-lang.org/std/usize/constant.MIN.html)

[std::f32::DIGITS](https://doc.rust-lang.org/std/f32/constant.DIGITS.html)

[std::f32::EPSILON](https://doc.rust-lang.org/std/f32/constant.EPSILON.html)

[std::f32::INFINITY](https://doc.rust-lang.org/std/f32/constant.INFINITY.html)

[std::f32::MANTISSA_DIGITS](https://doc.rust-lang.org/std/f32/constant.MANTISSA_DIGITS.html)

[std::f32::MAX](https://doc.rust-lang.org/std/f32/constant.MAX.html)

[std::f32::MAX_10_EXP](https://doc.rust-lang.org/std/f32/constant.MAX_10_EXP.html)

[std::f32::MAX_EXP](https://doc.rust-lang.org/std/f32/constant.MAX_EXP.html)

[std::f32::MIN](https://doc.rust-lang.org/std/f32/constant.MIN.html)

[std::f32::MIN_10_EXP](https://doc.rust-lang.org/std/f32/constant.MIN_10_EXP.html)

[std::f32::MIN_EXP](https://doc.rust-lang.org/std/f32/constant.MIN_EXP.html)

[std::f32::MIN_POSITIVE](https://doc.rust-lang.org/std/f32/constant.MIN_POSITIVE.html)

[std::f32::NAN](https://doc.rust-lang.org/std/f32/constant.NAN.html)

[std::f32::NEG_INFINITY](https://doc.rust-lang.org/std/f32/constant.NEG_INFINITY.html)

[std::f32::RADIX](https://doc.rust-lang.org/std/f32/constant.RADIX.html)

[std::f64::DIGITS](https://doc.rust-lang.org/std/f64/constant.DIGITS.html)

[std::f64::EPSILON](https://doc.rust-lang.org/std/f64/constant.EPSILON.html)

[std::f64::INFINITY](https://doc.rust-lang.org/std/f64/constant.INFINITY.html)

[std::f64::MANTISSA_DIGITS](https://doc.rust-lang.org/std/f64/constant.MANTISSA_DIGITS.html)

[std::f64::MAX](https://doc.rust-lang.org/std/f64/constant.MAX.html)

[std::f64::MAX_10_EXP](https://doc.rust-lang.org/std/f64/constant.MAX_10_EXP.html)

[std::f64::MAX_EXP](https://doc.rust-lang.org/std/f64/constant.MAX_EXP.html)

[std::f64::MIN](https://doc.rust-lang.org/std/f64/constant.MIN.html)

[std::f64::MIN_10_EXP](https://doc.rust-lang.org/std/f64/constant.MIN_10_EXP.html)

[std::f64::MIN_EXP](https://doc.rust-lang.org/std/f64/constant.MIN_EXP.html)

[std::f64::MIN_POSITIVE](https://doc.rust-lang.org/std/f64/constant.MIN_POSITIVE.html)

[std::f64::NAN](https://doc.rust-lang.org/std/f64/constant.NAN.html)

[std::f64::NEG_INFINITY](https://doc.rust-lang.org/std/f64/constant.NEG_INFINITY.html)

[std::f64::RADIX](https://doc.rust-lang.org/std/f64/constant.RADIX.html)

Replace each item with an associated const value on the appropriate type. Deprecate the twelve
integer type modules and remove them from the documentation.

# Drawbacks
[drawbacks]: #drawbacks

Deprecation warnings are annoying.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There is an alternative design where the proposed changes are only made to the integral numeric modules in the standard library, leaving alone `f32` and `f64`. Unlike the integral modules, these modules do not contain both constants and redundant associated items. In addition, these two modules contain submodules named `consts`, which contain constants of a more mathematical bent (the sort of thing other languages might put in a `std::math` module). This RFC argues for giving the float modules the same treatment as the integral modules, both since associated consts are "obviously the right thing" in this case and because we do not consider the mathematical/machine constant distinction to be particularly useful or intuitive. In particular, this distinction is not consistent with the existing set of associated functions implemented on `f32` and `f64`, which consist of a mix of both functions concerned with mathematical operations (e.g. `f32::atanh`) and functions concerned with machine representation (e.g. `f32::is_sign_negative`). As noted, if a `math` module existed in Rust's stdlib this would be the natural place to put them, however, such a module does not exist; further, any consideration of this hyptothetical module would, for the sake of consistency, want to also adopt not only the aforementioned mathematical associated functions that currently exist on `f32` and `f64`, but would also want to adopt the integral mathematical functions such as `i32::pow`--all while in some way recreating the module-level distinction between the operations as they exist on the various different numeric types. This is all to say that such a `std::math` module is out of scope for this proposal, in addition to lacking the technical motivation of this proposal. Ultimately, however, leaving `f32` and `f64` along and making the proposed changes only to the integral types would still be considered a success by this RFC.
