- Feature Name: extern_item
- Start Date: 2015-12-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Generalize the delayed resolution of language items to arbitrary items.

# Motivation
[motivation]: #motivation

Consider an allocator crate that defines a default allocator:

```rust
type Default = Jemalloc;
```

At the point in the dependency tree where the allocator crate is defined, the
default allocator might not yet exist or downstream crates might want to define
their own allocator. Users of the allocator crate will, however, want to use the
default allocator as a default type parameter in their allocating types:

```rust
struct Vec<T, A: Allocator = alloc::Default>
```

In order to satisfy both constraints, a delayed resolution of items is added:

```rust
#[extern]
type Default: Allocator;
```

# Detailed design
[design]: #detailed-design

## Declaring extern items

An `#[extern]` attribute is added. This attribute can be applied to

* type definitions,
* functions,
* statics, and
* constants.

For example

```rust
#[extern]
type T: Bounds;

#[extern]
fn f();

#[extern]
static S: Type;

#[extern]
const C: Type;
```

where `Bounds` is a collection of trait and lifetime bounds. Users of `T`
definition can only use the properties exposed by the bounds. And where `Type`
is an arbitrary (possibly extern) type.

## Defining extern items

The `#[extern]` attribute can be applied in the `#[extern = path]` form to
define an extern items. For example

```rust
#[extern = "alloc::Default"]
struct X;

impl Allocator for X { /* ... */ }
```

The `path` in `#[extern = path]` can be any path to the item including
re-exports. Any crate that has a path to an extern item can define it.

The extern definitions must satisfy the bounds, etc. that appear in the
declarations.

When an executable or static library is compiled, each extern item declared in
any of the involved crates must have exactly one definition in any of the
involved crates. Otherwise a helpful error message is printed that shows
duplicate or missing definitions.

# Drawbacks
[drawbacks]: #drawbacks

* None known

# Alternatives
[alternatives]: #alternatives

* Hacks such as `#[allocator]`.

# Unresolved questions
[unresolved]: #unresolved-questions

* Many
