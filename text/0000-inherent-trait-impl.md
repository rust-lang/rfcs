- Feature Name: inherent-trait-impl
- Start Date: 2018-03-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Provides a mechanism to declare "inherent traits" for a type defined in the same crate. Methods on these traits are callable on instances of the specified type without needing to import the trait.

# Motivation
[motivation]: #motivation

There are two similar cases where this is valuable:

- Frequently used traits.

  Sometimes getting the right abstractions require breaking up a type's implementation into many traits, with only a few methods per
  trait. Every use of such a type results in a large number of imports to ensure the correct traits are in scope. If such a type is used
  frequently, then this burden quickly becomes a pain point for users of the API,
  especially if users do not care about writing generic code over traits.

- Mapping object-oriented APIs.

  When mapping these APIs to rust, base classes are usually mapped to traits: methods on those base classes will need to be callable on any
  derived type. This is sub-optimal because to use a class method a user must now know which class in the hierachy defined that
  method, so that they can import and use the corresponding trait. This knowledge is not required when using the same API from an
  object-oriented language.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The feature is implemented using a new attribute which can be applied to trait
`impl` blocks:

```rust
pub trait Bar {
    fn bar(&self);
}

pub struct Foo;

#[inherent]
impl Bar for Foo {
    fn bar(&self) { println!("foo::bar"); }
}

impl Foo {
    fn foo(&self) { println!("foo::foo"); }
}
```

The method `bar` is now callable on any instance of `Foo`,
regardless of whether the `Bar` trait is currently in scope,
or even whether the `Bar` trait is publically visible. In other words if `Bar`
is defined in one crate and `Foo` in another, the user of `Foo` will be
able to explicitly depend only on the crate which defines `Foo` and still use
the inherent trait's methods.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `#[inherent]` attribute in the above example desugars equivalently to an inherent forwarding method (in addition to the trait impl):

```rust
impl Foo {
    #[inline]
    pub fn bar(&self) { <Self as Bar>::bar(self); }
}
```

Any questions regarding coherence, visibility or syntax can be resolved by
comparing against this expansion, although the feature need not be implemented
as an expansion within the compiler.

# Drawbacks
[drawbacks]: #drawbacks

- Increased complexity of the language.
- Hides use of traits from users.

# Rationale and alternatives
[alternatives]: #alternatives

- Define inherent traits either on a type `T` or on an `impl T { .. }` block.
- Implement as part of the delegation RFC.
- Do nothing: users may choose to workaround the issue by manually performing the expansion if required.

The most viable alternative is delegation proposal, although arguably inherent
traits and delegation solve different problems with the similar end result.
The former allows to use trait methods without importing traits and the latter
to delegate methods to the selected field. Nethertheless delegation RFC and
this RFC can be composable with each other:

```Rust
struct Foo1;
struct Foo2;

#[inherent]
impl T1 for Foo1 {
    fn a() {}
}

#[inherent]
impl T2 for Foo2 {
    fn b() {}
}

struct Bar {
    f1: Foo1,
    f2: Foo2,
}

impl Bar {
    // all methods from `T1` will be delegated as well
    // though `T1` will not be implemented for `Bar`
    delegate * to f1;
}

// method `b` will be accessable on `Bar` without importing `T2`
#[inherent]
impl T2 for Bar {
    delegate * to f2;
}
```

# Prior art
[prior-art]: #prior-art

- https://github.com/rust-lang/rfcs/pull/2309 (previous closed PR)
- https://github.com/rust-lang/rfcs/issues/1880
- https://github.com/rust-lang/rfcs/issues/1971

# Unresolved questions
[unresolved]: #unresolved-questions

- Do we want to introduce a new keword instead of using `#[inherent]`? In other
words do we want to write `inherent impl A for B { .. }`?
