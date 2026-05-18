- Feature Name: `doc_interp`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Add the ability to use `${...}` expressions in documentation comments for macro interpolation.

For example, `${$name}` would be replaced with the result of `stringify!($name)` if used in documentation comments, and expressions like `${my_macro!($name)}` also work.

## Motivation
[motivation]: #motivation

Right now, generating documentation via macros is incredibly overbearing. If any part of a documentation comment requires macro input, particularly doctests, then you need to replace the documentation comment with a `#[doc = ...]` macro, which usually takes the form of a `#[doc = concat!(...)]` expression, since splitting the attribute into multiple parts would result in the multiple pieces being displayed across separate lines.

An obvious example is `core/src/num/int_macros.rs` and `core/src/num/uint_macros.rs` from the standard library, and here's just one example:

```rust
/// Checked integer subtraction. Computes `self - rhs`, returning `None` if
/// overflow occurred.
///
/// # Examples
///
/// ```
#[doc = concat!("assert_eq!((", stringify!($SelfT), "::MIN + 2).checked_sub(1), Some(", stringify!($SelfT), "::MIN + 1));")]
#[doc = concat!("assert_eq!((", stringify!($SelfT), "::MIN + 2).checked_sub(3), None);")]
/// ```
```

This notation is incredibly difficult to read, especially when code is being output. It would be substantially easier to read this as:

```rust
/// Checked integer subtraction. Computes `self - rhs`, returning `None` if
/// overflow occurred.
///
/// # Examples
///
/// ```
/// assert_eq!((${$SelfT}::MIN + 2).checked_sub(1), Some(${$SelfT}::MIN + 1));
/// assert_eq!((${$SelfT}::MIN + 2).checked_sub(3), None);
/// ```
```

Similarly, even simple macro-generated documentation could be improved:

```rust
#[doc = concat!("Creates a ", $thing, ".")]
```

becomes:

```rust
/// Creates a ${$thing}.
```

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Internally, Rust converts `/// documentation comments` into `#[doc = "documentation comments"]` attributes. Multiple `#[doc = ...]` attributes are combined into a single documentation comment, where each attribute's text is put on its own line.

Taking this in mind, you *could* use this sugaring to generate documentation in macros, but the result is quite difficult to read:

```rust
#[doc = concat!("Creates a ", stringify!($thing), " without checking for validity.")]
///
/// # Safety
///
#[doc = concat!("Since this does not check if you've passed in a valid ", stringify!($thing), ",")]
/// you must check for validity yourself.
```

Instead, you can use the special `${...}` notation to include arbitrary macro content in documentation:

```rust
/// Creates a ${$thing}.
///
/// # Safety
///
/// Since this does not check if you've passed in a valid ${$thing},
/// you must check for validity yourself.
```

Internally, the contents inside `${...}` are replaced as if they were passed to a call to the `stringify!(...)` macro, and can accept arbitrary macro expressions. However, note that since the entire comment is a string, you don't need to use `concat!(...)` to combine multiple pieces:

```rust
/// Creates multiple ${concat!(stringify!($thing), "s")}.
```

Instead, you can do this instead:

```rust
/// Creates multiple ${$thing}${s}.
```

Which is ultimately:

```rust
/// Creates multiple ${$thing}s.
```

Invalid `${...}` are ignored entirely, so, the following:

```rust
/// Here's some ${(invalid rust code}.
```

Will just be output as-is and emit a warning when running `cargo doc`, rather than failing to compile or build documentation.

If for whatever reason, you want to write literal `${...}` in your code, use `${{...}}` instead.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`${...}` expressions are rustdoc-only, so, they require no changes to the compiler, only rustdoc.

There are lots of methods to implement this, but since rustdoc has access to all the compiler internals *and* already has its own tools for macro expansion, this should be relatively easy. If any part of macro expansion fails for a given expression, it should just emit a lint and display the original syntax in the generated documentation.

Note that falling back to the original source is *required* for backwards-compatibility, since we want to ensure that even documentation for old crates is able to be generated. For this reason, the `${...}` cannot simply be replaced with equivalent `#[doc = ...]` attributes before documenting, since this could cause entire files to fail to document due to invalid syntax.

It's unclear to what extent this syntax would disrupt existing crates' documentation, although considering how weird the `${...}` syntax is, this is expected to be minor. A simple grep of the code that would normally be run by crater should suffice, although there could also be lints added to rustdoc to be extra certain in case this ends up being more work than necessary.

Depending on the desire/need, a `#[doc(no_macro_interpolation)]` attribute could be added to opt out of this behaviour, or a `#[doc(macro_interpolation)]` attribute could be added to explicitly opt in, with the default being changed in a future edition.

Additionally, there are a few extra lints that could be emitted in addition to an "invalid syntax" lint:

* `${var}` is almost certainly meant to be `${$var}` and should be autofixable. (We don't have `cargo doc --fix`, do we?) Otherwise, this is just the literal string `var`.
* (I can't think of any more, but they might exist.)

## Drawbacks
[drawbacks]: #drawbacks

Obviously, this is a nontrivial syntax change, and that comes with its own downsides. However, the main downside of this approach is that it has the potential to disrupt the documentation of crates created before this change was added. While aesthetic changes to documentation are not part of Rust's stability guarantees, potential disruptions to doc tests should not be taken lightly, and we should verify that the change doesn't break anything before stabilising.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The `${...}` syntax, although, clunky, exists for the same reason that shell scripts have a `${variable}` syntax in addition to `$variable`; there are cases where this can become ambiguous. For example, `$X_$Y` is interpreted as `${X_}${Y}`, whereas `${X}_${Y}` will properly put an underscore between these two variables.

In the future, a simple `$variable` syntax could be adopted for cases where `${$variable}` is more than necessary, although this is explicitly left out of this RFC to keep things simple. The proposal for a lint to automatically fix `${variable}` to `${$variable}` should also help. We may also want to change the compilation of `$variable` to *not* emit a lint if used outside of a macro, since this likely represents code in another language and not improperly written Rust code.

## Prior art
[prior-art]: #prior-art

As mentioned, shell scripts use `${variable}` syntax to ensure that `$variable` isn't ambiguous when word characters are immediately adjacent to variable expansion. Many other languages offer a similar syntax for interpolation in string literals as well.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is `${{...}}` sufficient for escaping the syntax?
- To what extent would this slow down documenting libstd if it were implemented everywhere? Would it be substantially slower since all the parsing/macro expansion code is already loaded?
- Are there cases where `#[doc = ...]` comments are used instead of the normal syntax for macro generation that wouldn't be solved by this feature?

## Future possibilities
[future-possibilities]: #future-possibilities

- Should `$variable` be allowed by itself? Is this likely to cause issues?
- Technically, since rustdoc has access to constant evaluation too, we could permit things that are especially weird like allowing `${CONSTANT}` to expand to the literal value of a constant in documentation.
