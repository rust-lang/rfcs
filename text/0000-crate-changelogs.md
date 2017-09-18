- Feature Name: crate-changelogs
- Start Date: 2017-08-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add changelog support to `cargo` and [crates.io](https://crates.io/).

# Motivation
[motivation]: #motivation

Citing [keepachangelog.com](http://keepachangelog.com):

> What is a changelog?
>
>  A changelog is a file which contains a curated, chronologically ordered list of notable changes for each version of a project.
>
> Why keep a changelog?
>
> To make it easier for users and contributors to see precisely what notable changes have been made between each release (or version) of the project.
>
> Who needs a changelog?
>
> People do. Whether consumers or developers, the end users of software are
> human beings who care about what's in the software. When the software changes,
> people want to know why and how.

Encouraging crate authors to keep changelogs and increase its visibility for
crate users will definitely benefit crates ecosystem.

This topic was brought several times, most notable notions are:

- [Rustaceans: Please Keep a Changelog!](https://blog.dbrgn.ch/2015/12/1/rust-crates-keep-a-changelog/) ([reddit](https://www.reddit.com/r/rust/comments/3v1ndl/))
- [Cargo issue](https://github.com/rust-lang/cargo/issues/2188)
- [Recent reddit thread](https://www.reddit.com/r/rust/comments/6vvhjh/)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

There is two ways to add changelog support to your crate: providing changelog
file explicitly or linking to an external resource.

## Explicit changelog file

Add `changelog` field to `[package]` section of your `Cargo.toml` with the
relative path to your changelog file:
```toml
[package]
name = "foo"
version = "0.1.0"
changelog = "CHNAGELOG.md"
```

There is several requirements for changelogs, in case if one of them is not
upheld, `cargo publish` and crates.io will refuse to accept such crate:
- Must be written in MarkDown (number of supported markup languages can be
extended in future)
- Must contain headlines (#) or sub-headlines (##) (in the case if changelog
starts with the headline) which start with the correct semantic version,
possibly inside square brackets (`0.1.1` or `[0.1.1]`)
- As a consequence of the last requirement changelog must not contain
"Unreleased" section
- Optional: Version of first section defined by (sub)-headline should be equal
to the `version` field in the `Cargo.toml`

Last requirement is enabled by default, but can be disabled by executing
`cargo publish --allow-no-changelog`. This is the safeguard in case if crate
author will forget to update changelog for the version which getting
published.

Everything in the sections defined by (sub)-headlines will be
treated as changes which were made in the respective versions.

The good example to follow can be a format described by
[keepachangelog.com](http://keepachangelog.com):
```markdown
# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2017-03-03
### Removed
- Deprecated function `foo`

## [0.1.1] - 2017-02-02
### Deprecated
- Function `foo`

## [0.1.0] - 2017-01-01
### Added
- Functions: `foo`, `bar`
- Documentation for `Zoo`

### Fixed
- Bugs: #1, #2, #3
```

Although it's recommended to write changelogs by hand, you can use other
tools such as [`clog`](https://crates.io/crates/clog) to generate it based on
the structured commit history.

Additionally you can include migration notes, which will help crate users to
upgrade to the new version.

If `changelog` field is specified in the `Cargo.toml`, `cargo publish` will
check if changelog contains section for the new version producing error
otherwise. This behaviour can be disabled with `--allow-no-changelog`
option. But it will not disable other changelog checks.

### Additional changelog examples

```markdown
# Anything goes here

Anything goes here.

## 1.2.3

Anything goes here.

### Anything goes here.

It's still part of the changes made in 1.2.3

#### And this too

Anything goes here.

## 0.2.3

Anything goes here.
```

```markdown
# 1.2.3

Anything goes here.

# 0.2.3

Anything goes here.

# 0.0.3

Anything goes here.
```

```markdown
1.0.0
=====

Anything goes here.

1.0.0-rc1
=========

Anything goes here.

```

## Link to an external resource

Alternatively you can link to a changelog published on external resource. If
`changelog` field will start with `http:` or `https:` it will be treated as an
URL.

```toml
[package]
name = "foo"
version = "0.1.0"
changelog = "https://github.com/foo/bar/releases"
```

None of the rules described in the previous section will apply to changelogs
provided this way, even if they link to markdown files directly.

Before publishing crate `cargo publish` will attempt to check if provided URL is
reachable (e.g by checking if returned status code is equal to 200 or 30x),
returning error if not. This behaviour can be disabled by
`--allow-no-changelog` option.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementation of described features is fairly straightforward.

Rendering on crates.io can be done through additional link on top bar:
![1](https://user-images.githubusercontent.com/329626/29746037-86920e8a-8ad5-11e7-828c-4d32f6ac4cf2.png)

It will lead either to changelog section on crates.io with the rendered
changelog (for explicilty provided changelog file) or to external resource
provided in the `changelog` field.

# Future extensions
[feature-extensions]: #feature-extensions

It will be possible to define additional extensions which will use different
prefixes from `http` and `https` in the `changelog` field.
Such extension(s) can be used for example for handling stricter machine readable
changelog formats which will be generated by external tools like
[`clog`](https://crates.io/crates/clog) or other different approaches to
changelogs.

`changelog` field with such extensions can look like `foo:bar`, where `foo` is
format prefix and `bar` for example external `cargo` tool which will be executed
by `cargo publish` and which should produce result specified by `foo` prefix.

# Drawbacks
[drawbacks]: #drawbacks

- This proposal fixes MarkDown as markup language for changelogs (although
number of supported markup languages can be extended in future)
- It does not define convention for changelogs, just some bare-bone rules. Which
will result in different changelog formats used across ecosystem, thus hindering
machine readability of changelogs and will make it harder to build tools based
on this proposal.
- Some people consider proposed requirements for changelog files unnecessary and
overly strict.

# Rationale and Alternatives
[alternatives]: #alternatives

This proposal defines minimal format for changelog, leaving as much flexibilty
for crate authors as possible without dropping the basic machine readability.

Alternative would be to specify stricter or more relaxed conventions around
changelog content.

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we allow empty "unreleased" section in the changelog, so authors
will not delete and recreate it every time a release is made.
