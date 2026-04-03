- Feature Name: `cfg_alias`
- Start Date: 2025-04-23
- RFC PR: [rust-lang/rfcs#3804](https://github.com/rust-lang/rfcs/pull/3804)
- Rust Issue:
  [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC introduces a way to name configuration predicates for easy reuse
throughout a crate.

```rust
#![cfg_alias(x86_linux = all(
    any(target_arch = "x86", target_arch = "x86_64"), target_os = "linux"
))]

#[cfg(x86_linux)]
fn foo() { /* ... */ }

#[cfg(not(x86_linux))]
fn foo() { /* ... */ }
```

# Motivation

[motivation]: #motivation

It is very common that the same `#[cfg(...)]` options need to be repeated in
multiple places. Often this is because a `cfg(...)` needs to be matched with a
`cfg(not(...))`, or because code cannot easily be reorganized to group all code
for a specific `cfg` into a module. The solution is usually to copy a `#[cfg]`
group around, which is error-prone and noisy.

Adding aliases to config predicates reduces the amount of code that needs to be
duplicated, and giving it a name provides an easy way to show what a group of
configuration is intended to represent.

Something to this effect can be done using build scripts. This requires reading
various Cargo environment variables and potentially doing string manipulation
(for splitting target features), so it is often inconvenient enough to not be
worth doing. Allowing aliases to be defined within the crate and with the same
syntax as the `cfg` itself makes this much easier.

Another benefit is the ability to easily adjust configuration to many different
areas of code at once. A simple example is gating unfinished code that can be
toggled together:

```rust
#![cfg_alias(todo = false)] // change `false` to `true` to enable WIP code

#[cfg(todo)]
fn to_be_tested() { /* ... */ }


#[test]
#[cfg(todo)]
fn test_to_be_tested() { /* ... */ }
```

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

There is a new crate-level attribute that takes a name and a `cfg` predicate:

```rust
#![cfg_alias(some_alias = predicate)]
```

`predicate` can be anything that usually works within `#[cfg(...)]`, including
`all`, `any`, and `not`.

Once an alias is defined, `name` can be used as if it had been passed via
`--cfg`:

```rust
#[cfg(some_alias)]
struct Foo { /* ... */ }

#[cfg(not(some_alias))]
struct Foo { /* ... */ }

#[cfg(all(some_alias, target_os = "linux"))]
fn bar() { /* ... */ }
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The new crate-level attribute is introduced:

```text
CfgAliasAttribute:
    cfg_alias(IDENTIFIER `=` ConfigurationPredicate)
```

The identifier is added to the `cfg` namespace. It must not conflict with:

- Any builtin configuration names
- Any configuration passed via `--cfg`
- Any configuration passed with `--check-cfg`, since this indicates a possible
  but omitted `--cfg` option
- Other aliases that are in scope

Once defined, the alias can be used as a regular predicate.

The alias is only usable after it has been defined. For example, the following
will emit an unknown configuration lint:

```rust
#![cfg_attr(some_alias, some_attribute)]
// warning: unexpected_cfgs
//
// The lint could mention that `some_alias` was found in the
// crate but is not available here.

#![cfg_alias(some_alias =  true)]
```

_RFC question: "usable only after definition" is mentioned here to retain the
ability to parse attributes in order, rather than going back and updating
earlier attributes that may use the alias. Is this a reasonable limitation to
keep?_

_RFC question: two ways to implement this are with (1) near-literal
substitution, or (2) checking whether the alias should be set or not at the time
it is defined. Is there any user-visible behavior that would make us need to
specify one or the other?_

_If we go with the first option, we should limit to a single expansion to avoid
recursing (as is done for `#define` in C)._

## `cfg_alias` in non-crate attributes

`cfg_alias` may also be used as a module-level attribute rather than
crate-level:

```rust
#[cfg_alias(foo = bar)]
mod uses_bar {
    // Enabled/disabled based on `cfg(bar)`
    #[cfg(foo)]
    fn qux() { /* ... */ }
}

#[cfg_alias(foo = baz)]
mod uses_baz {
    // Enabled/disabled based on `cfg(baz)`
    #[cfg(foo)]
    fn qux() { /* ... */ }
}
```

This has the advantage of keeping aliases in closer proximity to where they are
used; if a configuration pattern is only used within a specific module, an alias
can be added at the top of the file rather than making it crate-global.

When defined at a module level, aliases are added to the configuration namespace
for everything within that module including later module-level configuration.
There is no conflict with aliases that use the same name in other modules.

This RFC proposes that the use of `cfg_alias` on modules _should_ be included if
possible. However, this may bring implementation complexity since, to the RFC
author's knowledge, the rustc configuration system is not designed to allow
scoped configuration. If implementation of module-level aliases turns out to be
nontrivial, this portion of the feature may be deferred or dropped before
stabilization.

# Drawbacks

[drawbacks]: #drawbacks

- This does not support more general attribute aliases, such as
  `#![alias(foo = derive(Clone, Copy, Debug, Default)`. This seems better suited
  for something like `declarative_attribute_macros` in [RFC3697].

[RFC3697]: https://github.com/rust-lang/rfcs/pull/3697

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

- The syntax `cfg_alias(name =  predicate)` was chosen to mimic assignment in
  Rust and key-value mappings in attributes. Alternatives include:
  - `cfg_alias(name, predicate)`, which is more similar to
    `cfg_attr(predicate, attributes)`.
- It may be possible to have `#[cfg_alias(...)]` work as an outer macro and only
  apply to a specific scope. This likely is not worth the complexity.

# Prior art

[prior-art]: #prior-art

In C it is possible to modify the define map in source:

```c
# if (defined(__x86_64__) || defined(__i386__)) && defined(__SSE2__)
#define X86_SSE2
#endif

#ifdef X86_SSE2
// ...
#endif
```

# Unresolved questions

[unresolved-questions]: #unresolved-questions

Questions to resolve before this RFC could merge:

- Which syntax should be used?
- Substitution vs. evaluation at define time (the question under the
  reference-level explanation)

# Future possibilities

[future-possibilities]: #future-possibilities

- A `--cfg-alias` CLI option would provide a way for Cargo to interact with this
  feature, such as defining config aliases in the workspace `Cargo.toml` for
  reuse in multiple crates.
