# Major Change Proposal RFC

- Feature Name: N/A
- Start Date: 2020-05-07
- RFC PR: [rust-lang/rfcs#2904](https://github.com/rust-lang/rfcs/pull/2904)
- Rust Issue: N/A

# Summary
[summary]: #summary

Introduce the **major change process** for the compiler team. This process has the following goals:

* to advertise major changes and give the team a chance to weigh in;
* to help scale the amount of work being done to reviewing bandwidth
  by choosing a reviewer in advance;
* to avoid a lot of process overhead.

The intent here is that if you have a plan to make some "major change" to the compiler, you will start with this process. It may either simply be approved, but if the change proves more controversial or complex, we may escalate towards design meetings, longer write-ups, or full RFCs before reaching a final decision.

This process does not apply to adding new language features, but it can be used for minor features such as adding new `-C` flags to the compiler.

# Motivation
[motivation]: #motivation

As the compiler grows in complexity, it becomes harder and harder to track what's going on. We don't currently have a clear channel for people to signal their intention to make "major changes" that may impact other developers in a lightweight way (and potentially receive feedback).

Our goal is to create a channel for signaling intentions that lies somewhere between opening a PR (and perhaps cc'ing others on that PR) and creating a compiler team design meeting proposal or RFC.

## Goals

Our goals with the MCP are as follows:

* Encourage people making a major change to write at least a few paragraphs about what they plan to do.
* Ensure that folks in the compiler team are aware the change is happening and given a chance to respond.
* Ensure that every proposal has a "second", meaning some expert from the team who thinks it's a good idea.
* Ensure that major changes have an assigned and willing reviewer.
* Avoid the phenomenon of large, sweeping PRs landing "out of nowhere" onto someone's review queue.
* Avoid the phenomenon of PRs living in limbo because it's not clear what level of approval is required for them to land.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Major Change Proposals

If you would like to make a [major change] to the compiler, the process
is as follows:

[major change]: #What-constitutes-a-major-change

* Open a tracking issue on the [rust-lang/compiler-team] repo using the [major change template].
    * A Zulip topic in the stream `#t-compiler/major changes` will automatically be created for you by a bot.
    * If concerns are raised, you may want to modify the proposal to address those concerns.
    * Alternatively, you can submit a [design meeting proposal] to have a longer, focused discussion.
* To be accepted, a major change proposal needs three things:
    * One or more **reviewers**, who commit to reviewing the work. This can be the person making the proposal, if they intend to mentor others.
    * A **second**, a member of the compiler team or a contributor who approves of the idea, but is not the one originating the proposal.
    * A **final comment period** (a 10 day wait to give people time to comment).
        * The FCP can be skipped if the change is easily reversed and/or further objections are considered unlikely. This often happens if there has been a lot of prior discussion, for example.
* Once the FCP completes, if there are no outstanding concerns, PRs can start to land.
    * If those PRs make outward-facing changes that affect stable
      code, then either the MCP or the PR(s) must be approved with a
      `rfcbot fcp merge` comment.

## Conditional acceptance

Some major change proposals will be conditionally accepted. This indicates that we'd like to see the work land, but we'd like to re-evaluate the decision of whether to commit to the design after we've had time to gain experience. We should try to be clear about the things we'd like to evaluate, and ideally a timeline.

## Deferred or not accepted

Some proposals will not be accepted. Some of the possible reasons:

* You may be asked to do some prototyping or experimentation before a final decision is reached
* The idea might be reasonable, but there may not be bandwidth to do the reviewing, or there may just be too many other things going on.
* The idea may be good, but it may be judged that the resulting code would be too complex to maintain, and not worth the benefits.
* There may be flaws in the idea or it may not sufficient benefit.

[rust-lang/compiler-team]: https://github.com/rust-lang/compiler-team
[design meeting proposal]: https://forge.rust-lang.org/compiler/steering-meeting.html

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## What happens if someone opens a PR that seems like a major change *without* doing this process?

The PR should be closed or marked as blocked, with a request to create
a major change proposal first.

If the PR description already contains suitable text that could serve
as an MCP, then simply copy and paste that into an MCP issue. Using an
issue consistently helps to ensure that the tooling and process works
smoothly.

## Can I work on code experimentally before a MCP is accepted?

Of course!  You are free to work on PRs or write code. But those PRs should be marked as experimental and they should not land, nor should anyone be expected to review them (unless folks want to).

## What constitutes a major change?

The rough intuition is "something that would require updates to the [rustc-dev-guide] or the [rustc book]". In other words:

* Something that alters the architecture of some part(s) of the compiler, since this is what the rustc-dev-guide aims to document.
* A simple change that affects a lot of people, such as altering the names of very common types or changing coding conventions.
* Adding a compiler flag or other public facing changes, which should be documented (ultimately) in the rustc book. This is only appropriate for "minor" tweaks, however, and not major things that may impact a lot of users. (Also, public facing changes will require a full FCP before landing on stable, but an MCP can be a good way to propose the idea.)

Note that, in some cases, the change may be deemed **too big** and a full FCP or RFC may be required to move forward. This could occur with significant public facing change or with sufficiently large changes to the architecture. The compiler team leads can make this call.

Note that whether something is a major change proposal is not necessarily related to the number of lines of code that are affected. Renaming a method can affect a large number of lines, and even require edits to the rustc-dev-guide, but it may not be a major change. At the same time, changing names that are very broadly used could constitute a major change (for example, renaming from the `tcx` context in the compiler to something else would be a major change).

[rustc-dev-guide]: https://rustc-dev-guide.rust-lang.org
[rustc book]: https://doc.rust-lang.org/rustc/index.html

## Public-facing changes require rfcbot fcp

The MCP "seconding" process is only meant to be used to get agreement
on the technical architecture we plan to use. It is not sufficient to
stabilize new features or make public-facing changes like adding a -C
flag. For that, an `rfcbot fcp` is required (or perhaps an RFC, if the
change is large enough).

For landing compiler flags in particular, a good approach is to start
with an MCP introducing a `-Z` flag and then "stabilize" the flag by
moving it to `-C` in a PR later (which would require `rfcbot fcp`).

Major change proposals are not sufficient for language changes or
changes that affect cargo.

## Steps to open a MCP

* Open a tracking issue on the [rust-lang/compiler-team] repo using the
  [major change template].
* Create a Zulip topic in the stream `#t-compiler/major changes`:
  * The topic should be named something like "modify the whiz-bang
    component compiler-team#123", which describes the change and links
    to the tracking issue.
  * The stream will be used for people to ask questions or propose changes.

## What kinds of comments should go on the tracking issue in compiler-team repo?

Please direct technical conversation to the Zulip stream.

The compiler-team repo issues are intended to be low traffic and used for procedural purposes. Note that to "second" a design or offer to review,  you should be someone who is familiar with the code, typically but not necessarily a compiler team member or contributor. 

* Announcing that you "second" or approve of the design.
* Announcing that you would be able to review or mentor the work.
* Noting a concern that you don't want to be overlooked.
* Announcing that the proposal will be entering FCP or is accepted.

## How does one register as reviewer, register approval, or raise an objection?

These types of procedural comments can be left on the issue (it's also good to leave a message in Zulip). See the previous section.

## Who decides whether a concern is unresolved?

Usually the experts in the given area will reach a consensus here. But if there is some need for a "tie breaker" vote or judgment call, the compiler-team leads make the final call.

## What are some examples of major changes from the past?

Here are some examples of changes that were made in the past that would warrant the major change process:

* overhauling the way we encode crate metadata
* merging the gcx, tcx arenas
* renaming a widely used, core abstraction, such as the `Ty` type
* introducing cargo pipelining 
* adding a new `-C` flag that exposes some minor variant

## What are some examples of things that are too big for the major change process?

Here are some examples of changes that are too big for the major change process, or which at least would require auxiliary design meetings or a more fleshed out design before they can proceed:

* introducing incremental or the query system
* introducing MIR or some new IR
* introducing parallel execution
* adding ThinLTO support

## What are some examples of things that are too small for the major change process?

Here are some examples of things that don't merit any MCP:

* adding new information into metadata
* fixing an ICE or tweaking diagnostics
* renaming "less widely used" methods

## When should Major Change Proposals be closed?

Major Change Proposals can be closed:

* by the author, if they have lost interest in pursuing it.
* by a team lead or expert, if there are strong objections from key
  members of the team that don't look likely to be overcome.
* by folks doing triage, if there have been three months of
  inactivity. In this case, people should feel free to re-open the
  issue if they would like to "rejuvenate" it.

# Template for major change proposals

[major change template]: #Template-for-major-change-proposals

The template for major change proposals is as follows:

```
# What is this issue?

This is a **major change proposal**, which means a proposal to make a notable change to the compiler -- one that either alters the architecture of some component, affects a lot of people, or makes a small but noticeable public change (e.g., adding a compiler flag). You can read more about the MCP process on https://forge.rust-lang.org/.

**This issue is not meant to be used for technical discussion. There is a Zulip stream for that. Use this issue to leave procedural comments, such as volunteering to review, indicating that you second the proposal (or third, etc), or raising a concern that you would like to be addressed.**

# MCP Checklist

* [x] MCP **filed**. Automatically, as a result of filing this issue:
  * The @rust-lang/wg-prioritization group will add this to the triage meeting agenda so folks see it.
  * A Zulip topic in the stream `#t-compiler/major changes` will be created for this issue.
* [ ] MCP **seconded**. The MCP is "seconded" when a compiler team member or contributor issues the `@rustbot second` command. This should only be done by someone knowledgable with the area -- before seconding, it may be a good idea to cc other stakeholders as well and get their opinion.
* [ ] **Final comment period** (FCP). Once the MCP is approved, the FCP begins and lasts for 10 days. This is a time for other members to review and raise concerns -- **concerns that should block acceptance should be noted as comments on the thread**, ideally with a link to Zulip for further discussion.
* [ ] MCP **Accepted**. At the end of the FCP, a compiler team lead will review the comments and discussion and decide whether to accept the MCP.
  * At this point, the `major-change-accepted` label is added and the issue is closed. You can link to it for future reference.
  
**A note on stability.** If your change is proposing a new stable feature, such as a `-C flag`, then a full team checkoff will be required before the feature can be landed. Often it is better to start with an unstable flag, like a `-Z` flag, and then move to stabilize as a secondary step.

# TL;DR

*Summarize what you'd like to do in a sentence or two, or a few bullet points.*

# Links and Details

*Add a few paragraphs explaining your design. The level of detail should be
sufficient for someone familiar with the compiler to understand what you're
proposing. Where possible, linking to relevant issues, old PRs, or external
documents like LLVM pages etc is very useful.*

# Mentors or Reviewers

*Who will review this work? If you are being mentored by someone, then list
their name here. If you are a compiler team member/contributor, and you
intend to mentor someone else, then you can put your own name here. You can
also leave it blank if you are looking for a reviewer. (Multiple names are ok
too.)*
```

# Drawbacks
[drawbacks]: #drawbacks

It adds procedural overhead.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not use the FCP process to do approvals?

We opted not to require an ordinary rfcbot fcp because that feels too cumbersome. We want this to be lightweight. Requesting at least one person to approve seems like the minimal process.

# Prior art
[prior-art]: #prior-art

The state of the art for these sorts of things in practice is that either people just write PRs, or perhaps someone opens a Zulip topic and pings a suitable set of people. This often works well in practice but can also lead to surprises, where stakeholders are overlooked. Moreover, it offers no means to manage review load or to have a chance to express concerns before a lot of code is written.

This idea was loosely based on the "intent to ship" convention that many browsers have adopted. See e.g. Mozilla's [Exposure Guidelines](https://wiki.mozilla.org/ExposureGuidelines) or Chrome's process for [launching features](https://www.chromium.org/blink/launching-features). Unlike those processes, however, it's meant for internal refactors as well as (minor) public facing features.

RFCs themselves are a form of "major change proposal", but they are much more heavyweight and suitable for longer conversations or more controversial decisions. They wind up requesting feedback from a broader audience and they require all team members to actively agree before being accepted. The MCP process is meant to be something we can use to float and advertise ideas and quickly either reach consensus or else -- if controversy is discovered -- move the proposal to a more involved process.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

The details of this procedure are sure to evolve, and we don't expect to use the RFC process for each such evolution. The main focus of this RFC is to approve of a **mandatory major change process** for major changes to the compiler.
