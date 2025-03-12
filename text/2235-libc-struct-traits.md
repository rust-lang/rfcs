- Feature Name: `libc_struct_traits`
- Start Date: 2017-12-05
- RFC PR: [rust-lang/rfcs#2235](https://github.com/rust-lang/rfcs/pull/2235)
- Rust Issue: [rust-lang/rust#57715](https://github.com/rust-lang/rust/issues/57715)

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

Add an `extra_traits` feature to the `libc` library that enables `Debug`, `Eq`, `Hash`, and `PartialEq` implementations for all structs.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `Debug`, `Eq`/`PartialEq`, and `Hash` traits will be added as automatic derives within the `s!` macro in `src/macros.rs` if the corresponding feature
flag is enabled. This won't work for some types because auto-derive doesn't work for arrays larger than 32 elements, so for these they'll be implemented manually. For `libc`
as of `bbda50d20937e570df5ec857eea0e2a098e76b2d` on `x86_64-unknown-linux-gnu` these many structs will need manual implementations:

 * `Debug` - 17
 * `Eq`/`PartialEq` - 46
 * `Hash` - 17

# Drawbacks
[drawbacks]: #drawbacks

While most structs will be able to derive these implementations automatically, some will not (for example arrays larger than 32 elements). This will make it harder to add
some structs to `libc`.

This extra trait will increase the testing requirements for `libc`.

# Rationale and alternatives
[alternatives]: #alternatives

Adding these trait implementations behind a singular feature flag has the best combination of utility and ergonomics out of the possible alternatives listed below:

## Always enabled with no feature flags

This was regarded as unsuitable because it increases compilation times by 100-200%. Compilation times of `libc` was tested at commit `bbda50d20937e570df5ec857eea0e2a098e76b2d`
with modifications to add derives for the traits discussed here under the `extra_traits` feature (with no other features). Some types failed to have these traits
derived because of specific fields, so these were removed from the struct declaration. The table below shows the compilation times:

|                              Build arguments                                                 | Time  |
|----------------------------------------------------------------------------------------------|-------|
| `cargo clean && cargo build --no-default-features`                                           | 0.84s |
| `cargo clean && cargo build --no-default-features --features extra_traits`                   | 2.17s |
| `cargo clean && cargo build --no-default-features --release`                                 | 0.64s |
| `cargo clean && cargo build --no-default-features --release --features extra_traits`         | 1.80s |
| `cargo clean && cargo build --no-default-features --features use_std`                        | 1.14s |
| `cargo clean && cargo build --no-default-features --features use_std,extra_traits`           | 2.34s |
| `cargo clean && cargo build --no-default-features --release --features use_std`              | 0.66s |
| `cargo clean && cargo build --no-default-features --release --features use_std,extra_traits` | 1.94s |

## Default-on feature

For crates that are more than one level above `libc` in the dependency chain it will be impossible for them to opt out. This could also happen with a default-off
feature flag, but it's more likely the library authors will expose it as a flag as well.

## Multiple feature flags

Instead of having a single `extra_traits` feature, have it and feature flags for each trait individually like:

 * `trait_debug` - Enables `Debug` for all structs
 * `trait_eg` - Enables `Eq` and `PartialEq` for all structs
 * `trait_hash` - Enables `Hash` for all structs
 * `extra_traits` - Enables all of the above through dependent features

This change should reduce compilation times when not all traits are desired. The downsides are that it complicates CI. It can be added in a backwards-compatible
manner later should compilation times or consumer demand changes.

# Unresolved questions
[unresolved]: #unresolved-questions
