- Feature Name: n/a
- Start Date: (2023-06-27)
- RFC PR: [rust-lang/rfcs#3455](https://github.com/rust-lang/rfcs/pull/3455)
- Rust Issue: n/a

# Summary
[summary]: #summary

Create a new subteam focused on testing in the development workflow and to be responsible for
ensuring Rust has a high quality automated testing experience that includes the capabilities
developers expect when working with a modern programming language.

# Motivation
[motivation]: #motivation

Currently, the overall automated testing experience spans multiple components owned by different
teams across the Rust Project (e.g. T-cargo with `cargo test` being the primary
touch point for most users, T-libs for libtest, T-rustdoc for doctests, etc.).
This makes it more difficult to establish a vision and coordinate efforts to achieve that vision.
However, there isn't any single team focused on the testing picture holistically.
Simultaneously, there are a number of well known needs and feature requests in the space that do not have an explicit
owner driving the efforts to completion.

For example, there's been some long standing requests to have additional test output
formats, such as JSON ([#49359]) and JUnit, available on Stable Rust.
While some of these are available as unstable features in Nightly Rust,
in [Rust 1.70.0](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html), a bug was fixed
so unstable test features, like `cargo test -- --format json`, require using a nightly
toolchain, like other unstable features. This caused breakage in certain editor/IDE and CI
related tooling as they had been relying on the prior behavior to get test data
programmatically, and little progress has been made in the last 5 years ([#49359]).

Furthermore, there's been a growing interest in improving testing generally, like [better CI integration][ci] as well as requests for things like better support for custom test harnesses and frameworks (.e.g [#2318]).

The new Testing team is intended to establish an overarching vision and provide focused attention on these areas. 

[#49359]: https://github.com/rust-lang/rust/issues/49359
[#50297]: https://github.com/rust-lang/rust/issues/50297
[#2318]: https://github.com/rust-lang/rfcs/pull/2318
[ci]: (https://internals.rust-lang.org/t/pre-rfc-implementing-test-binary-list-format-json-for-use-by-ide-test-explorers-runners/18308)

## Mission and responsibilities
[mission]: #mission

This team would be primarily focused on iterating on the test writing and analysis experience, `cargo test`, and enabling integration points and features for external tools like CI or IDEs.

Examples of issues to resolve:
- [Stabilize support for programmatic (json) test output](https://github.com/rust-lang/rust/issues/49359)
- What parts of [cargo nextest](https://nexte.st/) can we stabilize?
- With the proliferation of test frameworks (e.g. [rstest], [trybuild], [trycmd], [cargo_test_support], [criterion]), are there underlying needs we can resolve?

## Relationships to other teams

With the aforementioned breadth across the Project, the Testing team will need to have collaborative relationships with many other teams, and is conceptually a subteam of both the Libs and Dev Tools teams.

The rust-lang/team repo does not currently support representing dual-parent subteams, so for now the Testing team will be primarily under the Dev Tools team

T-devtools: This will be the primary top level parent team.

**T-cargo**: This is a sibling team that T-testing will need to work with similarly to T-rustfmt, T-clippy, etc.

**T-rustdoc**: This is a sibling team that T-testing will likely coordinate with if any changes are need to how we do doctesting

**T-IDEs and Editors**: This is a sibling team that T-testing will likely coordinate with to understand the needs of IDEs/editors related to incorporating test related capabilities

**T-libs**: This will be a second/secondary top level parent team as they are ultimately responsible for libtest.

## Processes

For decisions on vision and direction, T-testing will use a standard FCP process.  T-testing will be subject to [T-cargo's processes](https://doc.crates.io/contrib/team.html#decision-process) when dealing with `cargo test` and T-libs's processes for libtest.  For any newly developed crates and libraries, we will follow [T-cargo's processes](https://doc.crates.io/contrib/team.html#decision-process).

## Membership

Team members are expected to shepherd testing discussions and vote on FCPs. Team membership is independent of regular code contributions though contributing can help build up the relevant experience and/or demonstrate the qualities for team membership.

Qualifications that will be taken into account for membership include:

- Does the person have the judgement for when deciding when a larger consensus is needed?
- Does the person understand the constraints of backwards compatibility within `cargo test` and libtest and exercise caution when extending the compatibly constraints?

Someone can become a member of the Testing Team by requesting a review or being nominated by one of the existing members. They can be added by unanimous consent of the team. The team lead or another member of the team will also confirm with the moderation team that there are no concerns involving the proposed team member.

Team Leads are responsible for scheduling and facilitating team meetings and will be selected from the team members by consensus.

The initial members of the Testing team shall be:
- Lead: Caleb Cartwright (@calebcartwright)
- Ed Page (@epage)
- Weihang Lo (@weihanglo)
- Scott Schafer (@Muscraft)
- Thom Chiovoloni  (@thomcc)

## Drawbacks 

The proposed Testing team bears some similarity to other Rust teams (e.g. Types team) in the sense
that it would complicate and muddle the ownership of specific problems.
For example, there would be two teams dealing with `cargo test` and two dealing with libtest.

## Rationale and alternatives

- This could be a working group instead of a team. However, we believe the [reasoning articulated in the Types team RFC][team-not-wg] is applicable here as well. There is a need for focused effort on driving work to completion along with associated maintenance work; not a shorter-lived initiative to create recommendations.
- The Testing team could be a dual-parent subteam, but with the primary team under the Libs team. However, we believe Dev Tools is the better primary parent given the purview of the Testing team would extend well beyond libtest
- The Testing team could be a single-parent subteam. We think there's too much overlap with too many teams across multiple top level teams to be a single-parent subteam.
- We could do nothing and not form a new subteam nor a new working group. This would perpetuate the status quo and would most likely result in continued stagnation/lack of progress on the aforementioned focus areas.


[team-not-wg]: https://rust-lang.github.io/rfcs/3254-types-team.html#why-a-team-and-not-a-working-group-what-is-the-difference-between-those-anyway
[rstest]: https://crates.io/crates/rstest
[trybuild]: https://crates.io/crates/trybuild
[trycmd]: https://crates.io/crates/trycmd
[cargo_test_support]: https://doc.rust-lang.org/nightly/nightly-rustc/cargo_test_support/
[criterion]: https://crates.io/crates/criterion
