- Feature Name: `rustdoc_types_maintainers`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The [rustdoc-types](https://crates.io/crates/rustdoc-types) crate will go from being individually maintained to being officially maintained by the rustdoc team.

# Motivation
[motivation]: #motivation

[`rustdoc-types`](https://crates.io/crates/rustdoc-types) is a crate published on crates.io. It is used by users of the unstable [rustdoc JSON](https://github.com/rust-lang/rust/issues/76578) backend to provided a type representing the output of `rustdoc --output-format json`.  It's published on crates.io to be used by out-of-tree tools that take rustdoc-json as an input. Eg:.

| Name | Purpose |
|--|--|
| [awslabs/cargo-check-external-types] | Home-rolled version of [RFC 1977] "private dependencies". Checks if any types from the private dependency are used in a crate's public API. |
| [Enselic/cargo-public-api] | Compares the public API of two crates. Used to check for semver violations. |
| [obi1kenobi/trustfall-rustdoc-adapter] | Higher-level database-ish API for querying Rust API's. Used by [obi1kenobi/cargo-semver-checks] |

[awslabs/cargo-check-external-types]: https://github.com/awslabs/cargo-check-external-types/blob/dc15c5ee7674a495d807481402fee46fdbdbb140/Cargo.toml#L16

[Enselic/cargo-public-api]: https://github.com/Enselic/cargo-public-api/blob/19f15ce4146835691d489ec9db3518e021b638e8/public-api/Cargo.toml#L27

[obi1kenobi/trustfall-rustdoc-adapter]: https://github.com/obi1kenobi/trustfall-rustdoc-adapter/blob/92cbbf9bc6c9dfaf40bba8adfbc56c0bb7aff12f/Cargo.toml#L15

[obi1kenobi/cargo-semver-checks]: https://github.com/obi1kenobi/cargo-semver-checks

[RFC 1977]: https://rust-lang.github.io/rfcs/1977-public-private-dependencies.html

Currently I ([`@aDotInTheVoid`](https://github.com/aDotInTheVoid/)) maintain the `rustdoc-types` crate on [my personal github](https://github.com/aDotInTheVoid/rustdoc-types/). No-one else has either github or crates.io permissions. This means if I become unable (or more likely disinterested), the crate will not see updates.

Additionally, if an update to `rustdoc-json-types` happens while I'm away from a computer for an extended period of time, their will be a delay in this update being published on crates.io. This happened with format_version 29, which was merged on [April 8th](https://github.com/rust-lang/rust/commit/537aab7a2e7fe9cdf50b5ff18485e0793cd8db62),
but was only published to crates.io on
[April 19th](https://github.com/aDotInTheVoid/rustdoc-types/commit/ad92b911488dd42681e3dc7e496f777f556a94f6), due to personal reasons.
[This almost happened previously](https://github.com/aDotInTheVoid/rustdoc-types/issues/25), but was avoided due to the bors queue being quiet at the time.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This involves:

1. Moving the [github.com/aDotInTheVoid/rustdoc-types](https://github.com/aDotInTheVoid/rustdoc-types/) repo to the `rust-lang` organization, and make `rust-lang/rustdoc` maintainers/owners.
2. Move ownership of `rustdoc-types` on crates.io to the rustdoc team.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`rustdoc-types` is a republishing of the in-tree [`rustdoc-json-types`](https://github.com/rust-lang/rust/tree/b8536c1aa1973dd2438841815b1eeec129480e45/src/rustdoc-json-types) crate. `rustdoc-json-types` is a dependency of `librustdoc`, and is the canonical source of truth for the rustdoc-json output format. Changes to the format are made a a PR to `rust-lang/rust`, and will modify `src/rustdoc-json-types`, `src/librustdoc/json` and `tests/rustdoc-json`. None of this will change.

Republishing `rustdoc-json-types` as `rustdoc-types` is done with [a script](https://github.com/aDotInTheVoid/rustdoc-types/blob/17cbe9f8f07de954261dbb9536c394381770de7b/update.sh) so that it is as low maintenance as possible. This also ensures that all format/documentation changes happen in the rust-lang/rust repo, and go through the normal review process there.

The update/publishing process will be moved to T-Rustdoc. In the medium term, I (`@aDotInTheVoid`) will still do it, but
- In an official capacity
- With bus factor for when I stop.

We (T-Rustdoc) will continue to publish new version of the `rustdoc-types` crate
every time the upstream implementation changes, and these will be versioned with
normal semver. Changes to rustdoc-json in `rust-lang/rust` will not be accepted
if they would make it not possible to publish `rustdoc-types`.

## Actual Mechanics of the move

### Github

Github has a [list of requirements](https://docs.github.com/en/repositories/creating-and-managing-repositories/transferring-a-repository) for transferring repositories. T-Infra will handle the permissions of moving the repository into the rust-lang github organization.

At the end of this we should have a crate in the rust-lang github org with T-Rustdoc as contributors, and T-infra as owners.

### crates.io

crates.io ownership is managed [via the command line](https://doc.rust-lang.org/cargo/reference/publishing.html#cargo-owner).

I will run the following commands to move ownership.

```
cargo owner --add github:rust-lang:rustdoc
cargo owner --add rust-lang-owner
cargo owner --remove aDotInTheVoid
```

The `rust-lang-owner` is needed because team owners cannot add new owners. 

# Drawbacks
[drawbacks]: #drawbacks

- Adds additional maintenance burden to rustdoc team.
- One-time maintenance burden to infra team to support move.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could keep `rustdoc-types` as a personal project. This preserves the status quo (and is what will happen if this RFC (or something similar) isn't adopted). This is undesirable because
      - Bus factor: If I am unable or unwilling to maintain `rustdoc-types`, we cause a load of unnecessary churn when it becomes out of sync with the in-tree `rustdoc-json-types`
- We could bundle `rustdoc-types` through rustup. This is undesirable as it means users can't depend on it in stable rust, and can't depend on multiple versions.
- We could publish `rustdoc-json-types` directly from `rust-lang/rust`. However
   - `rust-lang/rust` doesn't currently publish to crates.io.
   - `rustdoc-json-types` doesn't currently bump the version field in `Cargo.toml`
   - It may be desirable to one day use different types for rustdoc serialization vs users deserialization

     Reasons for this:
     - It could enable performance optimizations by avoiding allocations into strings
     - It could help with stabilization:
       - Allows making structs/enums `#[non_exhaustive]`
       - Allows potentially supporting multiple format versions.
   - `rustdoc-types` is a nicer name, and what people already depend on.

# Prior art
[prior-art]: #prior-art

- [Rust RFC 3119](https://rust-lang.github.io/rfcs/3119-rust-crate-ownership.html) establishes the Rust crate ownership policy. Under it's categories, `rustdoc-types` would be a **Intentional artifact**
- [Some old zulip discussion about why `rustdoc-json-types` was created.](https://rust-lang.zulipchat.com/#narrow/stream/266220-t-rustdoc/topic/JSON.20Format/near/223685843) What was said then is that if T-Rustdoc wants to publish a crate, it needs to go through an RFC. This is that RFC.
- the [`cargo
  metadata`](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)
  command gives JSON information about a cargo package. The
  [cargo-metadata](https://docs.rs/cargo_metadata/latest/cargo_metadata/) crate
  provides access to this. Instead of being a export of the cargo-side type declarations,
  it's manually written, and not officially maintained. This has [lead to compatibility issues](https://github.com/oli-obk/cargo_metadata/issues/240)
  in the past. Despite being stable, the exact compatibility story [isn't yet determined](https://github.com/rust-lang/cargo/issues/12377).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None yet

# Future possibilities
[future-possibilities]: #future-possibilities

When the rustdoc-json feature is stabilized, we should release 1.0.0 to crates.io. How we can evolve the format post stabilization is an unanswered question. It's a blocker for stabilization, but not this RFC

