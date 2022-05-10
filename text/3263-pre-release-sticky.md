- Feature Name: `pre-release-sticky`
- Start Date: 2022-05-10
- RFC PR: [rust-lang/rfcs#3263](https://github.com/rust-lang/rfcs/pull/3263)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The following assume:

 - A release is a version without a "pre-release" tag (like `1.0.0`)
 - A pre-release is a version with a "pre-release" tag (like `1.0.0-alpha`)

Cargo will now apply no precedence rule on pre-release by default, this mean that `1.0.0-alpha.0` will be interpreted as `=1.0.0-alpha.0` by cargo instead of `^1.0.0-alpha.0`.

# Motivation
[motivation]: #motivation

The current behavior often break pre-release build, [Semver 2.0](https://semver.org/#spec-item-11) rules for pre-release doesn't include breaking change concept between pre-release. This mean that any new pre-release would be considerate by cargo as a "compatible update". Even worse, final version also match the requierement of any pre-release. Say otherwise, if an user put `version = "1.0.0-alpha.0"` in their `Cargo.toml` this will be considered by cargo as compatible with any pre-release that follow `1.0.0-alpha.1`, `1.0.0-alpha.2`, `1.0.0-beta.0`, etc... and also `1.0.0`, `1.1.0`, etc... (But not `2.0.0`). This can also lead to security problem.

This is an open problem since some time, [cargo #2222](https://github.com/rust-lang/cargo/issues/2222) but have receive some input in Oct 2021, specially in this [forum post](https://internals.rust-lang.org/t/changing-cargo-semver-compatibility-for-pre-releases/14820) where this proposition have been suggested.

Finally, contrary to release version where most user benefits from having cargo pick the latest compatible version as default, there is little sense to do the same for pre-release version cause there are very likely be breaking change between all pre-release version. This is a trap that most user don't expect to happen (why does my project don't compile anymore ?!?). Pre-release version precedence of `Cargo.toml` should default to something more user friendly. User expect bugs from a pre-release but no one want their build suddenly break when a new pre-release is out.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

By default Cargo follow Semver precedence rules for release of a crate but will make no assumption for pre-release.

```toml
[dependencies.foo]
version = "1.0.0-alpha.0"
```

will now be considered by Cargo as:

```toml
[dependencies.foo]
version = "=1.0.0-alpha.0"
```

while release version like:

```toml
[dependencies.foo]
version = "1.0.0"
```

are considered by Cargo as:

```toml
[dependencies.foo]
version = "^1.0.0"
```

The latter doesn't change.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Do it transparently

# Drawbacks
[drawbacks]: #drawbacks

 * This could break workflow
 * It's make pre-release semver rules have a different behavior than release for Cargo.
 * It's a change that affect the whole Rust ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

In practice, do it transparently should not break any heathy workflow, user that put a pre-release version clearly want to use this version and if user have been using a most recent pre-release version not on purpose it's very unlikely their build would break. We could check the impact of implementing this transparently with a [crater](https://github.com/rust-lang/crater) run.

To introduce this change we could also:

  1. Use the `resolver` value version, bump it to `3`:
   
     This may be overkill but it's a naturel way to make change in the way semver rule are trated by Cargo.
     
  2. Use a separate value `pre-release-updates = "sticky" # or "default"`:

     This is less overkill, but have the drawback to include a new value in `Cargo.toml` just for that.

Both these solutions would need a new Rust edition to be introduce by default.

Instead, We could change the rule of Semver for pre-release and say there is no precedence rule for pre-release. `1.0.0-alpha.0` would never match any other requirement that exact same version. That mean that `^1.0.0-alpha.0` could only match `1.0.0-alpha.0` version. This have the major benefit to not introduce inconsistency with pre-release and release in Cargo resolve but this would not follow Semver. This could also be adopted transparently or using opt-in solution.

# Prior art
[prior-art]: #prior-art

Unknown, Cargo behavior to by default use the most compatible version is unique AFAIK most other tool assume `version = "=1.0.0"` for `version = "1.0.0"`. So this problem may be unique to Cargo.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

If we expect other change in the way Cargo resolver work before the next Rust edition we could prefer ship this feature with a more big `resolver = 3` update.

# Future possibilities
[future-possibilities]: #future-possibilities

Introduce a more complex precedence rule for pre-release, for example we could say pre-release tag have its own rule about Semver like:

```none
1.0.0-0.0
1.0.0-0.1 // no breaking change
1.0.0-0.2 // no breaking change
1.0.0-1.0 // breaking change
1.0.0-1.1 // no breaking change
1.0.0-2.0 // breaking change
```

The problem is this is far than being universal. Should be it more complex? Have a patch version? How do we handle alphabetic tag?

While such feature could be nice, I believe this is overkill for pre-release. Pre-release are preview release, there are not mean to be bug free, there are not mean to be used in prod (but there are). Keep them simple should be better, treat them as unique snapshot that will never receive compatible update.
