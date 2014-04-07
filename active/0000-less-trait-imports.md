- Start Date: 2014-04-06
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

When doing method lookup, consider all implementations from the crate the type
is defined in, not just inherent methods and in-scope traits.

# Motivation

Today, the methods a type implements are split into two groups: inherent, and
trait-provided. Inherent methods are always visible, but trait-provided
methods require that the trait be in scope. This is to prevent nasty
compilation failure in the case when two crates each have a trait with
overlapping method names. For example:

```rust
// crate A
trait Foo {
    fn what(&self) -> int;
}

impl Foo for int { ... }

// crate B

trait Bar {
    fn what(&self) -> int;
}

impl Bar for int { ... }
```

If a crate C were to link to both A and B, and then call the method `what` on
an int, which method would it dispatch to? Furthermore, if it only links to
crate A and calls `what`, what happens when later on it starts linking to `B`?
Requiring that the trait be in scope is a good solution for this, but it also
has undesirable consequences. First, it adds additional imports whenever one
wants to call a trait-provided method on a type. This can become burdensome
for crates which have many abstractions. Second, it causes a backwards
compatability hazard when factoring out behavior into a common trait. In order
to do the refactor, all dependent crates must now start importing this new
trait.

This restriction causes pain for all non-extension-method uses of traits. A
less restrictive system would be to instead use methods from an extension
implementation only if that trait is in scope.

A more concrete example arises in `libterm`. Ideally `AnsiTerminal` would be a
trait with `Win32Terminal` and `TerminfoTerminal` implementing it. Doing so,
however, affects every use of the crate. Were `libterm` marked stable,
introducing such an abstraction would be impossible. I worry that this will be
a significant backwards compat hazard, since most crates do not have the
luxury of a language-supported prelude.

A nice benefit of this is that we will never need a trait in the prelude
again. Since the traits are all in std, there can never be an extension
implementation.

# Detailed design

When looking up a type's methods, instead of just looking at what traits are
in scope and the inherent methods, consider every implementation from the
crate the type is defined in. When both an non-extension and extension method
are found, use the method from the most-recently imported trait defining it
(this is the current shadowing behavior). Chosing a specific trait to call the
method with beyond this simple shadowing is the subject of
[UFCS](https://github.com/rust-lang/rfcs/pull/4).

# Alternatives

Do nothing. I believe this will impose a significant backwards compatability
hazard.

We could also, instead of looking at *all* implementations in the crate, look
only at implementations of traits defined in that crate. This will not allow
us to shrink the prelude, but is somewhat simpler.

# Unresolved questions

Does this affect the method lookup algorithm in any undesirable ways?
