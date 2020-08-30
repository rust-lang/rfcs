- Feature Name: NA
- Start Date: 2020-08-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: NA

# Summary
[summary]: #summary

This document is about establishing norms for working within the Libs team that will help it scale with the rest of the Rust project and remain effective.
The Libs team will adopt the Compiler team's process for [major changes] and [project groups] to help organize itself going forward.

# Motivation
[motivation]: #motivation

The motivation is collected inline in the _Guide-level explanation_ to keep new processes close to the goal they're trying to achieve.
That way we keep process focused, rather than possible introducing it for its own sake.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## What does Libs needs from governance?

This section outlines current state of the Libs team and its work along with some changes to help address some shortcomings.

### To create a team that is trusted by the Rust community to maintain the standard library.

There's a lot of work in the Rust project that spans multiple teams, especially between the Libs, Compiler, and Lang teams.
Aligning the processes of the Libs team with the Compiler and Lang teams should make it easier to share effort and liaise between them.

The Libs team will build trust by aligning the way it works with other teams.
This involves adopting a process around [major changes] to Libs governance.
This process will follow the same form already used by the Compiler team.
Major changes can be proposed through the [Libs team repository].

The Libs team will formalize its shared ownership of the standard library with the Compiler team.
The Libs team will own the public API of the standard library and the Compiler team will own its implementation.

### To establish points of visibility and collaboration for the Rust community and other teams.

People want to contribute to libraries in the Rust project, but there's no clear support for how and where to get started.
The review process for the standard library can be a joyless process, and is especially draining for newcomers.
The RFC process is similarly draining when there's just one person attempting to facilitate discussion from a lot of commenters.
These are currently the most visible units of Libs work, but aren't very appealing to new contributors.

There are other areas of Libs that may be more appealing than reviewing standard library PRs or RFCs though:
- Working on just some specific area of the standard library that align with a contributor's interests.
- Supporting specific libraries in the wider `rust-lang` organization.
- Working on docs and resources that support Rust developers.

It's not that these areas don't exist, there just isn't any entrypoint to Libs that makes them accessible to somebody that comes looking.

The Libs team will establish visibility by adopting tools that are discoverable through the [Libs team repository]:

- GitHub issues and GitBook for permanent documentation.
- [Shared Google Calendar](https://calendar.google.com/calendar?cid=OWt1dThldnE0ZWg2dWFjbTI2MmswcGhyaThAZ3JvdXAuY2FsZW5kYXIuZ29vZ2xlLmNvbQ) for scheduling.
- Zulip streams for discussion and meetings.
- Zoom for synchronous meetings. Where possible these should be recorded and uploaded to official channels.

The Libs team will also establish [project groups] around active topics that are discoverable through the [Libs team repository] with a clear scope.
These groups offer a starting point for contributors and a path to membership through participation.

### To get and keep a clear picture of what the state of unstable APIs are.

As of writing, there are [almost 200](https://github.com/rust-lang/rust/issues?q=is%3Aopen+label%3AC-tracking-issue+label%3AT-libs) unstable tracking issues tagged with Libs.
Almost half of those issues are more than twelve months old.

Unstable features with no path to stabilization are debt.
As these unstable features pile up they obscure visibility.
We don't implement new unstable features unless we want them to stabilize, but sometimes they fall by the wayside, hit blockers, or just lose stewardship.
The Libs team will resume its triage process to help keep track of its unstable features.

#### Triaging unstable features

The Libs team will manage its existing unstable features by running regular triage meetings that check in on the status of unstable features.

All unstable features will be tracked via their tracking issues in [the Libs triage project][triage project].

This process may take the following form:

- Each currently unstable feature should eventually be triaged to determine exactly what API it covers, whether it's blocked, whether it's got any active stewards.
- That status should be added to the original post with a timestamp in the form of an update report (the OP is the most discoverable place for this because when GitHub collapses threads it's hard to find and link to things).
- Unstable features should be bucketed by feature area into a project group that can steer them towards stabilization
- Unstable features that don't have any clear path to stabilization or stewardship should be deprecated.
- Unstable features that have been deprecated for a cycle should be removed and their tracking issues closed.

We should do this in a triage meeting at least once per release cycle.

#### Working on new APIs

Small new APIs can be stabilized through a final comment period.

Significant new APIs proposed as RFCs should not become the responsibility of a single person to steer towards stabilization.
That's a proven recipe for burnout.
Instead, the Libs team will manage significant new unstable features by establishing [project groups] around them.
The group will be responsible for the whole lifecycle of the unstable API, from finalizing the RCF through to implementation and proposing stabilization.

This process might take the following form:

- An API is written as an external crate that proves its utility as an ideal realization of the design space.
- An RFC is written proposing that external crate form the basis for a standard API.
- The Libs team decides through a [major change proposal][major changes] that the API is worth pursuing with an unstable implementation in the standard library ahead of a full final comment period on the RFC itself.
- A [project group][project groups] is formed to take ownership of the RFC and land an unstable implementation.
- The [project group][project groups] finalizes the API by updating the RFC and proposing a final comment period.
- The [project group][project groups] proposes stabilization of the API.

### To create space for pursuing broader strategic things.

Today, the Libs team doesn't have a clear answer to the question _what do we want to do for the 2021 edition?_
As an event that will probably come only once every few years it would be a wasted opportunity not to examine all the possibilities open to moving the standard library forward across edition boundaries.

There's been a _lot_ of work done on the standard library since the 2018 edition (just scan through the _Libraries_ and _Stabilized APIs_ sections of [the release notes since `1.32.0`](https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1320-2019-01-17)), but not really a clear direction to communicate and celebrate from the last few years.

The Libs team will create space for broader vision by running regular steering meetings that examine the state of Libs across the board.
These meetings will give Libs a chance to look proactively at longer term ideas, some of which have already been kicked around for several years, as well as reactively on incoming new features.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section summarizes the discussion above into just the relevant changes to the way Libs will be organized going forward.

The Libs team will be responsible for:

- Determing the shape and direction of the standard library.
- Maintaining library crates in the `rust-lang` organization.
- Establishing and collaborating on best practices for libraries with the wider Rust community through libraries and resources.

The Libs team will use the [Libs team repository] as its gateway for the Rust community.

### Major changes and project groups

The Libs team will adopt the Compiler team's process for [major changes] and [project groups] to take responsibility for moving various aspects of the standard library and official projects forward.

Some of these project groups have already been proposed as RFCs:

- [**Error handling**](https://github.com/rust-lang/rfcs/pull/2965): building off several years of ecosystem work on establishing error handling norms.
- [**Portable SIMD**](https://github.com/rust-lang/rfcs/pull/2977): building off `packed_simd`.

Going forward, [project groups] will be established through the [major change process][major changes] on the [Libs team repository]. Examples of other [project groups] the Libs team may want to establish include:

- **Docs and resources**: own the API guidelines, forge docs, any other resources we want to write.
- **Iterator**: own the `Iterator` API and manage new combinators proposed for it.
- **Collections**: own the lists, sets, and maps APIs.
- **Lazy**: getting `std::lazy` stabilized.

The Libs team will serve as a point-of-contact for the Rust community and other teams, helping keep its [project groups] discoverable.

### Final comment periods

The Libs team will continue using final comment periods in RFCs and tracking issues to accept and stabilize public changes. The set of Libs team members participating in FCPs may be a different set than those interested in high-five reviews.

# Drawbacks
[drawbacks]: #drawbacks

All process comes with overhead and has to be actively maintained for it to be useful.
This is a burden on the Libs team.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art
[prior-art]: #prior-art

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
[triage project]: https://github.com/rust-lang/libs-team/projects/2
[Libs team repository]: https://github.com/rust-lang/libs-team
[project groups]: https://github.com/rust-lang/rust-forge/blob/37cf4ea896bfb41b88f8891bab66565693afc181/src/governance/README.md#project-groups
[major changes]: https://github.com/rust-lang/rfcs/blob/master/text/2904-compiler-major-change-process.md
