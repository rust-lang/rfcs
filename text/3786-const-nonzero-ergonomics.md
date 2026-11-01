- Feature Name: `const-nonzero-ergonomics`
- Start Date: 2025-03-07
- RFC PR: [rust-lang/rfcs#3786](https://github.com/rust-lang/rfcs/pull/3786)
- Rust Issue: none

# Summary
[summary]: #summary

The `std::num::NonZero<T>` type allows non-zero integer semantics to be clearly expressed. Yet this
type is only seamlessly usable if all APIs with non-zero semantics use this type, due to required
to/from conversion at any API boundary that differs in its use of `NonZero`.

The burden of these conversions is especially heavy in tests and examples. This RFC proposes new
coercions to facilitate implicit conversion to `NonZero` from integer literals, simplifying usage
in tests and examples, where succinctness and readability are paramount.

# Motivation
[motivation]: #motivation

Using `NonZero` to express non-zero semantics is valuable because it leads to clarity of APIs,
removes the need for some runtime zero-checks and creates potential for niche optimizations.

In typical logic code, `NonZero<T>` values are validated early in the process, such as when parsing
user input. We start with a value of integer type `T` which may or may not be zero, and we parse
this into a `NonZero<T>` - if this succeeds, we know it is not zero and can skip zero-checks in any
further calls, as long as the API surface uses `NonZero<T>`.

**However, there is one area where `NonZero` has particularly poor ergonomics: tests and examples!**
In test code, numeric constants are common. By switching a function parameter from `u32` to
`NonZero<u32>` we add needless complexity to the test code.

Without `NonZero`:

```rust
fn item_fits_exactly_in_packaging(height: u32) -> bool {
    assert_ne!(0, height, "cannot package a product with a height of zero");
    1000 % height == 0
}

#[test]
fn item_fits_exactly_in_packaging_if_divides_1000() {
    // The packaging has a height of 1000, so any integer that divides it evenly will fit.
    assert!(item_fits_exactly_in_packaging(1));
    assert!(!item_fits_exactly_in_packaging(3));
    assert!(item_fits_exactly_in_packaging(25));
    assert!(!item_fits_exactly_in_packaging(999));
    assert!(item_fits_exactly_in_packaging(1000));
}
```

With `NonZero`:

```rust
use std::num::NonZero;

fn item_fits_exactly_in_packaging(height: NonZero<u32>) -> bool {
    // No need to worry about division by zero because we accept NonZero input.
    // This means we can avoid checking the denominator in every call to this function.
    1000 % height.get() == 0
}

#[test]
fn item_fits_exactly_in_packaging_if_divides_1000() {
    // The packaging has a height of 1000, so any integer that divides it evenly will fit.
    assert!(item_fits_exactly_in_packaging(NonZero::new(1).unwrap()));
    assert!(!item_fits_exactly_in_packaging(NonZero::new(3).unwrap()));
    assert!(item_fits_exactly_in_packaging(NonZero::new(25).unwrap()));
    assert!(!item_fits_exactly_in_packaging(NonZero::new(999).unwrap()));
    assert!(item_fits_exactly_in_packaging(NonZero::new(1000).unwrap()));
}
```

Having to manually construct the `NonZero` wrapper in test code can become very noisy and is
especially problematic in example code and doctests, where crate authors want to put their best
foot forward, to show off how easy it is to use the crate. This is because the nature of `NonZero`
creates a difference between the two categories of usage:

* In real use, the `NonZero` is typically constructed when parsing the input, not when calling some
  API that expresses its parameters via `NonZero`.
* In test and example use, there often is no parsing stage, so hardcoded test input must be manually wrapped at
  call sites.

**This means that tests and examples are much more noisy than real-world usage for an API that uses
`NonZero`, giving a false impression of API complexity and discouraging API authors from using
`NonZero` despite its advantages.**

In test and example code this commonly occurs with integer literals. The values are
known and can be validated at compile time to be non-zero and within expected bounds.

This RFC proposes that we omit the ceremony with literals, allowing implicit coercion of non-zero
integer literals to `NonZero<T>`, thereby encouraging Rust crate authors to
make more extensive use of `NonZero`, which they may today choose to avoid due to the extra cruft
it adds to tests and examples.

With this RFC implemented, this would be valid code:

```rust
use std::num::NonZero;

fn item_fits_exactly_in_packaging(height: NonZero<u32>) -> bool {
    // No need to worry about division by zero because we accept NonZero input.
    // This means we can avoid checking the denominator in every call to this function.
    1000 % height.get() == 0
}

#[test]
fn item_fits_exactly_in_packaging_if_divides_1000() {
    // The packaging has a height of 1000, so any integer that divides it evenly will fit.
    assert!(item_fits_exactly_in_packaging(1));
    assert!(!item_fits_exactly_in_packaging(3));
    assert!(item_fits_exactly_in_packaging(25));
    assert!(!item_fits_exactly_in_packaging(999));
    assert!(item_fits_exactly_in_packaging(1000));
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `std::num::NonZero<T>` type can be used to express that an integer of type `T` has a non-zero
value. This helps clearly express the intent of the API and encourages "thinking with types",
whereby function calls are maximally validated at compile time, reaching a state where if the code
compiles, it has a high probability of being correct.

In typical usage, you convert from a `T` such as `u32` into a `NonZero<T>` when parsing the input
and thereafter keep the value in the `NonZero` wrapper:

```rust ignore
use std::{env, num::NonZero};

fn main() {
    let Some(product_height) = env::args().nth(2) else {
        eprintln!("provide an integer as the first argument ('product_height') to this sample app");
        return;
    };

    let product_height = product_height
        .parse::<u32>()
        .expect("first argument ('product_height') must be an integer");

    // We validate that integers are non-zero as soon as possible and keep them in the
    // NonZero<T> wrapper type after that, to avoid further unnecessary zero-checks.
    let Some(product_height) = NonZero::new(product_height) else {
        eprintln!("first argument ('product_height') must be non-zero");
        return;
    };

    // ..
}
```

You can pass this value as-is to functions that take `NonZero<u32>`:

```rust ignore
fn main() {
    // ..

    if !item_fits_exactly_in_packaging(product_height) {
        eprintln!("product does not fit in packaging");
        return;
    }

    println!("product fits in packaging");
}

fn item_fits_exactly_in_packaging(height: NonZero<u32>) -> bool {
    // No need to worry about division by zero because we accept NonZero input.
    // This means we can avoid checking the denominator in every call to this function.
    1000 % height.get() == 0
}
```

When writing test or example code and using hardcoded constants, you can omit the conversion into
`NonZero<T>` - it is done implicitly at compile time. This only works with integer literals
like `1234`.

```rust ignore
#[test]
fn item_fits_exactly_in_packaging_if_divides_1000() {
    // The packaging has a height of 1000, so any integer that divides it evenly will fit.
    assert!(item_fits_exactly_in_packaging(1));
    assert!(!item_fits_exactly_in_packaging(3));
    assert!(item_fits_exactly_in_packaging(25));
    assert!(!item_fits_exactly_in_packaging(999));
    assert!(item_fits_exactly_in_packaging(1000));
}
```

This is similar to how in the statement `let foo: u8 = 123;`
the literal `123` is inferred to be `u8`.

Being able to skip the `NonZero::new` helps avoid unnecessary complexity in tests, encouraging high
test coverage and making test code easier to read. It also helps focus examples and doctests on the
usage of the `NonZero` capable API that you are publishing, without unnecessary noise from parsing
of constant values that would typically happen elsewhere in real code.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Untyped integer literals (e.g., `123`, `0xFF` but not `123_u32`) are implicitly coerced
to `std::num::NonZero<T>` (`T` being `u8`, `u32`, etc.) if all of the following are true:

* The value of the literal is not 0.
* The value of the literal fits within `T`’s range (e.g., `300` fails for `NonZero<u8>`).
* The target type is explicitly `NonZero<T>` or inferred as such.
* The type `T` is unambiguously resolved from the target `NonZero<T>` type.

The coercion happens at compile time, with the emitted code being the equivalent of
`const { NonZero::new(literal).unwrap() }` for valid cases, with no runtime checks.

The coercion is allowed when the source is an integer literal (`123`) or an integer literal
negation expression (`-123`). The coercion does not apply to other expressions besides unary
negation, even if the expressions combine literals (e.g. `123 + 456` does not qualify).

```rust ignore
fn foo(count: NonZero<i8>) { }

foo(123); // OK
foo(-123); // OK
foo(0x11); // OK

foo(0); // Error - value cannot be zero.
foo(300); // Error - out of bounds of i8.
foo(123_i8); // Error - only untyped literals accepted.
foo(123_usize); // Error - literal has non-matching type.
foo(123 - 1); // Error - coercion not applied for expressions.

const MAGIC_VALUE: NonZero<i8> = 123; // OK - coercion logic is same as when calling a fn.

let i = 123;
foo(i); // Error - the coercion only applies to literals and `i` is not a literal.
```

# Drawbacks
[drawbacks]: #drawbacks

Any implicit behavior in the language risks becoming "magic" that is hard to understand and reason
about. For this reason such behaviors are very uncommon in Rust. The greatest drawback is that we
may open a box that also contains some demons in addition to this simple enhancement - what other
implicit behaviors might get proposed using this as an example? The fact that this RFC is scoped
to constants and not variables is the mitigating factor here. After all in `let foo: u8 = 123` the
literals becomes a `u8` and is not too different in nature from `let foo: NonZero<u8> = 123`, though
the mechanics are of course different.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives`

An alternative with less magic would be to require an explicit "nonzero" suffix for constant
literals to activate this coercion:

```rust
fn foo(value: NonZero<u32>) {}

foo(1234); // Error
foo(1234_nz); // OK
foo(1234_u32_nz); // OK
```

This would still eliminate most of the cruft from tests and examples while still requiring an
explicit action by the user to invoke the coercion.

Given that the potential for "confusion by too much magic" seems minimal as the concept of
"non-zero" is trivial to understand and the proposal does not involve any size-coercion,
the value of avoiding fully implicit coercion here seems low - there is not an obvious benefit
from only taking half a step toward an improvement.

If we consider a library approach rather than a language approach, one viable alternative is to
create a macro to easily construct non-zero values and validate that they really are non-zero.
For example:

```rust
use std::num::nonzero;

fn foo(value: NonZero<u32>) {}

foo(1234); // Error
foo(nonzero!(1234)); // OK

macro_rules! nonzero {
    ($x:literal) => {
        const { ::std::num::NonZero::new($x).expect("literal must have non-zero value") }
    };
}
```

This macro will attempt to construct the value in a const context and emit a meaningful error if
that fails. This may offer a satisfactory user experience if such a macro were included in `std`,
though still comes at a cost of an unusual level of cruft for merely passing integer constants to
code, and remains something the user of the code has to think and know about.

Given that passing integer constants to code is an everyday task for a programming language, that
the `NonZero` wrapper type exists already in `std` and that there are so far no known corner cases
that could not be easily verified correct at compile time, having it "just work" seems to offer the
best tradeoff of factors to encourage wider usage of non-zero semantics, yielding more correct and
potentially more efficient code in APIs where `NonZero` is applicable.

That said, all these alternatives are better than what we have today - manually having to construct
an instance of `NonZero<T>`.

[Previous discussions](https://internals.rust-lang.org/t/elevate-nonzero-types-to-true-primitive-integer-types/18040/20)
have suggested that other ergonomic challenges exist with `NonZero` (e.g. because it lacks many of
the methods that you would expect to find on primitive integer types it wraps) and that improving
its ergonomics across the board would likely be challenging, if not due to the mechanics then at
least due to the scope of functionality associated with numeric types. Nevertheless, this does not
appear to be a meaningful argument against making `NonZero` ergonomics better for the scenarios
where it is useful to Rust users today, even if those cases are more limited than raw numerics.
In practice, `NonZero` types do appear to be gaining new members, so
there does not appear to be a consensus against improving them (e.g. 1.84 stabilized
[`NonZero::isqrt`](https://doc.rust-lang.org/std/num/struct.NonZero.html#method.isqrt))

# Prior art
[prior-art]: #prior-art

Exploration of other languages suggests that while refinement types like `NonZero` are common, they
generally require explicit conversion as they are not specific to integers as a general language
feature. In contrast, this RFC deals with integer refinement in particular, as the `NonZero` types
are focused specifically on this mode of refinement.

Similar functionality appears to be present in Ada if we use its subtyping feature to
define a `NonZero_Int` type:

```ada
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Integer_Text_IO; use Ada.Integer_Text_IO;

procedure Item_Fits_Test is
   subtype NonZero_Int is Integer range 1 .. Integer'Last;

   function Item_Fits_Exactly_In_Packaging(Height : NonZero_Int) return Boolean is
   begin
      -- No need to check for zero; the subtype guarantees it's non-zero
      return 1000 mod Height = 0;
   end Item_Fits_Exactly_In_Packaging;

   procedure Test_Item_Fits_Exactly_In_Packaging_If_Divides_1000 is
      Result : Boolean;
   begin
      Result := Item_Fits_Exactly_In_Packaging(1);
      pragma Assert(Result, "1 should divide 1000 evenly");
      Put_Line("Test 1 passed");

      Result := Item_Fits_Exactly_In_Packaging(3);
      pragma Assert(not Result, "3 should not divide 1000 evenly");
      Put_Line("Test 3 passed");

      Result := Item_Fits_Exactly_In_Packaging(25);
      pragma Assert(Result, "25 should divide 1000 evenly");
      Put_Line("Test 25 passed");

      Result := Item_Fits_Exactly_In_Packaging(999);
      pragma Assert(not Result, "999 should not divide 1000 evenly");
      Put_Line("Test 999 passed");

      Result := Item_Fits_Exactly_In_Packaging(1000);
      pragma Assert(Result, "1000 should divide 1000 evenly");
      Put_Line("Test 1000 passed");

      -- This would cause a compile-time error if uncommented:
      -- Result := Item_Fits_Exactly_In_Packaging(0);
      -- Error: constraint error, 0 not in range 1 .. Integer'Last
   end Test_Item_Fits_Exactly_In_Packaging_If_Divides_1000;

begin
   Test_Item_Fits_Exactly_In_Packaging_If_Divides_1000;
   Put_Line("All tests completed successfully!");
end Item_Fits_Test;
```

Pattern types [rust#123646](https://github.com/rust-lang/rust/issues/123646) are a generalized form
of the concept underpinning `NonZero` and may offer a generalized solution to the problems described
in this RFC, if they are adopted into Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The alternatives presented above appear inferior but only slightly so - we should carefully consider
which strategy to apply here, especially if there appear corner cases not yet explored, where the
different approaches may show off their respective strengths.

# Future possibilities
[future-possibilities]: #future-possibilities

If we accept that `NonZero<T>` deserves to be implicitly coerced from non-zero values of `T`, the
experience from implementing and stabilizing this may offer valuable insights for how deeply and
how explicitly/implicitly to integrate other bounded/restricted types such as
`Bounded<T, Min, Max>` with custom minimum and maximum values or other types of refinement.
This is out of scope of this RFC. Related discussions:
* [Is there a type like `std::num::NonZero*` but for other values than zero?](https://www.reddit.com/r/rust/comments/x0lwxt/is_there_a_type_like_stdnumnonzero_but_for_other/)
* [Range types for integers (or refinement types?)](https://github.com/rust-lang/rfcs/issues/671)
* [Pattern types MVP](https://github.com/rust-lang/rust/pull/107606)