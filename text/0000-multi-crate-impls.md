- Feature Name: coupled_workspaces
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This is a way to more tightly couple crates within a cargo workspace, allowing
inherent impls and blanket trait impls across multiple crates in the same
project. Its primary use case is for the standard library, but the feature is
extended so that it can be used by the entire crate ecosystem.

# Motivation
[motivation]: #motivation

Many projects have grown large enough to split their contents across multiple
crates, so that users of these crates can decide which parts they want to use
and which ones they don't. Notable examples on crates.io include the `num` and
`unic` crates, but the standard library is also split into several crates.

Unfortunately, this brings up a problem: inherent methods must be contained
within the crate that defines a type. This causes a lot of friction when
multiple crates build upon the functionality of each other, like the mess
between `std_unicode`, `alloc`, and `core`, which all provide varying levels of
functionality for `str` and `char`.

While a solution for the standard library is necessary, the ecosystem could
benefit from this functionality as well, and extending the concept of cargo
workspaces to include this use case seems natural.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC won't re-explain Cargo's workspace functionality, which is already
implemented and documented. However, a few particularly notable changes arise:

1. `pub(crate)` definitions become public to all crates in a workspace.
2. Traits and types are, for all intents and purposes, considered "in the same
   crate" as all crates in their workspace. This means that inherent impls are
   allowed across crates, and a trait can be implemented for a type as long as
   the type or the trait are in the same workspace, not just the same crate.
3. Workspaces which use virtual manifests must opt into this feature by
   specifying `workspace.root` in `Cargo.toml`.
4. Workspaces can opt out of this with `workspace.coupled = false` in
   `Cargo.toml` and opt in with `workspace.coupled = true`.
5. Coupled workspaces must be published to crates.io all at once.

For example, take the previously mentioned example of `core`, `alloc`,
`std_unicode`, and `std`. These crates are already organised in a workspace.
Here are a few changes we could make after this RFC is implemented:

1. `core` could directly implement methods on slices, `str`, and `char` rather
   than including the traits `SliceExt`, `StrExt`, and `CharExt` in the core
   prelude.
2. `core` could implement the methods on `f64` and `f32` that don't require
   hardware float support, which are currently inaccessible with `no_std`.
3. `alloc` could implement the inherent methods on slices and `str` like
   `into_boxed_str` and `to_vec`.
4. `std_unicode` could implement all Unicode-specifc functionality on `char` and
   `str`, like `to_lowercase`.
5. `HashMap` could be included in `alloc`, with the methods specific to
   `RandomState` included in `std`.
6. The `Error` trait could be moved to `core`, and the `Box`-specific
   implementations could be moved to `alloc`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature will be done by two attributes, which will be injected into
crates automatically by cargo: `#[workspace_members]` and `#[workspace_root]`.
The terms "root crate" and "member crate" will be used to specifically refer to
the crates at the root of a workspace, and those which are not.

Workspaces which are managed by virtual manifests will *not* inject these
attributes. To still allow this functionality through virtual manifests, a new
`root` field can be added to the `[workspace]` section of `Cargo.toml` to
indicate which crate is the root crate. Additionally, the field
`workspace.coupled` will be added to enable/disable this behaviour, defaulting
to `false`.

Workspaces will also be transitive; if X is in the workspace of Y, and Y is in
the workspace of Z, then X is in the workspace of Z. For example, because
`clippy` is in `core`'s workspace and `clippy_lints` is in `clippy`'s
workspace, `clippy_lints` is in `core`'s workspace.

Dependent crates will annotate their root crate like so:

```
#![workspace_root = "root"]
```

The root crate will list its members by name, like so:

```
#![workspace_members = "crate1, crate2, ..."]
```

A compile error will result if the following criteria are met:

1. A crate has a `workspace_root` attribute.
2. Either:
    a. The `workspace_root` crate is not linked to this crate.
    b. The `workspace_root` crate does not have a `workspace_members` attribute.
    c. The `workspace_root` crate does not include this crate in its members.

While the versions of the crates won't be required for `rustc`, `cargo` will
specifically keep track of the versions of the crates in a workspace and publish
them all at once, to ensure that what's on crates.io is entirely usable. This
would not be the case for crates which opt out of this functionality; those can
be published separately like the current behaviour.

# Drawbacks
[drawbacks]: #drawbacks

This would require extra work one the compiler to ensure that crates are linked
together properly, and is not as trivial as it seems upfront. Additionally,
substantial work to cargo and crates.io would have to be done to avoid the
potential of combining crates that aren't in the same workspace.

This may not be worth it for the ecosystem at large, as the largest benefit
comes from applying this to the standard library. Building this feature properly
will take time, whereas applying it to the standard library could potentially
done much faster.

# Rationale and alternatives
[alternatives]: #alternatives

This seems the best design of this feature, considering how cargo workspaces are
already implemented and working well, and this doesn't add too much on top of
that. Additionally, the two attributes added are probably the simplest way to
implement this feature.

Potentially, a permanently unstable `#![rustc_std_crate]` attribute could be
used to provide the same functionality specifically within the standard library
if the feature is not desired for the larger ecosystem.

The `pub(crate) => pub(workspace)` change may not be ideal, and potentially
adding a specific `pub(workspace)` visibility may be more ideal.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should this functionality be enabled by default?
* Is this functionality necessary beyond the standard library?
* Should `pub(crate)` become `pub(workspace)` or should a separate syntax be
  chosen?
