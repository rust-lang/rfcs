- Feature Name: `build-target-edition`
- Start Date: 2025-02-13
- RFC PR: [rust-lang/rfcs#3772](https://github.com/rust-lang/rfcs/pull/3772)
- Rust Issue: [rust-lang/cargo#15283](https://github.com/rust-lang/cargo/issues/15283)

## Summary
[summary]: #summary

Deprecate `lib.edition`, etc in favor of only setting `package.edition`, removing the fields in the next Edition.

## Motivation
[motivation]: #motivation

Cargo supports setting the edition per-build-target:
```toml
[package]
name = "foo"
edition = "2021"

[lib]
edition = "2015"

[[bin]]
name = "foo"
path = "src/main.rs"
edition = "2015"

[[example]]
name = "foo"
path = "examples/foo.rs"
edition = "2015"

[[test]]
name = "foo"
path = "tests/foo.rs"
edition = "2015"

[[bench]]
name = "foo"
path = "benches/foo.rs"
edition = "2015"
```

This was intended for ([cargo#5661](https://github.com/rust-lang/cargo/issues/5661)):
- Migrating to a new edition per build-target
- Per edition tests

In practice, this feature does not seem to be in common use.
Searching the latest `Cargo.toml` files of every package on crates.io,
we found 13 packages using this feature
([zulip](https://rust-lang.zulipchat.com/#narrow/channel/246057-t-cargo/topic/Deprecate.20build-target.20.60edition.60.20field.3F/near/499047806)):
- 4 set `edition` on the sole build-target, rather than on the `package`
- 3 set `edition` because they enumerated every build-target field but then forgot to update them when updating `package.edition`
- 3 (+2 forks) have per-edition tests
- 1 has every version yanked

While this does not account for transient use of this feature during an Edition migration,
from our experience and observing others, we think this practice is not very common.
In fact, it seems more likely that migrating a lint at a time may be more beneficial
([cargo#11125](https://github.com/rust-lang/cargo/issues/11125#issuecomment-2641119791)).
There is also an open question on the Cargo team on how much to focus on multiple build-targets per package
vs individual packages
(see [This Development-cycle in Cargo: 1.77](https://blog.rust-lang.org/inside-rust/2024/02/13/this-development-cycle-in-cargo-1-77.html#when-to-use-packages-or-workspaces)).

Drawbacks of this feature include:
- Using this has a lot of friction as users have to explicitly
  enumerate each build target they want to set `edition` on which usually requires
  also setting the `name` and `path`.
- This has led to bugs where people thought they migrated editions but did not
- This is an easily overlooked feature to take into account when extending Cargo
- Cargo cannot tell a `build.rs` what Edition to generate code for as it may generate code for one of several
  ([cargo#6408](https://github.com/rust-lang/cargo/issues/6408)).
  This will become more important once we have [metabuild](https://github.com/rust-lang/cargo/issues/14903) for delegating build scripts to dependencies.
- Making it more difficult for tools like `cargo fmt`
  ([rustfmt#5071](https://github.com/rust-lang/rustfmt/pull/5071)) which need to map
  a file back to its edition which requires heuristics to associate a `.rs`
  file with a `Cargo.toml` and then to associate it with a specific build-target.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Documentation updates:

### Configuring a target

*From the [Cargo book](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#configuring-a-target)*

...

```toml
[lib]
name = "foo"           # The name of the target.
path = "src/lib.rs"    # The source file of the target.
test = true            # Is tested by default.
doctest = true         # Documentation examples are tested by default.
bench = true           # Is benchmarked by default.
doc = true             # Is documented by default.
proc-macro = false     # Set to `true` for a proc-macro library.
harness = true         # Use libtest harness.
crate-type = ["lib"]   # The crate types to generate.
required-features = [] # Features required to build this target (N/A for lib).

edition = "2015"       # Deprecated, N/A for Edition 20XX+
```

...

#### The `edition` field

The `edition` field defines the [Rust edition] the target will use. If not
specified, it defaults to the [`edition` field][package-edition] for the
`[package]`.

This field is deprecated and unsupported for Edition 20XX+

### Migration guide

*From [Rust Edition Guide: Advanced migration strategies](https://doc.rust-lang.org/nightly/edition-guide/editions/advanced-migrations.html#migrating-a-large-project-or-workspace)*

#### Migrating a large project or workspace

You can migrate a large project incrementally to make the process easier if you run into problems.

In a [Cargo workspace], each package defines its own edition, so the process naturally involves migrating one package at a time.

Within a [Cargo package], you can either migrate the entire package at once, or migrate individual [Cargo targets] one at a time.
For example, if you have multiple binaries, tests, and examples, you can use specific target selection flags with `cargo fix --edition` to migrate just that one target.
By default, `cargo fix` uses `--all-targets`.

*(removed talk of the build-target `edition` field)*

#### Migrating macros

...

If you have macros, you are encouraged to make sure you have tests that fully
cover the macro's syntax. You may also want to test the macros by importing and
using them in crates from multiple editions, just to ensure it works correctly
everywhere.
You can do this in doctests by setting the [edition attribute](https://doc.rust-lang.org/stable/rustdoc/write-documentation/documentation-tests.html#attributes)
or by creating a package for your tests in your workspace for each edition.

If you run into issues, you'll need to read through the chapters of
this guide to understand how the code can be changed to work across all
editions.

*(added a testing strategy which was previously left unspoken)*

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A non-`None` edition will be considered deprecated
- A deprecation message will eventually be shown by Cargo
  - Timing depends on if this will be blocked on having `[lints]` control over this or not
- When `package.edition` is set to Edition 20XX+, an error will be reported when a `<build-target>.edition` field is set.

## Drawbacks
[drawbacks]: #drawbacks

- This makes testing macros more difficult as they are limited to either
  - doctests
  - creating packages just for the sake of defining tests

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### One Edition field controlling another

The exact semantics of `package.edition` vs `<build-target>.edition` have not been well defined when it comes to the manifest format itself.

`package.edition`'s [documentation](https://doc.rust-lang.org/cargo/reference/manifest.html#the-edition-field) says:

> [it] affects which Rust Edition your package is compiled with

while `<build-target>.edition` [documentation](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-edition-field) says:

> [it] defines the Rust edition the target will use

For Edition 2024, support for `<build-target>.proc_macro` and `<build-target>.crate_type`
was removed based on `package.edition` and not `<build-target>.edition`.

By having `package.edition` affect `<build-target>.edition`,
we are effectively saying that `package.edition` affects the manifest format
while `<build-target>.edition` affects only affects the source code of the build-target.

## Prior art
[prior-art]: #prior-art

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- When will Cargo start to report the deprecation message?
  - Cargo currently lacks lint control for itself ([cargo#12235](https://github.com/rust-lang/cargo/issues/12235)) which we could wait for
  - We could unconditionally report the warning but the Cargo team avoids adding warnings without a way of suppressing them without changing behavior

## Future possibilities
[future-possibilities]: #future-possibilities

- Reporting the Edition to `build.rs`
