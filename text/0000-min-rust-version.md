- Feature Name: min_rust_version
- Start Date: 2018-06-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `rust` field to the package section of `Cargo.toml` which will be used to
specify crate's Minimum Supported Rust Version (MSRV):
```toml
[package]
name = "foo"
version = "0.1.0"
rust = "1.30"
```

# Motivation
[motivation]: #motivation

Currently crates have no way to formally specify MSRV. As a result users can't
check if crate can be built on their toolchain without building it. It also
leads to the debate on how to handle crate version change on bumping MSRV,
conservative approach is to consider such changes as breaking ones, which can
hinder adoption of new features across ecosystem or result in version number
inflation, which makes it harder to keep downstream crates up-to-date. More
relaxed approach on another hand can result in broken crates for user of older
compiler versions.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`cargo init` will automatically create `Cargo.toml` with `rust` field equal to
`rust="stable"` or `rust="nightly: *"` depending on the currently used toolcahin.
On `cargo publish` cargo will take  currently used Rust compiler version and
will insert it before uploading the crate. In other words localy your `Cargo.toml`
willl still have `rust="stable"`, but version sent to crates.io will have
`rust="1.30"` if you've used Rust 1.30. If "nightly: \*" is used, then `cargo`
will not select current Nightly version, but will assume that cratecan be built
with all Nightly versions.

In case if you have `rust="stable"`, but execute `cargo publish` with Nightly
toolcahin you will get an error. Same goes for `rust="nightly: *"` which can be
published only using nightly toolchain.

If you are sure that your crate supports older Rust versions (e.g. by using CI
testing) you can specify this version explicitly, e.g. `rust="1.30"`.
On `cargo publish` it will be checked that crate indeed can be built with the
specified version, i.e. the respective toolchain will have to be installed on
your computer.

By default toolchain check is disabled for `cargo publish`, `cargo check` and
`cargo test`, but it can be enabled with `--check-msrv-toolchain` option.
To disable this check for `cargo publish` you can use `--no-verify`option.

The value of `rust` field (explicit or autmatically selected by `cargo`) will
be used to determine if crate can be used with the crate user's toolchain and
to select appropriate dependency versions.

For example, lets imagine that your crate depends on crate `foo` with 10 published
versions from `0.1.0` to `0.1.9`, in versions from `0.1.0` to `0.1.5` `rust`
field in the `Cargo.toml` sent to crates.io equals to "1.30" and for others to
"1.40". Now if you'll build your project with e.g. Rust 1.33, `cargo` will select
`foo v0.1.5`. `foo v0.1.9` will be selected only if you'll build your project with
Rust 1.40 or higher. But if you'll try to build your project with Rust 1.29 cargo
will issue an error.

Note that described MSRV constraints and checks for dependency versions resolution
can be disabled with `--no-msrv-check` option.

`rust` field should respect the following minimal requirements:
- value should be equal to "stable", "nightly: \*" or to a version in semver format.
Note that "1.50" is a valid value and implies "1.50.0". (also see "nightly versions"
extension)
- version should not be bigger than the current stable toolchain
- version should not be smaller than 1.27 (version in which  `package.rust` field
became a warning instead of an error)

`rust` will be a required field. For crates uploaded before introduction of this
feature 2015 edition crates will imply `rust="1.0"` and 2018 edition will imply
`rust = "1.30"`.

It will be an error to use `rust="1.27"` and `edition="2018"`, but `rust="1.40"`
and `edition="2015"` is a valid combination.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The describe functionality can be introduced in several stages:


## First stage: dumb field

At first the `rust` field can be simply a declarative optional field without any
functionality behind it with minimal checks. The reason for it is to reduce
implementation cost of the first stage to the minimum and ideally ship it as part
of Rust 2018. It will also allow crate authors who care about MSRV to start mark
their crates early.

## Second stage: `cargo publish` check

The next step is for `cargo publish` to require use of the toolchain specified
in the `rust` field, for example crates with:
- `rust="stable"` can be published only with a stable toolchain, though not
necessarily with the latest one. Cargo will insert toolchain version before
publishing the crate as was described in the "guide-level explanation".
- `rust="nightly: *"` can be published only with a nightly toolchain. If finer
grained "nightly: ..." (see "nightly versions" section) is selected, then one
of the selected Nightly versions will have to be used.
- `rust="1.30"` can be published only with (stable) Rust 1.30, even if it's
not the latest stable Rust version.

Using the usual build check `cargo publish` will verify that crate indeed can be
built using specified MSRV. This check can be used with exisiting `--no-verify`
option.

## Third stage: versions resolution

`rust` field will be used as a constraint for dependency versions resolution.
If user uses e.g. Rust 1.40 and uses crate `foo = "0.2"`, but
all selected versions of `foo` specify MSRV e.g. equal 1.41 or bigger (or even
nightly) `cargo` will issue an error.

`rust` field value will be checked as well, on crate build `cargo` will check if
all upstream dependencies can be built with the specified MSRV. (i.e. it will
check if there is exists solution for given crates and Rust versions constraints)

Yanked crates will be ignored in this process.

Implementing this functionality hopefully will allow to close the debate regarding
MSRV handling in crate versions and will allow crate authors to feel less
restrictive about bumping their crate's MSRV. (though it can be a usefull
convention for post-1.0 crates to bump minor version on MSRV change to allow
publishing backports which fix serious issues using patch version)

## Extension: nightly versions

For some bleeding-edge crates which experience frequent breaks on Nightly updates
(e.g. `rocket`) it can be useful to specify exact Nightly version(s) on which
crate can be built. One way to achieve this is by using the following syntax:
- auto-select: "nightly" This variant will behave in the same way as "stable", i.e.
it will take a current nightly version and will use it in a "more or equal" constraint.
- single version: "nightly: 2018-01-01" (tha main variant)
- enumeration: "nightly: 2018-01-01, 2018-01-15"
- semver-like conditions: "nightly: >=2018-01-01", "nightly: >=2018-01-01, <=2018-01-15",
"nightly: >=2018-01-01, <=2018-01-15, 2018-01-20". (the latter is interpreted as
"(version >= 2018-01-01 && version <= 2018-01-20) || version == 2018-01-20")

Such restrictions can be quite severe, but hopefully this functionality will be
used only by handful of crates.

## Extension: cfg based MSRV

Some crates can have different MSRVs depending on target architecture or enabled
features. In such cases it can be usefull to describe how MSRV depends on them,
e.g. in the following way:
```toml
[package]
rust = "1.30"

[target.x86_64-pc-windows-gnu]
rust = "1.35"

[target.'cfg(feature = "foo")']
rust = "1.33"
```

All `rust` values in the `target` sections should be equal or bigger to a `rust` value
specified in the `package` section.

If target condition is true, then `cargo ` will use `rust` value from this section.
If several target section conditions are true, then maximum value will be used.

# Drawbacks
[drawbacks]: #drawbacks

- Declaration of MSRV, even with the checks, does not guarantee that crate
will work correctly on the specified MSRV, only appropriate CI testing can do that.
- More complex dependency versions resolution algorithm.
- MSRV selected by `cargo publish` with `rust = "stable"` can be too
conservative.

# Rationale and Alternatives
[alternatives]: #alternatives

- Automatically calculate MSRV.
- Do nothing and rely on [LTS releases](https://github.com/rust-lang/rfcs/pull/2483)
for bumping crate MSRVs.

# Prior art
[prior-art]: #prior-art

Previous proposals:
- [RFC 1707](https://github.com/rust-lang/rfcs/pull/1707)
- [RFC 1709](https://github.com/rust-lang/rfcs/pull/1709)
- [RFC 1953](https://github.com/rust-lang/rfcs/pull/1953)
- [RFC 2182](https://github.com/rust-lang/rfcs/pull/2182) (arguably this one got off-track)

# Unresolved questions
[unresolved]: #unresolved-questions

- Name bike-shedding: `rust` vs `rustc` vs `min-rust-version`
- Additional checks?
- Better description of versions resolution algorithm.
- How nightly versions will work with "cfg based MSRV"?
