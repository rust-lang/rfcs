# Rust crate ownership policy

- Feature Name: none
- Start Date: 2021-05-04
- RFC PR: [rust-lang/rfcs#3119](https://github.com/rust-lang/rfcs/pull/3119)
- Rust Issue: [rust-lang/rust#88867](https://github.com/rust-lang/rust/issues/88867)


# Summary
[summary]: #summary

Have a more intentional policy around crates published by the Rust project, to be applied to existing and future crates published by us.

# Motivation
[motivation]: #motivation


Currently there are around a hundred crates that are maintained under a rust-lang GitHub organization and published to crates.io. These exist for a wide range of reasons: some are published for the express purposes of being used by the wider Rust community, others are internal dependencies of rustc (or otherwise), yet others are experiments.

Given that the stamp of an official Rust team carries a degree of weight, it is confusing for community members to have to differentiate between the two, and can lead to incorrect expectations being set. Over a prolonged period of time, this can end up in crates that were never intended to be used widely becoming key dependencies in the ecosystem.

Furthermore, these crates are not necessarily clear on who owns them. Some are owned (in the crates.io sense) by the generic `rust-lang-owner` crates.io account, some are owned by a GitHub team (like `rust-lang/libs`), and yet others are only owned by personal accounts. It seems like we should have some consistency here.

# Reference-Level Explanation

Once accepted, the policy sections of this RFC should be posted on
<https://forge.rust-lang.org/> in a "Rust-lang Crates Policy" section; this RFC will not be the canonical home of the up-to-date crates policy.

## Categories

We propose the following categories of published crates:


 - **Intentional artifacts**: These are crates which are intentionally released by some team (usually libs), are actively maintained, are intended to be used by external users, and intentionally have an air of officialness. Example: [libc](https://crates.io/crates/libc)
 - **Internal use**: These are crates which are used by some “internal client”, like rustc, crates.io, docs.rs, etc. Their primary purpose is not to be used by external users, though the teams that maintain them (typically the teams of their internal client) may wish for the crate to have wider adoption. The line can be blurry between these and “intentional artifacts” and ultimately depends on the goals of the team. Example: [conduit](https://crates.io/crates/conduit), [measureme](https://crates.io/crates/measureme). There are two subcategories based on whether they are intended to ever show up as a transitive dependency:
    - **Transitively intentional**: These are dependencies of intentional artifact libraries, and will show up in users' dependency trees, even if they are not intended to be _directly_ used. The Rust Project still needs to handle security issues in these crates _as if_ they are "intentional artifacts".
    - **Not transitively intentional**: These are dependencies of shipped binaries, CI tooling, the stdlib, or are otherwise not expected to show up in users' dependency trees. The Rust Project may need to handle security issues in these crates _internally_, but does not necessarily need to message the wider public about security issues in these crates. If a security issue in one of these crates affects a published binary (or crates.io, etc), that will still need to be handled as a bug in the binary or website.
 - **Experiment**: This was an experiment by a team, intended to be picked up by users to better inform API design (or whatever), without a long-term commitment to maintainership. Example: [failure](https://crates.io/crates/failure)
 - **Deprecated**: This used to be an “intentional artifact” (or experiment/internal use) but isn’t anymore. Example: [rustc-serialize](https://crates.io/crates/rustc-serialize)
 - **Placeholder**: Not a functional crate, used for holding on to the name of an official tool, etc. Example: [rustup](https://crates.io/crates/rustup)
 - **Expatriated**: This may have been an “intentional artifact”, and still is intended to be used by external users, but is no longer intended to be official. In such cases the crate is no longer owned/managed by the Rust project. Example: [rand](https://crates.io/crates/rand)

## Policy

Every crate in the organization must be owned by at least one team on crates.io. Teams should use `rust-lang/foo` teams for this. Non-expatriated crates may not have personal accounts as owners; if a crate needs additional owners that are not part of teams; the team should create a project group. Note that this does not forbid non-team (or project group) users from having maintainer access to the repository; it simply forbids them from _publishing_.

Currently it is not possible for a crate to be owned by _only_ a team; the `rust-lang-owner` account (or a similar account to be decided by the infra team) can be used as a stopgap in such cases. We should try to phase this account out as much as possible, in order to make sure it is clear who is responsible for each crate. For crates being auto-published, a `rust-lang/publish-bots` team (or individual bot accounts) can be used to allow bot accounts to publish crates.

Each crate in the organization, and any future crates in the organization, must decide which to which category they belong in according to the above categorization. If you're not sure what the category should be when registering a crate, or do not wish to make a decision just yet, pick "Experimental".

Each published crate must contain a README. At a minimum, this README must mention the primary owning team. Based on their categories, crates are also required to include the following information in their READMEs and documentation roots:

### Intentional artifact

“Intentional artifact” crates can choose their commitments but should be clear about what they are in their messaging. If and when a team has a charter, the crate should also be mentioned in the charter as an intentional artifact. Deprecating an intentional artifact should not be taken lightly and will require an RFC.

An example of such messaging would be text like:

> This crate is maintained by The Rust \[team\] Team for use by the wider ecosystem. This crate is post-1.0 and follows [semver compatibility](https://doc.rust-lang.org/cargo/reference/semver.html) for its APIs.


Security issues in these crates should be handled with the appropriate weight and careful messaging by the Security Response WG, and should be reported [according to the project's security policy](https://www.rust-lang.org/policies/security).

### Internal use
“Internal use” crates should contain the following text near the top of the readme/documentation:

> This crate is maintained by \[team\], primarily for use by \[rust project(s)\] and not intended for external use (except as a transitive dependency). This crate may make major changes to its APIs or be deprecated without warning.


The "except as a transitive dependency" text should be included if the crate is a dependency of an intentional-artifact library ("transitively intentional").

Security issues in transitively intentional libraries should be handled as if they were intentional artifacts.


### Experiment

“Experiment” crates should mention they are experiments. Experiment crates may be intended to be used in a scoped sort of way; so if they are intended to be used they should be clear about what they are guaranteeing.

An example of such messaging would be text like:

> This crate is maintained by \[team\] as a part of an experiment around \[thingy\]. We encourage people to try to use this crate in their projects and provide feedback through \[method\], but do not guarantee long term maintenance.

or, for experiments that are not intended to be used at all:

> This crate is maintained by \[team\] and is an internal experiment. We do not guarantee stability or long term maintenance, use at your own risk.

Ideally, experimental crates that are published for feedback purposes will have a document to link to that lists out the purpose, rough duration, and processes of the experiment.

### Deprecated
“Deprecated” crates should contain the following text near the top of the readme/documentation:

> This crate is deprecated and not intended to be used.

### Placeholder

“Placeholder” crates should contain the following text in their published readme/documentation:

> This crate is a functionally empty crate that exists to reserve the crate name of \[tool\]. It should not be used. 

In general it is better to have an empty placeholder crate published instead of reserving the crate via yanking, so that there is a readme that helps people understand why the crate is unavailable.


### Expatriated

It's unclear if any action should be taken on these beyond removing any semblance of officialness (including rust-lang/foo team owners). We currently have only one such crate (`rand`).

These should by and large not be considered to be "team managed" crates; this category is in this RFC for completeness to be able to talk about expatriation as an end state.

## Transitions and new crates

Teams should feel free to create new crates in any of these categories; however "Intentional Artifact" crates must be accompanied with an RFC. As we move towards having team charters, this can transition to being a charter change (which may require an RFC or use its own process). Teams should notify core@rust-lang.org when they've created such crates so that the core team may track these crates and ensure this policy is applied.

From time to time a team's plan for a crate may change: experiments may conclude, crates may need to be deprecated, or the team may decide to release something for wider usage.

In general, teams should notify core@rust-lang.org when such a transition is being made.

Any transition _away_ from "Intentional Artifact" requires an RFC.

Any transition to "Intentional Artifact" should ideally be accompanied by an RFC, and an update to the team charter if there is one.

Expatriation should basically _never_ occur anymore, but it also requires an RFC and core team approval in case it is really necessary. If a team wishes to stop working on a crate, they should deprecate it and encourage the community to fork it or build their own thing. The repository may be transferred out, however the `crates.io` name is kept by the Rust project and the new group of maintainers will need to pick a new crate name.

If "transitively intentional" crates are being deprecated care should be taken to ensure security issues will still be handled.

Transitions between the other types can be made at will since they explicitly and clearly state their lack of a strong stability/maintenance guarantee.


## Applying this to existing crates

An audit should be performed on all existing potentially "official" crates, collecting them in a list and roughly determining what their team and category should be.

(We have a list with a preliminary audit already and plan to post it to this RFC as an example soon)

Once we have this list, we can approach teams with lists of crates and request that they verify that the categorization is accurate. In the case of some crates this might take some time as the team may need to work out what their intentions are with a particular crate.

Then, working with the teams, we make these changes to their documentation. We also make sure all crates have the appropriate `rust-lang/teamname` github owner, and remove personal accounts from the owners list.

For crates that are in direct use by a lot of the wider community, if we end up categorizing them as anything other than "intentional artifact", there should be an attempt to announce this "change" to the community. While there was no formal commitment made in case of these crates, the vague sense of officialness may have made people believe there was, and we should at least try to rectify this so that people are not continually misled. Whether or not this needs to be done, and how, can be figured out by the individual teams.

A large part of this work can be parallelized; and it does not need to occur all at once.

# Drawbacks
[drawbacks]: #drawbacks

This is a lot of work, but as we move towards a more deliberately structured project, it is probably necessary work.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative here is mostly to continue as is. This will become increasingly untenable as we add more and more crates; with the constant danger of internal crates becoming accidental artifacts that the ecosystem depends on.

Another alternative is to ask teams to be clear about the level of support offered in their crates without standardizing the process. This could work, but could lead to less cross-team legibility and would be harder to track.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - How should we handle expatriated crates?
 - Are there any missing categories?
 - What should the text blurbs be for the various categories? Should we be mandating a specific text blurb, or just require a general idea be communicated with some leeway?
