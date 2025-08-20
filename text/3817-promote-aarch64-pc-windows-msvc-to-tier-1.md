- Feature Name: promote-aarch64-pc-windows-msvc-to-tier-1
- Start Date: 2025-05-22
- RFC PR: [rust-lang/rfcs#3817](https://github.com/rust-lang/rfcs/pull/3817)
- Rust Issue: [rust-lang/rust#145671](https://github.com/rust-lang/rust/issues/145671)

# Summary
[summary]: #summary

Promote aarch64-pc-windows-msvc to Tier 1 with Host Tools.

# Motivation
[motivation]: #motivation

About [30% of Rust users use Windows][survey-2024], while the majority of these developers and their
customers are using x64 hardware, the usage of Arm64 Windows has been growing since it was first
made available in Windows 10, and has been accelerating, especially with the availability of the
SnapDragon X processors.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

No changes required: Rust tooling for Arm64 Windows has been available for a while now so this
doesn't affect the end user experience.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Tier 1 targets must adhere to the [Tier 1 Target Policy][tier-1-policy]. Going through these
requirements point-by-point:

> Tier 1 targets must have substantial, widespread interest within the developer community, and must
> serve the ongoing needs of multiple production users of Rust across multiple organizations or
> projects. These requirements are subjective, and determined by consensus of the approving teams.

As mentioned above, Windows users comprise a substantial proportion of Rust developers, and Arm64
hardware is increasingly being used by them and their customers.

For the past two years, Arm64 PCs have accounted for 10-14% of Windows sales:
- <https://www.prnewswire.com/news-releases/2025-will-see-ai-pcs-become-the-new-normal-but-arm-based-pcs-will-not-grow-out-of-its-minority-segment-302340341.html>
- <https://www.counterpointresearch.com/insights/arm-based-pcs-to-nearly-double-market-share-by-2027/>
- <https://www.tomshardware.com/pc-components/cpus/arm-pc-market-share-shrinks-mercury-research>

Overall, they are estimated to account for 1 to 1.5% of the Windows population:
- <https://www.canalys.com/insights/arming-your-pc-for-the-upcoming-ai-era>
- <https://www.techpowerup.com/329255/snapdragon-x-failed-qualcomm-sold-720-000-pcs-in-q3-around-0-8-market-share>

While that's a small relative number, in absolute terms it works out to 140 to 210 million devices.

For Rust itself, per the [Rust download dashboard][download-dashboard] `aarch64-pc-windows-msvc` is
the third most downloaded rustc non-tier 1 flavor (after x64 and Arm64 Linux musl flavors) and sees
~3% the number of downloads of `x86_64-pc-windows-msvc`.

> The target maintainer team must include at least 3 developers.

`aarch64-pc-windows-msvc` is supported by [the 5 `*-pc-windows-msvc` maintainers][msvc-support].

> The target must build and pass tests reliably in CI, for all components that Rust's CI considers
> mandatory.
> The target must not disable an excessive number of tests or pieces of tests in the testsuite in
> order to do so. This is a subjective requirement.

[The `dist-aarch64-msvc` CI job has been running reliably for over 4 years now][promote-tier-2],
and I have [new CI jobs working where Rust is built and tested on Arm64 Windows runners][ci-draft-pr].

The following tests had to be disabled for `aarch64-pc-windows-msvc`:
- [Tests in `std::fs` that require symlinks][disable-fs]: this is a limitation of the runner image
  and I've [filed an issue to have it fixed][fix-symlinks].
- [Various debug info tests][disable-debuginfo]
  - `tests/debuginfo/step-into-match.rs`: Stepping out of functions behaves differently.
  - `tests/debuginfo/type-names.rs`: Arm64 Windows cdb doesn't support JavaScript extensions. I've
    filed a bug internally with the debugger team to have this fixed.
  - `tests/ui/runtime/backtrace-debuginfo.rs`: Backtraces are truncated. I've filed
    [an issue to investigate this][backtrace-issue].

> The target must provide as much of the Rust standard library as is feasible and appropriate to
> provide.

The full Standard Library is available.

> Building the target and running the testsuite for the target must not take substantially longer
> than other targets, and should not substantially raise the maintenance burden of the CI
> infrastructure.

[A `try` run of the new CI jobs completed in under 2 hours.][try-job]

> If running the testsuite requires additional infrastructure (such as physical systems running the
> target), the target maintainers must arrange to provide such resources to the Rust project, to the
> satisfaction and approval of the Rust infrastructure team.
> Such resources may be provided via cloud systems, via emulation, or via physical hardware.

The new CI jobs use the free [`windows-11-arm` runners provided by GitHub][runner-announcement].

> Tier 1 targets must not have a hard requirement for signed, verified, or otherwise "approved"
> binaries. Developers must be able to build, run, and test binaries for the target on systems they
> control, or provide such binaries for others to run. (Doing so may require enabling some
> appropriate "developer mode" on such systems, but must not require the payment of any additional
> fee or other consideration, or agreement to any onerous legal agreements.)

There are no differences between x64 and Arm64 Windows in this regard.

> All requirements for tier 2 apply.

Going through the Tier 2 policies:

> The target must not place undue burden on Rust developers not specifically concerned with that
> target. Rust developers are expected to not gratuitously break a tier 2 target, but are not
> expected to become experts in every tier 2 target, and are not expected to provide target-specific
> implementations for every tier 2 target.

Understood.

> The target must provide documentation for the Rust community explaining how to build for the
> target using cross-compilation, and explaining how to run tests for the target. If at al
> possible, this documentation should show how to run Rust programs and tests for the target using
> emulation, to allow anyone to do so. If the target cannot be feasibly emulated, the documentation
> should explain how to obtain and work with physical hardware, cloud systems, or equivalent.
> The target must document its baseline expectations for the features or versions of CPUs, operating
> systems, libraries, runtime environments, and similar.

Understood, as part of the promotion PR I will add a page to Platform Support.

> The code generation backend for the target should not have deficiencies that invalidate Rust
> safety properties, as evaluated by the Rust compiler team.

There are no known deficiencies in LLVM's support for Arm64 Windows.

> If the target supports C code, and the target has an interoperable calling convention for C code,
> the Rust target must support that C calling convention for the platform via `extern "C"`. The C
> calling convention does not need to be the default Rust calling convention for the target,
> however.

`extern "C"` correctly works for calling C code.

> Tier 2 targets should, if at all possible, support cross-compiling. Tier 2 targets should not
> require using the target as the host for builds, even if the target supports host tools.

`aarch64-pc-windows-msvc` can be cross-compiled from x86 and x64 Windows, or other platforms that
can run those tools.

> In addition to the legal requirements for all targets (specified in the tier 3 requirements),
> because a tier 2 target typically involves the Rust project building and supplying various
> compiled binaries, incorporating the target and redistributing any resulting compiled binaries
> (e.g. built libraries, host tools if any) must not impose any onerous license requirements on any
> members of the Rust project, including infrastructure team members and those operating CI systems.

There are no such license requirements for Arm64 Windows code.

> Tier 2 targets must not impose burden on the authors of pull requests, or other developers in the
> community, to ensure that tests pass for the target.

Understood.

> The target maintainers should regularly run the testsuite for the target, and should fix any test
> failures in a reasonably timely fashion.

Understood, and this will be automated once promoted to Tier 1.

# Drawbacks
[drawbacks]: #drawbacks

The `windows-11-arm` runners provided by GitHub are relatively new, and so we do not know what the
availability or reliability of these runners will be.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

`aarch64-pc-windows-msvc` could be left as a Tier 2 with Host Tools target, but given the importance
of this target to Microsoft and the increasing usage of Arm64 by Windows users, it will become more
and more likely that issues with this target will need to be treated as critical. Catching issues
early in development will prevent the need to Beta and Stable backports.

# Prior art
[prior-art]: #prior-art

- [RFC 2959][rfc-2959] promoted `aarch64-unknown-linux-gnu` to Tier 1.
- [RFC 3671][rfc-3671] promoted `aarch64-apple-darwin` to Tier 1.
- [`stdarch` has been using using `windows-11-arm` runners][stdarch-pr] since early May.
- LLVM has dedicated [Arm64 Windows builders][llvm-builders].

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

* Adding Arm64 Windows jobs to more Rust repos, such as `cargo`.
* Promoting `arm64ec-pc-windows-msvc` to Tier 1.
* Add a `aarch64-pc-windows-gnu` target.
* Promote `aarch64-pc-windows-gnullvm` to Tier 1.

[backtrace-issue]: https://github.com/rust-lang/rust/issues/140489
[ci-draft-pr]: https://github.com/rust-lang/rust/pull/140136
[disable-debuginfo]: https://github.com/rust-lang/rust/pull/140755
[disable-fs]: https://github.com/rust-lang/rust/pull/140759
[download-dashboard]: https://p.datadoghq.com/sb/3a172e20-e9e1-11ed-80e3-da7ad0900002-60425c7cb1b7beb2e8959a305a301c0c?fromUser=false&refresh_mode=sliding&from_ts=1747503249629&to_ts=1750095249629&live=true
[fix-symlinks]: https://github.com/actions/partner-runner-images/issues/94
[llvm-builders]: https://lab.llvm.org/buildbot/#/builders/161
[msvc-support]: https://doc.rust-lang.org/nightly/rustc/platform-support/windows-msvc.html
[platform-support]: https://github.com/rust-lang/rust/blob/e3892a40a9d06034fdf2432a9d3d29fa97726299/src/doc/rustc/src/platform-support.md?plain=1#:~:text=aarch64%2Dpc%2Dwindows%2Dmsvc
[promote-tier-2]: https://github.com/rust-lang/rust/pull/75914
[rfc-2959]: https://rust-lang.github.io/rfcs/2959-promote-aarch64-unknown-linux-gnu-to-tier1.html
[rfc-3671]: https://rust-lang.github.io/rfcs/3671-promote-aarch64-apple-darwin-to-tier-1.html
[runner-announcement]: https://github.com/orgs/community/discussions/155713
[stdarch-pr]: https://github.com/rust-lang/stdarch/pull/1785
[survey-2024]: https://blog.rust-lang.org/2025/02/13/2024-State-Of-Rust-Survey-results
[tier-1-policy]: https://doc.rust-lang.org/rustc/target-tier-policy.html#tier-1-target-policy
[try-job]: https://github.com/rust-lang-ci/rust/actions/runs/14871501014
