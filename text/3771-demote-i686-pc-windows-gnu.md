- Feature Name: none
- Start Date: 2025-02-11
- RFC PR: [rust-lang/rfcs#3771](https://github.com/rust-lang/rfcs/pull/3771)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Demote target `i686-pc-windows-gnu` from Tier 1 to Tier 2 (with host tools) to better reflect its current maintenance and usage status.

# Motivation
[motivation]: #motivation

## Background

Rust has supported Windows for a long time, with two different flavors of Windows targets: MSVC-based and GNU-based.
MSVC-based targets (for example the main Windows target `x86_64-pc-windows-msvc`) use Microsoft's native `link.exe` linker and libraries, while GNU-based targets (like `i686-pc-windows-gnu`) are built entirely from free software components like `gcc`, `ld`, and MinGW.

The major reason to use a GNU-based toolchain instead of the native MSVC-based one is cross-compilation and licensing. `link.exe` only runs on Windows (barring Wine hacks) and requires a license for commercial usage.

Rust currently supports the following major Windows targets. They all have host tools. The download count was extracted from [the public dashboard](https://p.datadoghq.com/sb/3a172e20-e9e1-11ed-80e3-da7ad0900002-60425c7cb1b7beb2e8959a305a301c0c?fromUser=false&refresh_mode=sliding&from_ts=1736618152507&to_ts=1739210152507&live=true) on 2025-02-10.
We also show the `std` download counts to account for cross-compilation usage.

| Name | Tier | `rustc` download count | `std` download count
| -------- | -------- | ---- | ------ |
| `x86_64-pc-windows-msvc` | 1 | 6.72M | 3.56M |
| `x86_64-pc-windows-gnu` | 1 | 375K | 1.06M |
| `i686-pc-windows-msvc` | 1 | 260k | 793K |
| `i686-pc-windows-gnu` | 1 | 76K | 56K |
| `aarch64-pc-windows-msvc` | 2 | 46K | 241K |

To put the download numbers into perspective, some other targets:

| Name | Tier | `rustc` download count | `std` download count
| -------- | -------- | ---- | ------ |
| `x86_64-unknown-linux-gnu` | 1 | 135M | 65M |
| `i686-unknown-linux-gnu` | 1 | 332K | 437K |
| `x86_64-unknown-freebsd`   | 2 | 138K | 89K |
| `x86_64-unknown-netbsd`    | 2 | 36K | 32K |

From the download count alone, `i686-pc-windows-gnu` better fits in next to other Tier 2 targets like FreeBSD and NetBSD.

But that is not everything. GNU-based Windows targets are, as the description at the start may imply, an alternative (you could say non-standard) way to compile for Windows, and as such subject to many kinds of unique problems.
The Rust Project currently does not have a lot of expertise for dealing with these issues.
The setup and build for Windows GNU is complicated and prone to errors, often failing in CI, leading to frequent efforts to fix them being carried out by people who are, at best, familiar with 64-bit Windows or Windows MSVC.
This results in Windows-GNU problems often being unaddressed, or worse: fixed in ways that turn up more errors later down the line.

Some example problems, found by searching for `ignore-windows-gnu` in rust-lang/rust.

- https://github.com/rust-lang/rust/issues/128973
- https://github.com/rust-lang/rust/issues/128981
- https://github.com/rust-lang/rust/pull/116837
- https://github.com/rust-lang/rust/issues/128911

### 32-bit x86 Problems

While some of these issues apply to all GNU-based targets, 32-bit x86 seems to be especially affected.
And when a 32-bit Windows GNU issue comes up, contributors rarely actually investigate it, because it is such a complex and nonstandard environment compared to 64-bit Windows GNU, which is a lot easier to set up and work with.

That the 32-bit x86 architecture is unusual, and made moreso by how Windows operates on it, has also been noted by Windows experts[^2].
The Windows GNU experts that provide direct support to the Rust project focus almost exclusively on the 64-bit targets, and have previously recommended the retirement of the 32-bit targets[^1].

MSYS2, a major distributor of the GNU-based Windows platform, has been [dropping some 32-bit packages](https://www.msys2.org/news/#2023-12-13-starting-to-drop-some-32-bit-packages) and [no longer distributes Clang for 32-bit](https://github.com/msys2/MINGW-packages/pull/21998), showing even their shift away from the platform.
In response to inquiries about their opinion on reducing support for the target, [MSYS2 folks were positive](https://github.com/msys2/MINGW-packages/issues/23346).

[^1]: despite saying he is only a maintainer for x86_64-pc-windows-gnullvm, mati865 is effectively also our maintainer for x86_64-pc-windows-gnu https://rust-lang.zulipchat.com/#narrow/channel/233931-t-compiler.2Fmajor-changes/topic/Demote.20.60i686-pc-windows-gnu.60.20compiler-team.23822/near/490675824
[^2]: https://devblogs.microsoft.com/oldnewthing/20220418-00/?p=106489

## Target Tier Policy Requirements

With this knowledge, we can look at the [Tier 1 requirements](https://doc.rust-lang.org/1.84.1/rustc/target-tier-policy.html#tier-1-target-policy) of the target tier policy to check whether they are fulfilled.

> *Tier 1 targets must have substantial, widespread interest within the developer community, and must serve the ongoing needs of multiple production users of Rust across multiple organizations or projects.*

While this cannot be quantified precisely, the download counts suggest that this target is less popular than some other Tier 2 targets like FreeBSD.
Therefore, we are going to treat this as false.

> The target maintainer team must include at least 3 developers.

This is not the case at all. There is currently no maintainer team.
Though we should note that this is currently also true for many other Tier 1 targets, as this is a new rule not upheld everywhere yet.
But experience tells that it is highly unlikely that 3 maintainers for 32 bit Windows GNU will be found.

> The target must build and pass tests reliably in CI, for all components that Rust's CI considers mandatory.

As mentioned above, there are issues and it does cause a fair share of problems.

> The target must not disable an excessive number of tests or pieces of tests in the testsuite in order to do so. This is a subjective requirement.

A fair amount of tests are disabled with an open issue with no comments.
I would say that it is on the edge of being excessive, not quite having reached that amount (but it is likely that will be reached eventually).
For example, [#134777](https://github.com/rust-lang/rust/pull/134777) observed and un-ignored a lot of ignored Windows tests, many of which were likely ignored on all of Windows because of Windows GNU issues.
Another example of an ignored test is [#135572](https://github.com/rust-lang/rust/pull/135572) that does not support Windows GNU because it was too mcuh effort to test locally.

> The target must provide as much of the Rust standard library as is feasible and appropriate to provide [...].

Windows is well-supported in the standard library.

> Building the target and running the testsuite for the target must not take substantially longer than other targets

Building `i686-pc-windows-gnu` is reasonably fast.

> If running the testsuite requires additional infrastructure

GitHub Actions has Windows support, which is used for `i686-pc-windows-gnu` (on a 64-bit host), no external infrastructure is required.

> Tier 1 targets must not have a hard requirement for signed, verified, or otherwise "approved" binaries.

There are no such requirements.

> All requirements for tier 2 apply.

We will not go through Tier 2 requirements here, but they are, apart from the (less strict than Tier 1) maintainer requirements, fulfilled.
When the maintainer requirements are enforced more strictly in the future, `i686-pc-windows-gnu` (and `x86_64-pc-windows-gnu` as well) may be demoted further if no maintainers are found.

## Conclusion

Given the usage count and lack of maintenance leading to more than one requirement not being fulfilled, it becomes clear that `i686-pc-windows-gnu` is not worthy of being a Tier 1 target and is already getting much worse support than expected from a Tier 1 target.

# Explanation
[explanation]: #explanation

`i686-pc-windows-gnu` is now a [Tier 2 with Host Tools](https://doc.rust-lang.org/1.84.1/rustc/target-tier-policy.html#tier-2-target-policy) target instead of a [Tier 1 With Host Tools](https://doc.rust-lang.org/1.84.1/rustc/target-tier-policy.html#tier-1-target-policy) target.
Official builds of the standard library and rustc **continue to be distributed** for this target, but it is no longer tested in CI.
If necessary, further demotions (for example removing host tools) will not require RFCs, but go through a simpler [MCP](https://forge.rust-lang.org/compiler/mcp.html) instead.

A blog post will be made to describe the change.

# Drawbacks
[drawbacks]: #drawbacks

By no longer doing automated testing for this target, this target will likely deteriorate more quickly than with continued automated testing.

Additionally, this opens the door for further demotions in the future, like removing host tools, which could still be useful to some people.
But such demotions will always have to be justified on their own.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The maintenance requirement violation can be solved by multiple people stepping up to maintain this target. This has not happened so far.

The popularity requirement could be fulfilled by more people using this target, but this does not seem possible, as 32-bit x86 as been on a decline for a long time, as new CPU models for this architecture are no longer being made.

# Prior art
[prior-art]: #prior-art

This is the first time [since the Target Tier Policy was created](https://rust-lang.github.io/rfcs/2803-target-tier-policy.html) (note that this links to an old version, see [the rustc book](https://doc.rust-lang.org/1.84.1/rustc/target-tier-policy.html) for the latest version at the time of writing) that a Tier 1 target is being demoted.

Before that, there has been the [demotion of `i686-apple-darwin` from Tier 1 to Tier 3 in 2019](https://github.com/rust-lang/rfcs/pull/2837).
The reasoning there was mostly Apple's support being removed, which is not the case here.
The measures in this RFC are much less drastic.

The [promotion of `aarch64-apple-darwin` to Tier 1](https://github.com/rust-lang/rfcs/pull/3671) cited popularity as the major motivation, matching unpopularity as one of the motivations here.

This is a continuation of [MCP 822](https://github.com/rust-lang/compiler-team/issues/822), which contains some additional details in the description and linked Zulip stream.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far.

# Future possibilities
[future-possibilities]: #future-possibilities

`x86_64-pc-windows-gnu` will remain a Tier 1 target after this RFC.
While its popularity is more aligned with Tier 1, it suffers from the same lack of maintenance (but to a lesser degree) as its 32-bit cousin.
It may be demoted as well in the future.

`i686-pc-windows-gnu` may be demoted to a Tier 2 target without host tools in the future if it is not deemed useful enough.
This will likely happen in the near future, but is not part of this RFC.
That demotion will not need an RFC.

The `*-windows-gnullvm` targets, which are based on LLVM instead of GNU tools, may see increased maintenance and popularity in the future, replacing the `*-windows-gnu` targets.
But it seems unlikely that `i686-pc-windows-gnullvm` would ever acquire Tier 1 status.
