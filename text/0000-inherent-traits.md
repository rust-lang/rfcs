- Feature Name: inherent_traits
- Start Date: 2018-01-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Provides a mechanism to declare "inherent traits" for a type defined in the same crate. Methods on these traits are callable on instances
of the specified type without needing to import the trait.

# Motivation
[motivation]: #motivation

There are two similar cases where this is valuable:

- Mapping object-oriented APIs.

  When mapping these APIs to rust, base classes are usually mapped to traits: methods on those base classes will need to be callable on any
  derived type. This is sub-optimal because to use a class method a user must now know which class in the hierachy defined that
  method, so that they can import and use the corresponding trait. This knowledge is not required when using the same API from an
  object-oriented language.

- Frequently used types.

  Sometimes getting the right abstractions require breaking up a type's implementation into many traits, with only a few methods per
  trait. Every use of such a type results in a large number of imports to ensure the correct traits are in scope. If such a type is used
  frequently, then this burden quickly becomes a pain point for users of the API.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The feature is implemented using a new attribute which can be applied to `impl` blocks:

```rust
pub struct Foo;

trait Bar {
    fn bar(&self);
}

impl Bar for Foo {
    fn bar(&self) { println!("foo::bar"); }
}

#[include(Bar)]
impl Foo {
    fn foo(&self) { println!("foo::foo"); }
}
```

The method `bar` is now callable on any instance of `Foo`, regardless of whether the `Bar` trait is currently in scope, or even whether
the `Bar` trait is publically visible.

The `impl` block may be empty, in which case the only methods defined on the type are those from any included traits.

The `include` attribute may include multiple traits.

The `include` attribute is not valid on `impl <trait> for <type>` style `impl` blocks.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `include` attribute in the above example makes the `impl` block equivalent to:

```rust
impl Foo {
    #[inline]
    pub fn bar(&self) { <Self as Bar>::bar(self); }

    fn foo(&self) { println!("foo::foo"); }
}
```

Any questions regarding coherence, visibility or syntax can be resolved by comparing against this expansion, although the feature need not
be implemented as an expansion within the compiler.

# Drawbacks
[drawbacks]: #drawbacks

- Increased complexity of the language.

# Rationale and alternatives
[alternatives]: #alternatives

- Do nothing: users may choose to workaround the issue by manually performing the expansion if required.

# Unresolved questions
[unresolved]: #unresolved-questions

- Syntax bike-shedding
