- Feature Name: N/A
- Start Date: 2015-10-14
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Unstable items and methods are considered for resolution even when the
feature isn't available to client code - whether on stable channels or
unstable when the relevant `#![feature(...)]` crate attribute isn't in use.
This particularly causes problems when user extension traits provide methods
that may conflict with newly added methods from the standard library.


# Motivation

In the current Rust 1.4 beta, the `std::io::Read::read_exact` method is provided,
which was previously a common extension method provided by libraries on `crates.io`.
Any downstream crates that were previously using it must use UFCS syntax to invoke
the method now, as `read_exact` is unstable and cannot be used without opting in to
the feature on unstable Rust.

The dilemma is as follows: crates that provide extension methods must rename them
in order to be ergonomic, or otherwise expect downstream crates to use the obtuse
UFCS syntx. They cannot defer to the standard library version until it becomes
stable after a couple release cycles - and once that happens, another adoption
step must be taken by removing the method and suggesting users move to the
`libstd` version.

This is prone to happen any time a common library extension becomes popular
enough for inclusion in the standard library. The language should provide a
smooth migration path for everyone involved.


# Detailed design

This RFC proposes that any unstable items (structs, traits, functions, etc.)
must not be considered as valid candidates for name resolution unless its
related feature gate has been opted in to via the `#![feature(...)]` crate
attribute. Attempts to use unstable features will result in an appropriate
"item not found" error message, along with a suggestion note that it may be
available if they enable the appropriate feature. Stable builds of `rustc`
will use slightly modified wording explaining that the feature isn't available
as is done now.

Note that is not only an issue in stable rust, as long as one expects to retain
the ability to use their code on unstable/nightly as well.


# Drawbacks

Extra complexity in `rustc` for new features is always a drawback. Some might
argue some form of migration will have to be done anyway once the feature
becomes stable. At that time moving to the stable version is a valid upgrade
path, but not currently possible for as long as the feature is unstable, and
so it forces two migration points months apart.


# Alternatives

- Unstable items could instead be considered "weak", where external items
  can override them when there are conflicts, but would otherwise behave as
  they currently do.
- "Fast-track" the offending methods to stability considering that they've
  already had time to mature in the greater `crates.io` ecosystem and have
  proven their use and design.
- Not doing anything. Some may consider renaming, moving to UFCS syntax, or
  standalone methods to be good enough as an interim migration step.


# Unresolved questions

Exactly what type of items are affected? Trait methods are the most obvious.
Structures and traits themselves may also affect those using glob imports,
whether or not they are added to library prelude modules.

Are there concerns or implementation issues here around interactions with
macros and `#[allow_internal_unstable]`?

Should we go one step further and remove any mention of unstable items from
rustdoc's output, except when building unstable docs for nightly? I believe
this topic has come up occasionally in previous discussions.

Shall this also apply to `unstable` trait implementations? Rust currently
has a problem with landing unstable features that add new trait implementations
to existing types, since they will become "insta-stable". We could also change
resolution to avoid considering those traits unless the feature is opted into.
