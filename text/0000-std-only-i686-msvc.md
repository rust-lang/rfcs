- Feature Name: none
- Start Date: 2026-07-02
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Change `i686-pc-windows-msvc` from a Tier 1 target with `std` support, and Tier 1 host tools, into *only* a Tier 1 target with `std support`.

# Motivation
[motivation]: #motivation

## Background

Rust has, since 1.0, supported every architecture for which Microsoft has supported an official build of the Windows operating system.
This has included x86, x86-64, AArch32, and AArch64. We have supported its use with both official MSVC-based toolchains as well as libre GNU-based toolchains.

Even in 2026, Rust continues to support 32-bit x86 Windows hosts and supply toolchains for them.
We build a toolchain and run all compiler tests for the `i686-pc-windows-msvc` target.
However, this increasingly seems like a strange decision, as it does not comport with the support for 32-bit Windows that Microsoft offers,
nor does it match actual ongoing usage of `i686-pc-windows-msvc` as a target.

In October of 2025, Microsoft's general support for Windows 10 concluded. That was the last version of Windows that distributed a 32-bit version.
Further, the distance between now and the availability of 64-bit x86 CPUs is over 20 years, starting from the Athlon 64 in 2003.

Rust currently supports the following major Windows targets.
Download counts for the host toolchain and `std` were extracted from the public dashboard [covering the period from 2026-06-01 to 2026-06-30][static-rlo-dl-counts].

| Name | Tier | `rustc` download count | `std` download count
| -------- | -------- | ---- | ------ |
| `i686-pc-windows-msvc` | 1 | 436.26k | 6.83M |
| `x86_64-pc-windows-msvc` | 1 | 34.65M | 21.05M |
| `x86_64-pc-windows-gnu` | 1 | 1.32M | 8.64M |
| `aarch64-pc-windows-msvc` | 2 | 5.18M | 6.96M |
| `i686-pc-windows-gnu` | 2 | 72.09k | 3.62M |

To put the download numbers into perspective, some other targets:

| Name | Tier | `rustc` download count | `std` download count
| -------- | -------- | --- | ------ |
| `aarch64-apple-darwin` | 1 | 37.66M | 26.79M |
| `aarch64-unknown-linux-gnu` | 1 | 48.85M | 40.41M |
| `i686-unknown-linux-gnu` | 1 | 305.19k | 4.22M |
| `x86_64-apple-darwin` | 1 | 15.05M | 16.53M |
| `x86_64-unknown-linux-gnu` | 1 | 640.95M | 286.75M |
| `x86_64-unknown-freebsd`   | 2 | 276.94k | 4.36M |
| `x86_64-unknown-netbsd`    | 2 | 63.75k | 3.28M |

From this we can see that `i686-pc-windows-msvc`'s `std` receives far more downloads than its toolchain, because it is primarily used by cross-compiling.
Its only peer in host usage at tier 1 is i686-unknown-linux-gnu, which is also a 32-bit host.
Many 64-bit hosts at tier 2 see significantly more usage, and not for no reason.

[static-rlo-dl-counts]: https://p.datadoghq.com/sb/3a172e20-e9e1-11ed-80e3-da7ad0900002-60425c7cb1b7beb2e8959a305a301c0c?fromUser=false&refresh_mode=sliding&from_ts=1736618152507&to_ts=1739210152507&live=true

### Compiling to and from 32-bit x86

There are many benefits to using `i686-pc-windows-msvc` as a target.
In particular, it is supported via means compatibility layers such as Windows 32-bit on Windows 64-bit ("WoW64"),
across almost all supported (and most unsupported) versions of Windows, including 10 and 11.
For Windows Server, WoW64 remains available as an optional component.
For applications with lower memory use, being able to build an executable that runs on almost every Windows, and sometimes even Linux, is fairly desirable.

These reasons do not apply as neatly to rustc, however. 64-bit hosts are simply more capable of running a compiler.
For many builds, rustc's resident set size remains well under 1GiB, and thus within a 32-bit address space, but "many builds" is not "all".
When builds wish to use more advanced features such as link-time optimization, the requirements can often exceed 4GiB.
Due to the hard limit for addressing, a 32-bit host toolchain is already in danger of not having first-class support for rustc's capabilities.

Further, it is unclear whether the duty of tier 1 host testing is even being met. The 32-bit x86 host toolchains are tested by executing them on a 64-bit host.
This is a deliberate feature of x86 and the operating systems that support this execution mode, but it is not the same as running them under a 32-bit kernel.
To the extent that these tests have been run, they have incurred increasing maintenance issues.

### Maintenance Problems

Rust has many outstanding issues related to `i686-pc-windows-msvc`:

- https://github.com/rust-lang/rust/issues/44282 - An unaddressed problem with `export_name` on 32-bit Windows.
- https://github.com/rust-lang/rust/issues/73527 - An undiagnosed problem with building Rust code on 32-bit Windows.
- https://github.com/rust-lang/rust/issues/72212 - An undiagnosed problem with building Rust code on 32-bit Windows.
- https://github.com/rust-lang/rust/issues/110290 - An undiagnosed issue in LLVM's code on 32-bit Windows.
- https://github.com/rust-lang/rust/issues/112480 - A seemingly-unfixable problem involving 32-bit Windows using self-contradictory layout rules for C code.
- https://github.com/rust-lang/rust/issues/134683 - A 32-bit test issue "temporarily" marked 64-bit only to unblock the tree's continuous integration.
- https://github.com/rust-lang/rust/issues/158378 - A hang while building documentation. We have stopped building some of our documentation for this target.
- https://github.com/rust-lang/rust/issues/159076 - An issue with creating processes on Windows that incurs additional limitations on `i686-pc-windows-msvc`.

That the 32-bit x86 architecture is unusual, and made moreso by how Windows operates on it, has been noted by Windows experts[^2].

[^2]: https://devblogs.microsoft.com/oldnewthing/20220418-00/?p=106489

## Target Tier Policy Requirements

With this knowledge, we can look at the [Tier 1 requirements](https://doc.rust-lang.org/1.96.1/rustc/target-tier-policy.html#tier-1-target-policy) of the target tier policy to check whether they are fulfilled.

> *Tier 1 targets must have substantial, widespread interest within the developer community, and must serve the ongoing needs of multiple production users of Rust across multiple organizations or projects.*

This is in fact met, considering the usage of its target's standard library, but not for its usage as a host toolchain.

> The target maintainer team must include at least 3 developers.

This has been met. However, the target's maintainer team has agreed on some degree of demotion, so we cannot consider its current maintainer team for the specific purpose of evaluating it as a tier 1 host.

> The target must build and pass tests reliably in CI, for all components that Rust's CI considers mandatory.

We have recently had to disable building rustc's documentation for this target per [rust-lang/rust#158378]. This sort of thing seems likely to be a recurrent issue.

[rust-lang/rust#158378]: https://github.com/rust-lang/rust/issues/158378

> The target must not disable an excessive number of tests or pieces of tests in the testsuite in order to do so. This is a subjective requirement.

We ignore it in 1 run-make test due to not being able to fully symbolicate its backtraces and 10 UI or codegen tests due to issues with the target's ABI alignment.
While we may be able to overcome the backtrace problem if someone invests technical effort, so far the ABI alignment issue seems to be fundamental and unfixable, so we cannot test some of our functionality for the platform.

> The target must provide as much of the Rust standard library as is feasible and appropriate to provide [...].

Windows is well-supported in the standard library, but we have found quirks in the implementation, such as [rust-lang/rust#159076], that seemingly arise when we call into Windows libraries and the call is rerouted into legacy APIs for specifically 32-bit support. Thus our support on 32-bit Windows can be fairly described as reduced due to fundamental issues in the Windows platform, but is otherwise technically sufficient for usage.

[rust-lang/rust#159076]: https://github.com/rust-lang/rust/issues/159076

> Building the target and running the testsuite for the target must not take substantially longer than other targets

The jobs which implement building and testing our toolchain for `i686-pc-windows-msvc` are not the slowest.
That peculiar distinction currently goes to our aarch64-apple jobs.
The i686-msvc jobs are, however, near the top, despite being already split across multiple runners.

> If running the testsuite requires additional infrastructure

GitHub Actions has Windows support, which is used for `i686-pc-windows-msvc` (on a 64-bit host), so no external infrastructure is required.

> Tier 1 targets must not have a hard requirement for signed, verified, or otherwise "approved" binaries.

There are no such requirements.

> All requirements for tier 2 apply.

These are redundant with the considerations above.

## Conclusion

For these reasons and more, the target's maintainers have agreed that the target's host tools should not be provided, much less at Tier 1 support.

Given the lower usage count, lack of maintenance, and diminishing upstream support all leading to rapid deterioration, it becomes clear that host tools for `i686-pc-windows-msvc` are becoming untenable to maintain.

# Explanation
[explanation]: #explanation

`i686-pc-windows-msvc` is now a [Tier 1] target that implements `std`, instead of a [Tier 1 With Host Tools][Tier 1] target.

Official builds of the standard library **will continue to be distributed** for this target, and it will continue to receive some testing, notably including `std`'s library testing.

We will no longer build or test the target *as a compiler host*.

A blog post will be made to describe the change.

[Tier 1]: https://doc.rust-lang.org/1.96.1/rustc/target-tier-policy.html#tier-1-target-policy

# Drawbacks
[drawbacks]: #drawbacks

This will be the first target for which we have this sort of configuration.
Since a large amount of our test suite is the compiler test suite, it is unclear how good our coverage using the standard library's test suite alone will be.
It will certainly result in less functionality testing for certain end-to-end concerns.

Some people may intentionally use this toolchain and would be disadvantaged by us ceasing to distribute it.
However, it is unclear how much intentional usage of the toolchain is included in its statistical usage, as we do not have telemetry in `rustup`
which would allow us to answer questions like "Are most users of the `i686-pc-windows-msvc` toolchain running a 32-bit Windows kernel or a 64-bit Windows kernel?" 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

During the drafting of this RFC, the windows-msvc maintainers and the Rust compiler team as a whole were consulted about what should be the fate of this target. All that responded at the time of writing this RFC unanimously agreed on some form of demotion.
Thus, in order to fill the maintenance requirement of a tier 1 host, we can infer we would need more maintainers that commit additional time.

The popularity requirement could be fulfilled by more people using the host tools, but this seems unlikely due to the RAM limitations and new Windows kernels moving exclusively towards 64-bit implementations.
It simply does not make sense to use a host compiler that has access to less RAM than the computer has available if the compiler may ever make use of such space.
It is also difficult to argue that the functionality is equal, even when address limits are not exceeded, if 32-bit x86 Windows APIs can react differently.
For these reasons, we should not expect this userbase to grow significantly.

# Prior art
[prior-art]: #prior-art

The `i686-pc-windows-gnu` target was demoted according to the Target Tier Policy ([RFC#2803][ttp-rfc], latest version [in the rustc book][ttp-v1.97]]).
It was demoted wholesale to tier 2 as it had more severe test failures and lacked the same usage for its standard library.
The `windows-gnu` target however did retain its host tools, but these may prove to be difficult to maintain for much longer, for similar reasons.

Before that, there has been the [demotion of `i686-apple-darwin` from Tier 1 to Tier 3 in 2019](https://github.com/rust-lang/rfcs/pull/2837). The reasoning there was mostly Apple's support being removed, which is similar to this case. The measures in this RFC are much less drastic.

The [promotion of `aarch64-apple-darwin` to Tier 1](https://github.com/rust-lang/rfcs/pull/3671) cited popularity as the major motivation, matching unpopularity as one of the motivations here.

[ttp-rfc]: https://rust-lang.github.io/rfcs/2803-target-tier-policy.html
[ttp-v1.97]: https://doc.rust-lang.org/1.97.1/rustc/target-tier-policy.html

# Unresolved questions
[unresolved-questions]: #unresolved-questions

What is the minimum bound we expect for tier 1 target testing, and can `i686-pc-windows-msvc` meet it?

# Future possibilities
[future-possibilities]: #future-possibilities

Host tools could be added to the target again at tier 2 support, without compiler testing.
This change is omitted because we will only reap the full benefit of removing this target from our continuous integration if we stop building its toolchain. Addressing the issues it causes with additional work would allow adding such lower-tier support with only the simpler [MCP] process instead.

[MCP]: https://forge.rust-lang.org/compiler/mcp.html
