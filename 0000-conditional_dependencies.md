- Feature Name: `conditional_dependencies`
- Start Date: 2016-11-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Support "conditional" dependencies, which will be used only if that dependency
is already present. Users will be able to write code to integrate with another
library (for example, serializations for their type) without introducing a
*necessary* dependency on that library for users who don't need it.

# Motivation
[motivation]: #motivation

Currently, a common pattern exists for libraries to have feature flags which
turn on a dependency. For example, if I have a library which defines a new
type, I might want users to be able to serialize it with `serde`, but I don't
want users who don't care about this functionality to have to add `serde` to
their dependency graph. So I add a `serialization` feature to my Cargo.toml,
and only if you turn on this feature do you get the feature for serialization.

Ultimately, the source of this problem is the orphan rules - because only *I*
can define the impls for my type, I have to add this dependency. For every
commonly used trait defining library, I have to add a new feature, and all of
my clients have to turn on that feature.

cargo features have many uses, but this particular pattern ends up being a lot
of fiddling for a very common use case. Instead of requiring users go through
the feature flag rigmarole, we can provide first class support for this
pattern through "conditional dependencies" - dependencies that are turned on
automatically if they were already present.

# Detailed design
[design]: #detailed-design

## Conditional dependencies in `Cargo.toml`

When declaring a dependency in a Cargo.toml file, if that dependency is an
object, it can contain the `conditional` member, which is a boolean, similar to
the `optional` dependency. For example:

```toml
[dependencies]
foobar = { version = "1.0.0", conditional = true }
```

Or:

```toml
[dependencies.foobar]
version = "1.0.0"
conditional = true
```

A single dependency objecy cannot contain both a `conditional` key and an
`optional` key.

When generating the dependency graph, cargo will not include dependencies
tagged conditional. It will then traverse the graph looking for conditional
dependencies; if a matching source for a conditional dependency (e.g. a
matching version number) is already present in the graph, cargo will
automatically add that dependency.

## `#[cfg(dependency=)]`

A new kind of cfg attribute will be added to rustc. The `dependency` attribute
will only be compiled when a certain dependency has been explicitly passed to
rustc using the --extern flag.

Because cargo automatically passes dependencies explicitly with this command,
this code will be compiled only when cargo triggers the conditional dependency.

## Convention for using conditional dependencies

The above to features can be composed in any way, but to assist in adopting
this functionality, we can define an encouraged convention.

If you want to add a new conditional dependency, you should add a top level
submodule (that is, mounted directly under lib, main, etc) with the same name
as the conditional dependency. This module will be tagged with the cfg, and the
extern crate declaration and all code that uses symbols from that crate will
go inside of that module.

For example:

```rust
/// lib.rs

#[cfg(dependency=foobar]
mod foobar;
```

```rust
/// foobar.rs

extern crate foobar;

impl foobar::Bar for MyType {
    ...
}
```

## Preventing cyclic dependencies on upload to crates.io

When packaging and uploading a crate to crates.io, some strategy will be used
to ensure that its conditional dependencies could not introduce a cycle into
the dependency graph of your crate.

## `cargo doc` compiles with all dependencies

Unlike other forms of compilation, the `cargo doc` command will treat
conditional dependencies as present by default, in order to document the APIs
which exist only when conditional dependencies are present. The conditional
dependency may be provided somehow.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Alternatives
[alternatives]: #alternatives

What other designs have been considered? What is the impact of not doing this?

# Unresolved questions
[unresolved]: #unresolved-questions

What parts of the design are still TBD?
