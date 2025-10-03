- Feature Name: `spaceship_operator`
- Start Date: 2025-02-21
- RFC PR: [rust-lang/rfcs#3776](https://github.com/rust-lang/rfcs/pull/3776/)
- Rust Issue: TBA

# Summary
[summary]: #summary

Add the spaceship operator `<=>` to Rust, equivalent to calling `PartialOrd::partial_cmp` with the same operands.

# Motivation
[motivation]: #motivation

Suppose a case where a three-way comparison was explicitly needed to be tested in all possible outcomes (for example, in a binary searcher).

A usual approach would be to use `if`-statements for all of these paths:

```rust
let lhs = todo!();
let rhs = todo!();

if lhs < rhs {
	todo!();
} else if lhs == rhs {
	todo!();
} else if lhs > rhs {
	todo!();
} else {
	todo!();
}
```

This could also be simplified to:

```rust
use core::cmp::Ordering;

let lhs = todo!();
let rhs = todo!();

match lhs.partial_cmp(&rhs) {
	Some(Ordering::Less)    => todo!(),
	Some(Ordering::Equal)   => todo!(),
	Some(Ordering::Greater) => todo!(),
	None                    => todo!(),
}
```

This RFC simply proposes that the following would be possible as well:

```rust
use core::cmp::Ordering;

let lhs = todo!();
let rhs = todo!();

match lhs <=> rhs {
	Some(Ordering::Less)    => todo!(),
	Some(Ordering::Equal)   => todo!(),
	Some(Ordering::Greater) => todo!(),
	None                    => todo!(),
}
```

# Guide- and reference-level explanation
[guide-and-reference-level-explanation]: #guide-and-reference-level-explanation

Usage of this operator will map identically to calling `PartialOrd::partial_cmp` (or `Ord::cmp` if this is desired instead):

```rust
use core::cmp::Ordering;

// `T` and `U` are unspecified here, but both im-
// plement `PartialOrd` for demonstration's sake.

let lhs: T = todo!();
let rhs: U = todo!();

// `cmp0` and `cmp1` are defined completely equiva-
// lently:

let cmp0: Option<Ordering> = <T as PartialOrd<U>>::partial_cmp(&lhs, &rhs);
let cmp1: Option<Ordering> = lhs <=> rhs;
```

The operator would be implemented similarly to the existing ones (see [Appendix B](https://doc.rust-lang.org/nightly/book/appendix-02-operators.html) of the book).
In cases where `PartialOrd` is not implement between the compared objects, a compilation error would be yielded.

# Drawbacks
[drawbacks]: #drawbacks

The greatest drawback, as far as I can tell, would be that this operator may seem confusing to programmers that are unfamiliar with this construct in other languages (see [prior art](prior-art) for a list hereof).
But this does not actually seem to be an issue, considering the other features that might also be confusing for beginners.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The main (and perhaps only) use for this operator would be to reduce boilerplate in cases where three-way comparisons are needed.

There is the possibility that this operation is not considered fundamental enough for an operator to be considered worthwhile.
Currently, Rust has the `PartialOrd::partial_cmp` and `Ord::cmp` methods that achieve the same *operation* as proposed here.
Not merging this RFC would (obviously) not change status quo, and thus one would simply resort to using either of these functions instead.

I personally believe that the same reasonings for the preexisting operators also apply (to some extent) in this case.
I will, however, concede that the demand for this operator will not be as great as that for e.g. `==`, but in the end this will also be a matter of taste.

# Prior art
[prior-art]: #prior-art

The *spaceship* operator itself currently exists in other languages, albeit with slightly different syntaxes.

## C++

In C++, this feature has existed since ISO/IEC 14882:2020 (C++20), stable since 15 December 2020 (see [`cppreference.com`](https://en.cppreference.com/w/cpp/language/operator_comparison)).
C++, however, does not strictly define the precise semantics of this operator, merely requiring that the returned type must itself be comparable against `0`.

For integer types, such as `int` or `long long int`, the returned type is `::std::strong_ordering`, which is equivalent to Rust's `core::cmp::Ordering`.
Floating-point types, like `long double` or `::std::bfloat_16`, on the other hand, return `::std::weak_ordering`, which is equivalent to Rust's `Option<core::cmp::Ordering>`.

## PHP

PHP has had the spaceship operator since PHP 7, stable since 1 December 2015 (see the [PHP Manual](https://www.php.net/manual/en/language.operators.comparison.php)).
Here, it is defined as returning `-1` as an equivalent to our `Some(Ordering::Less)`, `0` as an equivalent to our `Some(Ordering::Equal)`, and `1` as an equivalent to our `Some(Ordering::Greater)`.

The operator is only defined for integral types.

## Perl

Perl defines the spaceship operator the same way as PHP (see the [Perldoc Browser](https://perldoc.perl.org/perlop#Equality-Operators)), with the added support for floating-point types (where `NaN` yields `undef` as an equivalent to our `None`).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should this operator map to `PartialOrd::partial_cmp` or `Ord::cmp`? <sub>(Maybe obvious, but still worth discussing).</sub>
