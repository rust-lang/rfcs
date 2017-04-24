- Feature Name: profile_features
- Start Date: 22-03-2017
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Support profile-specific overrides for the `default-features` key.

# Motivation
[motivation]: #motivation

Sometimes, users wish to control the build of their crates in a manner which
is dependent on which profile it is being built for. Today, this is not easy to
achieve directly; what users tend to do is use a `cfg` option that *usually*
corresponds to the profile being run in, suhc as using `debug_assertions` to
get a certain behavior in dev that is not desired in release.

For example, a user writing a web service may wish to load their HTML templates
dynamically during `dev`, so that they can edit the templates without
recompiling their application. But for both performance and ease of deployment,
they would want their HTML templates to be compiled into the data section of
the binary when they perform a `release` build.

The system today has several problems. First, it is very ad hoc, and couples
what should be independent "feature" based behavior to an unrelated feature
(debug_assertions). Second, if a library chooses to use a flag like this to
control which features they provide, it is difficult for downstream users to
opt out of that behavior (e.g. because they want dev and release to compile the
same code).

Instead we propose that users should adopt standard `features` for behaviors
like this. In order to make this easier to do, users will be able to specify
the `default-features` separately in each profile, causing certain features to
be turned on in one profile and not in others.

# Detailed design
[design]: #detailed-design

## Specifying profile features for your package

Each profile gains the member `default-features`, which has the same structure
as the `features.default` member (that is, it is an array of strings, which
must be feature or dependency names). When preparing a build, if the active
profile has a `default-features` key present, cargo will use that set of
features instead of the `features.default` key.

For example, you might write:

```toml
[features]
dynamic-templates = []

[profile.dev]
default-features = ["dynamic-templates"]
```

The dynamic-templates feature would be on by default, but only in the dev
profile.

## Controlling profile features from dependencies

Both the `features` and `default-features` members of a dependency object can
be TOML objects as alternative to their current form. As TOML objects, their
members are the names of profiles as well as the key `other`; the value at
each of those keys is the same as what the `features` or `default-features`
keys would otherwise contain (an array of features or a boolean respectively).

In a particular profile, if these items are objects, that profile's key is used
to specify this value. If no key is present for this profile, the `other` key
is used.

For example:

```toml
[dependencies.foobar]
version = "1.2.0"

[dependencies.foobar.default-features]
dev = false
test = false
other = true

[dependencies.foobar.features]
dev = ["alpha", "beta"]
test = ["alpha"]
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

None known.
