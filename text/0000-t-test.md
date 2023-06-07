- Feature Name: n/a
- Start Date: (2023-06-07)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: n/a

# Summary
[summary]: #summary

Create a team focused on the testing in the development workflow under primarily under [t-devtools](https://www.rust-lang.org/governance/teams/dev-tools) but also working closely with [t-libs](https://www.rust-lang.org/governance/teams/library) when it comes to the API and implementation of the built-in test harness.

# Motivation
[motivation]: #motivation

In [rust 1.70.0](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html), a bug was fixed so unstable test features, like `cargo test -- --format json`, require using a nightly toolchain, like other unstable features.  The problem is IDEs and CIs have been relying on this behavior to get programmatic test data and little progress has been made in the last 5 years ([#49359](https://github.com/rust-lang/rust/issues/49359)).

Before this, there was a growing interest in improving testing generally, like [better CI integration](https://internals.rust-lang.org/t/pre-rfc-implementing-test-binary-list-format-json-for-use-by-ide-test-explorers-runners/18308).

One challenge with improving the situation is that a lot of concerns cross multiple teams, in particular cargo (for `cargo test`), libs (for libtest).  This makes it more difficult to establish a vision and coordinate efforts to achieve that vision.

There is also precedence for this today with t-devtools having more specific sub-teams like rustdoc and rustfmt.

## Mission and responsibilities
[mission]: #mission

This team would primarily be focused on iterating on the test writing experience, `cargo test`, and enabling integration with external tools like CI or IDEs.

Examples issues to resolve:
- [Stabilize support for programmatic (json) test output](https://github.com/rust-lang/rust/issues/49359)
- What parts of [cargo nextest](https://nexte.st/) can we stabilize?
- With the proliferation of test frameworks (e.g. [rstest], [trybuild], [trycmd], [cargo_test_support], [criterion]), are there underlying needs we can resolve?

## Relationships to other teams

T-devtools: This will be the parent team.

**T-cargo**: This is a sibling team that T-testing will need to work with similarly to T-rustfmt, T-clippy, etc.

**T-rustdoc**: This is a sibling team that T-testing will likely coordinate with if any changes are need to how we do doctesting

**T-libs**: This will effectively be another parent team as they are ultimately responsible for libtest.

## Processes

For decisions on vision and direction, T-testing will use a standard FCP process.  T-testing will be subject to [T-cargo's processes](https://doc.crates.io/contrib/team.html#decision-process) when dealing with `cargo test` and T-libs's processes for libtest.  For any newly developed crates and libraries, we will follow [T-cargo's processes](https://doc.crates.io/contrib/team.html#decision-process).

## Membership

Team members are expected to shepherd testing discussions and vote on FCPs and is independent of regular code contributions though contributing can help build up the relevant experience and/or demonstrate the qualities for team membership.

Qualifications that will be taken into account for membership include:

- Does the person have the judgement for when deciding when a larger consensus is needed?
- Does the person understand the constraints of backwards compatibility within `cargo test` and libtest and exercise caution when extending the compatibly constraints?

Someone can become a member of the Cargo Team by requesting a review or being nominated by one of the existing members. They can be added by unanimous consent of the team. The team lead or another member of the team will also confirm with the moderation team that there are no concerns involving the proposed team member.

Leads are responsible for scheduling and facilitating team meetings and will be selected from the team members by consensus.

The initial members of the testing team shall be:
- Lead: Caleb Cartwright (@calebcartwright)
- Ed Page (@epage)
- Weihang Lo (@weihanglo)
- Scott Schafer (@Muscraft)

## Drawbacks and Alternatives

- The ownership lines are becoming more muddled with the two teams dealing with `cargo test` and two teams dealing with libtest.

[rstest]: https://crates.io/crates/rstest
[trybuild]: https://crates.io/crates/trybuild
[trycmd]: https://crates.io/crates/trycmd
[cargo_test_support]: https://doc.rust-lang.org/nightly/nightly-rustc/cargo_test_support/
[criterion]: https://crates.io/crates/criterion
