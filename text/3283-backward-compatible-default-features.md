- Feature Name: `backward_compatible_default_features`
- Start Date: 2022-06-23
- RFC PR: [rust-lang/rfcs#3283](https://github.com/rust-lang/rfcs/pull/3283)
- Rust Issue: None

# Summary
[summary]: #summary

Add feature bases as an alternative to `default-features = false` that allows
adding new default-enabled features for existing functionality.

# Motivation
[motivation]: #motivation

Currently, crates cannot add new default-enabled features for existing
functionality. This is a bad thing, because it means that crates cannot allow
users to opt out of more features, because adding new default-enabled features
for these would break existing users with `default-features = false`.

To fix this, I propose we add the notion of a feature `base`. Users of crates
have to select at least one of its base features. This means that in order to
make another part of a crate optional, people can simply add a new optional
feature and add it to all existing feature bases. Then they can create another
feature base that excludes this new optional feature. The syntax of
`default-features = false` will get replaced by `base = "<feature base>"`, with
the default being `base = "default"`.

In order to introduce this in a way that works with the existing
`default-features = false` syntax, `default-features = false` will imply a
feature base, `no-default-features`.

For example, the warp crate enables HTTP2 by default and would like to make
that optional. This was done in a PR, but the backward incompatibility was
noticed (https://github.com/seanmonstar/warp/issues/913) and the PR was
reverted. With this feature proposal, they could instead add a base called
`no-default-features = ["http2"]`, so old crates depending on warp still get
HTTP2 support. People can then opt out of the HTTP2 dependency by specifying a
feature base like `base = "minimal_v1"`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When wanting to give users of your crate the possibility to opt out of parts of
the functionality, you can turn to feature bases: Say you have a dependency on
`large_crate`:

```toml
[dependencies]
large_crate = "3.4"
```

You want to make that dependency optional, but since this would take away parts
of the functionality of your crate as well, existing users must continue to use
your crate with that dependency. The solution is to create a new feature base:

```toml
[package]
feature-bases = ["minimal_v1"] # "default" is automatically a feature base

[dependencies]
large_crate = { version = "3.4", optional = true }

[features]
default = ["large_crate"]
minimal_v1 = []
```

Users of your crate can now opt out of the large crate by specifying the
following in their `Cargo.toml`:

```toml
[dependencies]
your_crate = { version = "5.7", base = "minimal_v1" }
```

## Migration from default-features
[migration-from-default-features]: #migration-from-default-features

The `default-features = false` feature allowed to do the same migration, but
only once per major version of a crate. To be backward-compatible with that
approach, `default-features = false` implies the feature base called
`default-features-false`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A new key `package.feature-bases` is introduced. This key specifies a list of
features that, augmented with `default` and `default-features-false`, are
eligible as to be specified in the `base` key of a dependency.

A new key `base` is introduced for dependencies, its value is a string. It is
well-formed if it is in the above-mentioned set of possible bases.

The keys `default-features` and `base` on dependencies are mutually exclusive;
`default-features = false` implies `base = "default-features-false"`.

# Drawbacks
[drawbacks]: #drawbacks

This specifies one way to solve this problem. It is kind of clear that this
problem should be solved, the only question is whether this is the right or
best solution to the problem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There is another proposal to fix this problem in [RFC 3146: Cargo feature
migrations](https://github.com/rust-lang/rfcs/pull/3146).

One upside of this over RFC 3146 is that 

Not doing this lets problems such as the one in
https://github.com/rust-lang/rfcs/pull/3140#issuecomment-862020840 continue
happening, not allowing crate authors (or even the standard library) to create
new default-enabled features for existing functionality:

> The general concept of `infallible_allocation` seems sensible. However, I
> want to point out a forward-compatibility concern with the proposed way of
> handling this in cargo, which has also applied to previous proposals that
> suggest the same thing:

> If crates start declaring dependencies on `std` with `default-features =
> false`, they'll break if we introduce a new enabled-by-default feature in the
> future.  We'd likely find that we could never add a new default feature.

> I think, instead, we'd need to have a syntax to say "default minus
> `infallible_allocations`", which would be forwards-compatible.

# Prior art
[prior-art]: #prior-art

Slint has done a hack that is similar to this proposal:
https://slint-ui.com/blog/rust-adding-default-cargo-feature.html.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should feature bases instead be treated like normal features entirely, with
  the only difference being that cargo enforces to use at least one of them?

# Future possibilities
[future-possibilities]: #future-possibilities

No future possibilities were thought of.
