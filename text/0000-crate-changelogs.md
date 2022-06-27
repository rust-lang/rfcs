- Feature Name: crate-changelogs
- Start Date: 2022-06-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

<!-- https://github.com/rust-lang/rfcs/pull/2129/files -->

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
crate users will definitely benefit crates ecosystem and facilitate dependencies updates.

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
changelog = "CHANGELOG.md"
```

There is several requirements for changelogs, if one of them is not
upheld, `cargo publish` and crates.io will refuse to accept such crate:
- Must be written in MarkDown (number of supported markup languages can be
extended in future)
- Must contain headlines (#) or sub-headlines (##) (in the case if changelog
starts with the headline) which start with a correct semantic version,
possibly inside square brackets (`0.1.1` or `[0.1.1]`), or contain a correct
semantic version preceded by a case-insensitive `V` (`v0.1.1` or `V0.1.1`).
This control may be disabled with `--allow-outdated-changelog`
  - Rationale: Simple control to ensure basic machine-readability as well as
  proper formatting and up-to-dateness of the changelog
- The version of the first versionned section defined by (sub)-headlines
should be equal to the `version` field in `cargo.toml`
  - Rationale: Safeguard in case the crate's author forgot to update the
  changelog
- Optional: May contain (sub-)headlines without a version when `--allow-unversionned-changelog-head` is provided
  - Rationale: Accomodate "Unreleased" sections in changelogs as well as 
  introductory sections of which the aim is to explain the notations used after

Everything in the sections defined by (sub)-headlines will be
treated as changes which were made in the respective versions.

Although it's recommended to write changelogs by hand, you can use other
tools such as [`clog`](https://crates.io/crates/clog) to generate it based on
the structured commit history.
Additionally you can include migration notes, which will help crate users to
upgrade to the new version.

If `changelog` field is specified in the `Cargo.toml`, `cargo publish` will
check if changelog contains section for the new version producing error
otherwise. Changelog content controls can be disabled with `--no-changelog`.

### Changelog examples

```markdown
# Anything goes here

Anything goes here.

## 1.2.3 - 2022-01-01

Anything goes here.

### Anything goes here.

It's still part of the changes made in 1.2.3

#### And this too

Anything goes here.

## 0.2.3 - 2022-04-04

Anything goes here.
```

```markdown
# 1.0.0 My crate - First official release

Anything goes here.

# [0.2.0] My crate - API redesign release

Anything goes here.

# My crate v0.1.3 - Public alpha release

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

Such a changelog file should be saved as `CHANGELOG.md` or `CHANGELOG.X.md`, X
the lowest number available so as to not replace an already-included file,
in the published crate. The changelog's path in the generated cargo.toml
should be updated accordingly and the original path set as `package.metadata.changelog`.

All the rules described in the previous section will apply to changelogs
provided this way.

If an error occurs while fetching the file (status code is neither 200 nor 30x),
`cargo publish`/`cargo package` will return an error. This behaviour can be disabled by
`--no-changelog` option, in which case the changelog field's value will
simply be moved to package.metadata.changelog if it is an url without additionnal
processing (the original `package.changelog` attribute will be empty in the
generated cargo.toml).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementation of described features is fairly straightforward.

Rendering on crates.io can be done through additional link on top bar:
![1](https://user-images.githubusercontent.com/329626/29746037-86920e8a-8ad5-11e7-828c-4d32f6ac4cf2.png)

It will lead to the changelog section on crates.io with the rendered
changelog. If `package.metadata.changelog` is set, the changelog's page
and/or tab will feature a link to the external source provided in the field
for up-to-date/after-release information (i.e. forgotten changes in the
changelog at release-time).

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

# Rationale and Alternatives
[alternatives]: #alternatives

This proposal defines minimal format for changelog, leaving as much flexibilty
for crate authors as possible while keeping all relevant information within
the packaged crate for offline use.

Alternative would be to specify stricter or more relaxed conventions around
changelog content or keeping on relying on links in README or manual searches
within a crate's files. 

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we allow empty "unreleased" section in the changelog, so authors
will not delete and recreate it every time a release is made.

