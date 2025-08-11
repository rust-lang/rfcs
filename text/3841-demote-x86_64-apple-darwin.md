- Feature Name: none
- Start Date: 2025-07-23
- RFC PR: [rust-lang/rfcs#3841](https://github.com/rust-lang/rfcs/pull/3841)
- Rust Issue: [rust-lang/rust#145252](https://github.com/rust-lang/rust/pull/145252)

# Summary
[summary]: #summary

Demote target `x86_64-apple-darwin` from Tier 1 to Tier 2 with host tools as this platform's lifetime is limited.

Tier 2 with host tools means that the `x86_64-apple-darwin` target,
including tools like `rustc` and `cargo`,
is guaranteed to build but is not guaranteed to pass tests.

This RFC does **not** propose removing the target completely from the codebase.

# Motivation
[motivation]: #motivation

The `x86_64-apple-darwin` target has no long-term future.
Upcoming changes will affect Rust's ability to ensure that the target meets the Tier 1 requirements,
so we should demote it to Tier 2 with host tools in a controlled fashion.

The most immediate critical change is that the free GitHub Actions macOS x86\_64 runners that the Rust project relies on will be [discontinued soon][macos-13-sunset].
There is no known long-term replacement for these runners.

## A brief timeline

- 2020-06-22: Apple [announced plans][trans] to shift away from the x86\_64 architecture.
- 2020-12-31: Rust [promoted `aarch64-apple-darwin`][aarch-tier-2] to Tier 2 with host tools.
- 2023-06-05: Apple [announced the replacement][trans] of the last x86\_64 hardware.
- 2023-10-02: GitHub [announces public GitHub Actions runners][m1-runners] for Apple silicon.
- 2024-10-17: Rust [promoted `aarch64-apple-darwin`][aarch-tier-1] to Tier 1.
- **2025-07-23**: This RFC opened.
- 2025-09-01: GitHub [will discontinue][macos-13-sunset] providing free macOS x86\_64 runners for public repositories.
- 2025 (Fall): [macOS 26][tahoe] will be the last macOS to support the x86\_64 architecture.
- 2027: The [Rosetta 2][trans] compatibility layer will be mostly removed.

[trans]: https://en.wikipedia.org/wiki/Mac_transition_to_Apple_silicon
[aarch-tier-2]: https://blog.rust-lang.org/2020/12/31/Rust-1.49.0/#64-bit-arm-macos-and-windows-reach-tier-2
[aarch-tier-1]: https://blog.rust-lang.org/2024/10/17/Rust-1.82.0/#macos-on-64-bit-arm-is-now-tier-1
[m1-runners]: https://github.blog/changelog/2023-10-02-github-actions-apple-silicon-m1-macos-runners-are-now-available-in-public-beta/
[macos-13-sunset]: https://github.blog/changelog/2025-07-11-upcoming-changes-to-macos-hosted-runners-macos-latest-migration-and-xcode-support-policy-updates/#macos-13-is-closing-down
[tahoe]: https://en.wikipedia.org/wiki/MacOS_Tahoe

## `x86_64-apple-darwin` popularity

Looking at the [public download statistics][dl-stats] for the previous month (retrieved on 2025-07-21),
we can see that `x86_64-apple-darwin` has substantially fewer downloads than `aarch64-apple-darwin`:

[dl-stats]: https://p.datadoghq.com/sb/3a172e20-e9e1-11ed-80e3-da7ad0900002-60425c7cb1b7beb2e8959a305a301c0c?fromUser=false&refresh_mode=sliding&from_ts=1750525313022&to_ts=1753117313022&live=true

### `rustc`

| platform                   | downloads | percentage |
|----------------------------|----------:|-----------:|
| `x86_64-unknown-linux-gnu` |   194.38M |     81.58% |
| `aarch64-apple-darwin`     |     7.15M |      3.00% |
| `x86_64-apple-darwin`      |     2.74M |      1.15% |

### `std`

| platform                   | downloads | percentage |
|----------------------------|----------:|-----------:|
| `x86_64-unknown-linux-gnu` |    95.12M |     66.20% |
| `aarch64-apple-darwin`     |     4.82M |      3.35% |
| `x86_64-apple-darwin`      |     2.76M |      1.92% |

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The first release after this RFC is merged will be the last one with Tier 1 support for the `x86_64-apple-darwin` target.
The release after that will demote the target to Tier 2 with host tools,
which means we no longer guarantee that it will be tested by CI.

Once this RFC is merged,
a blog post will be published on the main Rust Blog announcing the change to alert users of the demotion.

The demotion will also be mentioned in the release announcement for the last
release with Tier 1 support, as well as the first release with Tier
2 support.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The CI setup for [`rust-lang/rust`][r-l/r] will be modified to change the `dist-x86_64-apple` builder to no longer build the tests or run them.

[r-l/r]: https://github.com/rust-lang/rust

# Drawbacks
[drawbacks]: #drawbacks

Without automated testing,
this target will likely deteriorate more quickly.

Users may be relying on Rust's Tier 1 support to provide confidence for their own artifacts.
These users will be stuck on an old compiler version.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Rust CI could use emulation,
  such as that provided by [Rosetta 2][rosetta-2].
  Lightweight experiments show that this may increase CI times by a factor of 3
  (e.g. a step taking 200 seconds would now take 600 seconds).
  This would be a temporary solution,
  as eventually Apple will [sunset Rosetta 2][trans].

  We may choose to run tests in emulation even after the target is demoted to Tier 2 with host tools.
  That change would be evaluated independently from this RFC and in a similar fashion to other non-Tier-1 targets with extra testing.
  This evaluation would include aspects like CI complexity, test flakiness, test execution time, ability of contributors to have access to the hardware to fix issues, etc.
  Any extra testing would be at the whim of various Rust teams to reduce or remove at any point with no prior notice.

- The Rust Foundation could pay for GitHub Actions runners that will continue to use the x86\_64 architecture,
  such as `macos-13-large`, `macos-14-large`, or `macos-15-large`.
  This would be a temporary solution,
  as eventually GitHub will [sunset all x86\_64-compatible runners][n-1-policy].

- A third party could indefinitely provide all appropriate CI resources for the x86\_64 architecture.
  No such third party has made themselves known,
  nor has the Rust infrastructure team determined how to best integrate such resources.

[rosetta-2]: https://en.wikipedia.org/wiki/Rosetta_(software)
[n-1-policy]: https://github.com/actions/runner-images?tab=readme-ov-file#software-and-image-support

# Prior art
[prior-art]: #prior-art

- The `i686-pc-windows-gnu` target was demoted in [RFC 3771][rfc-3771].
  Similar to this RFC,
  the ability to reliably test the target was questionable.

- The `i686-apple-darwin` target was demoted in [RFC 2837][rfc-2837].
  Similar to this RFC,
  relevant hardware was no longer produced and it had been announced that upcoming operating systems would no longer support the architecture.

[rfc-2837]: https://rust-lang.github.io/rfcs/2837-demote-apple-32bit.html
[rfc-3771]: https://rust-lang.github.io/rfcs/3771-demote-i686-pc-windows-gnu.html

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None

# Future possibilities
[future-possibilities]: #future-possibilities

`x86_64-apple-darwin` could be demoted to Tier 3 or support completely removed.
There's no strong technical or financial reason to do this at this point in time.
Should further demotions be proposed,
those will be evaluated separately and on thier own merits,
using the [target tier policy][tier-policy] as guidance.

[tier-policy]: https://doc.rust-lang.org/stable/rustc/target-tier-policy.html
