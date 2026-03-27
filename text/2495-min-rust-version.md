- Feature Name: `min_rust_version`
- Start Date: 2018-06-28
- RFC PR: [rust-lang/rfcs#2495](https://github.com/rust-lang/rfcs/pull/2495)
- Rust Issue: [rust-lang/rust#65262](https://github.com/rust-lang/rust/issues/65262)

## Summary
[summary]: #summary

Add `rust` field to the package section of `Cargo.toml` which will be used to
specify crate's Minimum Supported Rust Version (MSRV):
```toml
[package]
name = "foo"
version = "0.1.0"
rust = "1.30"
```

## Motivation
[motivation]: #motivation

Currently crates have no way to formally specify MSRV. As a result users can't
check if crate can be built on their toolchain without building it. It also
leads to the debate on how to handle crate version change on bumping MSRV,
conservative approach is to consider such changes as breaking ones, which can
hinder adoption of new features across ecosystem or result in version number
inflation, which makes it harder to keep downstream crates up-to-date. More
relaxed approach on another hand can result in broken crates for user of older
compiler versions.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If you target a specific MSRV add a `rust` field to the `[package]` section of
your `Cargo.toml` with a value equal to the targeted Rust version. If you build
a crate with a dependency which has MSRV higher than the current version of your
toolchain, `cargo` will return a compilation error stating the dependency and
its MSRV. This behavior can be disabled by using `--no-msrv-check` flag.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

During build process (including `run`, `test`, `benchmark`, `verify` and `publish`
sub-commands) `cargo` will check MSRV requirements of all crates in a dependency
tree scheduled to be built or checked. Crates which are part of the dependency
tree, but will not be built are excluded from this check (e.g. target-dependent
or optional crates).

`rust` field should respect the following minimal requirements:
- Value should be a version in semver format **without** range operators. Note
that "1.50" is a valid value and implies "1.50.0".
- Version can not be bigger than a current stable toolchain (it will be checked
by crates.io during crate upload).
- Version can not be smaller than 1.27 (version in which  `package.rust` field
became a warning instead of an error).
- Version can not be smaller than release version of a used edition, i.e.
combination of `rust = "1.27"` and `edition = "2018"` is an invalid one.

## Future work and extensions
[future-work]: #future-work

### Influencing version resolution

The value of `rust` field (explicit or automatically selected by `cargo`) will
be used to select appropriate dependency versions.

For example, let's imagine that your crate depends on crate `foo` with 10 published
versions from `0.1.0` to `0.1.9`, in versions from `0.1.0` to `0.1.5` `rust`
field in the `Cargo.toml` sent to crates.io equals to "1.30" and for others to
"1.40". Now if you'll build your project with e.g. Rust 1.33, `cargo` will select
`foo v0.1.5`. `foo v0.1.9` will be selected only if you'll build your project with
Rust 1.40 or higher. But if you'll try to build your project with Rust 1.29 cargo
will issue an error.

`rust` field value will be checked as well. During crate build `cargo` will check
if all upstream dependencies can be built with the specified MSRV. (i.e. it will
check if there is exists solution for given crates and Rust versions constraints)
Yanked crates will be ignored in this process.

Implementing this functionality hopefully will allow us to close the long-standing
debate regarding whether MSRV bump is a breaking change or not and will allow
crate authors to feel less restrictive about bumping their crate's MSRV. (though
it may be a useful convention for post-1.0 crates to bump minor version on MSRV
bump to allow publishing backports which fix serious issues using patch version)

Note that described MSRV constraints and checks for dependency versions resolution
can be disabled with the `--no-msrv-check` option.

### Checking MSRV during publishing

`cargo publish` will check that upload is done with a toolchain version specified
in the `rust` field. If toolchain version is different, `cargo` will refuse to
upload the crate. It will be a failsafe to prevent uses of incorrect `rust` values
due to unintended MSRV bumps. This check can be disabled by using the existing
`--no-verify` option.

### Making `rust` field mandatory

In future (probably in a next edition) we could make `rust` field mandatory for
a newly uploaded crates. MSRV for older crates will be determined by the `edition`
field. In other words `edition = "2018"` will imply `rust = "1.31"` and
`edition = "2015"` will imply `rust = "1.0"`.

`cargo init` would use the version of the toolchain used.

### `cfg`-based MSRVs

Some crates can have different MSRVs depending on target architecture or enabled
features. In such cases it can be useful to describe how MSRV depends on them,
e.g. in the following way:
```toml
[package]
rust = "1.30"

[target.x86_64-pc-windows-gnu.package]
rust = "1.35"

[target.'cfg(feature = "foo")'.package]
rust = "1.33"
```

All `rust` values in the `target` sections should be equal or bigger to a `rust` value
specified in the `package` section.

If target condition is true, then `cargo ` will use `rust` value from this section.
If several target section conditions are true, then maximum value will be used.

### Nightly and stable versions

Some crates may prefer to target only the most recent stable or nightly toolchain.
In addition to the versions we could allow `stable` and `nightly` values to declare
that maintainers do not track MSRV for the crate.

For some bleeding-edge crates which experience frequent breaks on Nightly updates
(e.g. `rocket`) it can be useful to specify exact Nightly version(s) on which
crate can be built. One way to achieve this is by using the following syntax:
- auto-select: "nightly" This variant will behave in the same way as "stable", i.e.
it will take a current nightly version and will use it in a "more or equal" constraint.
- single version: "nightly: 2018-01-01" (the main variant)
- enumeration: "nightly: 2018-01-01, 2018-01-15"
- semver-like conditions: "nightly: >=2018-01-01", "nightly: >=2018-01-01, <=2018-01-15",
"nightly: >=2018-01-01, <=2018-01-15, 2018-01-20". (the latter is interpreted as
"(version >= 2018-01-01 && version <= 2018-01-20) || version == 2018-01-20")

Such restrictions can be quite severe, but hopefully this functionality will be
used only by handful of crates.

## Drawbacks
[drawbacks]: #drawbacks

- Declaration of MSRV, even with the checks, does not guarantee that crate
will work correctly on the specified MSRV, only appropriate CI testing can do that.
- More complex dependency versions resolution algorithm.
- MSRV selected by `cargo publish` with `rust = "stable"` can be too
conservative.

## Alternatives
[alternatives]: #alternatives

- Automatically calculate MSRV.
- Do nothing and rely on [LTS releases](https://github.com/rust-lang/rfcs/pull/2483)
for bumping crate MSRVs.
- Allow version and path based `cfg` attributes as proposed in [RFC 2523](https://github.com/rust-lang/rfcs/pull/2523).

## Prior art
[prior-art]: #prior-art

Previous proposals:
- [RFC 1707](https://github.com/rust-lang/rfcs/pull/1707)
- [RFC 1709](https://github.com/rust-lang/rfcs/pull/1709)
- [RFC 1953](https://github.com/rust-lang/rfcs/pull/1953)
- [RFC 2182](https://github.com/rust-lang/rfcs/pull/2182) (arguably this one got off-track)

## Unresolved questions
[unresolved]: #unresolved-questions

- Name bike-shedding: `rust` vs `rustc` vs `min-rust-version`
- Additional checks?
- Better description of versions resolution algorithm.
- How nightly versions will work with "cfg based MSRV"?
