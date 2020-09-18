- Feature Name: stdsimd_project_group
- Start Date: 2020-08-28
- RFC PR: [rust-lang/rfcs#2977](https://github.com/rust-lang/rfcs/pull/2977)
- Rust Issue: [rust-lang/rust-libs#4](https://github.com/rust-lang/libs-team/issues/4)

# Summary
[summary]: #summary

This is a project group RFC version of [`lang-team#29`].

This RFC establishes a new project group, under the libs team, to produce a portable SIMD API in a new `rust-lang/stdsimd` repository, exposed through a new `std::simd` (and `core::simd`) module in the standard library in the same manner as [`stdarch`]. The output of this project group will be the finalization of [RFC 2948] and stabilization of `std::simd`.

# Motivation
[motivation]: #motivation

The current stable `core::arch` module is described by [RFC 2325], which considers a portable API desirable but out-of-scope. The current [RFC 2948] provides a good motivation for this API. Various ecosystem implementations of portable SIMD have appeared over the years, including [`packed_simd`], and [`wide`], each taking a different set of trade-offs in implementation while retaining some similarities in their public API. The group will pull together a "blessed" implementation in the standard library with the explicit goal of stabilization for the [2021 edition].

# Charter
[charter]: #charter

## Goals

- Determine the shape of the portable SIMD API.
- Get an unstable `std::simd` and `core::simd` API in the standard library. This may mean renaming `packed_simd` to `stdsimd` and working directly on it, or creating a new repository and pulling in chunks of code as needed.
- Produce a stabilization plan to allow portions of `std::simd` to be stabilized when they're ready, and coordinate with other unstable features.
- Respond to user feedback and review contributions to the API.
- Update [RFC 2948] based on the final API and stabilization plan.
- Stabilize `std::simd`!

## Non Goals

- This group isn't directly attempting to build out more `core::arch` APIs.

## Membership Requirements

- Group membership is open, any interested party can participate in discussions, repeat contributors will be added to appropriate teams.

## Additional Questions

### What support do you need, and separately want, from the Rust organization?

Support scaffolding a space to work and integrating `stdsimd` into `libcore` and input from engineers who are familiar with this space.

### Why should this be a project group over a community effort?

Community efforts have already produced libraries that are in use, but pulling those together in the standard library needs a group with permissions to get things merged.

### What do you expect the relationship to the team be?

The project group will regularly update libs on how things are going, whether there are any blockers

### Who are the initial shepherds/leaders? (This is preferably 2–3 individuals, but not required.)

- @BurntSushi
- @calebzulawski
- @hsivonen
- @KodrAus
- @Lokathor

### Is your group long-running or temporary?

Temporary

### If it is temporary, how long do you see it running for?

Until the 2021 edition, which is probably mid 2021.

### If applicable, which other groups or teams do you expect to have close contact with?

The project group will interact with:

- libs
- compiler

### Where do you see your group needing help?

There will be lots of feedback to gather from users and input from compiler developers on how to approach implementation.

[`packed_simd`]: https://github.com/rust-lang/packed_simd
[`wide`]: https://github.com/Lokathor/wide
[`stdarch`]: https://github.com/rust-lang/stdarch
[2021 edition]: https://github.com/rust-lang/rfcs/pull/2966
[RFC 2948]: https://github.com/rust-lang/rfcs/pull/2948
[RFC 2325]: https://rust-lang.github.io/rfcs/2325-stable-simd.html
[`lang-team#29`]: https://github.com/rust-lang/lang-team/issues/29
