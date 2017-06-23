- Feature Name: nightly_only_crates
- Start Date: 2017-06-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

The goal of this RFC is to improve the usability of working with crates that require a nightly compiler due to their use of unstable language features.
This will be achieved by enhancing Cargo and crates.io to account for crates that use unstable features and to surface this information to users through visual indications on crates.io.

# Motivation
[motivation]: #motivation

Rust has the useful but unusual mechanism of allowing users to opt-in to unstable language features when using a nightly compiler.
This is beneficial for the project because it allows the community to experiment with ideas and ensure the stability of new features before final release.
Rust also uses Semantic Versioning (SemVer) to communicate stability information about crates to users.
Unfortunately, SemVer's version numbers do not offer a way to indicate that a crate uses unstable language features and hence requires a nightly compiler to use.
The version number only provides information about the program's public API, not about its build requirements.
Because this information is not surfaced anywhere except in the crate's source code, it is difficult for users to know when a crate they want to use will be compatible with projects that prefer to use (or must use) a stable or beta compiler.

What happens currently is that users assume that crates published to crates.io work with the stable compiler, and only find out this is not the case after adding it to a project, attempting to compile it, and getting an error message telling them a nightly compiler is needed.
The result is frustration for the user, and perpetuation of the view that Rust development is unpleasant or impossible on a stable compiler.
While we don't want to discourage the use of unstable features, because of the benefits it brings to the improvement of Rust, we want to make it easier for users to know which crates have this "hidden" requirement of a nightly compiler.
Making this information more visible will also be useful for the community to get a better sense of just how much of the crate ecosystem is nightly-only.

Improving the user experience in finding and selecting high quality, stable crates is in line with Rust's 2017 roadmap: https://github.com/rust-lang/rust-roadmap/issues/9

# Detailed design
[design]: #detailed-design

The enhancements proposed are twofold, but straightforward:

## New Cargo attribute

A new attribute will be added for build targets in Cargo manifests:

``` toml
# [lib] or [[bin]]
nightly = true
```

This is optional and defaults to `false`, but is automatically inferred to be `true` if the target's source code enables any unstable features through use of the `#![feature]` attribute.

Build targets who link to a target for which `nightly = true` has been inferred are themselves defaulted to `nightly = true`.
In other words, if crate A depends on crate B, and crate B is `nightly = true`, then crate A is `nightly = true`.

In essence, anything that uses unstable features is marked as requiring a nightly compiler, and anything that depends on such a target is as well, all the way up the dependency chain.
The result of these rules for default values is that users work the same as they do today, and never _need_ to manually update their Cargo manifest files.

Users may choose, however, to explicitly set `nightly = false` in the Cargo manifest.
This acts as a safeguard and communicates the user's intent to disallow dependencies on nightly-only crates, even if the user happens to be using a nightly compiler that would otherwise allow it.
If `nightly = false` is explicitly set, and the crate or a dependent crate uses any unstable features, the following error is produced, even on a nightly compiler:

```
$ cargo build
Error: lib target `my_lib` must support a stable compiler (`nightly = false`), but the lib enables unstable features.
```
## crates.io indicator

Any crate with at least one `nightly = true` target is given a visual indicator next to its version number on crates.io.
The precise appearance of this indicator is not specified by this RFC and should be decided by the designer implementing it.
For the sake of example, it could be a small red "N" which displays "This crate requires a nightly Rust compiler" either in a tooltip or in a legend shown elsewhere on the page.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

The big benefit of these enhancements are that they don't really _have_ to be taught.
Users browsing crates.io are simply given new information about a crate's requirements along with all the other metadata about the crate.
Because the nightly compiler requirement is inferred by Cargo, users do not need to make any changes to their existing practices.
To address the one case of the explicit `nightly = false` safeguard, the attribute will be documented along with the rest of the Cargo manifest format documentation.

The crates.io documentation should also be updated to explain what the visual indicator means and what the nightly compiler is, since the stable/nightly distinction is something that new users will not be familiar with.
It doesn't need to go into a lot of detail about the nightly compiler, as this is already covered in the Rust book.
For the purposes of consuming a crate, it's only important to understand that a special, unstable version of the compiler is necessary.

# Drawbacks
[drawbacks]: #drawbacks

* Introduces more features to Cargo and crates.io, which need to be built and maintained.
* When Rust is more mature, prevalence of crates that require a nightly compiler may diminish substantially, making this all less of a problem.
* Making it more obvious that some crates require a nightly compiler could potentially harm public perception of Rust's stability, although it would just be surfacing facts that were already there.

# Alternatives
[alternatives]: #alternatives

* Require crates which depend on nightly-only crates but don't use unstable features themselves to explicitly set `nightly = true` to ensure they really intended to restrict their crate to nightly.
  This is essentially the inverse of the explicit `nightly = false` safeguard, and could be considered more intuitive.
  The advantage to that approach is that users would not need to proactively "defend" themselves from an accidental nightly-only requirement.
  The disadvantage is that intentionally adding a nightly-only requirement would now always require the extra step of setting this attribute, which would increase friction for the use of unstable features.
* Take these enhancements a step further and have Cargo's dependency resolution take nightly-only restrictions into account.
  For example, displaying an error when trying to resolve the Cargo lock file if there is a nightly-only dependency and the consuming crate has not explicitly opted in to `nightly = true`, or perhaps even failing to "find" a nightly-only dependent crate if the user hasn't opted-in.
  This is likely overkill, and if not, still better left to a separate RFC.
* Use a different mechanism for storing the nightly-only metadata so that a dedicated attribute is not needed, such as adding "nightly" to the crate's keywords.
* Do not make these changes and live with the current state of users only discovering that crates require nightly after trying to use them.
  Perhaps this is not a big enough problem as to warrant changes to Cargo or crates.io.

# Unresolved questions
[unresolved]: #unresolved-questions

* If a Cargo project includes some targets that require nightly and some that don't, is it worth trying to surface this information on crates.io?
  Currently the individual build targets are not listed at all.
* The exact appearance of the visual indicator, as well as exactly where it appears, is left to be determined by the designer(s) implementing it.
