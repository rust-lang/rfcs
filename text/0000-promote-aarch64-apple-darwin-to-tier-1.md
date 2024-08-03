- Feature Name: promote-aarch64-apple-darwin-to-tier-1
- Start Date: 2024-07-09
- RFC PR: [rust-lang/rfcs#3671](https://github.com/rust-lang/rfcs/pull/3671)
- Rust Issue: [rust-lang/rust#73908](https://github.com/rust-lang/rust/issues/73908)

# Summary
[summary]: #summary

Promote aarch64-apple-darwin to Tier 1.

# Motivation
[motivation]: #motivation

Approximately [33% of Rust users][survey-2023] use macOS for
development. Hardware using Apple Silicon CPUs is noticeably more
performant than previous x86\_64 Apple hardware and many developers
have already transitioned to using aarch64-apple-darwin. This number
is expected to increase as Apple no longer produces x86\_64 hardware.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This change will not require additional explanation to Rust
programmers as many people believe that aarch64-apple-darwin is
_already_ Tier 1. As such, I expect this change will reduce potential
confusion.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Tier 1 targets must adhere to the [Tier 1 Target
Policy][tier-1-policy]. This RFC intends that aarch64-apple-darwin
will be Tier 1 [with host tools][host-tools]. Going through these
requirements point-by-point:

> Tier 1 targets must have substantial, widespread interest within the
> developer community, and must serve the ongoing needs of multiple
> production users of Rust across multiple organizations or projects.

As [stated above][motivation], macOS users comprise a non-trivial
percentage of overall Rust users.

> The target maintainer team must include at least 3 developers.

There is an existing [team for Apple and macOS specific
concerns][apple-team]. The aarch64-apple-darwin target is actively
used and maintained. Rust has been [tracking and fixing Apple Silicon
specific issues][silicon-issues] and the LLVM team has been doing the
same.

> The target must build and pass tests reliably in CI, for all
> components that Rust's CI considers mandatory.

Since [2024-02-06][enabled-m1], Rust continuous integration has been
building and testing the aarch64-apple-darwin compiler and host tools
with roughly the same settings as x86\_64.

> The target must provide as much of the Rust standard library as is
> feasible and appropriate to provide.

No material difference exists between the x86\_64-apple-darwin and
aarch64-apple-darwin targets in this regard.

> Building the target and running the testsuite for the target must not take
> substantially longer than other targets, and should not substantially raise
> the maintenance burden of the CI infrastructure.

Due to improved hardware performance, aarch64-apple-darwin is usually
faster than x86\_64-apple-darwin. As a recent example,
[aarch64-apple-darwin took 61 minutes][dist-build-aarch64] while
[x86\_64-apple-darwin took 118 minutes][dist-build-x86\_64].

> Tier 1 targets must not have a hard requirement for signed, verified, or
> otherwise "approved" binaries.

No material difference exists between the x86\_64-apple-darwin and
aarch64-apple-darwin targets in this regard.

# Drawbacks
[drawbacks]: #drawbacks

Tier 1 status requires that we are able to build and run binaries for
the platform. While x86\_64 machines have been available in continuous
integration workflows for many years, aarch64 machines are relatively
new. The first Apple Silicon runners for GitHub Actions were [released
on 2023-10-02][runner-m1] with free runners for open source projects
[released on 2024-01-03][runner-m1-oss]. Availability or robustness of
these runners may be lower compared to x86\_64.

Tier 1 status requires increased continuous integration resource usage
which means increased cost to the project. However, the
aarch64-apple-darwin target has been treated as Tier 1 since
[2024-02-06][enabled-m1] without causing financial concern.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Apple Silicon is the _de facto_ path forward for macOS.

# Prior art
[prior-art]: #prior-art

- [RFC 2959][rfc-2959] promoted `aarch64-unknown-linux-gnu` to Tier 1.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

It is expected that **x86\_64**-apple-darwin will be demoted to Tier 2
at some future point as hardware for this platform is [no longer being
produced][transition]. This may reduce our continuous integration
costs, offsetting any increases from adding
aarch64-apple-darwin. There are **no concrete plans** to demote
x86\_64-apple-darwin at this time and any such demotion would need its
own well-publicized RFC.

[apple-team]: https://github.com/rust-lang/team/blob/16fc8a96bf2733bc0e7ca553a645f3840ed0a7a4/teams/apple.toml
[dist-build-aarch64]: https://github.com/rust-lang-ci/rust/actions/runs/9856130302/job/27212491241
[dist-build-x86\_64]: https://github.com/rust-lang-ci/rust/actions/runs/9856130302/job/27212490161
[enabled-m1]: https://github.com/rust-lang/rust/pull/120509
[host-tools]: https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html#tier-1-with-host-tools
[rfc-2959]: https://rust-lang.github.io/rfcs/2959-promote-aarch64-unknown-linux-gnu-to-tier1.html
[runner-m1-oss]: https://github.blog/changelog/2024-01-30-github-actions-introducing-the-new-m1-macos-runner-available-to-open-source/
[runner-m1]: https://github.blog/2023-10-02-introducing-the-new-apple-silicon-powered-m1-macos-larger-runner-for-github-actions/
[silicon-issues]: https://github.com/rust-lang/rust/issues?q=is%3Aissue+sort%3Aupdated-desc+label%3AO-macos+label%3AO-AArch64
[survey-2023]: https://blog.rust-lang.org/2024/02/19/2023-Rust-Annual-Survey-2023-results.html
[tier-1-policy]: https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html#tier-1-target-policy
[transition]: https://en.wikipedia.org/wiki/Mac_transition_to_Apple_silicon
