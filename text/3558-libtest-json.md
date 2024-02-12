- Feature Name: `libtest-json`
- Start Date: 2024-01-18
- Pre-RFC: [Internals](https://internals.rust-lang.org/t/path-for-stabilizing-libtests-json-output/20163)
- eRFC PR: [rust-lang/rfcs#3558](https://github.com/rust-lang/rfcs/pull/3558)
- Tracking Issue: [rust-lang/testing-devex-team1](https://github.com/rust-lang/testing-devex-team/issues/1)

# Summary
[summary]: #summary

This eRFC lays out a path for [stabilizing programmatic output for libtest](https://github.com/rust-lang/rust/issues/49359).

# Motivation
[motivation]: #motivation

[libtest](https://github.com/rust-lang/rust/tree/master/library/test)
is the test harness used by default for tests in cargo projects.
It provides the CLI that cargo calls into and enumerates and runs the tests discovered in that binary.
It ships with rustup and has the same compatibility guarantees as the standard library.

Before 1.70, anyone could pass `--format json` despite it being unstable.
When this was fixed to require nightly,
this helped show [how much people have come to rely on programmatic output](https://www.reddit.com/r/rust/comments/13xqhbm/announcing_rust_1700/jmji422/).

Cargo could also benefit from programmatic test output to improve user interactions, including
- [Wanting to run test binaries in parallel](https://github.com/rust-lang/cargo/issues/5609), like `cargo nextest`
- [Lack of summary across all binaries](https://github.com/rust-lang/cargo/issues/4324)
- [Noisy test output](https://github.com/rust-lang/cargo/issues/2832) (see also [#5089](https://github.com/rust-lang/cargo/issues/5089))
- [Confusing command-line interactions](https://github.com/rust-lang/cargo/issues/1983) (see also [#8903](https://github.com/rust-lang/cargo/issues/8903), [#10392](https://github.com/rust-lang/cargo/issues/10392))
- [Poor messaging when a filter doesn't match](https://github.com/rust-lang/cargo/issues/6151)
- [Smarter test execution order](https://github.com/rust-lang/cargo/issues/6266) (see also [#8685](https://github.com/rust-lang/cargo/issues/8685), [#10673](https://github.com/rust-lang/cargo/issues/10673))
- [JUnit output is incorrect when running multiple test binaries](https://github.com/rust-lang/rust/issues/85563)
- [Lack of failure when test binaries exit unexpectedly](https://github.com/rust-lang/rust/issues/87323)

Most of that involves shifting responsibilities from the test harness to the test runner which has the side effects of:
- Allowing more powerful experiments with custom test runners (e.g. [`cargo nextest`](https://crates.io/crates/cargo-nextest)) as they'll have more information to operate on
- Lowering the barrier for custom test harnesses (like [`libtest-mimic`](https://crates.io/crates/libtest-mimic)) as UI responsibilities are shifted to the test runner (`cargo test`)

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The intended outcomes of this experiment are:
- Updates to libtest's unstable output
- A stabilization request to [T-libs-api](https://www.rust-lang.org/governance/teams/library#Library%20API%20team) using the process of their choosing

Additional outcomes we hope for are:
- A change proposal for [T-cargo](https://www.rust-lang.org/governance/teams/dev-tools#Cargo%20team) for `cargo test` and `cargo bench` to provide their own UX on top of the programmatic output
- A change proposal for [T-cargo](https://www.rust-lang.org/governance/teams/dev-tools#Cargo%20team) to allow users of custom test harnesses to opt-in to the new UX using programmatic output

While having a plan for evolution takes some burden off of the format,
we should still do some due diligence in ensuring the format works well for our intended uses.
Our rough plan for vetting a proposal is:
1. Create an experimental test harness where each `--format <mode>` is a skin over a common internal `serde` structure, emulating what `libtest` and `cargo`s relationship will be like on a smaller scale for faster iteration
2. Transition libtest to this proposed interface
3. Add experimental support for cargo to interact with test binaries through the unstable programmatic output
4. Create a stabilization report for programmatic output for T-libs-api and a cargo RFC for custom test harnesses to opt into this new protocol

It is expected that the experimental test harness have functional parity with `libtest`, including
- Ignored tests
- Parallel running of tests
- Benches being both a bench and a test
- Test discovery

We should evaluate the design against the capabilities of test runners from different ecosystems to ensure the format has the expandability for what people may do with custom test harnesses or `cargo test`, including:
- Ability to implement different format modes on top
  - Both test running and `--list` mode
- Ability to run test harnesses in parallel
- [Tests with multiple failures](https://docs.rs/googletest/0.10.0/googletest/prelude/macro.expect_that.html)
- Bench support
- Static and dynamic [parameterized tests / test fixtures](https://crates.io/crates/rstest)
- Static and [dynamic test skipping](https://doc.crates.io/contrib/tests/writing.html#cargo_test-attribute)
- [Test markers](https://docs.pytest.org/en/7.4.x/example/markers.html#mark-examples)
- doctests
- Test location (for IDEs)
- Collect metrics related to tests
  - Elapsed time
  - Temp dir sizes
  - RNG seed

**Warning:** This doesn't mean they'll all be supported in the initial stabilization just that we feel confident the format will support them)

We also need to evaluate how we'll support evolving the format.
An important consideration is that the compile-time burden we put on custom
test harnesses as that will be an important factor for people's willingness to
pull them in as `libtest` comes pre-built today.

Custom test harnesses are important for this discussion because
- Many already exist today, directly or shoe-horned on top of `libtest`, like
  - [libtest-mimic](https://crates.io/crates/libtest-mimic)
  - [criterion](https://crates.io/crates/criterion)
  - [divan](https://crates.io/crates/divan)
  - [cargo-test-support](https://doc.rust-lang.org/nightly/nightly-rustc/cargo_test_support/index.html)
  - [rstest](https://crates.io/crates/rstest)
  - [trybuild](https://crates.io/crates/trybuild)
- The compatibility guarantees around libtest mean that development of new ideas is easier through custom test harnesses

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Resources

Comments made on libtests format
- [Format is complex](https://github.com/rust-lang/rust/issues/49359#issuecomment-467994590) (see also [1](https://github.com/rust-lang/rust/issues/49359#issuecomment-1531369119))
- [Benches need love](https://github.com/rust-lang/rust/issues/49359#issuecomment-467994590)
- [Type field is overloaded](https://github.com/rust-lang/rust/issues/49359#issuecomment-467994590)
- [Suite/child relationship is missing](https://github.com/rust-lang/rust/issues/49359)
- [Lack of suite name makes it hard to use programmatic output from Cargo](https://github.com/rust-lang/rust/issues/49359#issuecomment-533154674) (see also [1](https://github.com/rust-lang/rust/issues/49359#issuecomment-699691296))
- [Format is underspecified](https://github.com/rust-lang/rust/issues/49359#issuecomment-706566635)
- ~~[Lacks ignored reason](https://github.com/rust-lang/rust/issues/49359#issuecomment-715877950)~~ ([resolved?](https://github.com/rust-lang/rust/issues/49359#issuecomment-1531369119))
- [Lack of `rendered` field](https://github.com/rust-lang/rust/issues/49359#issuecomment-1531369119)

# Drawbacks
[drawbacks]: #drawbacks

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

See also
- https://internals.rust-lang.org/t/alternate-libtest-output-format/6121
- https://internals.rust-lang.org/t/past-present-and-future-for-rust-testing/6354

# Prior art
[prior-art]: #prior-art

Existing formats
- junit
- [subunit](https://github.com/testing-cabal/subunit)
- [TAP](https://testanything.org/)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

## Improve custom test harness experience

With less of a burden being placed on custom test harnesses,
we can more easily explore what is needed for making them be a first-class experience.

See
- [eRFC 2318: Custom Test Frameworks](https://rust-lang.github.io/rfcs/2318-custom-test-frameworks.html)
- [Blog Post: Iterating on Test](https://epage.github.io/blog/2023/06/iterating-on-test/)
