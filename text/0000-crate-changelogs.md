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

To add changelog support to you crate just add `changelog` field to `[package]`
section of your `Cargo.toml` with the relative path to your changelog file:
```
[package]
name = "foo"
version = "0.1.0"
changelog = "CHNAGELOG.md"
```

There is several requirements for changelogs:
- Must be written in MarkDown
- Must contain headers (#) or sub-headers (##) (in case if file starts with
the header) which start from crate version (`0.1.1` or `[0.1.1]`), except the
first (sub)-header which can contain "Unreleased" changes

Everything in the section defined by (sub)-header will be treated as changes
which were made in the specified version.

The good example to follow can be a format described by [keepachangelog.com](http://keepachangelog.com):
```
# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).
## [Unreleased]
### Added
- Function `bar`

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

If `changelog` field is specified in the `Cargo.toml`, `cargo publish` will
check if changelog contains section for the new version and producing error if
it will not be found. This behaviour can be disabled with `--allow-no-changelog`
option. Also crates.io will refuse to accept crates if changelog file does not
follow rules stated earlier.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Implementation of described features is fairly straightforward.

Rendering on crates.io can be done through additional link on top bar:
![1](https://user-images.githubusercontent.com/329626/29746037-86920e8a-8ad5-11e7-828c-4d32f6ac4cf2.png)

# Drawbacks
[drawbacks]: #drawbacks

- This proposal fixes MD as markup language for changelogs
- It does not define convention for changelogs, just some bare-bone rules. Which
will result in different changelog formats used across ecosystem, thus hindering
machine readability of changelogs and will make it harder to build tools based
on this proposal.

# Rationale and Alternatives
[alternatives]: #alternatives

This proposal defines minimal format for changelog, leaving as much flexibilty
for crate authors as possible without dropping the basic machine readability.

Alternative would be to specify stricter conventions around changelog content.

# Unresolved questions
[unresolved]: #unresolved-questions

- Should we add a badge for changelog?
