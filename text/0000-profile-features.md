- Feature Name: profile_features
- Start Date: 22-03-2017
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Support profile-specific overrides for cargo features and dependencies.

# Motivation
[motivation]: #motivation

Sometimes, users wish to control the build of their crates in a manner which
is dependent on which profile it is being built for, because of the differences
in use cases between the profiles. Today, this is not easy to achieve directly,
and instead requires hacking with the existing `cfg` options that roughly
correspond to some of the profiles (such as `test` and `debug_assertions`).

This system is ad hoc and doesn't allow downstream users to effectively control
the way these features are used by their dependencies. To solve this problem,
we allow profiles to play a first class role in feature and dependency
resolution.

# Detailed design
[design]: #detailed-design

Each profile gains these additional members:

* `features`
* `dependencies`
* `dev-dependencies`
* `build-dependencies
* `target`

Each of these corresponds to the same top-level object. When compiling in
a profile, its members under these objects are merged with the top level
equivalent, overriding any overlapping members. This enables users to configure
thier builds differently in each profile if necessary.

Specifically:

* The `features` table for a profile is *merged* with the base `features`
table; each member of the `features` table defined in the profile-specific
table *replaces* the member in the base table (they are not merged together).
* An individual object representing a specific dependency is *merged* with the
base object for that dependency; each member of that object *replaces* the
member in the base table (they are not merged together).

So from this TOML:

```toml
[dependencies.foo]
version = "1.0.0"
features = ["foo"]

[profile.dev.dependencies.foo]
version = ["bar"]
```

In the `dev` profile, the foo dependency looks like:

```toml
[dependencies.foo]
version = "1.0.0"
features = ["bar"] # Note that the foo feature has been dropped
```

## Use cases

### Profile-specific default features

Some features are intended to be used in different profiles - for example, a
feature for testing or for optimizations which is intended for bench and
release. Today, these sorts of things are managed in an ad hoc and imprecise
way through the `test` and `debug_assertions` cfg flags.

Instead, authors can turn features on by default in only some profiles. For
example:

```toml
[features]
default = []
go-fast = []

[profile.release.features]
default = ["go-fast"]

[profile.bench.features]
default = ["go-fast"]
```

### Turning on dependency features in specific profiles

Possibly, the profile-specific feature is not a default feature for this crate,
but a feature the user wants in a dependency, but only in some profiles. This
can be done using by overriding that crate's dependency member in those
profiles.

```toml
[profile.release.dependencies.another-crate]
features = ["go-fast"]

[profile.bench.dependencies.another-crate]
features = ["go-fast"]
```

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This should be documented in the crates.io documentation for the Cargo.toml
manifest format.

# Drawbacks
[drawbacks]: #drawbacks

The primary drawback is that crates which perform conditional compilation on
profiles may not build correctly in one profile. Since most crates are tested
primarily in the `test` profile, this could be a problem.

However, this is already an issue today, because of the way the `test` and
`debug_assertion` cfgs can already be used to conditionally compile code only
in certain profiles. This RFC gives users greater control over this by tying
the conditional compilation to explicit features which can be turned on or off.

# Alternatives
[alternatives]: #alternatives

One alternative would be to add profiles as a unique cfg marker:

```rust
#[cfg(profile = "release")]
```

This would not be as flexible as this RFC - by tying any conditional
compilation to a _feature_, we allow dowstream users to have ultimate control
over which code is compiled in the dependency. In addition to being flexible,
this gives users a way out if a library is broken because of a broken feature
turned on by default in one profile.

# Unresolved questions
[unresolved]: #unresolved-questions

Does this feature overlap in purpose with `dev-dependencies` at all, and if so
is there a way to subsume `dev-dependencies` into it & deprecate that feature?
