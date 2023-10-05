- Feature Name: `rustdoc_types_maintainers`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

The [rustdoc-types](https://crates.io/crates/rustdoc-types) crate will go from being privately maintained to being officially maintained by the rustdoc team.

# Motivation
[motivation]: #motivation

[`rustdoc-types`](https://crates.io/crates/rustdoc-types) is a crate published on crates.io. It is used by users of the unstable [rustdoc JSON](https://github.com/rust-lang/rust/issues/76578) backend to provided a type representing the output of `rustdoc --output-format json`. 


Currently I ([`@aDotInTheVoid`](https://github.com/aDotInTheVoid/)) maintain the `rustdoc-types` crate on [my personal github](https://github.com/aDotInTheVoid/rustdoc-types/). No-one else has either github or crates.io permissions. This means if I become unable (or more likely disinterested), the crate will not see updates.

Additionally, if an update to `rustdoc-json-types` happens while i'm away from a computer for an extended period of time, their will be a delay in this update being published on crates.io. [This almost happened](https://github.com/aDotInTheVoid/rustdoc-types/issues/25), but was avoided due to the bors queue being quiet at the time.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This involves:

1. Moving the [github.com/aDotInTheVoid/rustdoc-types](https://github.com/aDotInTheVoid/rustdoc-types/) repo to the `rust-lang` organization, and make `rust-lang/rustdoc` maintainers/owners.
2. Move overship of `rustdoc-types` on crates.io to the rustdoc team.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`rustdoc-types` is a republishing of the in-tree [`rustdoc-json-types`](https://github.com/rust-lang/rust/tree/b8536c1aa1973dd2438841815b1eeec129480e45/src/rustdoc-json-types) crate. `rustdoc-json-types` is a dependency of `librustdoc`, and is the canonical source of truth for the canonical description of the rustdoc-json output format. Changes to the format are made a a PR to `rust-lang/rust`, and will modify `src/rustdoc-json-types`, `src/librustdoc/json` and `tests/rustdoc-json`. None of this will change.

However, the publishing of this to crates.io, so that it can be used by out-of-tree tools that take rustdoc-json as an input
([eg](https://github.com/awslabs/cargo-check-external-types/blob/dc15c5ee7674a495d807481402fee46fdbdbb140/Cargo.toml#L16),
[eg](https://github.com/Enselic/cargo-public-api/blob/19f15ce4146835691d489ec9db3518e021b638e8/public-api/Cargo.toml#L27),
[eg](https://github.com/obi1kenobi/trustfall-rustdoc-adapter/blob/92cbbf9bc6c9dfaf40bba8adfbc56c0bb7aff12f/Cargo.toml#L15)). This is done with [a scipt](https://github.com/aDotInTheVoid/rustdoc-types/blob/577a774c2433beda669271102a201910c4169c0c/update.sh) so that it is as low maintence as possible. This also ensures that all format/documentation changes happen in the rust-lang/rust repo, and go through the normal review process there.

The update/publishing process will be moved to T-Rustdoc. In the medium term, I (`@aDotInTheVoid`) will still do it, but
- In an offical capacity
- With bus factor for when I stop.

## Actual Mechanics of the move

### Github

Github has a [list of requirements](https://docs.github.com/en/repositories/creating-and-managing-repositories/transferring-a-repository) for transfering repositories.


- When you transfer a repository that you own to another personal account, the new owner will receive a confirmation email. The confirmation email includes instructions for accepting the transfer. If the new owner doesn't accept the transfer within one day, the invitation will expire.
   - N/A: Not transfering to 
- To transfer a repository that you own to an organization, you must have permission to create a repository in the target organization.
   - I (`@aDotInTheVoid`) don't have create-repo perms in the `rust-lang` org. Therefor I'll add a member of T-Infra as an owner to `aDotInTheVoid/rustdoc-types` (I can't add teams, as it's not in an org). They'll then move it to the repo to the `rust-lang` org. Once moved, T-Infra can become owners.
- The target account must not have a repository with the same name, or a fork in the same network.
   - OK.
- The original owner of the repository is added as a collaborator on the transferred repository. Other collaborators to the transferred repository remain intact.
   - OK. After the transfer. T-Rustdoc should be added as a colaborator, and I should be removed so that I only have permissions via my membership in T-Rustdoc. 
- Single repositories forked from a private upstream network cannot be transferred.
   - OK.

At the end of this we should have a crate in the rust-lang github org with T-Rustdoc as contributors, and T-infra as owners.

### crates.io

crats.io ownership is managed [via the command line](https://doc.rust-lang.org/cargo/reference/publishing.html#cargo-owner).

I will run the following commands to move ownership.

```
cargo owner --add github:rust-lang:owners
cargo owner --add rust-lang-owner
cargo owner --remove aDotInTheVoid
```

The `rust-lang-owner` is neaded because team owners cannot add new owners. 

# Drawbacks
[drawbacks]: #drawbacks

- Adds additional maintenence burden to rustdoc team.
- One-time maintenece burden to infra team to support move.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could keep `rustdoc-types` as a personal project. This preserves the status quo (and is what will happen if this RFC (or something similar) isn't addopted). This is undesirable because
      - Bus factor: If I am unable or unwilling to maintain `rustdoc-types`, we cause a load of unnessessary churn when it becomes out of sync with 
- We could bundle `rustdoc-types` through rustup. This is undesirable as it means users can't depend on it in stable rust, and can't depend on multiple versions.
- We could publish `rustdoc-json-types` directly from `rust-lang/rust`. However
   - `rust-lang/rust` doesn't currently publish to crates.io.
   - `rustdoc-json-types` doesn't currently bump the version field in cargo.toml
   - It may be desirable to use different types in rustdoc vs users, eg to have a specialized version of `Id` that doesn't allocate
   - `rustdoc-types` is a nicer name, and what people already depend on.


# Prior art
[prior-art]: #prior-art

- [Rust RFC 3119](https://rust-lang.github.io/rfcs/3119-rust-crate-ownership.html) establishes the Rust crate ownership policy. Under it's categories, `rustdoc-types` would be a **Intentional artifact**
- [Some old zulip discussion about why `rustdoc-json-types` was created.](https://rust-lang.zulipchat.com/#narrow/stream/266220-t-rustdoc/topic/JSON.20Format/near/223685843) What was said then is that if T-Rustdoc want's to publish a crate, it needs to go through an RFC. This is that RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None yet

# Future possibilities
[future-possibilities]: #future-possibilities

When the rustdoc-json feature is stabilized, we'll should release 1.0.0 to crates.io. How we can evolve the format post stabilization is an unanswered question. It's a blocker for stabilization, but not this RFC

