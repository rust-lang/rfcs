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
`rust="stable"` or `rust="nightly"` depending on the currently used toolcahin.
On `cargo publish` cargo will take  currently used Rust compiler version and
will insert it before uploading the crate. In other words localy your `Cargo.toml`
willl still have `rust="stable"`, but version sent to crates.io will have
`rust="1.30"` if you've used Rust 1.30. This version will be used to determine if
crate can be used with the crate user's toolchain and to select appropriate
dependency versions. In case if you have `rust="stable"`, but execute
`cargo publish` with Nigthly toolcahin you will get an error.

If you are sure that your crate supports older Rust versions (e.g. by using CI
testing) you can change `rust` field accordingly. On `cargo publish` it will be
checked that crate indeed can be built with the specified version. (though this
check can be disabled with `--no-verify` option)

For example, lets imagine that your crate depends on crate `foo` with 10 published
versions from `0.1.0` to `0.1.10`, in versions from `0.1.0` to `0.1.5` `rust`
field in the `Cargo.toml` sent to crates.io equals to "1.30" and for others to
"1.40". Now if you'll build your project with Rust 1.33 `cargo` will select
`foo v0.1.5`, and `foo v0.1.10` if you'll build your project with Rust 1.30 or
later. But if you'll try to build your project with Rust 1.29 cargo will issue an
error. Although this check can be disabled with `--no-rust-check` option.

`rust` field should respect the following minimal requirements:
- value should be equal to "stable", "nigthly" or to a version in semver format
("1.50" is a valid value and implies "1.50.0")
- version should not be bigger than the current stable toolchain
- version should not be smaller than 1.27 (version in which  `package.rust` field
became a warning instead of an error)

`rust` will be a required field. For crates uploaded before introduction of this
feature 2015 edition crates will imply `rust="1.0"` and 2018 ediiton will imply
`rust = "1.30"`.

It will be an error to use `rust="1.27"` and `edition="2018"`, but `rust="1.40"` and `edition="2015"` is a valid combination.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The describe functionality can be introduced in several stages:


## First stage: dumb field

At first the `rust` field can be simply a declarative optional field without any
functionality behind it. The reason for it is to reduce implementation cost of
the first stage to the minimum and ideally ship it as part of Rust 2018.
It will also allow crate authors who care about MSRV to start mark their crates
early.

## Second stage: versions resolution

`rust` field becomes required and cargo will add it as a constraint to dependency
versions resolution. If user uses e.g. Rust 1.40 and uses crate `foo = "0.2"`, but
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

## Third stage: better crate checks

Here we introduce two-level check for crates. First level will check if all used
items were stabilised before or on given MSRV using `#[stable(since=version)]`
attribute, issuing compile errors otherwise.

Second level will try to build crate with the specified MSRV on `cargo publish`,
i.e. words it will be required to install MSRV toolchain. (this check can be
disabled using `--no-verify` option)

While these two checks will not replace proper CI testing, they will help to
reduce number of improper MSRV configuration to the minimum.

Note that `rust` field must be equal to MSRV with default features for all
supported targets.

## Extension: nightly versions

For some bleeding-edge crates which experience frequent breaks on Nightly updates
(e.g. `rocket`) it can be useful to specify exact Nigthly version(s) on which
crate can be built. One way to achieve this is by using the following syntax:
- single version: rust = "nightly: 2018-01-01"
- enumeration: "nightly: 2018-01-01, 2018-01-15"
- (inclusive) range: "nightly: 2018-01-01..2018-01-15"
- enumeration+range: "nightly: 2018-01-01, 2018-01-08..2018-01-15"

Such restrictions can be quite severe, but hopefully this functionality will be
used only by handful of crates.

## Extension: cfg based MSRV

Some crates can have different MSRVs depending on target architecture or enabled
features. In such cases it can be usefull to extend `rust` field, e.g. in the
following way:
```toml
rust = "1.30"
rust-cases = [
    { cfg = "x86_64-pc-windows-gnu", version = "1.35" },
    { cfg = 'cfg(feature = "foo")', version = "1.33" },
]
```

Version resolution will filter all cases with `cfg` equal to true and will take
max `version` value from them as a MSRV. If all `cfg`s are false, value in the
`rust` field will be used.

# Drawbacks
[drawbacks]: #drawbacks

- Declaration of MSRV, even with the checks, does not guarantee that crate
will work correctly on the specified MSRV, only appropriate CI testing can do that.
- More complex dependency versions resolution algorithm.
- MSRV selected by `cargo publish` with `rust = "stable"` can be too
conservative.
- Checking `#[stable(since=version)]` of used items will make compiler more complex.

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
- [RFC 2182](https://github.com/rust-lang/rfcs/pull/2128) (arguably this one got off-track)

# Unresolved questions
[unresolved]: #unresolved-questions

- Name bike-shedding: `rust` vs `rustc` vs `min-rust-version`
- Additional checks?
- Better description of versions resolution algorithm.
