- Feature Name: `trait_method_impl_restrictions`
- Start Date: 2024-07-20
- RFC PR: [rust-lang/rfcs#3678](https://github.com/rust-lang/rfcs/pull/3678)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Support restricting implementation of individual methods within traits, as an
extension of the restriction mechanism defined in
[RFC 3323](https://rust-lang.github.io/rfcs/3323-restrictions.html).

# Motivation
[motivation]: #motivation

When defining a trait, the trait can provide optional methods with default
implementations, which become available on every implementation of the trait.
However, the implementer of the trait can still provide their own
implementation of such a method. In some cases, the trait does not want to
allow implementations to vary, and instead wants to guarantee that all
implementations of the trait (or all third-party implementations outside the
module or crate) use an identical method implementation. For instance, this may
be an assumption required for correctness.

This RFC allows restricting the implementation of trait methods.

Alternatively, a trait may wish to allow implementations within the same crate
to override a method, but not allow external implementations to override that
method.

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
    impl(crate) fn method(&self) {
        println!("MyTrait::method: {self}");
    }
}
```

Note that if a method or associated function marked with an `impl` restriction
does not have a default body, the trait will not be possible to implement
outside the indicated visibility. For instance, if a trait has a method with
`impl(crate)` and no default body, the trait will not be possible to implement
outside the crate.

When implementing a trait, the compiler checks if any implemented methods or
associated functions have an `impl` restriction. If so, and the restriction
does not include the current module, the compiler will emit an error on the
implementation. If the method or associated function has a default
implementation in the trait, the compiler will suggest removing the
implementation from the `impl Trait for` block; otherwise, the compiler will
state that the trait cannot be implemented here because the method or
associated function cannot be implemented here.

In every other way, an `impl`-restricted method or associated function acts
identically to any other method or associated function, and can be invoked
accordingly:

```rust
fn takes_mytrait(m: &impl MyTrait) {
    m.method();
}
```

Note that in some cases, the compiler may be able to avoid placing an
`impl`-restricted method in the trait's vtable, if there is only a single
implementation that can ever be invoked.

# Drawbacks
[drawbacks]: #drawbacks

As with any language feature, this adds more surface area to the language.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rather than using the `impl(visibility)` syntax from
[RFC 3323](https://rust-lang.github.io/rfcs/3323-restrictions.html), we could
instead use the `final` keyword, which is a reserved and unused keyword in all
editions of Rust. This would be similar to the use of `final` in other
languages. However, we already have RFC 3323, and the use of the
`impl(visibility)` syntax is completely consistent with the semantics defined
there. The `impl` syntax is more flexible, supporting restrictions such as
`impl(self)` or `impl(crate)` that permit implementation in certain contexts.

We could potentially use the `final` keyword with an optional visibility,
effectively using `final` in place of `impl` in this proposal. However, that
would be inconsistent with RFC 3323, and seems potentially confusing depending
on whether users perceive `final` as having positive or negative polarity. (In
other words, does `final(crate)` mean "it's final within the crate" or "it's
final *except* within the crate"?)

`impl(self)` on a trait method may look slightly incongruous, because the
`self` in `impl(self)` refers to the scope of the module, while the method will
reference `self` referring to an instance of an object. However, `impl(self)`
syntax is already used within RFC 3323, and introducing a different syntax
seems likely to lead to *greater* confusion.

We could introduce a syntax for not permitting implementations of the method at
*all*, even in the same module; for instance, `impl()`. In practice, this seems
likely to be *more* common than implementing the method for trait impls in the
same module, and it would eliminate the potential incongruity of `impl(self)`.

It's possible to work around the lack of this functionality by placing the
additional methods in an extension trait with a blanket implementation.
However, this is a user-visible API difference: the user must import the
extension trait, and use methods from the extension trait rather than from the
base trait.

In addition, relaxing an `impl` restriction is always forwards-compatible,
without any additional complexity.

# Prior art
[prior-art]: #prior-art

This feature is similar to `final` methods in Java or C++.

# Future possibilities
[future-possibilities]: #future-possibilities

We could theoretically allow `impl` restrictions on associated types, as well.
This seems less useful, but if it's trivial to implement we might want to
support it.

We could support `impl(unsafe)`, to make a trait safe to implement if *not*
overriding a method, and only unsafe to implement if overriding a method.
