- Feature Name: `promote-riscv64gc-unknown-linux-gnu-to-tier-1-without-host-tools`
- Start Date: 2024/10/03
- RFC PR: TODO
- Rust Issue: TODO

# Summary
[summary]: #summary

Promote the `riscv64gc-unknown-linux-gnu` Rust target to be the first Tier-1 (without host tools) platform.


# Motivation
[motivation]: #motivation

The `riscv64gc-unknown-linux-gnu` target is [currently a Tier 2 (with host tools) Rust target](https://forge.rust-lang.org/release/platform-support.html#tier-2), in accordance with the target tier policy [here](https://doc.rust-lang.org/nightly/rustc/platform-support.html).

Since the introduction of the target, there has been an upward trend in use. Several operating system environments (Linux, FreeBSD, Android, NuttX) support RISC-V systems based on the `riscv64gc` ISA extension and this number is increasing.

During discussions with users and partners, the [RISE project](https://riseproject.dev/) has received feedback from users that they would like to use Rust, but they are hesitant due to the Tier 2 status.

In the last 2 quarters, good progress has been made in understanding and filling the gaps that remain in the path to attaining [Tier 1 (without host tools)](https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html#tier-1-target-policy) status for this target.

As a direct result, those gaps have either already been filled or are very close to being filled.

As such, this RFC aims to demonstrate what has been done.

Please note that this RFC's authors are performing this work as part of the [RISE Project](https://wiki.riseproject.dev/display/HOME/Project+RP004%3A+Support+64-bit+RISC-V+Linux+port+of+Rust+to+Tier-1).


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Currently, users of the `riscv64gc-unknown-linux-gnu` target can add it to their local installation with:

```bash
rustup target add riscv64gc-unknown-linux-gnu
```

This is possible because `riscv64gc-unknown-linux-gnu` is a tier 2 target as described in the [Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html) document, and the Rust project produces official binaries of the host tools used on the target  (eg. `cargo`) and libraries used in binaries for the target (eg. `std`).

These binaries are only "guaranteed to build," not "guaranteed to work" like they would be if the target was Tier 1. While these host tools  and libraries are created, there is no promise that all (or any) of the tests pass.

This RFC seeks to demonstrate that libraries of the target are currently in a state where all tests are passing. It seeks to demonstrate that the target sufficiently fulfills the other criteria required to promote it to be the first Tier 1 (without host tools) target.

This RFC does not seek to demonstrate that `rustc`, `cargo`, or other host platform tools are passing all tests, or that they are suitable for tier promotion.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following is a point by point breakdown of [the Tier 1 Target Policy](https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html#tier-1-target-policy) and the state of the `riscv64gc-unknown-linux-gnu` target in regards to it.

> 1.a. Tier 1 targets must have substantial, widespread interest within the
  developer community, and must serve the ongoing needs of multiple production
  users of Rust across multiple organizations or projects. These requirements
  are subjective, and determined by consensus of the approving teams

It is also generally fair to state that there is a clear upward trend in the use of `riscv64gc-unknown-linux-gnu` as a compile target. Several operating system environments (Linux, FreeBSD, Android, NuttX) support riscv64gc based systems and this trend is increasing.

One key production user of this compilation target is Google: Rust is used to implement several key Android subsystems. Here is a quote from Google, with permission:

> "Android has added support for RISC-V as a target as of Android 15, with the projected RVA23 profile as a baseline for Android 16. With Android's well-known reliance on Rust as a memory safe alternative to C/C++, it's critical to have RISC-V support at Tier-1." Lars Bergstrom (@larsbergstrom), Google.

The RISE project has received feedback from other users that they would like to use Rust, but they are hesitant due to the Tier 2 status.

> 1.b. The target maintainer team must include at least 3 developers.

There are currently 4 maintainers listed in [the target's platform page](https://doc.rust-lang.org/nightly/rustc/platform-support/riscv64gc-unknown-linux-gnu.html) including Kito Cheng, Michael Maitland, Robin Randhawa, and Craig Topper.

> 1.c. The target must build and pass tests reliably in CI, for all components that Rust's CI considers mandatory.

In https://github.com/rust-lang/rust/pull/126641 the `riscv64gc-gnu` job will be enabled in bors pre-merge tests. Those tests have passed for several months and the PR has no current blockers.

There are a few ignored tests on the platform:
   - `tests/codegen/call-llvm-intrinsics.rs`: Covered by `tests/codegen/riscv-abi/call-llvm-intrinsics.rs` instead.
   - `tests/codegen/catch-unwind.rs`: The closure is another function, placed before fn foo so CHECK can't find it.
   - `tests/codegen/repr/transparent.rs`: Ignored because RISC-V has an i128 type used with `test_Vector`.
   - `tests/run-make/inaccessible-temp-dir/`: Ignored because the test container runs as root and the test cannot create a directory it cannot access. (This issue is also present in arm test containers)
   - `tests/run-make/rustdoc-io-error/rmake.rs`: Ignored for the same reason as `inaccessible-temp-dir` above.
   - `tests/run-make/split-debuginfo/`: On this platform only `-Csplit-debuginfo=off` is supported, see [#120518](https://github.com/rust-lang/rust/pull/120518).
   - `tests/ui/debuginfo/debuginfo-emit-llvm-ir-and-split-debuginfo.rs`: On this platform `-Csplit-debuginfo=unpacked` is unstable, see [#120518](https://github.com/rust-lang/rust/pull/120518).


> 1.d. The target must provide as much of the Rust standard library as is feasible
   and appropriate to provide. For instance, if the target can support dynamic
   memory allocation, it must provide an implementation of `alloc` and the
   associated data structures.

`alloc` is implemented. There is currently no specific `std` functionality disabled for `riscv64gc-unknown-linux-gnu`.

> 1.e. Building the target and running the testsuite for the target must not take
   substantially longer than other targets, and should not substantially raise
   the maintenance burden of the CI infrastructure.

Running the `riscv-gnu` job from scratch takes approximately 73 minutes on CI. This is less time than the `i686-gnu` (78 minutes) job, or the `x86_64-gnu` job (93 minutes). It's fair to conclude that this proposal would not substantially lengthen CI jobs.

The existing `riscv64-gnu` test job is nearly identical to the `armhf-gnu` job and works as expected in existing processes. Emulating `riscv64gc-unknown-linux-gnu` can be done using normal tools like `qemu`, `docker`, or `lima` like other platforms such as `aarch64-unknown-linux-gnu` or `x86_64-unknown-linux-gnu`. It's fair to conclude that this proposal would not substantially raise the maintenance burden of the CI infrastructure.

> 1.f. If running the testsuite requires additional infrastructure (such as physical
   systems running the target), the target maintainers must arrange to provide
   such resources to the Rust project, to the satisfaction and approval of the
   Rust infrastructure team.

Running the test suite does not require physical systems running the target. Emulating `riscv64gc-unknown-linux-gnu` can be done using normal tools like `qemu`, `docker`, or `lima` like other platforms such as `aarch64-unknown-linux-gnu` or `x86_64-unknown-linux-gnu`.

An emulated or real `riscv64gc-unknown-linux-gnu` can make use of the existing tier 2 host tools, or self-bootstrap in the event the host system cannot cross compile the appropriate artifacts to run the necessary tests.


> 1.g. Tier 1 targets must not have a hard requirement for signed, verified, or
   otherwise "approved" binaries. Developers must be able to build, run, and
   test binaries for the target on systems they control, or provide such
   binaries for others to run. (Doing so may require enabling some appropriate
   "developer mode" on such systems, but must not require the payment of any
   additional fee or other consideration, or agreement to any onerous legal
   agreements.)

No hard requirement of signing, verifying, or "approving" binaries exists for the `riscv64gc-unknown-linux-gnu` platform.

> 2.a. The long term viability of the existence of a target specific ecosystem should be clear.

RISC-V has a roughly 9 year history and there are a variety of vendors providing silicon using this instruction set. They include (but are not limited to) [Alibaba Cloud](https://www.alibabagroup.com/), [AllWinner](http://www.allwinnertech.com/), [antimicro](http://antmicro.com/), [BeagleBoard](https://beagleboard.org/), [Deep Computing](https://deepcomputing.io/), [Microchip](https://www.microchip.com/), [RIOS](http://rioslab.org/), [SiFive](https://sifive.com/), [SOPHGO](https://en.sophgo.com/site/index.html), and [StarFive](https://starfivetech.com/).

There is already an existing ecosystem of downstream users of this target. [Debian](https://wiki.debian.org/Ports/riscv64), [Ubuntu](https://ubuntu.com/download/risc-v) and [OpenSUSE](https://en.opensuse.org/openSUSE:RISC-V) all provide `riscv64` distributions of Linux and also package Rust. [Scaleway](https://labs.scaleway.com/en/em-rv1/) is offering `riscv64gc-unknown-linux-gnu` cloud instances.

Some of the ongoing development of this target has been supported by the [RISE Project](https://riseproject.dev/) which represents a broad array of industrial interests including, for example, Google, Intel, NVIDIA, and SiFive.

It is fair to say that the target specific ecosystem has been a viable target for some time now, and that this is likely to continue into the long term.

> 2.b. The long term viability of supporting the target should be clear.

It is hard to concretely quantify this aspect. This work was initiated and supported by the [RISE Project](https://riseproject.dev/) and there is an intention to continue to support the target and eventually propose a *Tier 1 with Host Tools* RFC when sufficiently fast hardware exists.

# Drawbacks
[drawbacks]: #drawbacks

Adopting the platform would require additional commitments by the Rust project. Future contributions may impact the target and cause changes to become delayed or halted entirely due to problems on the target.

In general, it should be uncomplicated for contributors to build for and use a `riscv64gc-unknown-linux-gnu` emulator like `qemu`, `docker`, or `lima`. Additionally, the platform is a `*-unknown-linux-gnu` target which is generally quite well understood, contributors do not need to learn what could be an otherwise unfamiliar operating system.

This target does not place significant burdens on the project that would not be present on any other target.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There exist two alternatives: Promoting the target to Tier 1 (with host tools), or not promoting the target at all.

## Tier 1 (with host tools)

The [Tier 1 Target Tier Policy](https://doc.rust-lang.org/nightly/rustc/target-tier-policy.html#tier-1-target-policy) section 1.e states:

> 1.e. Building the target and running the testsuite for the target must not take
>   substantially longer than other targets...

During testing on [Scaleway Elastic Metal RV1](https://labs.scaleway.com/en/em-rv1/) it was determined that running a full `x.py test` run takes roughly 6 hours. A similar amount of time was taken on a 6 CPU, 16GB RAM VM.

During this testing, it was noted that all tests pass.

It's fair to conclude that existing available virtualization and hardware for `riscv64gc-unknown-linux-gnu` takes substantially longer to run the test suite than other targets. If sufficiently fast hardware existed, this RFC would be for Tier-1 with host tools instead.

## Not promoting the target

Not promoting the target could lead to a situation where the `riscv64gc-unknown-linux-gnu` tests are no longer passing, and this could impact users.

Anecdotally, not having the Tier 1 'badge' has been seen to become an obstacle to increasing mindshare in Rust for this target. Organisations tend to associate a Tier 1 categorisation with better quality, suitability for key projects, longevity etc. With a reasonably justified Tier 1 'badge' in place, the likelihood is that such organisations will tend to pick up and promote the use of Rust in production.

Because of this, not proceeding with promoting `riscv64gc-unknown-linux-gnu` to Tier 1 could result in a degradation of the state of the platform and impact users.

# Prior art
[prior-art]: #prior-art

There are currently no Tier 1 (without host tools) targets, so existing Tier 1 targets represent the closest prior-art. In addition, no RISC-V based target has ever been promoted to Tier 1 (with or without host tools).

Therefore, the `riscv64gc-unknown-linux-gnu` target is in somewhat uncharted territory.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

No unresolved questions or issues remain.

# Future possibilities
[future-possibilities]: #future-possibilities

As the first non i686/x86_64/aarch64 target to be considered for promotion to Tier-1, the `riscv64gc-unknown-linux-gnu` target will likely set a precedent for other `riscv*` targets to follow in the future.

As the first Tier 1 (without Host Tools) target, the `riscv64gc-unknown-linux-gnu` target will likely set a precedent for other Tier 1 (without host tools) targets to follow in the future.
