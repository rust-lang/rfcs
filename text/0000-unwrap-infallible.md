- Feature Name: `unwrap_infallible`
- Start Date: 2019-11-01
- RFC PR: [rust-lang/rfcs#2799](https://github.com/rust-lang/rfcs/pull/2799)
- Rust Issue: 

# Summary
[summary]: #summary

Add method `Result::unwrap_infallible` to provide a convenient alternative
to `unwrap` for converting `Result` values with an uninhabitable `Err` type,
while ensuring infallibility at compile time.

# Motivation
[motivation]: #motivation

`Result<T, Infallible>`, soon to be equivalent to `Result<T, !>`,
has been occurring quite often in recent code. The first instinct is to
use `unwrap` on it, knowing it can never panic, but herein lies a
maintainability hazard: if the error parameter type at such a use site is
later changed to an inhabitable one, the `unwrap` call quietly becomes liable
to panic.

Therefore, it would make sense to add an alternative conversion method
to the standard library that would only be applicable to `Result`
with an uninhabitable error type.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Add a new method to `Result`, available when the `Err` type parameter is `!`
or is claimed to be infallibly convertible to `!` by a provided `From` impl:

```rust
impl<T, E: Into<!>> Result<T, E> {
    pub fn unwrap_infallible(self) -> T {
        match self {
            Ok(x) => x,
            Err(e) => e.into(),
        }
    }
}
```

This method should be used in preference to `Result::unwrap` when the user
wishes to ensure at compile time that an `Err` value cannot occur and therefore
this conversion can't fail (provided compliance by the `Into<!>` impl of
the error type).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The method `unwrap_infallible` is most readily avalable on `Result<_, !>`.
The failure branch is eliminated by the compiler even in the debug profile.

Extending the error type to `E: Into<!>` allows crate authors who have defined
their own uninhabitable types to benefit from this API without redefining
their "never-type" as an alias of `!`. Such "never-types", most prominently
`core::convert::Infallible` prior to `never_type`
[stabilization][never_type], serve the purpose of `!` before it is stabilized,
but aliasing the type to `!` when it is stabilized later
[can break][infallible-compat] pre-existing stable code in some corner cases.

[never_type]: https://github.com/rust-lang/rust/pull/65355
[infallible-compat]: https://doc.rust-lang.org/std/convert/enum.Infallible.html#future-compatibility

To fully preserve backward compatibility, custom "never-types" can be made
convertible into `!`:

```rust
enum MyNeverToken {}

impl From<MyNeverToken> for ! {
    fn from(never: MyNeverToken) -> Self {
        match never {}
    }
}
```

The implementation of `unwrap_infallible`, as provided above, relies on
the `From` impl being divergent without panicking, which fits the general
contract of `From`/`Into`.

# Drawbacks
[drawbacks]: #drawbacks

Can't think of any.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Adding an inherent method to `Result` is a backward-compatible way to
provide convenience in supporting a safe coding practice.
The method `unwrap_infallible` fits in nicely with other methods in the
`unwrap*` family and is easily discoverable in the documentation.

## A blanket From impl

Another possible way to provide a convenient conversion is adding a blanket
`From` impl to `libcore`:

```rust
impl<T, E: Into<!>> From<Result<T, E>> for T {
    fn from(res: Result<T, E>) -> T {
        match res {
            Ok(x) => x,
            Err(e) => e.into(),
        }
    }
}
```

Or, more restricted (shown implemented with the `exhaustive_patterns` compiler
feature for coolness):

```rust
impl<T> From<Result<T, !>> for T {
    fn from(Ok(t): Result<T, !>) -> T { t }
}
```

Either way, the `From` impl may overlap with impls defined in
other crates for their own types.

## Going without

Without a convenient conversion, developers conscious of the refactoring hazard
have to write verbose statements like
`res.unwrap_or_else(|never| match never {})`, or invent their own utilities
for a shorthand. Others may never learn of the pitfall and use `unwrap` as
the quickest suitable way to write a conversion found in the documentation of
`Result`.

## Third-party crate

The crate [unwrap-infallible][ext-crate] provides the `unwrap_infallible`
method in an extension trait. A third-party crate, though, is not as
discoverable as having a method available on `Result` in the standard library.

[ext-crate]: https://crates.io/crates/unwrap-infallible

## Exhaustive single variant pattern match

It will be possible to irrefutably match enums by the single inhabitable variant
if the `exhaustive_patterns` feature
([rust-lang/rust#51085][exhaustive_patterns]) is stabilized:

```rust
let Ok(x) = u64::try_from(x);
```

This may be more convenient in some cases like the argument pattern match
example above, but in other cases it is less ergonomic than a method call.
So, this can be considered a complementary solution.

[exhaustive_patterns]: https://github.com/rust-lang/rust/issues/51085

## Question the infallible

It's been [proposed][question-infallible] to make `Result<_, !>` work with
the `?` operator in any function:

```rust
fn g(x: u32) {
    let x = u64::try_from(x)?;
    ...
}
```

[question-infallible]: https://internals.rust-lang.org/t/a-distinct-way-to-unwrap-result-t-e-where-e-into/11212/8

This would not completely eliminate the refactoring hazard that motivates
this proposal, but would in fact make it worse: the infallibility would be
conditional on both the return type of the containing function and
the error type of the expression under `?`, with possibility to change either
without compile-time breakage.

Additionally, the usual semantics of `?` meaning "may return an error early
here" would obscure the intent to make use of infallibility.

# Prior art
[prior-art]: #prior-art

The original author is certain that the equivalent concern exists and may be
resolved more elegantly in the Haskell type system, but lacks detailed
knowledge of the language and spare time to research. Fill this in if necessary.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

Authors of custom never-types could find creative ways to implement `Into<!>`
that will be invoked by this method.
