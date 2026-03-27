- Feature Name: `final`
- Start Date: 2024-07-20
- RFC PR: [rust-lang/rfcs#3678](https://github.com/rust-lang/rfcs/pull/3678)
- Rust Issue: [rust-lang/rust#131179](https://github.com/rust-lang/rust/issues/131179)

## Summary
[summary]: #summary

Support restricting implementation of individual methods within traits, using
the existing unused `final` keyword.

## Motivation
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

Another would be the `Read::read_buf_exact` method. Making this `final` would
allow callers to rely on its implementation to be correct, while keeping the
function safe to call. Without this, callers using `unsafe` code must defend
against the possibility of an incorrect `read_buf_exact` implementation (e.g.
returning `Ok(())` without filling the buffer) to avoid UB.

## Explanation
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

In every other way, an `final` method or associated function acts identically
to any other method or associated function, and can be invoked accordingly:

```rust
fn takes_mytrait(m: &impl MyTrait) {
    m.method();
}
```

Note that in some cases, the compiler might choose to avoid placing a `final`
method in the trait's vtable, if the one-and-only implementation does not
benefit from monomorphization.

Note that removing a `final` restriction is a compatible change. (Removing a
default implementation remains a breaking change.)

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

At runtime, a `final fn` behaves exactly the same as a `fn`.

Removing `final` may be a non-breaking change. (If `final` was preventing
implementation to prevent a soundness issue, though, this would require
additional care.)

Adding `final` is a breaking change, unless the trait already did not allow
third-party implementations (such as via a sealed trait).

At compile-time, a method declared as `final fn` in a trait must have a
provided body, and cannot be overridden in any `impl`, even an `impl` in the
same crate or module.

`final fn` cannot be combined with `default fn`.

`final` is only allowed in trait definitions. `final` is not allowed on impls
or their items, non-trait functions, or `extern` blocks.

A `final fn` never prevents a trait from having `dyn`-compatibility; the trait
can remain `dyn`-compatible as long as all non-`final` methods support
`dyn`-compatibility. This also means that a `final fn` can always be called on
a `dyn Trait`, even if the same method as a non-`final` `fn` would not have
been `dyn`-compatible.

## Drawbacks
[drawbacks]: #drawbacks

As with any language feature, this adds more surface area to the language.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Instead of or in addition to this, we could allow inherent `impl` blocks for a
`Trait` (e.g. `impl Trait { ... }` without `for Type`). People today already
occasionally write `impl dyn Trait` blocks, since `dyn Trait` is a type and
supports inherent impl blocks; this change would allow generalizing such blocks
by deleting the `dyn`. This has the potential for conceptual complexity or
confusion for new users, as well as potentially affecting the quality of
diagnostics. (It also used to have a meaning in Rust 2015: the same meaning
`impl dyn Trait` now has.) However, it would provide orthogonality, and an
interesting conceptual model.

Rather than using `final`, we could use the `impl(visibility)` syntax from
[RFC 3323](https://rust-lang.github.io/rfcs/3323-restrictions.html). This would
allow more flexibility (such as overriding a method within the crate but not
outside the crate), and would be consistent with other uses of RFC 3323. On the
other hand, such flexibility would come at the cost of additional complexity.
We can always add such syntax for the more general cases in the future if
needed; see the future possibilities section.

Rather than using `final`, we could use `#[final]`. This
concept is somewhat similar to "final" methods in other languages, and we
already have the `final` keyword reserved so we could use either an attribute
or a keyword.

It's possible to work around the lack of this functionality by placing the
additional methods in an extension trait with a blanket implementation.
However, this is a user-visible API difference: the user must import the
extension trait, and use methods from the extension trait rather than from the
base trait.

## Prior art
[prior-art]: #prior-art

This feature is similar to `final` methods in Java or C++.

It's also similar to `sealed` in C#, where `sealed class` is something from
which you can't derive and a base class can use `sealed` on a method to say
derived classes can't `override` it.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None yet.

## Future possibilities
[future-possibilities]: #future-possibilities

`final` methods do not need to appear in a trait's vtable. However, *if* a
method is `dyn`-compatible, and if it would benefit from monomorphization, we
could optionally put it in the trait's vtable, perhaps with an explicit option
to do so.

We could allow `final fn` methods on `#[marker]` traits, which are currently
not allowed to have any methods (because they can't allow different
implementations in different `impl`s).

As mentioned in the alternatives section, we could allow inherent `impl` blocks
for a `Trait` (e.g. `impl Trait { ... }` without `for Type`). People today
already occasionally write `impl dyn Trait` blocks, since `dyn Trait` is a type
and supports inherent impl blocks; this change would allow generalizing such
blocks by deleting the `dyn`.

When evaluating possible future syntaxes such as `impl Trait { ... }` blocks,
we should take into account:
- The conceptual model we want to present to users
- Whether we anticipate user confusion due to the former meaning of this syntax
  in Rust 2015 (prior to the move from `Trait` to `dyn Trait` to write trait
  objects)
- Any effect on diagnostic quality
- Whether an additional syntax adds excessive implementation complexity
- How much we want the benefit of allowing `impl dyn Trait` blocks to be
  generalized by deleting the `dyn`

We could add additional flexibility using the restriction mechanism defined in
[RFC 3323](https://rust-lang.github.io/rfcs/3323-restrictions.html), using
syntax like `impl(crate)` to restrict implementation of a method or associated
function outside a crate while allowing implementations within the crate.
(Likewise with `impl(self)` or any other visibility.)

We could theoretically allow `final` restrictions on associated consts and
types, as well. If this is simple to implement, we should implement it for all
items that can appear in a trait simultaneously; if it proves difficult to
implement, we should prioritize methods.

We could support some syntax (e.g. `impl(unsafe)`), to make a method safe to
call, but unsafe to override. This would allow the implementation to be
trusted, so that unsafe code can rely on it rather than defending against
incorrect implementations.

We could integrate this with stability markers, to stabilize calling a method
but keep it unstable to *implement*.
