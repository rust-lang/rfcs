- Feature Name: `print_std_support`
- Start Date: 2024-09-12
- RFC PR: [rust-lang/rfcs#3693](https://github.com/rust-lang/rfcs/pull/3693)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce a new argument for `rustc --print`, named `std-support`, that prints "true" if the currently selected target supports `std`, and "false" if it does not. It will get this value from the target specification. The intent is for this to be stabilised before `build-std`.

# Motivation
[motivation]: #motivation

## Build-std

In order for Cargo to support building `std` from source it needs to know more about the target than it currently does. For example, Cargo would like to return an error when attempting to build `std` for a target that does not, or cannot, support `std`. This is currently handled by marking `std` as unstable on certain targets with a filter in `std`'s `build.rs` file but this method often [confuses](https://github.com/rust-lang/wg-cargo-std-aware/issues/87) users. In addition, it only produces a warning **after** building `std`, so any build-time error that occurs while building `std` will lead to further confusion.

Cargo will query `rustc` using `--print std-support`, as it already does so with `--print cfg` and other options, and use that information to return a user-readable error with an explanation. This is implemented in [#14183](https://github.com/rust-lang/cargo/pull/14183) which currently tries to use the target-spec.

## Single source of truth

Currently there are 3 sources of truth for std support:

1. The `rustc` docs on [platform support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
2. `std`'s [`build.rs` file](https://github.com/rust-lang/rust/blob/28e684b47000d4ed4cdb5d982331e5ff8141c1ce/library/std/build.rs#L20) has some target filtering used to mark `std` as unstable
3. `bootstrap` has its own target filtering [here](https://github.com/rust-lang/rust/blob/54be9ad5eb47207d155904f6c912a9526133f75f/src/bootstrap/src/core/config/config.rs#L573)

This proposal will make the `Metadata.std` field in the target specification the single source of truth. Work is currently [ongoing](https://github.com/rust-lang/rust/issues/120745) to generate the target docs using the Metadata part of the target-spec. Additionally, with this feature, `std`'s `build.rs` and `bootstrap` can query `rustc` for std support instead of having their own filtering.

These changes can happen before this feature is stable, though stabilising `build-std` will require stabilising this feature.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust's [`--print` option](https://doc.rust-lang.org/rustc/command-line-arguments.html#--print-print-compiler-information) can be used to query the compiler for information it knows about and supports a variety of stable and unstable arguments. This RFC proposes adding one more, named `std-support`.

The option will print one of two results on the command line.

- `true`
- `false`

The true result means that the current target can support `std` and false means it does not - likely due to the target not including an operating system.

Note that the [Target Tier Policy](https://doc.rust-lang.org/rustc/target-tier-policy.html) applies as usual. A tier 2 or 3 target may attempt to support `std` (and return true accordingly) but the implementation of `std` may be incomplete, incorrect or possibly fail to build due a lack of CI testing.

The overall behaviour of `--print` will not be changed. For example, the output will be printed on its own line in the order any arguments are presented:

```bash
$ rustc --print std-support --target=x86_64-unknown-linux-gnu
true

$ rustc --print std-support --print split-debuginfo --target=aarch64-unknown-none
false
off
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `std-support` flag will print out the value of the `Metadata.std` field in the target-spec-json. This is currently an `Option<bool>` and will be amended to a `bool`, making it mandatory for all targets, as there isn't a sensible default for this flag in the absence of data.

The [Target Tier Policy](https://doc.rust-lang.org/rustc/target-tier-policy.html) mentions how even tier 3 targets "should attempt to implement as much of the standard libraries as possible". Any target that attempts to do so should be marked as supporting std. This was already done in [#127264](https://github.com/rust-lang/rust/pull/127265) which filled out the targets according to the [platform support documentation](https://doc.rust-lang.org/nightly/rustc/platform-support.html).

# Drawbacks
[drawbacks]: #drawbacks

## `--print` bloat

Stabilising the printing of parts of the target-spec JSON has the risk of bloating the `--print` option's argument list as the `build-std` feature may require other parts of the JSON in the future. A longer period of instability could help mitigate this and allow for reassessing the interface when `build-std` is in more of a position to be stabilised.

## Target-spec file inputs

As documented in [this issue](https://github.com/rust-lang/wg-cargo-std-aware/issues/90), the stability of JSON targets and what guarantees they have isn't explicit. Target specification JSON as a file input to `--target` is de facto stable despite the fact that the project [considers them unstable](https://github.com/rust-lang/rust/issues/71009).

As a result of this proposal, whether `std` is explicitly marked as stable or unstable, under `restricted_std`, depends on the JSON target's `Metadata.std` field. Currently Rust isn't clear about the guarantees here.

Note that currently the `restricted_std` mechanism depends on various fields in the target spec such as `vendor` and `os` so this proposal doesn't clearly make the situation better or worse.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Stabilise target-spec-json

This proposed flag exposes part of the target-spec JSON, so stabilising the `--print target-spec-json` could satisfy this flag's use cases. There is [some support](https://github.com/rust-lang/rust/issues/38338) for this already. However, the structure of the JSON itself cannot be stabilised at this point. This makes it difficult for Cargo and other consumers of the JSON to support multiple versions of Rust.

It may be possible to give the JSON some forward-compatibility by making all current fields mandatory and their types unchangeable. This would lock in some decisions made with its structure. Overall it's likely to be more complex and risky compared to stabilising parts of it through extra print options like this RFC proposes.

## Distinguishing unsupported vs incomplete std

This feature does not attempt to distinguish between targets that do not support std and targets that currently support it in an incomplete state. This is represented in the [platform support documentation](https://doc.rust-lang.org/nightly/rustc/platform-support.html) as a `?` state. This state seems to be inconsistent, ill-defined and not particularly useful. It's better to assume that a tier 2 or 3 target possibly has incomplete support.

More nuance is possible - we could introduce a third "incomplete/unknown" status to represent `?`. We could also go into more detail to distinguish between different parts of `std` and detail how, for example, a target supports `std::alloc` and `std::collections` but not `std::fs`.

It's unclear how well either of these solutions map to the reality of targets today and how we would keep these labels in sync with reality in the future, especially for tier 3 targets. For this reason this RFC proposes a simpler boolean approach.

# Prior art
[prior-art]: #prior-art

Related work includes:

- [Platform Support documentation](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [Tracking Issue for changing rustc target docs #120745](https://github.com/rust-lang/rust/issues/120745)
- [Check build target supports std when building with -Zbuild-std=std #14183](https://github.com/rust-lang/cargo/pull/14183)

## `restricted_std`

`std`'s [`build.rs` file](https://github.com/rust-lang/rust/blob/28e684b47000d4ed4cdb5d982331e5ff8141c1ce/library/std/build.rs#L20) has some target filtering and uses this to mark std as unstable on certain targets to indicate that they're unsupported. As discussed in [motivation](#motivation) this solution is confusing and only surfaces after building `std`, assuming it builds at all.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Do we need to de-stabilise target-spec file inputs? [#71009](https://github.com/rust-lang/rust/issues/71009)
- Is a boolean value enough detail here? Can we do better? See [Rationale and alternatives](#rationale-and-alternatives).

# Future possibilities
[future-possibilities]: #future-possibilities

As discussed in [Rationale and alternatives](#rationale-and-alternatives) we could introduce more nuance for targets that cannot support the entirety of std.
