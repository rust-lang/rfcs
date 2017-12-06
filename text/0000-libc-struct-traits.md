- Feature Name: (libc_struct_traits)
- Start Date: (2017-12-05)
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Expand the traits implemented by structs `libc` crate to include `Debug`, `Eq`, `Hash`, and `PartialEq`.

# Motivation
[motivation]: #motivation

This will allow downstream crates to easily support similar operations with any types they
provide that contain `libc` structs. Additionally [The Rust API Guidelines](https://rust-lang-nursery.github.io/api-guidelines/checklist.html) specify that it is
considered useful to expose as many traits as possible from the standard library. In order to facilitate the
following of these guidelines, official Rust libraries should lead by example.

For many of these traits, it is trivial for downstream crates to implement them for these types by using
newtype wrappers. As a specific example, the `nix` crate offers the `TimeSpec` wrapper type around the `timespec` struct. This
wrapper could easily implement `Eq` through comparing both fields in the struct.

Unfortunately there are a great many structs that are large and vary widely between platforms. Some of these in use by `nix`
are `dqblk`, `utsname`, and `statvfs`. These structs have fields and field types that vary across platforms. As `nix` aims to
support as many platforms as `libc` does, this variation makes implementing these traits manually on wrapper types time consuming and
error prone.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The feature flag `derive_all` is added to the `libc` library that when enabled adds `Debug`, `Eq`, `Hash`, and `PartialEq` traits for all structs. It will default to off.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `Debug`, `Eq`, `Hash`, and `PartialEq` traits will be added as automatic derives within the `s!` macro in `src/macros.rs` if the `derive_all` feature
flag is enabled. This won't work for some types because auto-derive doesn't work for arrays larger than 32 elements, so for these they'll be implemented manually. For `libc`
as of `bbda50d20937e570df5ec857eea0e2a098e76b2d` on `x86_64-unknown-linux-gnu` these many structs will need manual implementations:

 * `Debug` - 17
 * `Eq` and `PartialEq` - 46
 * `Hash` - 17

# Drawbacks
[drawbacks]: #drawbacks

The addition of this behind a feature flag does not have a significant effect on build times, but the burden of adding these implementations for new types that
require manual implementations will be high, possibly hindering new contributors.

# Rationale and alternatives
[alternatives]: #alternatives

Adding these trait implementations behind a singular feature flag has the best compination of utility and ergonomics out of the possible alternatives listed below:

## Always enabled

This was regarded as unsuitable because it doubles to triples compilation time. Compilation times of `libc` was tested at commit `bbda50d20937e570df5ec857eea0e2a098e76b2d`
with modifications to add derives for the traits discussed here. Some types failed to have these traits derived because of specific fields, so these were removed from the
struct declaration. The table below shows the results:

|                              Build arguments                                               | Time  |
|--------------------------------------------------------------------------------------------|-------|
| `cargo clean && cargo build --no-default-features`                                         | 0.84s |
| `cargo clean && cargo build --no-default-features --features derive_all`                   | 2.17s |
| `cargo clean && cargo build --no-default-features --release`                               | 0.64s |
| `cargo clean && cargo build --no-default-features --release --features derive_all`         | 1.80s |
| `cargo clean && cargo build --no-default-features --features use_std`                      | 1.14s |
| `cargo clean && cargo build --no-default-features --features use_std,derive_all`           | 2.34s |
| `cargo clean && cargo build --no-default-features --release --features use_std`            | 0.66s |
| `cargo clean && cargo build --no-default-features --release --features use_std,derive_all` | 1.94s |

## Default-on feature flag

For crates that are more than one level above `libc` in the dependency chain it will be impossible for them to opt out. This could also happen with a default-off
feature flag, but it's more likely the library authors will expose it as a flag as well.

## Independent feature flags

It wasn't tested how much compilation times increased per-trait, but further mitigation of slow compilation times could done by exposing all traits mentioned here
behind individual feature flags. By doing this it becomes harder for downstream crates to pass-through these feature flags, so it's likely not a worthwhile tradeoff.

# Unresolved questions
[unresolved]: #unresolved-questions

Is `derive_all` a suitable name for this feature flag?
