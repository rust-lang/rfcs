- Feature Name: N/A
- Start Date: 2022-04-12
- RFC PR: [rust-lang/rfcs#3254](https://github.com/rust-lang/rfcs/pull/3254)
- Rust Issue: n/a

# Summary
[summary]: #summary

* Introduce a new "type system team" (the "types team" for short) that works to **implement and formally define** the semantics of Rust as decided by the lang team.
* The type team owns and maintains:
    * The implementation of the Rust type checker, trait system, and borrow checker that is used in rustc.
    * Formal definitions of Rust, its type checker, and its semantics, as they are developed.
    * The "unsafe code guidelines" (once decided).

# Motivation
[motivation]: #motivation

The types team is meant to build a base of maintainers for the formal side of Rust, both design and implementation. This has traditionally been an area with a very low "[bus factor]", both in terms of the compiler (few maintainers to the code) and the language design (few people who fully understand the entire space). This has led to a general paralysis in which new features (implied bounds, const generics, specialization, etc) are stalled for long periods of time due to a combination of an inflexible implementation, a lack of maintainers, and a general difficulty in reasoning about their interactions. 

Focusing a team on just Rust's "type system" will allow us to do targeted outreach and to help people to learn the background that is needed to contribute here.

[bus factor]: https://en.wikipedia.org/wiki/Bus_factor

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Mission and charter

The “types team” owns the design, implementation, and maintenance of the Rust type system (but not the user-facing syntax of Rust). This includes the Rust type checker, trait system, borrow checker, and the operational semantics of MIR (and eventually Rust itself). As part of the operational semantics, the team is also ultimately responsible for deciding what is “undefined behavior”.

## Relationship to other teams

The types team is conceptually a subteam of both the lang and compiler teams. Since the team repo can't really model that, we'll put it primarily under the lang team (but this may be changed in the future if support is added).

### Lang team

* Stabilizing new features that intersect the type system will require approval by the types team.
    * There is some parallel work being proposed to better "formalize" multi-team "signoffs" of language features, which would be relevant here
    * Eventually this is expected to require extending the "formality" models to include the feature.
    * This approval is not meant to be used to make "policy" decisions but to enforce the soundness and implementability of the feature itself.
* Advising on how to address soundness bugs or other subtle questions that arise.
* Assisting with the design of language extensions that extends the core Rust type system capabilities
    * Maintaining the Rust model and extending it to model new proposals that interact with the type checker

### Compiler team

* Assessing and implementing fixes for soundness bugs that have to do with the type system.
* Maintaining shared libraries that implement the Rust type/trait system and the borrow checker.

### Initiatives

Initiatives that are extending the semantics of the language, such as the [generic associated types](https://rust-lang.github.io/generic-associated-types-initiative/) initiative, will work closely with this team to integrate those changes into the formality models.

## Examples

Here are some examples to illustrate how the types team will interact.

### Evaluating the implication of equality constraints or negative trait bounds

The original RFC for where clauses included equality constraints like `where T == U` (tracked by [#20041]); similarly, people have regularly considered including the option to have "negated" where clauses like `where T: !Debug`. Both of these features, if led to their full generality, turn out to make the implementation of Rust's trait solver significantly more complicated. Therefore, the types team is within its rights to veto these features or to suggest appropriate modifications to how they work and what they mean. On the other hand, it is up to the lang team to decide the syntax of those where clauses and whether they'd be a useful addition to Rust (presuming they had a semantics the types team was happy with). The types team cannot add a feature to Rust all by itself, but it can either remove one (because it is not feasible to implement) or tweak its formal semantics (to ensure it is sound, feasible etc). In this respect, it is just like the compiler team. Of course, this would and should be done with the full spirit of collaboration between teams that the Rust Project today already employs. Decisions are not made in a vacuum. Ultimately, the types team exists as a chunk of the middle-ground between the lang and compiler teams that involves the Rust type system, and by creating this team, both the lang and compiler teams give the types team the authority to work through that chunk of problems, propose solutions, and ultimately guide the decisions there.

[#20041]: https://github.com/rust-lang/rust/issues/20041

### Bug in the type checker

When a soundness bug is found in Rust's type system, the compiler team can contact the types team to request the bug be triaged and to prepare a fix. The types team owns that code and hence is ultimately responsible for reviewing changes to the relevant code. Along these lines, if a breaking change must be made to the language to fix unsound code, it is the responsibility of the types team to make the final decision - a responsibility previously held by the lang team.

### Unsafe code guidelines

One initiative worth calling out is the unsafe code guidelines working group. This group is quite old and needs at some point to become active again; at that point, it would effectively be a domain working group, much like the async working group. The role of the types there would be to help integrate and model the unsafe code guidelines proposals and to advise the lang team on the technical implications of what is being proposed. The role of the lang team would be to decide which model end users would prefer and so forth.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Leads

Leads are responsible for:

* Leading and scheduling team meetings
* Selecting the deep dive meetings
* Making decisions regarding team membership
* General "buck stops here"-type decisions

Leads typically serve for 6 months to 1 year, at which point we will consider whether to rotate.

## Team membership

Membership in the team is awarded on the basis of experience working on the implementation, source, or design of an active initiative. Membership is kept up to date; if you are inactive for >6 months, we may move you to the alumni team.

To become a member you typically have to (a) contribute consistently over a period of multiple months and (b) lead at least one "deep dive" (see below). The second point may be waived.

Note that, like compiler or lang teams, membership is not generally granted just by asking. Those interested in joining should consider getting involved by attending planning and deep dive meetings, or by getting involved in initiative-specific meetings and work.

## Initial details

Here are some details of the team that are true at the time of this writing. They are expected to change over time.

### Leads and membership

The current team leads are Nicholas Matsakis (nikomatsakis) and Jack Huey (jackh726). Initial membership will be determined by the leads after team is created.

### Team meetings

In general, each active initiative coordinates its own activity. For team-wide, cross-initiative communication, the team currently has a single weekly meeting which serves two purposes:

* The first meeting of the month is the **planning meeting**, in which we review status updates and schedule the topics to cover in the remaining meetings of that month.
* Subsequent meetings are **deep dive meetings**, in which we spend 60-90 minutes doing a deep read through a PR, design document, or other material and having team-wide discussion.

These meetings are currently held on Zulip.

#### Planning meeting

We review the overall roadmap for each active initiative and set some kind of goals.

Each initiative is responsible for preparing a short (1-2 paragraph) document in the leadup to the planning meeting with the following structure:

* What was the plan at the start of the month (list of goals)?
* What happened? (brief narrative)
* What is the plan for this month (list of goals)?

The meeting begins by reading these documents and asking questions. The goal is to adjust the goals for each initiative so that they are realistic; we should be helping each other to calibrate and set expectations. The final document is then published as a blog post.

#### Deep dive meeting

A "deep dive" meeting takes the form of reviewing a write-up, a PR, or otherwise diving into some topic together. They are expected to last 90 minutes but perhaps longer. Deep dive meetings for a given month are scheduled during the planning meeting. 

An [example transcript of a previous deep dive meeting can be found here.](https://zulip-archive.rust-lang.org/stream/144729-wg-traits/topic/deep.20dive.202022-03-18.3A.20intro.20to.20formality.html)

### Github Projects

The types team maintains the following projects:

* [chalk](https://github.com/rust-lang/chalk) and chalk-ty
* [polonius](https://github.com/rust-lang/polonius)
* [a-mir-formality](https://github.com/nikomatsakis/a-mir-formality)
    * this may eventually grow to multiple 'formality' models

The team is also responsible for those portions of [rust-lang/rust] that implement the type system. This will be done jointly, with the compiler team, since those portions are not cleanly separable. Specific details of how this is done, however, are left out of this RFC, as they will likely change over time.

[a-mir-formality]: https://github.com/nikomatsakis/a-mir-formality
[rust-lang/rust]: https://github.com/rust-lang/rust

### Active initiatives

The currently active initiatives of the types team are as follows:

* [Impl trait](https://rust-lang.github.io/impl-trait-initiative/): currently focused on RPITIT and TAIT
    * Membership: oli-obk, nikomatsakis, spastorino
* [GATs](https://github.com/rust-lang/generic-associated-types-initiative): currently focused on stabilizing
    * Membership: jackh726, compiler-errors, nikomatsakis
* [Negative impls in coherence](https://github.com/rust-lang/negative-impls-initiative)
    * Membership: spastorino, nikomatsakis
* [Const eval](https://github.com/rust-lang/lang-team/issues/22) and [const generics](https://github.com/rust-lang/lang-team/issues/51)
    * Membership: lcnr, nikomatsakis
* [a MIR formality](https://github.com/nikomatsakis/a-mir-formality), a model for the Rust type system
    * Membership: nikomatsakis
* [dyn upcasting initiative](https://rust-lang.github.io/dyn-upcasting-coercion-initiative/)
    * Membership: nikomatsakis, crlf0710

#### Stalled initiatives

Other initiatives that are somewhat stalled but looking to be rejuvenated:

* Polonius: re-implementing the Rust borrow checker in a more flexible, alias-analysis-like fashion
* Chalk-ty: implementing a shared library to represent types in rustc, chalk, rust-analyzer, and beyond
* Chalk: implementing a library
* Unsafe code guidelines: deciding what behavior is legal or not legal for unsafe code


# Drawbacks
[drawbacks]: #drawbacks

### Complicates "ownership" of things a bit.

The ownership of specific problems becomes a little bit more complicated. It's harder to tell who "owns" the final decision of some problems: lang vs types or compiler.

Related, but more technical, it may make sense for type team members to get r+ access to the rustc repo (to review and manage traits or borrowck related code). However, the line between traits or borrowck code and everything can be blurry.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives


### Why a team and not a working group? What is the difference between those anyway?

The distinction between a team and working group are ongoing and will likely result in a separate RFC. However, the following explanation gives *one* opinion on how this might look. It's not expected that this RFC be used as reference for future justification of decisions related to this.

A **team** focuses on maintaining and extending some aspect of Rust (e.g., compiler, language, stdlib) so that it works well for all Rust users. As the owners of that area, they have **final decision making power**. Teams are (relatively) **permanent**, as they tend to an area of functionality and need to do maintenance, bug fixes, and the like. (It's possible of course to decomission a team, either because there is no one to do the work, because the work is now being done by another team, or because the product the team was maintaining has gone away.)

A **working group** focuses on improving Rust for a particular purpose or target domain (e.g., async, CLI, but also more abstract purposes like error handling). Typically, they do that work by preparing **recommendations** (e.g., in the form of RFCs) that are then adopted by teams (though in some cases, working groups own and maintain repositories as well, which they have jurisdiction over). Working groups are **temporary** -- at some point, the domain is served "well enough" and the action moves out to the ecosystem at large. (This may, however, take a long time.)

For completeness, an **initiative** is a specific project undertaken by some team(s) or working group(s). Initiatives lie at the intersection of teams and working groups, where the team(s) are tasks with ensuring that the initiative is a good, general purpose addition to Rust, and the working groups are tasked with making sure it will satisfy their specific needs.

The amount of organization involved in a team or working group is another factor. Both of them should have a lead and some amount of coordination, though that coordination doesn't have to come in the form of weekly meetings. For things that don't meet that level of organization, we probably want another term, such as "notification group".

**NB:** We have traditionally used these terms in a variety of ways, not all of which fit the above definition. For example, the compiler team's LLVM working group is, by these definitions, a subteam (or perhaps notification group, as the LLVM subteam doesn't have a lead or agenda to my knowledge). I would argue that we should change wg-llvm to match these definitions. --nikomatsakis

### Wait, it has TWO parent teams?? Can you do that???

Why the heck not! The team really has two aspects to its character, and so it likely belongs in both. This is further supported that the decisions the types team will make come at the intersection between the design of the language and implementation of that design.

### OK, so it should be a team, but why the "types" team?

We went back and forth on the name and decided that "types" (or "type system", in full) hit the "sweet spot" in terms of being short, suggestive, and memorable. The "type system" for Rust in general encompasses all of its static checking, so the name is appropriate in that regard; the team is also responsible for defining Rust's operational semantics (what effect Rust code has when it executes), which is not part of the type system, but that's ok. 

Other names considered:

* the traits team
* the semantics team
* the formality team

### What do you expect the planning meetings to do?

The role of the planning meeting is to...

* Encourage initiatives to set goals and track their progress.
* Give us a simple way to advertise the work that is getting done.
* Help the various initiatives stay in touch with each other at a high-level.

### What do you expect the deep dive meetings to do?

Whereas the planning meetings aim to keep people in touch at a high-level, the deep dive meetings...

* Give an opportunity to understand a single topic in depth
* Give an opportunity for an initiative to ask for help, with enough time to get the requisite context
* The act of preparing for the meeting often helps on its own
* Because they are centered around documents that can be published, gives a record of important material

# Prior art
[prior-art]: #prior-art

There are already multiple other "subteams". For example, [rustfmt and rustup were recently "converted" from working groups to teams](https://github.com/rust-lang/team/pull/723).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

None.
