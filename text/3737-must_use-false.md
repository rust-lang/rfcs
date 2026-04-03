- Feature Name: `must_use_false`
- Start Date: 2024-12-07
- RFC PR: [rust-lang/rfcs#3737](https://github.com/rust-lang/rfcs/pull/3737)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow annotating functions with `#[must_use = false]` to suppress any warning generated due to its return type being marked as `#[must_use]`.

# Motivation
[motivation]: #motivation

The primary motivation is that there are some situations where a function is technically fallible, but it is almost always correct to not check its result.

example:
```rust
/// Try to open a file before it is required.
///
/// This result can be safely ignored,
/// as the file will be automatically opened again
/// when it is actually needed.
#[must_use = false]
pub fn hint_premptive_open(&mut self) -> io::Result<()> {
   /* ... */
}
```

Another example is closing files and similar operations, where the only possible error recovery is to just ignoring the error.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `#[must_use]` attribute can also take a boolean value instead of a string value.

When a function is annotated with `#[must_use = false]`, its result can be ignored without triggering the `unused_must_use` lint, even if its return type is marked as `must_use`.

`#[must_use = true]` is equivelent to `#[must_use]`.

When used on a type, `#[must_use = false]` is equivelent to not having any `#[must_use]` attribute.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `must_use` attribute is used to control when the `unused_must_use` lint is emitted.

It has three forms:
1. the plain `#[must_use]` form, equivalent to `#[must_use = true]`.
2. the boolean form, `#[must_use = BOOLEAN]`, where `BOOLEAN` is either `true` or `false`.
3. the string form, `#[must_use = "MESSAGE"]`, where `MESSAGE` is a help message to emmitted alongside the `unused_must_use` lint.

For the purposes of calculating whether to emit `unused_must_use` for an expression, every function and type item is in one of the following `must_use` states:
1. a positive state, corresponding to any `must_use` attribute besides `must_use = false`.
2. a neutral state, corresponding to no `must_use` attribute
3. a negative state, corresponding to `must_use = false`.

For each
[expression statement](https://doc.rust-lang.org/reference/statements.html#expression-statements),
the following process is used to calculate a positive (emit the lint) or negative (do not emit the lint) result:
1. if the expression of the expression statement is a function call, check the `must_use` state of that function, otherwise skip to step 3
2. if the function has a neutral state, continue to step 3, otherwise the result is positive/negative according the the state
3. if the type of the expression has a positive `must_use` state, the result is positive, otherwise, the result is negative.


# Drawbacks
[drawbacks]: #drawbacks

Yet another "opt-in/opt-out" dance, similar to `#[non_exhaustive]`.

The usecase is fairly niche.

Implementation complexity.

Passing the result of a `#[must_use = false]` function through an identity function will cause the lint to trigger.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An `=` style attribute is used since this is already used for `must_use`, and it intuativly evokes the idea of "overwriting" a variable.

`#[must_use = true]` is provided for symmetry.

# Prior art
[prior-art]: #prior-art

* [Pre-RFC on IRLO](https://internals.rust-lang.org/t/pre-rfc-must-use-false/21861)
* Scala's `@CanIgnoreResult` annotation

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is this the best way we can handle this, or is there a more elegant/general solution that doesn't involve marking every single function?

# Future possibilities
[future-possibilities]: #future-possibilities

* A future edition making `#[must_use]` the default for functions that return something other than `()`.
* Allow annotating a module/crate with `#[must_use]` to make all its functions and methods default to `must_use`.

