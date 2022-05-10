- Feature Name: `precise-pre-release-deps`
- Start Date: 2022-05-10
- RFC PR: [rust-lang/rfcs#3263](https://github.com/rust-lang/rfcs/pull/3263)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Cargo will not use the more compatible version for pre-release by default, this mean that `1.0.0-alpha.0` will be interpreted as `=1.0.0-alpha.0` by cargo instead of `^1.0.0-alpha.0`.

This doesn't require any change in `Cargo.toml`.

# Motivation
[motivation]: #motivation

[Semver 2.0](https://semver.org) have two concepts, compatible version rules, and precedence. Compatible rules apply when you use `^` in front of the version like `^1.0.0` this mean you ask the resolver to take the biggest compatible version of `1.0.0`. Semver, clearly define breaking change is when MAJOR version increase. Precedence define ordering `1.0.0 < 1.1.0 < 1.1.1 < 2.0.0-alpha < 2.0.0`. Here there is a inconstancy in [Cargo doc](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-cratesio), cause `2.0.0-alpha` is not considerate by `1.0.0 >= X < 2.0.0` for compatibility rules by cargo. But `2.0.0-alpha` is clearly between `1.0.0` and `2.0.0`. That mean that we mix up the precedence concept of Semver and the compatible version rules.

Semver rules for pre-release doesn't include breaking change concept between pre-release, it's only specify precedence. Semver rule 9 say "pre-release version indicates that the version is unstable and might not satisfy the intended compatibility requirements as denoted by its associated normal version.". Cargo consider any higher pre-release as a compatible version. Even worse, [VersionReq for prereleases matches release versions](https://github.com/dtolnay/semver/issues/236). Say otherwise, if an user put `version = "1.0.0-alpha.0"` in their `Cargo.toml` this will be considered by cargo as compatible with any pre-release that follow `1.0.0-alpha.1`, `1.0.0-alpha.2`, `1.0.0-beta.0`, etc... and also `1.0.0`, `1.1.0`, etc... (But not `2.0.0`).

The current behavior break pre-release build, here a no exhaustive list:

 - [0.5.0-rc2 break 0.5.0-rc1](https://github.com/SergioBenitez/Rocket/issues/2166)
 - [Dependencies in pre-release should always use fixed version](https://github.com/rust-lang/cargo/issues/9999)
 - [Force cargo to install winit-0.20.0-alpha4](https://github.com/hecrj/wgpu_glyph/pull/31)
 - [Breaking change 0.6.0-alpha4 to 0.6.0](https://github.com/PyO3/pyo3/issues/430)

Pre-release are often bugged and unstable so using a newer pre-release implicitly could lead to security problem.

This is an open problem since some time, [cargo #2222](https://github.com/rust-lang/cargo/issues/2222) but have receive some input in Jun 2021, specially in this [forum post](https://internals.rust-lang.org/t/changing-cargo-semver-compatibility-for-pre-releases/14820) where this proposition have been suggested.

Finally, contrary to release version where most user benefits from having cargo pick the latest compatible version as default, there is little sense to do the same for pre-release version cause there are very likely be breaking change between all pre-release version. This is a trap that most user don't expect to happen (why does my project don't compile anymore ?) and increase the work of maintainers (who want deal with pre-release issue cause they don't compile anymore when make a new pre-release). `Cargo.toml` should default to something more user friendly and less error prone. User expect bugs from a pre-release but no one want their build suddenly break when a new pre-release is out.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The following assumes:

 - A release is a version without a "pre-release" tag (like `1.0.0`)
 - A pre-release is a version with a "pre-release" tag (like `1.0.0-alpha`)

By default Cargo use the more compatible version for release of a crate but will not for pre-release.

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

Do it transparently for the user.

Cargo will need to differentiate how they resolve a pre-release and a release version.

# Drawbacks
[drawbacks]: #drawbacks

 * This could break some build. A code that was using a most recent pre-release version could not compile anymore.
 * It's make Cargo have a different behavior for version field between a pre-release and a release. Making both maintenance of Cargo harder but also could confuse user of Rust.
 * It's a change that affect the whole Rust ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

In practice, do it transparently should not break any heathy workflow, user that put a pre-release version clearly want to use this version and if user have been using a most recent pre-release version not on purpose it's very unlikely their build would break (build would have break when new pre release was out). We could check the impact of implementing this transparently with a [crater](https://github.com/rust-lang/crater) run. If we see that there are unacceptable break, This RFC will then say we will use an opt-in option as follow:

  1. Use the `resolver` value version, bump it to `3`:
   
     This may be overkill but it's a naturel way to make change in the way semver rule are treated by Cargo.
     
  2. Use a separate value `pre-release-updates = "sticky" # or "default"`:

     This is less overkill and can be remove at the next Edition of Rust.

Instead of changing resolver cargo behavior, we could decide that there is no compatible version for pre-release as explaining pre-release having compatible version don't make a lot of sense. So `1.0.0-alpha.0` would never match any other requirement that exact same version. That mean that `^1.0.0-alpha.0` could only match `1.0.0-alpha.0` version. This have the major benefit to not introduce inconsistency with pre-release and release in Cargo resolve. This could also be adopted transparently or using opt-in solution.

The latter alternative could be preferred. Cause it doesn't add complex behavior in cargo resolver, It's will make the maintenance of Cargo simpler. It's also follow the rule 9 of Semver that say pre-release don't have any compatibility requirement. And we teach this by simply say that pre-release don't have any compatible version.

# Prior art
[prior-art]: #prior-art

Unknown, Cargo behavior to by default use the most compatible version is unique AFAIK most other tool assume `version = "=1.0.0"` for `version = "1.0.0"`. So this problem may be unique to Cargo.

[Npm rules](https://docs.npmjs.com/cli/v6/using-npm/semver) follow the same than cargo for compatibility version but npm default to `=` for everything while Cargo default to `^`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

If we expect other change in the way Cargo resolver work before the next Rust edition we could prefer ship this feature with a more big `resolver = 3` update.

It's unclear how an opt-in feature interact with dependencies. For example, `A` depend on a pre-release of `B`, but `B` also depend on a pre-release of `C`. `B` didn't opt-in to this resolver but `A` did. Will Cargo use `A` choice and only use the strictly same version for pre-release of `C`? This could be done but what if `B` break cause it was also using the newer pre-release of `C` without know it? That why I think we should try to introduce this change transparently, we must carefully check for break in the existing ecosystem. There should be very few break if not zero.

# Future possibilities
[future-possibilities]: #future-possibilities

Since Semver doesn't clearly specify compatibility rules for pre-release we could introduce a more complex compatible version rules for pre-release, for example we could say pre-release tag have its own rule about Semver like:

```none
1.0.0-0.0
1.0.0-0.1 // no breaking change
1.0.0-0.2 // no breaking change
1.0.0-1.0 // breaking change
1.0.0-1.1 // no breaking change
1.0.0-2.0 // breaking change
```

The biggest problem is that there is no precedent of such rules. Should be it more complex? Have a patch version? How do we handle alphabetic tag?

While such feature could be nice, I believe this is overkill for pre-release. Pre-release are preview release, there are not mean to be bug free, there are not mean to be used in prod (but there are). Keep them simple should be better, treat them as unique snapshot that will never receive compatible update.
