- Feature Name: `cargo-hints-min-opt-level`
- Start Date: 2026-02-22
- RFC PR: [rust-lang/rfcs#3924](https://github.com/rust-lang/rfcs/pull/3924)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Allow Rust library to provide a simple hint about the minimum opt-level to
build them with.

## Motivation
[motivation]: #motivation

When people build Rust projects, they have a choice of tradeoffs between the
speed of the build and the performance of the code at runtime. Builds using the
dev profile aim for minimal build time at the expense of unoptimized code at
runtime; builds using the release profile spend more time building in order to
improve runtime performance.

However, there are some library crates for which almost every use case wants
optimization, because an unoptimized build provides so little performance as to
be unusable. Such libraries typically include a note in their `README.md`
warning users to turn on optimization even in the dev profile:

```toml
[profile.dev.package."example-high-performance-package"]
opt-level = 3
```

This RFC adds a mechanism for crates to *hint* that builds should use
optimization by default, while still allowing the top-level crate to easily
override this. Handling this in dependencies makes it automatic for users, and
also makes it easier to keep up with an evolving dependency tree.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Some libraries benefit strongly from optimization, or are hurt especially by
builds without optimization, to the point that they expect almost no users to
ever want to run them unoptimized. Such libraries can use the Cargo `hints`
system to provide a minimum optimization level:

```toml
[hints]
min-opt-level = 2
```

When building with a profile with a default optimization level lower than the
`hints.min-opt-level` value, the crate will be built with the specified minimum
optimization level instead. When building with a profile with a higher
default optimization level, that higher optimization level will take precedence.

This applies whether the optimization level is the default for the profile, or
is set by the user explicitly for the profile:

```toml
[profile.dev]
opt-level = 1
# Does not override a higher hints.min-opt-level specified in a dependency.
```

To override the hinted minimum optimization level, the top-level crate can use
profile overrides to set the opt-level. Any opt-level specified via profile
overrides will take precedence over any hint, whether the profile override
applies to a specific crate or to all dependencies:

```toml
[profile.dev.package."*"]
opt-level = 1
# Overrides hints.min-opt-level specified in a dependency.

[profile.dev.package."random-package"]
opt-level = 0
# Overrides hints.min-opt-level specified in a dependency.
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `hints.min-opt-level` key requires an integer, and only supports numeric
hint levels (0, 1, 2, 3). Non-numeric hint levels like `s` and `z` are not
supported, because they don't fit into a strictly ordered progression.

`hints.min-opt-level`, like any hint, does not affect a crate's MSRV; older
versions of Cargo will ignore it.

Any profile override will take precedence over `hint.min-opt-level`, including
an override for all dependencies, an override for all build dependencies, or an
override for a specific dependency.

A profile specifying an `opt-level` will not override a higher
`hints.min-opt-level` specified in a dependency.

Note that a hint provided by a given library crate only applies to that
specific crate, not that package's dependencies. If the code that needs
optimizing is in a dependency, that dependency would need to add the hint.

## Drawbacks
[drawbacks]: #drawbacks

Crates could overuse this mechanism, requiring optimization even when they
don't actually need it. We should provide clear documentation recommending when
to use it and when not to use it.

If a crate using this mechanism wishes to nonetheless build with different
optimizations within its own workspace, it would have to add an override.

### Limitations

Library crates cannot set this for dependencies they do not maintain; a crate
can only set the min-opt-level for itself. This may cause issues for crates
whose performance depends heavily on its dependencies; such crates may still
have to rely on user documentation.

Library optimizations may not apply to code inlined or monomorphized by a
user's crate.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This mechanism intentionally does not offer a "maximum optimization level", nor
does it support optimizing for size.

This mechanism intentionally does not provide access to any specific target
feature flags, as this is typically something the top-level crate needs to
retain full control over based on its minimum system requirements.

We could support multiple min-opt-level hints, one for dev-like profiles and
one for release-like profiles, for crates that want a lower min-opt-level in
the dev profile. However, `profile.release` already defaults to `opt-level =
3`, so `min-opt-level` will generally never affect the release profile, only
the dev profile. Thus, a single setting seems sufficient.

We could have a simple boolean, e.g. `optimize-in-dev = true`, and leave it to
Cargo whether that means opt-level 1, 2, or 3. This would be simpler, but would
prevent crates from determining whether they benefit from opt-level 3 (e.g.
aggressive vectorization and loop unrolling) or not.

We could have a hint apply recursively to dependencies. This seems like more
control than a library crate could have, as a dependency may be used in
multiple places in the crate graph.

## Prior art
[prior-art]: #prior-art

Cargo already provides the profile overrides mechanism, for the top-level crate
to specify the opt-level of individual crates.

This RFC builds on the `hints` mechanism, currently used for
`hints.mostly-unused`. The
[post announcing `hints.mostly-unused`](https://blog.rust-lang.org/inside-rust/2025/07/15/call-for-testing-hint-mostly-unused/)
included
[a section on future hints such as `min-opt-level`](https://blog.rust-lang.org/inside-rust/2025/07/15/call-for-testing-hint-mostly-unused/#future-hints).
Cargo has
[an issue discussing `min-opt-level`](https://github.com/rust-lang/cargo/issues/8501).

C and C++ compilers provide directives such as `#pragma optimize` or
`__attribute__((optimize))`, which let individual files or functions define an
optimization level.

## Future possibilities
[future-possibilities]: #future-possibilities

`hints.min-opt-level` is a simple mechanism providing a single hint. There are
many other possible optimization hints a library *might* wish to provide, and
we could consider adding further hints for those in the future. Any such hint
would need to balance the tradeoffs between value, additional complexity, and
whether crates in the ecosystem know the right optimization level for their
crate better than their users do.

We could in particular have a `max-opt-level`, for crates that don't benefit
from `opt-level = 3` to lower the optimiation level to 2.

The mechanism to override a dependency's opt-level using `profile.dev.package`
forces a given opt-level whether the dependency asks for higher or lower. Users
of overrides, particularly those for `"*"`, might want a mechanism for setting
a minimum without overriding a higher optimization level.

We may in the future want to change the default optimization level for the dev
profile to 1, rather than 0. opt-level 1 includes optimizations that can make
compilation *faster*, such as by sending less code to the codegen backend (e.g.
LLVM). This might reduce the number of crates motivated to use this mechanism,
but the mechanism would remain important, as there are library crates which
strongly benefit from opt-level 2 or 3.

We may want to provide a further mechanism for libraries whose performance
depends heavily on their dependencies to optimize those dependencies.

We may want to provide a mechanism for libraries to optimize code inlined or
monomorphized by a user's crate. That would likely require compiler
enhancements.
