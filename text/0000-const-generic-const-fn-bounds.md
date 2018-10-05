- Feature Name: const_generic_const_fn_bounds
- Start Date: 2018-10-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Allow `const impl`s for trait impls where all method impls are checked as const fn.

Make it legal to declare trait bounds on generic parameters of `const fn` and allow
the body of the const fn to call methods on these generic parameters.

# Motivation
[motivation]: #motivation

Currently one can declare const fns with generic parameters, but one cannot add trait bounds to these
generic parameters. Thus one is not able to call methods on the generic parameters (or on objects of the
generic parameter type), because they are fully unconstrained.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can call call methods of generic parameters of `const fn`, because they are implicitly assumed to be
`const fn`. For example, even though the `Add` trait declaration does not contain any mention of `const`,
you can use it as a trait bound on your generic parameters:

```rust
const fn triple_add<T: Add>(a: T, b: T, c: T) -> T {
    a + b + c
}
```

The obligation is passed to the caller of your `triple_add` function to supply a type whose `Add` impl is fully
`const`. Since `Add` only has `add` as a method, in this case one only needs to ensure that the `add` method is
`const fn`. Instead of adding a `const` modifier to all methods of a trait impl, the modifier is added to the entire
`impl` block:

```rust
struct MyInt(i8);
const impl Add for MyInt {
    fn add(self, other: Self) -> Self {
        MyInt(self.0, other.0)
    }
}
```

The const requirement is propagated to all bounds of the impl or its methods,
so in the following `H` is required to have a const impl of `Hasher`, so that
methods on `state` are callable.

```rust
const impl Hash for MyInt {
    fn hash<H>(
        &self,
        state: &mut H,
    )
        where H: Hasher
    {
        state.write(&[self.0 as u8]);
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

One cannot add trait bounds to `const fn` without them automatically
requiring `const impl`s for all monomorphizations. Even if one does not
call any method on the generic parameter, the methods are still required to be constant.

It is not a fully general design that supports every possible use case,
but only covers the most common cases. See also the alternatives.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Effect system

A fully powered effect system can allow us to do fine grained constness propagation
(or no propagation where undesirable). This is way out of scope in the near future
and this RFC is forward compatible to have its background impl be an effect system.

## Fine grained `const` annotations

One could annotate methods instead of impls, allowing just marking some method impls
as const fn. This would require some sort of "const bounds" in generic functions that
can be applied to specific methods. E.g. `where <T as Add>::add: const` or something of
the sort.

## Explicit `const` bounds

One could require `T: const Trait` bounds to differentiate between bounds on which methods
can be called and bounds on which no methods can be called. This can backwards compatibly be
added as an opt-out via `T: ?const Trait` if the desire for such differences is strong.

## Infer all the things

We can just throw all this complexity out the door and allow calling any method on generic
parameters without an extra annotation `iff` that method satisfies `const fn`. So we'd still
annotate methods in trait impls, but we would not block calling a function on whether the
generic parameters fulfill some sort of constness rules. Instead we'd catch this during
const evaluation.

This is strictly the most powerful and generic variant, but is an enormous backwards compatibility
hazard as changing a const fn's body to suddenly call a method that it did not before can break
users of the function.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None