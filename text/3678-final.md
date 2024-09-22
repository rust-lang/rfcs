- Feature Name: `final`
- Start Date: 2024-07-20
- RFC PR: [rust-lang/rfcs#3678](https://github.com/rust-lang/rfcs/pull/3678)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Support restricting implementation of individual methods within traits, using
the already reserved `final` keyword.

# Motivation
[motivation]: #motivation

When defining a trait, the trait can provide optional methods with default
implementations, which become available on every implementation of the trait.
However, the implementer of the trait can still provide their own
implementation of such a method. In some cases, the trait does not want to
allow implementations to vary, and instead wants to guarantee that all
implementations of the trait use an identical method implementation. For
instance, this may be an assumption required for correctness.

This RFC allows restricting the implementation of trait methods.

This mechanism also faciliates marker-like traits providing no implementable
methods, such that implementers only choose whether to provide the trait and
never how to implement it; the trait then provides all the method
implementations.

One example of a trait in the standard library benefiting from this:
`Error::type_id`, which has thus far remained unstable because it's unsafe to
override. This RFC would allow stabilizing that method so users can call it,
without permitting reimplementation of it.

# Explanation
[explanation]: #explanation

When defining a trait, the definition can annotate methods or associated
functions to restrict whether implementations of the trait can define them. For
instance:

```rust
trait MyTrait: Display {
    final fn method(&self) {
        println!("MyTrait::method: {self}");
    }
}
```

A method or associated function marked as `final` must have a default body.

When implementing a trait, the compiler will emit an error if the
implementation attempts to define any method or associated function marked as
`final`, and will emit a suggestion to delete the implementation.

In every other way, a `final` method or associated function acts identically to
any other method or associated function, and can be invoked accordingly:

```rust
fn takes_mytrait(m: &impl MyTrait) {
    m.method();
}
```

Note that in some cases, the compiler might choose to avoid placing a `final`
method in the trait's vtable, if the one-and-only implementation does not
benefit from monomorphization.

Note that removing a `final` restriction is always forwards-compatible.

The keyword `final` has been reserved since Rust 1.0, so this feature can ship
identically in all editions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

At runtime, a `final fn` behaves exactly the same as a `fn`.

Removing `final` is always a non-breaking change. (If `final` was preventing
implementation to prevent a soundness issue, though, this would require
additional care.)

Adding `final` is a breaking change, unless the trait already did not allow
third-party implementations (such as via a sealed trait).

At compile-time, a method declared as `final fn` in a trait must have a
provided body, and cannot be overridden in any `impl`, even an `impl` in the
same crate or module.

`final fn` cannot be combined with `default fn`.

`final` is only allowed in trait definitions. `final` is not allowed on
inherent impls, non-trait functions, or `extern` blocks.

`final` comes after visibility but before any qualifiers such as `async`.

`final` has no impact on the `dyn`-compatibility of a trait.

# Drawbacks
[drawbacks]: #drawbacks

As with any language feature, this adds more surface area to the language.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rather than using `final`, we could use the `impl(visibility)` syntax from [RFC
3323](https://rust-lang.github.io/rfcs/3323-restrictions.html). This would
allow more flexibility (such as overriding a method within the crate but not
outside the crate), and would be consistent with other uses of RFC 3323. On the
other hand, such flexibility would come at the cost of additional complexity,
and would be less familiar to people who have seen `final` in other languages.
We can always add such syntax for the more general cases in the future if
needed; see the future possibilities section.

We could use `#[final]` rather than `final`. We already have the `final`
keyword reserved, so there's no compatibility concern either way. This is
entirely a matter of taste and syntax bikeshedding.

It's possible to work around the lack of this functionality by placing the
additional methods in an extension trait with a blanket implementation.
However, this is a user-visible API difference: the user must import the
extension trait, and use methods from the extension trait rather than from the
base trait.

# Prior art
[prior-art]: #prior-art

This feature is similar to `final` methods in Java or C++.

# Future possibilities
[future-possibilities]: #future-possibilities

We could add additional flexibility using the restriction mechanism defined in
[RFC 3323](https://rust-lang.github.io/rfcs/3323-restrictions.html), using
syntax like `impl(crate)` to restrict implementation of a method or associated
function outside a crate while allowing implementations within the crate.
(Likewise with `impl(self)` or any other visibility.)

We could theoretically allow `final` restrictions on associated consts and
types, as well.

We could support `impl(unsafe)`, to make a trait safe to implement if *not*
overriding the method, and only unsafe to implement if overriding the method.

We could integrate this with stability markers, to stabilize calling a method
but keep it unstable to *implement*.
