- Feature Name: `automatic_features`
- Start Date: 2016-11-05
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Support "automatic" features, which will be used only if the dependencies of
that feature are already present. Users will be able to write code to integrate
with another library (for example, serializations for their type) without
introducing a *necessary* dependency on that library for users who don't need
it.

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

## Automatic features in `Cargo.toml`

When declaring a feature in a Cargo.toml file, if that feature is an
object, it can contain the `automatic` member. For example:

```toml
[features]
foobar = { dependencies = ["foo"], automatic = true }
```

Or:

```toml
[features.foobar]
dependencies = ["foo"]
automatic = true
```

When generating the dependency graph, cargo will at first not include features
tagged automatic (unless asked to do so specifically via `--features` or if the
feature is a default feature and `--no-default-features` has not been past). It
will then look for features (in all crates in the graph) that only need
dependencies which are already part of the graph. These features will be
enabled. This will add more edges to the dependency graph, but not more nodes.

Note that features can depend on crates as well as _other features_, so this
might be implemented as a multi-pass process.

For example, I can have the `foo` crate with an automatic feature `serialize`
which enables a dependency on serde. I can also have the `bar` crate (which
depends on `foo`) with an automatic feature of the same name which enables both
a dependency on serde and a dependency on `foo/serialize`, i.e. it enables the
`serialize` feature of its dependency `foo`. If serde was included, then in the
first pass, the automatic feature `serialize` on `foo` will be enabled, but not
on `bar`. In the second pass, because `foo/serialize` now exists,
`bar/serialize` is automatically enabled as well.

The process terminates because it is monotonic -- it only ever adds edges to the
graph, never removing them.

## Convention for using automatic features

If you want to add a new automatic feature, you should add a top level
submodule (that is, mounted directly under lib, main, etc) with the same name
as the automatic feature. This module will be tagged with the cfg, and the
extern crate declaration and all code that uses symbols from that crate will
go inside of that module.

For example:

```rust
/// lib.rs

#[cfg(feature=foobar)]
mod foobar;
```

```rust
/// foobar.rs

extern crate foobar;

impl foobar::Bar for MyType {
    ...
}
```

## `cargo doc` compiles with all dependencies

Unlike other forms of compilation, the `cargo doc` command will treat
automatic features as present by default, in order to document the APIs
which exist only when automatic features are present.

# Alternatives
[alternatives]: #alternatives

We considered options like allowing the orphan rules to be broken by certain
crates that were recognized as "sibling" crates by a parent, as a way to get
around the orphan rule issue. However, this alternative would create a
soundness hole in the orphan rules and violate good layering, by making them
only fully enforced by cargo and not by rustc. Ultimately, conditionally
altering the shape of code compiled by the flags passed by cargo seemed like
a better solution to the problem.

A previous version of this RFC proposed "conditional dependencies", which let
you specify individual dependencies as "conditional", which would be turned on
if the dependency already existed in the graph. However, this introduces a
parallel way of specifying optional dependencies which doesn't integrate with
features. Automatic features provide a smooth transition and also allow
dependencies to be grouped together.

# Unresolved questions
[unresolved]: #unresolved-questions

The Cargo.toml key names ("dependencies" and "automatic") could be bikeshedded.

Should we allow automatic features which depend on other features? This seems
like a natural thing to do, but it complicates the resolution process.
