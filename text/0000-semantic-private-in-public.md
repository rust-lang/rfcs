- Feature Name: semantic_private_in_public
- Start Date: 2016-07-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Enforce that public APIs do not expose private definitions at the semantic level, while allowing the use of private aliases and blanket implementations for convenience and automation.

# Motivation
[motivation]: #motivation

The "private-in-public" rules ensure the transitivity of abstraction. That is, one must be able to name types and bounds used in public APIs, from anywhere else.
This property can be relied upon to create perfect proxies for generic types, functions and trait implementations.
However, the current set of rules is too strict and ignores any semantic equivalence:
```rust
type MyResult<T> = Result<T, String>;

#[derive(Clone)]
struct Wrap<T>(T);

pub fn foo<T>(_: T) -> MyResult<()>
    where Wrap<T>: Clone { Ok(()) }
```

The example above does not compile right now, because of `MyResult` and `Wrap` being private, even though a perfect proxy can be written as such:

```rust
fn bar<T: Clone>(x: T) -> Result<(), String> {
    api::foo(x)
}
```

This limitation most notably prevents derive and similar macros from generating bounds on field types, as they may contain private types, although most of the time the bound can be written in terms of type parameters and public types (`T` below), or is not needed at all (`U` below):
```rust
#[derive(Clone)]
pub struct Foo<T, U>(Wrap<T>, Wrap<Rc<U>>);
```

Deriving cannot but add both a `T: Clone` and a `U: Clone` bound, in the current implementation, which is more restrictive than necessary, and ironically prevents automatic generation of perfect wrapper types.

# Detailed design
[design]: #detailed-design

Function signatures, public field types, types of statics, constants and associated constants, types assigned to type aliases and associated types, and where clauses (after elaboration) must not be *less public* than the item they are found in.

The previous definition of *less public* relied solely on paths as they appear in the source, but after this RFC, it is more fine-grained:

An item `X` is *less public* than another item `Y` if there exists a module from where `Y` can be referred to (by any name) whereas `X` can't, taking into account `pub(restricted)` and any other privacy semantics.

```rust
pub mod m {
    struct A;
    // A is less public than B
    pub(crate) trait B {}
    // B is less public than c
    pub fn c() {}
}
```

A type or bound is *less public* than an item `X` if it refers to any type or trait definition that is *less public* than `X`, after resolving aliases and associated types.

Where clauses in an item `X` are elabored as follows:
 * type aliases and associated types are resolved as with all types
 * lifetime bounds *less public* than `X` are replaced with lifetime bounds on type and lifetime parameters
 * for each trait bound *less public* than `X`:
  * a list of applicable implementations is computed
  * because the bound refers to items that cannot be exported, coherence will prevent applicable implementations from existing in downstream crates
  * type parameters of `X` are assumed to match any type, regardless of what other bounds `X` has
  * if there is exactly one applicable `impl`, the bound is replaced with the where clauses of that `impl`, after elaborating them as well

The set of bounds left after the recursive elaboration of `X`'s where clauses must not be *less public* than `X`, even if the original where clauses are allowed to.

Example for use in deriving, without restricting the user or exposing private details:
```rust
#[derive(Debug)]
struct Wrap<T>(T);

#[derive(Copy, Clone, Debug)]
pub struct Ref<'a, T: 'a>(&'a Wrap<T>);

// deriving will produce:
impl<'a, T> Copy for Ref<'a, T>
    where &'a Wrap<T>: Copy {}
impl<'a, T> Clone for Ref<'a, T>
    where &'a Wrap<T>: Clone {...}

impl<T> Debug for Wrap<T>
    where T: Debug {...}
impl<'a, T> Debug for Ref<'a, T>
    where &'a Wrap<T>: Debug {...}

// after elaborating where clauses:
impl<'a, T> Copy for Ref<'a, T> {}
impl<'a, T> Clone for Ref<'a, T> {...}

impl<T> Debug for Wrap<T>
    where T: Debug {...}
impl<'a, T> Debug for Ref<'a, T>
    where T: Debug {...}
```

# Drawbacks
[drawbacks]: #drawbacks

Browsing sources and generating documentation becomes more complex, as private details need to be replaced with equivalent public versions before using them from other modules/crates.

The "one applicable `impl`" rule works well with deriving, but adding a private `impl` can break adjacent public items, whereas the existing strategy of placing bounds on type parameters would continue to work, however restrictive it may be in general.

# Alternatives
[alternatives]: #alternatives

We could leave the current situation as-is, or just resolve type aliases and associated types, leaving deriving in the same suboptimal state.

# Unresolved questions
[unresolved]: #unresolved-questions

How much catering do we need to do to public re-exports out of private modules?

Is coherence guaranteed to prevent the existence of downstream trait implementations that match a bound using both type parameters and unexported type definitions?
