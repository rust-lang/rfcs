- Feature Name: `dyn_trait_declarations`
- Start Date: 2020-11-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC specifies an extension to the trait declaration syntax to explicitly mark that the trait must be
trait-object-safe and provide better errors when a declaration does not uphold that invariant.

The aim of this RFC is to reduce the usage of a common design pattern for trait objects and replace it with a more
idiomatic, understandable, and explicit approach.

The transition from the old design pattern to the explicit syntax will be done through lints which discourage usages
of common patterns such as `fn _assert_is_object_safe(_: &dyn Trait) {}`. As well as lints which suggest to mark a trait as
dyn if it is being used as a trait object. Moreover, the current error of `the trait MyTrait cannot be made into an object` will
be augmented to suggest to the user to mark the trait as `dyn`. The lints will be warn by default and will be further upgraded to errors in edition 2024.

# Motivation

[motivation]: #motivation

A very common design pattern observed in rust projects, is creating a trait which will be purely used as a trait object. However, rustc has no way of knowing if the user intends to use the trait as an object. This leads to errors of object-unsafety only showing up when the user tries using the trait as an object, such as the following:

```rust
struct Objects {
  objects: Vec<Box<dyn Trait>>
  //               ^^^^^^^^^ error: the trait `Trait` cannot be made into an object
}
```

However, the range of the error will always be the usage of the trait, which leads to confusion for errors not through the CLI. A complement to this pattern is a function to ensure that the trait is object safe to avoid breakage across changes:

```rust
fn _assert_is_object_safe(_: &dyn MyTrait) {}
```

To avoid the need for making such a test case to enforce object safety, this RFC proposes a modification to the trait declaration syntax to explicitly mark that a trait must be object safe:

```rust
dyn trait MyTrait { /* */ }
```

If any conditions in the implementation make it not object safe, a compiler error is emitted with an accurate range. Accurate ranges for the error solves the issue of hard to pinpoint errors as previously pointed out. The addition of this syntax is fairly simple (parsing-wise) as `dyn` is a reserved keyword and the definition does not clash with any other productions.

And finally, this proposal has been proposed to a certain degree before [here](https://github.com/rust-lang/rust/issues/57893#issuecomment-546972824). This RFC would be a way to gradually achieve the goal of this issue comment in the future.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

Rust provides support for explicitly marking that a trait must always be object safe. If your trait is being used as an object, you should explicitly mark it as a `dyn` trait as such:

```rust
dyn trait MyTrait { /* */ }
```

This will enforce that `MyTrait` is able to be made into a trait object. If this is not the case, then errors in the implementation will be issued. If you were previously using a pattern such as the following, you can simply delete it:

```rust
fn _assert_is_object_safe(_: &dyn MyTrait) {}
```

**Note**: note this does not mean traits not marked with `dyn` cannot be made into trait objects. It means the trait is not guaranteed to be object safe and it will not be always automatically enforced.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

When a trait is marked as `dyn`, a further step in type checking is run, which runs the same trait object safety resolution algorithm as the check for a usage such as `&dyn MyTrait`. However, instead of issuing a single error with multiple labels, multiple errors with their own ranges will be issued to accurately reflect the root of the issue.

Parsing this syntax is simple, since `dyn` is a reserved keyword, and there is nothing else that could match in an item context.

The same rules as object safety resolution would be applied, with the exception that every method in the trait must be object safe.

# Drawbacks

[drawbacks]: #drawbacks

It is more syntax to think about for the user, it may also introduce confusion if object safety errors are issued twice, once in the usage, and in the implementation block. It may also raise confusion for users who may think traits cannot be made into trait objects without `dyn`.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

The pattern of using a trait purely as a trait object or simply enforcing that it must be a trait object is very common, projects such as libcore and rust-analyzer use it in multiple places. This pattern has become common enough that adding a simple language feature to idiomatically support it is viable.

The rationale between the syntax is the following:

- Users already associate `dyn` with trait objects.
- The syntax is intuitive and unambiguous.
- The syntax is simple to parse

Another consideration of a proc-macro based approach were brought up too, e.g.

```rust
#[object_safe]
trait MyTrait { /* */ }
```

similar to `#[non_exhaustive]`, this was ultimately ruled out in favor of dedicated syntax.

# Prior art

[prior-art]: #prior-art

- [dyn trait syntax RFC](./2113-dyn-trait-syntax.md)

# Unresolved questions

[unresolved-questions]: #unresolved-questions

- How complex is isolating the trait object safety resolution logic?
- How do we decide what to lint? should `_assert_foo_is_object_safe` and `_assert_is_object_safe` both be linted?
  What about no underscore?

# Future possibilities

[future-possibilities]: #future-possibilities

I cannot think of any at this moment.
