- Feature Name: `custom_named_cargo_profiles`
- Start Date: 2019-04-04
- RFC PR: [rust-lang/rfcs#2678](https://github.com/rust-lang/rfcs/pull/2678)
- Cargo Issue: [rust-lang/cargo#6988](https://github.com/rust-lang/cargo/issues/6988)

## Summary
[summary]: #summary

The proposed change to Cargo is to add the ability to specify user-defined
profiles in addition to the five predefined profiles, `dev`, `release`, `test`,
`bench`. It is also desired in this scope to reduce confusion regarding where
final outputs reside, and increase the flexibility to specify the user-defined
profile attributes.

## Motivation
[motivation]: #motivation

Past proposal to increase flexibility of Cargo’s build flags for crates within
a single cargo build invocation, has resulted in [RFC 2282](https://github.com/rust-lang/rfcs/blob/master/text/2282-profile-dependencies.md),
which adds the flexibility of changing attributes of specific crates under one
of the default profiles. However, it does not allow for a full custom profile
name definition that can have its own additional final outputs.

The motivation is illustrated by a prominent example — the ability to easily
throw everything under a custom compilation mode in addition to the existing
compilation modes.

For example, suppose we are frequently comparing between both a release build
and a super-optimized release+LTO build, we would like Cargo to having two
separate `target/` directories, e.g. `target/release`, and
`target/release-lto`, for which the binaries and incremental compilation is
managed separately. This is so that we can easily switch between the two modes
without penalty.

Here's an example for a real-world user: [tikv/issue/4189](https://github.com/tikv/tikv/issues/4189)

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

With this proposal implemented, a user can define custom profiles under new
names, provided that an `inherits` key is used in order to receive attributes
from other profiles.

For example:

    [profile.release-lto]
    inherits = "release"
    lto = true

Valid profile names are: must not be empty, use only alphanumeric characters or
`-` or `_`.

Passing `--profile` with the profile's name to various Cargo commands will
resolve to the custom profile. Overrides specified in the profiles from which
the custom profile inherits will be inherited too, and all final outputs may
go to a different directory by default:

    $ cargo build
    $ cargo build --release
    $ cargo build --profile release-lto
    $ ls -l target
    debug release release-lto

Cargo will emit errors in case `inherits` loops are detected. When considering
inheritance hierarchy, all profiles directly or indirectly inherit from
either from `release` or from `dev`.

This also affects other Cargo commands:

* `cargo test` also receives `--profile`, but unless it is specified, uses
  the predefined `test` profile which is described below.
* `cargo bench` also receives `--profile`, but unless it is specified, uses
  the predefined `bench` profile which is described below.

### Effect over the use of profile in commands

The mixtures of profiles used for `--all-targets` is still in effect, as
long as `--profile` is not specified.

### Combined specification with `--release`

For now, `--release` is supported for backward-compatibility.

Using `--profile` and `--release` together in the same invocation emits an
error unless `--profile=release`.  Using `--release` on its own is equivalent
to specifying `--profile=release`

### New `dir-name` attribute

Some of the paths generated under `target/` have resulted in a de-facto "build
protocol", where `cargo` is invoked as a part of a larger project build. So, to
preserve the existing behavior, there is also a new attribute `dir-name`, which
when left unspecified, defaults to the name of the profile. For example:

    [profile.release-lto]
    inherits = "release"
	dir-name = "lto"  # Emits to target/lto instead of target/release-lto
    lto = true

* The `dir-name` attribute is used mainly to direct the outputs of `bench` and
  `test` to their respective directories: `target/release` and `target/debug`.
  This preserves existing behavior.
* The `dir-name` attribute is the only attribute not passed by inheritance.
* Valid directory names are: must not be empty, use only alphanumeric
  characters or `-` or `_`.

### Cross compilation

Under cross compilation with a profile, paths corresponding to
`target/<platform-triple>/<dir-name>` will be created.

### Treatment to the pre-defined profiles

* The `release` profile remains as it is, with settings overridable as
  before.
* The `dev` profile receives the `dir-name = "debug"` attribute, so that its
  final outputs are emitted to `target/debug`, as existing Rust developers will
  expect this behavior. This should be added in the official
  documentation for the Cargo manifest, to make this fact clearer for users.
* A `debug` profile name is not allowed, with a warning saying to use the
  already established `dev` name.
* The `bench` profile defaults to the following definition, to preserve current
  behavior:

```
[profile.bench]
inherits = "release"
dir-name = "release"
```

* The `test` profile defaults to the following definition, to preserve current behavior:
```
[profile.test]
inherits = "dev"
dir-name = "debug"
```

* The (upcoming) `build` profile defaults to the following definition:

```
[profile.build]
inherits = "dev"
dir-name = "build"
debug = false
```

(NOTE: the `build` profile is experimental and may be removed later)

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The 'final outputs' phrasing was used in this RFC, knowing that there are
intermediate outputs that live under `target/` that are usually not a concern
for most Cargo users. The paths that constitute the final build outputs however,
constitute as sort of a protocol for invokers of Cargo. This RFC extends on
that protocol, allowing for outputs in more directories.

### Cargo code changes

In implementation details, there are various hand-coded references to pre-defined
profiles, that we would like to remove.

The `BuildConfig` structure currently has a `release` boolean. The
implementation will replace it with a value of type `enum Profile {Dev,
Release, Custom(String))`.

* The `Profiles` struct in `cargo/core/profiles.rs` currently has hardcoded
  `dev`, `release`, `test`, `bench`. This should be changed into a `BTreeMap`
  based on profile names. The pre-defined profiles can be loaded into it,
  before `TomlProfile` overrides are applied to them.
* Similarly, `TomlProfiles` will be changed to hold profiles in a `BTreeMap`.
* We would need to compute the actual `build_override` for a profile based on
  resolution through the `inherits` key.
* Custom build scripts: For compatibility, the `PROFILE` environment currently
  being passed to the `build.rs` script is set to either `release` or `debug`,
  based on `inherits` relationship of the specified profile, in case it is not
  `release` or `dev` directly.

### Profile name and directory name exclusion

To prevent collisions under the target directory, predefined set of string
excludes both the custom profile names and the dir-name. For example,
`package`, `build`, `debug`, `doc`, and strings that start with `.`.

## Drawbacks
[drawbacks]: #drawbacks

The main drawback is that future ideas regarding Cargo workflows, if
implemented, may supersede the benefits gained from implementing this RFC,
making the added complexity unjustified in retrospect.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Considering the example provided above, there could be other ways to accomplish
the same result.

### Direct `cargo build` flags alternative

If comparing between final build outputs is the main concern to address, there
could be an alternative, in the form of providing those overrides from the
command line.  For example, a `--enable-lto` flag to `cargo build`. Used
together with `CARGO_TARGET_DIR` we would be able to do the following:


	$ cargo build --release
	$ CARGO_TARGET_DIR=target/lto cargo build --release --enable-lto

	$ ls -1 target/release/exe target/lto/release/exe
	target/release/exe target/lto/release/exe

The main drawback for this alternative is invocation complexity, and not being able
to utilize a future implementation of a binary cache under the target directory
(see 'future possibilities').


### Workspace `Cargo.toml` auto-generation

By generating the workspace's `Cargo.toml` from a script, per build, we can
control the parameters of the `release` profile without editing
source-controlled files. Beside build-time complexity, this has another
drawback, for example — it would trip the timestamp comparison with
`Cargo.lock` and cause unnecessary updates to it.

### Cargo workflows

It is unclear when the ideas concerning [Cargo
workflows](http://aturon.github.io/2018/04/05/workflows/) will manifest in
changes that would allow similar functionality.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

* Bikeshedding the `inherits` keyword name.
* Should we keep user profiles under a Toml namespace of their own?

For example:

	[profile.custom.release-lto]
    inherits = "release"
    lto = true

* If so, should the `inherits` keyword be able to refer to custom and
  pre-defined profiles differently?
* Profile names would collide with rustc target names under `target/`. Should
  the output directory be also under a different namespace, e.g.
  `target/custom/release-lto`?
* Do we really need pre-defined profiles for `test`, `bench`, or
  can we make them obsolete?
* Is it worthwhile keeping `test` and `bench` outputs in `target/debug` and
  `target/release`? Doing so would save compilation time and space.
* If so, is the `dir-name` keyword necessary?  Alternatively, we can hand-code
  the output directory of `bench` and `test` to be `release` and `debug` to
  keep the current behavior. This may be sufficient until a "global binary cache"
  feature is implemented, or a per-workspace `target/.cache`
  ([related discussion](https://github.com/rust-lang/cargo/pull/6577#issuecomment-459415283)).

### Existing `--profile` parameters in Cargo

The `check`, `fix` and `rustc` commands receive a profile name via `--profile`.
However these only control how `rustc` is invoked and is not related directly
to the actual Cargo profile whether pre-defined or custom. For example, `cargo rustc`
can receive `--profile bench` and `--release` together or separately, with
rather confusing results. If we move forward with this change, it's maybe
worthwhile to remove this parameter to avoid further confusion, and provide a
similar functionality in a different way.


## Future possibilities
[future-possibilities]: #future-possibilities

This RFC mentions a global binary cache. A global binary cache can reside under
`target/.cache` or in the user home directory under `.cargo`, to be shared by
multiple workspaces. This may further assist in reducing compilation times when
switching between compilation flags.

### Treatment to Cargo's 'Finished' print

Currently, the `Finished` line being emitted when Cargo is done building, is
confusing, and sometimes does not bear a relation to the specified profile. We
may take this opportunity to revise the output of this line to include the name
of the profile.

Some targets use more than one profile in their compilation process, so we may
want to pick a different scheme than simply printing out the name of the main
profile being used. One option is to print a line for each one of the built
targets with concise description of profiles that used to build it, but there
may be better options worth considering following the implementation of this
RFC.
