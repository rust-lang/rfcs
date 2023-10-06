---
title: Lang team advisors RFC
---

- Feature Name: N/A
- Start Date: 2022-09-21
- RFC PR: [rust-lang/rfcs#3327](https://github.com/rust-lang/rfcs/pull/3327)
- Rust Issue: N/A

# Summary
[summary]: #summary

Create a new subteam of the lang team entitled **Lang Team Advisors**:

* Advisors are people whose feedback and judgment is highly valued by the lang team.
* Advisors are notified when the lang team makes FCP decisions; while they don't need to approve explicitly, they may raise blocking objections.
* Advisors are not generally expected or required to attend meetings, unless the meeting pertains to their area of expertise.

# Motivation
[motivation]: #motivation

There are many folks who regularly aid the Rust community and the lang team in particular in language design decisions, but who for various reasons it doesn't make sense to add to the team as full members. In practice, if one of those people raises an objection on a feature, that is given quite a lot of weight, but our process doesn't have any official way to recognize them. The lang team advisors subteam closes this gap, allowing us to recognize advisors publicly and to give them the ability to lodge formal objections that block FCP.

Lang team advisors can be useful in a number of situations:

* Someone who is offered membership, but declines because they don't have time to attend meetings and the like, may find the advisors team a better fit, helping to keep them engaged in the Rust project (and to recognize their contributions).
* Advisors is a great fit for domain experts who are consulted regularly on particular topics, but who are not interested in all aspects of Rust language design.
* Advisors can also serve as a stepping stone to full membership: this gives the team a chance to recognize someone who is participating actively before committing to full membership.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The **lang-team advisors** is a subteam of the lang team that contains people who the lang team consults on a regular basis. Advisors are notified when the lang team is making a decision via FCP; while they are not required to approve explicitly (e.g. check a checkbox), an advisor may raise a blocking objection.

Members of the advisors team are typically domain experts or Rust community members with limited time and availability. The advisors team allows us to formalize their relatioship with Rust without asking them to take on the full responsibilities of being a lang team member. The advisors team can also be a useful stepping stone towards full membership, giving someone a chance to interact more fully with the lang team process.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Adding a new lang-team advisor

Lang team advisors are added through the [same general process as regular lang team members][new-member-process]:

[new-member-process]: https://github.com/rust-lang/lang-team/pull/174/

* Any lang team member can send a message with a proposal for lang-team advisors.
    * The message should include a short write-up giving answers to the questions below. It is particularly useful to provide examples (e.g., we sought their opinion at this point).

These are the questions we ask ourselves when deciding whether someone would be a good choice as a lang team advisor.

* Do we regularly seek this person's opinion when **deliberating**?
    * For example, during triage, do we often say "let's check what this person thinks".
* Does this person have particular knowledge of some domain, or some particular part of Rust? Alternatively, do they have broad knowledge of Rust?
* If, even after a long protracted debate, you knew that this person had concerns about a design, would you want to block the design from going forward until you had a chance to hear them out?

Naturally, the [questions for lang team membership][new-member-process] are also appropriate, but they are "nice to haves"; the bar for an advisor is lower. (And none of the requirements regarding meeting attendance or other team duties apply.)

When adding an advisor primarily for a specific area of expertise, we should document that area of expertise in a comment in the `lang-team-advisors.toml` file.

## Removing a lang-team advisor

An advisor may be removed at their request, or if the team feels they've been inactive for an extended period. However, advisors (like any team member) are free to take vacations and otherwise maintain life/Rust balance.

## Integration into the decision process

There will be a team in the rust repo (`rust-lang/lang-team-advisors`). When a lang team FCP is initiated, we will cc this team, making them aware it is happening. Advisors will be able to raise blocking objections with the "concern" functionality of rfcbot, or equivalent functionality in future decision tooling. (As an interim measure until rfcbot includes this functionality, team members may raise concerns on behalf of advisors on request.)

The precise details of how advisors fit into the lang team [decision making process](https://lang-team.rust-lang.org/decision_process/reference.html) are as follows:

* Like lang-team members, advisors may raise a blocking concern on an FCP. The expectation is that the advisor will work with the implementors to resolve the concern to everyone's mutual satisfaction.
* Unlike lang-team members, advisors cannot sustain a concern to prevent it from being overruled; only full lang-team members can opt to sustain a concern. A concern raised by an advisor may be overruled if "all but one" lang-team members agree that it has been adequately heard and understood (this rule ensures that there is no incentive for an advisor to "proxy" a concern on behalf of a full member).

## Integration into the experiment process

Advisors can serve as the liaison for an [experimental feature gate](https://lang-team.rust-lang.org/how_to/experiment.html) if a lang team member approves. This is only recommended for advisors that attend triage/design meetings regularly and who have a strong sense for what might be controversial or likely to be accepted (as opposed to advisors who are domain experts but not following all aspects of Rust).

# Drawbacks
[drawbacks]: #drawbacks

## More sources for blocking objections

There will be more people able to raise blocking objections than there were before. However, note that we only add people to the list whose opinion we would seek and likely block on regardless, so this would primarily be an issue if we add advisors injudiciously.

## Potential for out-of-date records

It is always challenging to keep our lists of team members up to date, and this adds a new list.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The lang-team already regularly consults with many of the people we consider prospective advisors. The primary alternative would be to continue using the existing ad-hoc mechanisms for such consultation.

# Prior art
[prior-art]: #prior-art

The compiler team contributors team plays many purposes, but one of them is that it is a place to add members who have contributed in specific areas of the compiler but who are not overall maintainers or experts across the entire compiler codebase. It can also serve as a stepping stone towards full compiler-team membership. The lang-team advisors can fulfill a similar role.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

A recent trend has been forming specialized subteams, like the [types team](https://github.com/rust-lang/rfcs/pull/3254), that focus on particular areas of the language. We would like to enable members of those teams to raise blocking objections when they see a problem pertaining to their expertise. While we can add members of those teams as individual advisors, we may also choose to recognize the team as a whole.
