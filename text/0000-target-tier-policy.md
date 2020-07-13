- Feature Name: `target_tier_policy`
- Start Date: 2019-09-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

We should have an official policy for adding new tier 3 targets, and for
raising targets to tier 2 (with `rustup` builds) or even tier 1. This RFC
codifies the requirements for targets at each tier.

# Motivation
[motivation]: #motivation

Rust developers regularly implement new targets in the Rust compiler, and
reviewers of Rust pull requests for such new targets would like a clear,
consistent policy to cite for accepting or rejecting such targets. Currently,
individual reviewers do not know what overall policy to apply, and whether to
apply solely their own judgment or defer to a Rust governance team.

Rust developers regularly ask how they can raise an existing target to tier 2
(and in particular how they can make it available via `rustup`), and
occasionally ask what it would take to add a new tier 1 target. The Rust
project has no clear policy for target tiers. People not only don't know, they
don't know who to ask or where to start.

See <https://forge.rust-lang.org/release/platform-support.html> for more
information about targets and tiers.

Once accepted, the policy sections of this RFC should be posted on
<https://forge.rust-lang.org/> in a "Target Tier Policy" section; this RFC will
not be the canonical home of the up-to-date target tier policy.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

At a high level, the three tiers break down as follows:

- Tier 3 targets provide no guarantees of support.
- Tier 2 targets will always build, but may not pass tests.
- Tier 1 targets will always build and pass tests.

Adding a new tier 3 target imposes minimal requirements; we focus primarily on
avoiding disruption to other ongoing Rust development.

Tier 2 and tier 1 targets place work on the Rust community as a whole, to avoid
breaking the target. Thus, these tiers require commensurate efforts from the
maintainers of the target, to demonstrate value and to minimize any disruptions
to ongoing Rust development.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Rust targets fall into three "tiers" of support:
- Tier 3 targets, which provide no guarantees of support.
- Tier 2 targets, which will always build but not pass tests.
- Tier 1 targets, which will always build and pass tests.

This policy defines the requirements for accepting a proposed target at a given
level of support.

Each tier builds on all the requirements from the previous tier, unless
overridden by a stronger requirement.

The policy for each tier also documents the Rust governance teams that must
approve the addition of any target at that tier. Those teams are responsible
for reviewing and evaluating the target, based on these requirements and their
own judgment. Those teams may apply additional requirements, including
subjective requirements, such as to deal with issues not foreseen by this
policy. (Such requirements may subsequently motivate additions of this policy.)

While these criteria attempt to document the policy, that policy still involves
human judgment. Targets must fulfill the spirit of the requirements as well, as
determined by the judgment of the approving teams. Reviewers and team members
evaluating targets and target-specific patches should always use their own best
judgment regarding the quality of work, and the suitability of a target for the
Rust project.

Before filing an issue or pull request (PR) to introduce or promote a target,
the target should already meet the corresponding tier requirements. (This does
not preclude a target's maintainers using issues to track requirements that
have not yet been met, as appropriate; however, before officially proposing the
introduction or promotion of a target, it should meet all of the necessary
requirements.) A target proposal is encouraged to quote the corresponding
requirements verbatim as part of explaining how the target meets those
requirements.

In this policy, the words "must" and "must not" specify absolute requirements
that a target must meet to qualify for a tier. The words "should" and "should
not" specify requirements that apply in almost all cases, but for which the
approving teams may grant an exception for good reason.  The word "may"
indicates something entirely optional, and does not indicate guidance or
recommendations. This language is based on [IETF RFC
2119](https://tools.ietf.org/html/rfc2119).

## Tier 3 target policy

At this tier, the Rust project provides no official support for a target, so we
place minimal requirements on the introduction of targets.

A proposed new tier 3 target must be reviewed and approved by a member of the
compiler team based on these requirements. The reviewer may choose to gauge
broader compiler team consensus via a Major Change Proposal (MCP).

A proposed target or target-specific patch that substantially changes code
shared with other targets (not just target-specific code) must be reviewed and
approved by the appropriate team for that shared code before acceptance.

- Any new tier 3 target must have a designated developer or developers (the
  "target maintainers") on record to be CCed when issues arise regarding the
  target.
- Tier 3 targets must use naming consistent with any existing targets; for
  instance, a target for the same CPU or OS as an existing Rust target should
  use the same name for that CPU or OS. Targets should normally use the same
  names as used elsewhere in the broader ecosystem beyond Rust (such as in
  other toolchains), unless they have a very good reason to diverge.
- Tier 3 targets may have unusual requirements to build or use, but must not
  create legal issues for the Rust project or for developers who work on those
  targets.
  - The target must not introduce license incompatibilities.
  - Anything added to the Rust repository must be under the standard Rust
    license (`MIT OR Apache-2.0`).
  - The target must not cause the Rust tools or libraries built for any other
    host (even when supporting cross-compilation to the new target) to depend
    on any new dependency less permissive than the Rust licensing policy. This
    applies whether the dependency is a Rust crate that would require adding
    new license exceptions (as specified by `src/tools/tidy/src/deps.rs` in the
    rust-lang/rust repository), or whether the dependency is a native library
    or binary. In other words, the introduction of the target must not cause a
    user installing or running a version of Rust or the Rust tools to be
    subject to any new license requirements.
  - If the target supports building host tools (such as `rustc` or `cargo`),
    those host tools must not depend on proprietary (non-FOSS) libraries, other
    than ordinary runtime libraries supplied by the platform and commonly used
    by other binaries built for the target. For instance, `rustc` built for the
    target may depend on a common proprietary C runtime library or console
    output library, but must not depend on a proprietary code generation
    library or code optimization library. Rust's license permits such
    combinations, but the Rust project has no interest in maintaining such
    combinations within the scope of Rust itself, even at tier 3.
  - New targets should not require proprietary (non-FOSS) components to link a
    functional binary or library.
- Tier 3 targets should attempt to implement as much of the standard libraries as
  possible and appropriate (`core` for most targets, `alloc` for any target
  with a standard memory allocator, `std` for targets with an operating
  system), but may leave some code `unimplemented!()`, whether because the
  target makes it impossible to implement or challenging to implement. The
  authors of pull requests are not obligated to avoid calling any portions of
  the standard library on the basis of a tier 3 target not implementing those
  portions.
- The target must provide documentation for the Rust community explaining how
  to build for the target, using cross-compilation if possible. If the target
  supports running tests (even if they do not pass), the documentation must
  explain how to run tests for the target, using emulation if possible or
  dedicated hardware if necessary.
- Tier 3 targets must not impose burden on the authors of pull requests, or
  other developers in the community, to maintain the target. In particular,
  do not post comments (automated or manual) on a PR that suggests a block on
  the PR based on the target. (A PR author may choose to help with a tier 3
  target, but is not required to.)
- Patches adding or updating tier 3 targets must not break any existing tier 2
  or tier 1 target, and must not break another tier 3 target without approval
  of either the compiler team or the maintainers of the other tier 3 target.
  - In particular, this may come up when working on closely related targets,
    such as variations of the same architecture with different features. Avoid
    introducing unconditional uses of features that another variation of the
    target may not have; use conditional compilation or runtime detection, as
    appropriate, to let each target run code supported by that target.

If a tier 3 target stops meeting these requirements, or the target maintainers
no longer have interest or time, or the target shows no signs of activity and
has not built for some time, or removing the target would improve the quality
of the Rust codebase, we may post a PR to remove it; any such PR will be CCed
to the target maintainers (and potentially other people who have previously
worked on the target), to check potential interest in improving the situation.

## Tier 2 target policy

At this tier, the Rust project guarantees that a target builds, and will reject
patches that fail to build on a target. Thus, we place requirements that ensure
the target will not block forward progress of the Rust project.

A proposed new tier 2 target must be reviewed and approved by the compiler team
based on these requirements. Such review and approval may occur via a Major
Change Proposal (MCP).

In addition, the infrastructure team must approve the integration of the target
into Continuous Integration (CI), and the tier 2 CI-related requirements. This
review and approval may take place in a PR adding the target to CI, or simply
by an infrastructure team member reporting the outcome of a team discussion.

- A tier 2 target must have value to people other than its maintainers.
- Any new tier 2 target must have a designated team of developers (the "target
  maintainers") available to consult on target-specific build-breaking issues,
  or if necessary to develop target-specific language or library implementation
  details. This team must have at least 2 developers.
  - The target maintainers should not only fix target-specific issues, but
    should use any such issue as an opportunity to educate the Rust community
    about portability to their target, and enhance their documentation of the
    target.
- The target must not place undue burden on Rust developers not specifically
  concerned with that target. Rust developers are expected to not gratuitously
  break a tier 2 target, but are not expected to become experts in every tier 2
  target, and are not expected to provide target-specific implementations for
  every tier 2 target.
- The target must provide documentation for the Rust community explaining how
  to build for the target using cross-compilation, and explaining how to run
  tests for the target, using emulation if possible or dedicated hardware if
  necessary.
- The target must document its baseline expectations for the features or
  versions of CPUs, operating systems, libraries, runtime environments, and
  similar.
- If introducing a new tier 2 or higher target that is identical to an existing
  Rust target except for the baseline expectations for the features or versions
  of CPUs, operating systems, libraries, runtime environments, and similar,
  then the proposed target must document to the satisfaction of the approving
  teams why the specific difference in baseline expectations provides
  sufficient value to justify a separate target.
  - Note that in some cases, based on the usage of existing targets within the
    Rust community, Rust developers or a target's maintainers may wish to
    modify the baseline expectations of a target, or split an existing target
    into multiple targets with different baseline expectations. A proposal to
    do so will be treated similarly to the analogous promotion, demotion, or
    removal of a target, according to this policy, with the same team approvals
    required.
    - For instance, if an OS version has become obsolete and unsupported, a
      target for that OS may raise its baseline expectations for OS version
      (treated as though removing a target corresponding to the older
      versions), or a target for that OS may split out support for older OS
      versions into a lower-tier target (treated as though demoting a target
      corresponding to the older versions, and requiring justification for a
      new target at a lower tier for the older OS versions).
- Tier 2 targets must not leave any significant portions of `core` or the
  standard library `unimplemented!()`, unless they cannot possibly be supported
  on the target.
- If the target supports C code, the new Rust target should support the C
  calling convention for the platform via `extern "C"`. The C calling
  convention does not need to be the default Rust calling convention for the
  target, however.
- The target must build reliably in CI.
- The approving teams may additionally require that a subset of tests pass in
  CI, such as enough to build a functional "hello world" program, or equivalent
  "smoke tests". In particular, this requirement may apply if the target builds
  host tools, or if the tests in question provide substantial value via early
  detection of critical problems.
- Building the target in CI must not take substantially longer than the current
  slowest target in CI. This requirement is subjective, to be evaluated by the
  infrastructure team, and will take the community importance of the target
  into account.
- Tier 2 targets must support building on the existing CI infrastructure. In
  particular, new tier 2 targets must support cross-compiling, and must not
  require using the target as the host for builds.
- In addition to the legal requirements for all targets (specified in the tier
  3 requirements), because a tier 2 target typically involves the Rust project
  building and supplying various compiled binaries, incorporating the target
  and redistributing any resulting compiled binaries (e.g. built libraries,
  host tools if any) must not impose any onerous license requirements on any
  members of the Rust project, including infrastructure team members and those
  operating CI systems. This is a subjective requirement, to be evaluated by
  the approving teams.
  - As an exception to this, if the target's primary purpose is to build
    components for a Free and Open Source Software (FOSS) project licensed
    under "copyleft" terms (terms which require licensing other code under the
    compatible FOSS terms), such as kernel modules or plugins, then the
    standard libraries for the target may potentially be subject to copyleft
    terms, as long as such terms are satisfied by Rust's existing practices of
    providing of full corresponding source code. Note that anything added to
    the Rust repository itself must still use Rust's standard license terms.
- Tier 2 targets must not impose burden on the authors of pull requests, or
  other developers in the community, to ensure that tests pass for the target.
  In particular, do not post comments (automated or manual) on a PR that
  suggests a block on the PR based on tests failing for the target. (A PR
  author must not break the build of a tier 2 target, but need not ensure the
  tests pass for a tier 2 target, even if notified that they fail.)
- The target maintainers should regularly run the testsuite for the target, and
  should fix any test failures in a reasonably timely fashion.
- All tier 3 requirements apply.

Note: some tier 2 targets additionally have binaries built to run on them as a
host (such as `rustc` and `cargo`). Such a target must meet all the
requirements above, and must additionally get the compiler and infrastructure
team to approve the building of host tools. Depending on the target and its
capabilities, this may include only `rustc` and `cargo`, or may include
additional tools such as `clippy` and `rustfmt`.

A tier 2 target may be demoted or removed if it no longer meets these
requirements. Any proposal for demotion or removal will be CCed to people who
have previously worked on the target, and will be communicated widely to the
Rust community before being dropped from a stable release.

In some circumstances, especially if the target maintainers do not respond in a
timely fashion, Rust teams may land pull requests that temporarily disable some
targets in the nightly compiler, in order to implement a feature not yet
supported by those targets. (As an example, this happened when introducing the
128-bit types `u128` and `i128`.) Such a pull request will include notification
and coordination with the maintainers of such targets. The maintainers of such
targets will then be expected to implement the corresponding target-specific
support in order to re-enable the target. If the maintainers of such targets
cannot provide such support in time for the next stable release, this may
result in demoting or removing the targets.

## Tier 1 target policy

At this tier, the Rust project guarantees that a target builds and passes all
tests, and will reject patches that fail to build or pass the testsuite on a
target. We hold tier 1 targets to our highest standard of requirements.

A proposed new tier 1 target must be reviewed and approved by the compiler team
based on these requirements. In addition, the release team must approve the
long-term viability of the target, and the additional work of supporting the
target. For a tier 1 target, this will typically take place via a full RFC
proposing the target, to be jointly reviewed and approved by the compiler team
and release team.

In addition, the infrastructure team must approve the integration of the target
into Continuous Integration (CI), and the tier 1 CI-related requirements. This
review and approval may take place in a PR adding the target to CI, by an
infrastructure team member reporting the outcome of a team discussion, or by
including the infrastructure team in the RFC proposing the target.

- Tier 1 targets must have substantial, widespread interest within the
  developer community, and must serve the ongoing needs of multiple production
  users of Rust across multiple organizations or projects. These requirements
  are subjective, and determined by consensus of the approving teams. A tier 1
  target may be demoted or removed if it becomes obsolete or no longer meets
  this requirement.
- The target must build and pass tests reliably in CI.
  - The target must not disable an excessive number of tests or pieces of tests
    in the testsuite in order to do so. This is a subjective requirement.
- The target must provide as much of the Rust standard library as is feasible
  and appropriate to provide. For instance, if the target has a standard memory
  allocator, it must provide an implementation of `alloc` and the associated
  data structures.
- Building the target and running the testsuite for the target must not take
  substantially longer than other targets.
- If running the testsuite requires additional infrastructure (such as physical
  systems running the target), the target maintainers must arrange to provide
  such resources to the Rust project, to the satisfaction and approval of the
  Rust infrastructure team.
- All tier 2 requirements apply.

A tier 1 target may be demoted if it no longer meets these requirements. Any
proposal for demotion or removal will be communicated widely to the Rust
community, both when initially proposed and before being dropped from a stable
release. A tier 1 target is highly unlikely to be directly removed without
first being demoted to tier 2 or tier 3.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The set of approving teams for each tier arose out of discussion with the
various teams involved with aspects of the Rust project impacted by new
targets.

Policies that require the approval of multiple teams could instead require a
core team approval. This would have the advantage of reducing the number of
people involved in the final approval, but would put more coordination effort
on the core team and the various team leads to ensure that the individual teams
approve. As another alternative, we could separate the individual team
approvals (into separate issues or separate rfcbot polls), to simplify checking
for consensus and reduce diffusion of responsibility; however, this could also
increase the resulting complexity and result in discussions in multiple places.

We could introduce specific time requirements for the amount of time a target
must spend at a tier before becoming eligible for promotion to a higher tier.

# Prior art
[prior-art]: #prior-art

This attempts to formalize and document Rust policy around targets and
architectures. Some requirements of such a policy appear on the [Rust Platform
Support page](https://forge.rust-lang.org/release/platform-support.html).

Future expansions of such policy may find requirements from other communities
useful as examples, such as Debian's [arch
policy](https://release.debian.org/bullseye/arch_policy.html) and [archive
criteria](https://ftp-master.debian.org/archive-criteria.html).

Other precedents for tiered target support include [Firefox's supported build
targets](https://firefox-source-docs.mozilla.org/build/buildsystem/supported-configurations.html),
[node.js supported
platforms](https://github.com/nodejs/node/blob/master/BUILDING.md#supported-platforms),
and [GHC's platform
support](https://gitlab.haskell.org/ghc/ghc/-/wikis/platforms).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- How should we track the maintainers of a target, so that we can page them if
  we need an issue addressed involving that target?
  - We could use github teams, which could be directly mentioned in an issue or
    PR to get their attention. However, this would also put a "Member" badge on
    the members of those teams, which may give an unwarranted appearance of
    official status.
  - We could track them in a document posted somewhere, and manually copy-paste
    the list to ping them.
  - We could add them as a "marker team" (e.g. `target-xyz`) in the
    [rust-lang/team](https://github.com/rust-lang/team) repository. For
    example, see [the icebreakers-llvm
    team](https://github.com/rust-lang/team/blob/master/teams/icebreakers-llvm.toml).
    This would allow pinging them with `@rustbot ping target-xyz`.
    - We could additionally teach rustbot to automatically ping a target team
      when an issue is labeled with a target-specific label.

# Future possibilities
[future-possibilities]: #future-possibilities

Eventually, as more targets seek tier 1 status, we may want to document more
criteria used to evaluate requirements such as "long-term viability of the
target". We should also update these requirements if corner cases arise.

Some of our existing targets may not meet all of these criteria today. We may
wish to audit existing targets against these criteria, but this RFC does not
constitute a commitment to do so in a timely fashion.

Future enhancements to the target tier policy may introduce more detailed
requirements for supplying host tools for a target, or for supplying part or
all of the standard library.
