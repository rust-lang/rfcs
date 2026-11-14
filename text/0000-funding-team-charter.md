- Feature Name: `funding_team_charter`
- Start Date: 2026-11-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC further clarifies the scope, responsibilities and membership requirements of the Funding team, which was originally introduced in [RFC #3931][rfc-3931] together with the Rust Foundation Maintainers Fund (RFMF). It does so by defining the charter of the Funding team, which fills in some gaps and answers questions that were left unresolved in that RFC.

[rfc-3931]: ./3931-rfmf-rust-foundation-maintainer-fund.md

## Motivation
[motivation]: #motivation

The Rust Project has access to funds for supporting Rust Project maintainers (for example from the Rust Foundation Maintainers Fund), and someone has to decide how will those funds be allocated to support Rust maintainers and teams. That is a considerable responsibility, as supporting maintainers with funding has a large effect on the Rust Project as a whole. [RFC #3931][rfc-3931] established the Funding team, but it was a bit light on details.

This RFC defines a charter for the Funding team, which contains high-level tenets that the Funding team should follow, and also some ground rules that help ensure that the Funding team will operate in a transparent way and that its members will be held accountable. The charter is a high-level document, and it does not specify detailed minutiae of how should the Funding team operate day-to-day, nor does it define details of the programs that the Funding team is expected to operate (e.g. the Maintainer in Residence program or Maintainer grants).

Given the amount of influence that the Funding team holds, due to being responsible for dispersing funds amongst Rust Project members, its membership and affiliation rules are more complex than for most other Rust teams. The charter thus also documents several rules related to team membership, affiliation limits, conflicts of interest and the relationship of the team to the Rust Leadership Council.

Note that the motivation for why maintenance funding is required was already established in [RFC #3931][rfc-3931], so it will not be repeated here.

The Funding team charter will be formed by the Guide-level and Reference-level explanation parts of this RFC. It will be located in the [repository][funding-team-repo] of the Funding team. Large changes to the charter, which would invalidate the spirit of this RFC, should be landed via a new RFC. Smaller changes, clarifications and extensions of the charter may be performed by the Rust Leadership Council via an FCP vote.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

> The words Rust Project contributors and maintainers are used interchangeably within this document. They refer to members of the Rust Project who contribute to the Rust language and its toolchain (i.e. projects and repositories under the `rust-lang` and related GitHub organizations).

The goal of the Funding team is to improve the overall health and long-term sustainability of the Rust Project and its teams, primarily by financially supporting Project team members for maintaining the Rust language and its toolchain.

Its high-level duties are to:
- Communicate with Rust Project teams and maintainers to figure out their maintenance and funding needs.
- Distribute available funds to financially support Rust Project maintainers.
- Communicate with funders and promote the work of supported maintainers to help find and keep sustainable funding.
- Work transparently and in public as much as possible, unless the discussion cannot be made public due to private details being involved.

The Funding team has to make difficult choices about whom to financially support, as often there are several great choices, but not enough funds to support all of them. It should strive to make decisions that will be the best for the overall health of the Rust Project and its teams, including the long-term sustainability of the funding program itself.

### Activities of the team

Below is a non-exhaustive list of activities that the team is expected to perform to fulfill the duties described above.

- Communicate with Rust Project teams to learn about their maintenance needs and other kinds of desired support.
- Communicate with Rust Project members to learn about their funding needs and other kinds of desired support.
- Distribute available funds to financially support Rust Project contributors, based on evaluating the most pressing maintenance needs of Rust Project teams.
- Ensure the success of, and run, the Maintainer in Residence program, as defined by [RFC #3931][rfc-3931].
- Support Rust Project contributors via [grants].
- Ensure that Rust Project contributors funded by this team are satisfied with their arrangement, are not blocked from doing their designed work, and are not under undue pressure from funders.
- Promote the work of Rust Project contributors funded by this team and its positive effect on the Rust Project to attract more funding, for example by writing blog posts or recording podcasts or videos.
- Communicate with funders (both companies and individuals) to find new funding opportunities and ensure that they are satisfied with the way their provided funds are being spent.
- Provide funding suggestions to external maintainer funds or companies interested to hire maintainers themselves, to help steer funds not managed by the Funding team itself towards the betterment of the Rust Project. The Funding team may also pool funds with such funding entities to co-fund Rust Project contributors.
- Help finding long-term sustainable funding and new funding sources to sustain Rust Project contributors, for example by coordinating promotional fundraisers together with the Rust Foundation.
- Responsibly manage the available budget and the funds from the Rust Foundation Maintainers Fund, to ensure long-term sustainable funding for Rust Project contributors.
- Communicate with the [goals][t-goals] teams to ensure that maintainers are properly supported to provide reviews and maintenance of accepted Rust [Project Goals][goals].
- Serve as a point of contact for various funding initiatives related to Rust Project maintenance.
- Regularly evauate the effectiveness of applied funding to determine whether it still serves the best interests of the Rust Project.
- Work in the open, unless the discussion requires privacy (for example personal or hiring, funders' interests, etc.), by sharing its meeting notes publicly and proactively sharing their plans with Rust Project members.
- Publicly document its decision-making processes, funding decision rationale and funding program details.
- Be open to feedback about their processes and funding decisions, in particular from other Rust Project members.
- Work actively, to ensure that funding opportunities are not wasted unnecesarily, and that Rust Project members are not unnecessarily delayed from receiving funds that were already allocated for their support. But also be mindful of waiting for potentially better opportunities, rather than spending all available funds as soon as possible.

[grants]: https://github.com/rust-lang/leadership-council/issues/301

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Funding scope

There are a lot of possible areas that can be funded, but the Funding team cannot possibly be in charge of all of them. The team is expected to be rather small, it will be mostly handling funds gathered for a specific purpose, and same as for many other teams, some of its members might be volunteers, so we have to be mindful of the expected scope, so that the team is able to handle it in a reasonable way.

As an example, here are some areas that are explicitly out of scope for the Funding team:
- Paying for costs unrelated to supporting contributors, e.g. funding Rust infrastructure. This is completely out of scope for the team, as it is handled by the Rust Foundation.
- Financially supporting contributions to Rust code outside of repositories managed by the Rust Project, e.g. third-party crates.
- Financially supporting contributors that are not a part of the Rust Project. Vetting and interviewing people whom the Funding team is not already familiar with would be a lot of additional work. We usually expect to have enough Project Members to fund before it would be necessary to look for people outside the Project for a given position.
  - That being said, the start of funding someone could coincide with them joining the Rust Project.

What is in scope, and should be the Funding team's primary focus, is financially supporting Rust Project members who contribute to the Rust language and its toolchain, in order to make their contributions sustainable.

### Funding maintenance vs feature development

The relevant funding scope can be further tightened by specifying which kinds of contributions the Funding team should focus on. Very broadly, contributions can be divided into two related, but also somewhat distinct, areas: maintenance and feature development.

Maintenance is a process of keeping a given project or repository working as expected, by triaging and resolving issues, fixing bugs, improving its quality, and performing code reviews or refactorings that make it possible to evolve the project over the long-term. It is often very broad and somewhat hard to define, as it can span many diverse activities. Typically, maintaining a project requires a lot of upfront context that can take a long time to gather, and so the pool of people who can act as maintainers tends to be limited. Because of its open-ended nature, and the fact that its immediate benefits can be difficult to demonstrate, it can also be quite difficult to find funding for maintenance work.

Feature development typically has a more concrete goal, e.g. implementing a specific new feature or improvement to Rust. It tends to be more approachable to newer contributors, and it is also easier to demonstrate progress on it and show its benefits to potential funders.

Companies are usually more motivated to fund development of specific features that can unblock their use-cases, rather than to fund open-ended maintenance of the Rust toolchain as a whole.

The Rust Funding team will primarily focus on funding Rust Project members for maintenance work on the Rust toolchain. While that might include also some development of new features, that is typically not the primary goal of the funding that the Funding team will provide. Funding large-scale feature development is under the scope of Rust [Project Goals][goals], which have a separate funding mechanism.

That being said, feature development and maintenance is symbiotic. By supporting maintainers, we also fund code reviews and refactorings that are crucial for enabling new features to land more quickly, in a way that will not uncontrollably increase the technical debt of the given codebase.

The Rust Funding team will also cooperate with the [Goals team][t-goals] and in general be involved in discussions about funding Rust Project Goals, for example to provide incentives for companies who fund Project Goals to also contribute to funding maintenance.

### Grants and other forms of support

[RFC #3931][rfc-3931] had an [unresolved question][rfc-3931-unresolved-question] about handling grants. Based on the initial experience of the Funding team, we consider it better to handle all maintenance related funding at one place, to have as much context as possible, and to avoid duplicating and fragmenting work.

The Rust Funding team thus also becomes responsible for allocating small-scale grants to Rust Project members for their Rust contributions.

At the time of writing of this RFC, there are currently some Rust maintainers funded directly from the Rust Project Priorities budget, who report to the Leadership Council. To avoid fragmenting similar efforts, the Funding team should strive to unify all maintainers funded from Rust Project budgets under its banner.

[goals]: https://goals.rust-lang.org
[t-goals]: https://rust-lang.org/governance/teams/#team-goals
[rfc-3931-unresolved-question]: https://rust-lang.github.io/rfcs/3931-rfmf-rust-foundation-maintainer-fund.html#organizational-form-of-the-funding-team

### Relation to the Leadership Council

Given the amount of influence that the Funding team has, it has a special relation with the top-level governance body of the Rust Project, the Rust Leadership Council:
- The Funding team becomes a subteam of the Rust Leadership Council.
- The Leadership Council determines the [membership](#team-membership) of the Funding team.
- The Leadership Council has to approve any funding that the team allocates to a member of the Funding team itself or to a member of the Leadership Council (see [Conflicts of interest](#conflicts-of-interest)).

The Funding team reports its budget (i.e. how much money it has and expects to spend in the following year) and general status (how are funded maintainers happy with their role, what is the status of finding new funding sources, etc.), along with additional context periodically to the Council, at least once a year. This is so that the Council can evaluate the effectiveness of the funding, and make informed decisions when planning its own Project Priorities budget.

### Team membership
[team-membership]: #team-membership

The members of the Funding team are selected by the Rust Leadership Council. The Council ensures that the team is staffed with enough members, as there is a lot of continuous work to do in the team. Ideally, the team should always have at least four members.

The Council is encouraged to examine the membership of the team at a regular cadence (e.g. every six months) to ensure that its members still want to actively participate in the team activities. This opportunity can be used to add more members to the team, if it is understaffed, or remove members that no longer want to be a part of the team. The Council will ask for nominations for new Funding team members from Rust Project members.

When the team membership is updated, we suggest for the members who are stepping down to help with onboarding new team members for some time, to ensure continuity.

Members of the Funding team must be, and remain, in good standing with the Rust Project. The moderation team should be consulted prior to adding a new member to the Funding team. Only existing Rust Project members may join the Funding team, because the work that the team does requires knowing how the Project works and what are its needs.

The Funding team has a special and tight relationship with the Rust Foundation, which handles the funding budget, contracting, legal matters and the Rust Foundation Maintainers Fund, which is the primary source of funds that the Funding team works with. Because of that, at least one member of the Funding team should come from the Rust Foundation, to streamline communication and provide insight into financial and budgeting matters.

The Council is also encouraged to consider having at least one member of the Funding team be a part of the Project's governance (either a Council member or a Project Director), to facilitate communication between the Funding team and the governing bodies of the Rust Project.

The Funding team selects its lead(s) amongst themselves.

### Affiliation limits

> This section supersedes the affiliation limits defined in [RFC #3931][rfc-3931].

Funding team members are subject to affiliation limits, to avoid disproportionate influence of companies or other legal entities on its agenda. No more than one member of the Funding team may come from the same company, legal entity, or a closely related set of legal entities.

If this limit is breached due to employment changes, the affected members should come to an agreement to decide which one of them will step down. If they cannot agree on this, the Leadership Council should promptly decide it.

This affiliation limit does not apply to the Rust Foundation, given its special relationship with the Rust Project. Many Rust Project members may be somehow affiliated with the Rust Foundation, and it does not make sense to restrict them from being members of the Funding team only because of that.

For more details about affiliation limits, see the [Leadership Council Affiliation policy][lc-affiliation-limits], upon which this rule is based.

[lc-affiliation-limits]: https://rust-lang.github.io/rfcs/3392-leadership-council.html#limits-on-representatives-from-a-single-companyentity

### Conflicts of interest
[conflicts-of-interest]: #conflicts-of-interest

Members of the Funding team and members of the Rust Leadership Council might also be interested in asking for being funded by the Funding team. That support could either be for performing work inside the Funding team or the Leadership Council (both can be quite time intensive), but also for unrelated Rust work in other teams. We will use the term "conflicted candidate" in the rest of this section to refer to Funding team or Council members asking for being funded by the Funding team.

Conflicted candidates are also Rust Project members, and deserve the possibility to ask for being funded for their work. However, this of course also constitutes a clear conflict of interest.

Conflicted candidates may ask to be financially supported by the Funding team for a specific role (e.g. being a Maintainer in Residence for a specific team). However, we instate the following rules to lessen (though not completely remove) the conflict of interest that would arise from this:

- They must not vote on themselves, and "leave the room" when any decisions about them being funded are made.
- They must not privately discuss the role that they are actively applying for (or being considered for) with the Funding team, Funding team advisers, or the Leadership Council.
- They must abstain from all decisions made about the role that they are actively applying (or being considered) for. This includes decisions whether the given role (or supported team) should be prioritized against other considered roles.

Additionally, if the Funding team decides to financially support a conflicted candidate, that decision has to be explicitly approved via an FCP vote by the Leadership Council prior to being finalized. If the conflicted candidate is a member of the Leadership Council, that member must abstain from this approval and all related discussions. The Funding team will present arguments to the Council for why they consider funding a member of the Funding team (or the Council) to be the best use of available funds, so that Council members can make an informed decision.

The Funding team should also discuss decisions to fund conflicted candidates with the broader Rust Project to gather feedback on it, prior to that decision being made.

When making funding decisions about conflicted candidates, the Funding team should strive to decide as if the conflicted candidate was any other Project member.

[funding-team-repo]: https://github.com/rust-lang/funding

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Should we allow funding Council/Funding team members?

There are two primary factors that cause a conflict of interest in this situation: the conflicted candidate actively participating in the decision to fund themselves, and the conflicted candidate having influence that affects the decisions of other Funding team members.

The first conflict is resolved by the rule [described earlier](#conflicts-of-interest); Funding team members cannot vote on themselves nor participate in any decisions leading to themselves being funded by the Funding team.

The second conflict is much more difficult to resolve. Suppose that we forbid Funding team members from being funded completely, to ensure that their Funding team membership will not influence their colleagues' decisions. If a Funding team member would need funding, they would thus have to step down from the team. But even that would not completely remove their influence on other members of the team! It would still be their former colleague, with which they worked recently.

How much time would they have to wait to remove any semblance of a remaining conflict of interest? Or would a former Funding team member be banned from getting funding even after leaving the team? While in the meantime, they might reduce or stop their contributions due to not getting the necessary funding, and the team they were supposed to support will miss out (unless there is someone else the Funding team could hire instead). The Funding team itself might also lose an opportunity to help their inner workings by having an additional person on the team being funded.

The reality is that there is some amount of conflict of interest with almost any candidate who asks for being funded. Members of the Funding team might be team colleagues with the candidate in other teams, as many Project contributors are members of several teams. And even outside of team boundaries, many people within the Project know each other and are acquaintances, if not outright friends.

The (relatively permissive) rule thus acknowledges that this conflict can exist regardless of whether the candidate is a Funding team or a Council member or not, and puts trust into the Funding team members to decide what is best for the Rust Project.

It also adds an additional safety guard on top by requiring the Leadership Council to explicitly confirm financial support of Funding team members. This would also prevent a (theoretical) possibility where the members of the Funding team would collude and cross-fund each other, even if that would not be in the best interest of the Rust Project.

As an alternative, we could forbid Funding team and Council members to receive any funding from the Funding team. That would remove a possible perception of conflicts of interest arising in this situation, at the cost of some Project members being unable to ask for funding.

### Increase funding scope

We could consider expanding the scope of the Funding team, to also explicitly support feature development, or funding people who are not members of the Rust Project, or even funding contributions outside of Rust Project repositories.

However, we should be mindful of two things:
- The Funding team budget will be limited, and it cannot be used to fund everyone and everything. The Funding team can cause a large positive effect on the Rust Project by focusing on a single, but very important, area (maintenance), rather than trying to improve too many areas by a small amount. Maintenance is also typically very difficult to find external funding for, so it makes sense to focus on it and fund things that no one else would.
- The amount of projects and people that the Funding team will be funding is limited by its management capabilities and available time. Supporting too many initiatives would likely stretch the team very thin; we cannot reasonably make the Funding team act as executive officers of a small company. Focusing on a single area, and projects and people that the Funding team members are already familiar with, will make it easier to manage the funded maintainers and stay on top of things.

That being said, this RFC does not *prevent* the Funding team from also being active e.g. in trying to find funding for Project Goals or feature development. It merely states that funding maintenance work should be its priority.
