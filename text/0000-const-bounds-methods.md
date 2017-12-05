- Feature Name: const_bounds_methods
- Start Date: 2017-12-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allows for:

1. `const fn` in `trait`s.

2. over-constraining trait `fn`s as `const fn` in `impl`s.

3. syntactic sugar `const impl` for `impl`s where all fn:s are const.

4. `const` bounds as in `T: const Trait` satisfied by `T`s with only
`const fn`s in their `impl Trait for T {..}`.

# Motivation
[motivation]: #motivation

This RFC adds a non-trivial amount of expressive power to the language.

Let us go throw the motivation for each bit in order.

## 1. `const fn` in traits

Currently, associated constants are the only part of the constant time
evaluation that is available for `trait`s.

But there are useful `const fn`s besides those which associated constants model
(those from `() -> T` - i.e: a value not depending on inputs) where the
`const fn`s depend on inputs, be they `const` or other.

It is also inconsistent not to have `const fn`s in `trait`s as `fn` and
`unsafe fn` are both allowed today.

## 2. over-constraining `trait` `fn`s as `const fn` in `impl`s

This allows the user to be more strict and less allowing than the `trait`
permits. The expressive power gained here is a) that the user may statically
check that the `fn` may not do certain things, b) that when all `fn`s are
marked as `const fn` in the `impl`, the user may use the `impl` as the target
of a `const` trait bound which is discussed below.

## 3. syntactic sugar `const impl`

Prefixing an `impl` with `const` as in `const impl` is sugar for prefixing all
`fn`s in the `impl`, be it a trait `impl` or an inherent `impl`. As this is
sugar, it adds no additional expressive power to the language, but makes the
use of 2. and existing `const fn` use in inherent `impl`s more ergonomic.

It also aids searchability by allowing the reader to know directly from the
header that a trait impl is usable as a const trait bound, as opposed to
checking every `fn` for a `const` modifier.

By doubling as sugar usable for inherent `impl`s, the introduced syntax carries
its own weight. It allows the user to separate `const fn`s and normal `fn`s in
the documentation of inherent `impl`s.

## 4. `const` trait bounds, `T: const Trait`

Such a bound `T: const Trait` denotes that `impl Trait for T` must be a
constant trait impl (with only `const fn`s) as suggested in 2-3. In a
`const fn foo<T: const Trait>(..)`, this fact may be used to call methods
from `Trait` in `foo`.

It may also be used for `const` and `static` bindings or as input for const 
generics inside a normal `fn foo<T: const Trait>(..)` declaration. And in such
a context, the user can be certain that no I/O may happen inside the called
`const fn`s. When the methods of `Trait` are called with input that is `const`,
the user may also be certain that the call is cheap at runtime.

The new form of bound also allows reuse of traits, an important step to avoid
a bifurcation of existing traits along the lines of `const fn` vs. `fn`, both
in the standard library and elsewhere. A canonical example that const trait
bounds would solve is not having both `Default` and `ConstDefault`. Doing that
is important because it considerably reduces the amount of duplication of
`impl`s for such traits.

A consequence of const trait bounds is at least that `F: const FnOnce` is now
possible, allowing the user to effectively expect a `const fn` closure.

# Vocabulary
[vocabulary]: #vocabulary

Let's introduce the new terms used in this RFC.

+ **const impl syntax**, refers to the specific syntax `const impl`.
+ **constant trait impl**, refers to `impl`s of `trait`s where all `fn`s are
marked as `const fn`.
+ **const trait bound**, refers to a trait bound of the form `T: const Trait`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We will now go through all the points discussed in the summary and explain
what they entail.

## 1. `const fn` in traits

Simply put, the following is now allowed:

```rust
trait Ada {
    const fn foo<type_params..>(inputs..) -> return_type;
}
```

In other words, `trait`s may now require of their `impl`s that a certain `fn`
be a `const fn`. Naturally, `foo` will now be type-checked as a `const fn`.

This is of course not specific to this trait or one method. Any trait,
including those with type parameters can now have `const fn`s in them,
and these `const fn`s may have any number of type parameters, parameters,
and any return type.

## 2. over-constraining `trait` `fn`s as `const fn` in `impl`s

Consider the `Default` trait:

```rust
pub trait Default {
    fn default() -> Self;
}
```

And and some type - for instance:

```rust
struct Foo(usize);
```

As a rustacean, you may now write:

```rust
impl Default for Foo {
    const fn default() -> Self {
        Foo(0)
    }
}
```

Note that this `impl` has constrained `default` more than required by the
`Default` trait. Why this is useful will be made clear in the [motivation]
and in the [guide-level-explanation] of `const` trait bounds.

Naturally, `default` for `Foo` will now be type-checked as a `const fn`.

## 3. syntactic sugar `const impl`



## 4. `const` trait bounds, `T: const Trait`






Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[alternatives]: #alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
