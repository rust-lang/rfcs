- Feature Name: N/A
- Start Date: 2026-07-15
- RFC PR: [rust-lang/rfcs#3984](https://github.com/rust-lang/rfcs/pull/3984)
- Rust Issue: N/A

## Summary
[summary]: #summary

Re-organise the library team:

- Re-define and rename the categories of membership
- Change how team members are selected
- Document expectations of team members and maintainers
- Define how FCPs are handled by the team

## Motivation
[motivation]: #motivation

The library team has evolved considerably over time, with its current structure having been set by [team repository PR 588][team588] in 2021. This change defined the extant separation between the libs and libs-api teams, and introduced a libs-contributors team in line with the structure of the compiler team at the time. However, since that restructure, the composition of the two top-level teams has drifted increasingly closer; as of writing, 3/5 libs members are also libs-api members and vice versa, with meetings for the two teams seeing similar participants in attendance.

This fuzzy overlap between top level teams results in occasional friction during their respective team meetings, with time often spent attempting to determine whether a given agenda point is indeed relevant for said meeting or should be discussed in the other top level team's meeting. Attendance of these meetings also does not necessarily follow either the distinction between the teams or the separation between libs/libs-api and libs-contributors, with the FCP rights of the top-level teams not matching either those involved in conversations or those doing work on various aspects of libraries.

Furthermore, the libs-contributors team now numbers 20 members, alongside auxiliary teams that strongly overlap in responsibilities such as crate-maintainers. The permissions afforded to the members of these teams are approximately equal between each other, and the distinction moreso represents the historical circumstances whereby a given member entered the broader libs team family than their concrete current areas of involvement.

Given these changes, we propose a simplified structure in order to facilitate the work done by the library team and its subteams.

[team588]: https://github.com/rust-lang/team/pull/588

## Definitions
[definitions]: #definitions

There are various permissions/privileges/responsibilities which will be referenced in later sections of this RFC, defined here:

- **umbrella privileges**
  - Several privileges are to be afforded to all members of all library teams. These constitute the *umbrella privileges*.
  - Most notable is *r+* privilege, namely permission to approve pull requests to be merged by *bors*. Members should not merge their own pull requests (with the exception of re-approving their own work on behalf of another member after a rebase or similarly trivial change). *r+* permissions apply to the whole repository, but it is expected that members limit themselves to only those (parts of) *rust-lang* organisation repositories that are under the purview of the library team (unless granted *r+* from other teams too), and that best judgement is exercised with regard to the areas of the relevant codebase(s) involved being only the one(s) that they are confident reviewing. All members are granted this privilege.
  - All members are also granted permission to initiate a *try* build with *bors*, alongside performance benchmarking with *rust-timer*, ecosystem testing with *crater*, broader permissions with *[triagebot]*, and use of the [developer desktops][desktop].
  - All members are also added to the *rust-lang/libs* team in the GitHub organization and can be assigned to issues/pull requests, modify labels, receive group mentions, receive a "Member" badge next to their name, and similar such privileges.
  - All members are able to begin the FCP process or raise blocking concerns in it, which must be addressed for the process to resolve. If technical limitations, such as the current implementation of [rfcbot], prevent a member from beginning an FCP or raising such a concern, a member with *FCP checkbox* privileges (see below) will start the FCP or raise the concern on their behalf.
- **review rotation**
  - Members on the *review rotation* will be randomly assigned to new pull requests submitted to the standard library. Being on the review rotation is one of the best ways for members to help the libs team and learn new parts of the library. Review capacity is one of the most important resources that the team has, as it enables our progress in the standard library's continued development and maintenance. There is no concrete expectation that libs members be on the review rotation, but presence on rotation may be used as one of multiple indicators of a member's activity in the project and all members of the libs team are encouraged to add themselves to the rotation.
- **FCP checkbox**
  - Members with an *FCP checkbox* are those whose positive consent and not mere lack of objections is generally mandatory in order to sign off on decisions that require the final comment period process. That is, in order for an FCP to be resolved, there must be no outstanding concerns, at most two unticked checkboxes may be outstanding, and this must stay the case for 10 days. Each member with an *FCP checkbox* contributes one to the total count of checkboxes that exist.

Further, several terms which appear throughout this RFC should be understood to have a particular meaning:
- **second**
    - In the situations listed below where team members have the option to *second* a proposal or nomination, followed by some period of time passing, the intended meaning is that there must exist some particular seconding which is in effect for the entire duration in question. The longest-standing seconding being withdrawn from some given proposal results in the relevant countdown being turned back to the duration of the next-longest-standing seconding.
- **deadlock**
    - A conversation is understood to be in *deadlock* when, following extensive conversation, neither party to the conversation is willing or able to reach a mutual compromise with the other(s). The mechanisms about deadlock resolution specifically apply when these parties are members of the libs team.
- **self-nominate**
    - Self-nomination to various positions is to be effected privately to a team lead. This does not necessarily need to be in writing. It is the duty of the team lead(s) to present the list of nominees to the appropriate body afterwards.

[triagebot]: https://forge.rust-lang.org/triagebot/index.html
[desktop]: https://forge.rust-lang.org/infra/docs/dev-desktop.html

## Team structure
[team-structure]: #team-structure

The libs, libs-api, and libs-contributors teams will be consolidated into only two distinct teams, as below.

### Libs team
[libs-team]: #libs-team

The top-level team consists of all members of any team previously under the libs team family. That is, all former members of libs, libs-api, and libs-contributors are to be consolidated into a new libs team. Members of the crate-maintainers team are automatically extended an invitation to join the new libs team should they so desire. Members are granted *umbrella privileges* as above, with the possibility of adding themselves to *review rotation*. The existing crate-maintainers, regex, portable SIMD project group, and allocator working group are left unaffected.

The libs team should hold at least one weekly meeting, with its scheduling determined by [libs-fcp][libs-fcp-team], wherein at least items nominated for the libs team are discussed. The libs-fcp team is free to decide internally when these meetings are held, their duration, and which topics are or are not relevant to any given meeting, so long as they are held on a consistent schedule which reasonably accomodates for the availabilities of all libs-fcp members and enables participation from other libs members. All meetings are to be open to the public unless the libs-fcp team decides by consensus to restrict attendance for some particular meeting. Minutes and agendas from public meetings must also be posted publicly on Zulip.

#### Selection process

Anyone whose addition may be valuable to the development of the Rust standard library or related crates under the purview of the libs team may be nominated for membership in the team by an extant member. This nomination does not necessarily have to be expansive, but should be sufficient to assure the team that the nominee can be trusted with *umbrella privileges*. This may be as brief as a single sentence if there is no serious ground for concerns. Note that contributions need not necessarily consist of code, but also include documentation, productive engagement in complex conversations, and anything that could be seen as facilitating the work of other members.

Any preexisting member may issue an objection to a nominee. Objections are considered blocking in the nomination process, and are to be accompanied by motivating reasoning. The criteria which may justify an objection are left intentionally unspecified, so as to allow for flexibility in the decision-making process. By courtesy, it is recommended that objections be communicated privately to the team leads instead of being announced publicly; however an anonymised copy of the motivation should, if possible, be made public.

Members may also second a nominee if they believe them to be a valuable addition to the team, without necessarily requiring any motivation.

If a period of 10 days has passed within which a nominee has been seconded and has no outstanding objections, the team leads must assess whether the nomination has been sufficiently discussed by team membership; if so, the nominee is added to the team and granted the relevant privileges. If the nominee was not previously part of the Rust project, moderation will first be asked to ensure there are no potential issues with their joining, with the addition to the team only taking effect after moderation signoff.

If on the contrary, 20 days after a nomination, no seconds are outstanding and/or there are still outstanding objections, a countdown of 10 days begins. If objections persist, or, lacking objections, no seconds are put forward, the nomination is automatically withdrawn. Note that this requires that there exists some singular objection which is outstanding for the entirety of these 10 days.

#### Expected activities

Libs team members should make an effort to remain engaged with the Rust project in some capacity, but such criteria are to be interpeted loosely. Engagement in pull request or issue discussions, being on *review rotation*, activity in team meetings, and/or activity on Zulip or other official platforms of the project in matters relevant to the standard library and related crates all constitute relevant engagement with the project.

More broadly, members of the libs team constitute a core part of the Rust project and thus are bound by the expectations placed upon such members, including upholding both [the spirit and the letter of the Code of Conduct][coc].

After some member of libs has been inactive for at least twelve months, they may be asked if they wish to remain on the team; otheriwse, they may be moved into alumni, with privileges revoked. An alum may at any point self-nominate to be reinstated, requiring a second and 10 day period without any objections as if they had been nominated by a team member in order to be reinstated.

#### ACP process

Any libs team member may approve an [API change proposal][acp] that has been submitted (excepting the author of the ACP). It is highly recommended, though not explicitly required, that a waiting period of 10 days from submission be observed between an ACP being submitted and it being approved.

Approval of an ACP does not necessarily guarantee that it will be included stably in the standard library, but signals that the relevant proposal may be of interest to the team, and that it seems *reasonably likely* in the judgment of a libs member to pass a subsequent libs-fcp team FCP without requiring changes on the scale of a total overhaul. It is left to the judgement of individual team members whether some given ACP has received the necessary amount of scrutiny before being approved, and approval should signal confidence from the approver in the ACP's chances of future stabilisation. Note that ACPs proposing a [breaking change][breaking] require a subsequent FCP.

An ACP should *not* be approved if its author does not appear to have thoroughly explored alternative solutions and there appear to be, or will be in the near future, outstanding conflicting proposals that have not also been given due consideration.

Any libs team member may object to an ACP's approval. Note that if the only concerns regard naming/bikeshedding, without outstanding concerns over a proposal's structure, these concerns should be deferred until future stabilization to allow development to proceed.

If an ACP appears major enough to warrant in-depth discussion or its implementation requires sufficient work that a stronger signal from the team in confidence for its success is necessary, or if it has both objections and support, any participant to conversation may nominate that ACP for synchronous discussion in a team meeting, at which point meeting participants on the libs team are expected to issue a joint opinion on the proposal.

An FCP resolving in favour of a proposal overrides any standing objections to the given ACP.

This process does not currently have any explicit mechanism to make a decision to reject an ACP and close it; designing and evolving a process for doing so is left as the prerogative of the libs-fcp team, to avoid making it part of this RFC.

The libs team will attempt to regularly curate and manage the ACP backlog.

[acp]: https://std-dev-guide.rust-lang.org/development/feature-lifecycle.html
[coc]: https://www.rust-lang.org/policies/code-of-conduct

### Libs FCP team
[libs-fcp-team]: #libs-fcp-team

A subset of the libs team is to be granted an *FCP checkbox* in order to decide on matters relevant to the standard library more broadly. This team is to consist of at least 5 and at most 8 members, with the exact number free to oscillate between these extremes.

#### Selection process

Every 12 months, the membership of the libs-fcp team is reshuffled. Any member of the libs team may self-nominate for the position, including those already on the libs-fcp team. The old libs-fcp team (possibly by delegating this responsibility to a [facilitator], if they so choose) is then to pick between 5 and 8 (inclusive) of the nominees, which will constitute the new team. The new membership list must be approved by all former members of the team; that is, there must be no active objection to the new list.

The self-nomination period lasts 14 days, following which the old libs-fcp team has 14 more days to announce the new membership list. The old team is free to determine the process by which it makes this selection, so long as a new team is assembled within this timeframe. While not mandatory, the recommended process for assembling a new team is the appointment of one of the team leads as the [facilitator] as for the leadership council elections, even if they have nominated themselves; this facilitator is to select from amongst the nominees sufficiently many candidates so as to constitute a new libs-fcp team, with members of the old libs-fcp team having the right to object to any subset of these candidates. This feedback is to be relayed to the facilitator, and once a list is proposed wherein no candidate has outstanding objections from any libs-fcp member, the list is approved and can be announced. Once the new membership list is announced, it comes into effect immediately.

If insufficient nominees exist to reach the minimum size threshold, or if a member of libs-fcp resigns or is removed during the course of their 12-month term dropping the membership count below 5, the team must attempt to explicitly invite libs team members to join libs-fcp until the team meets the minimum membership count again. If no potential candidates have consensus approval, libs-fcp may continue at a reduced size until such candidates are found. Libs-fcp should further attempt to nominate promising candidates from outside of the libs team for membership in libs to facilitate its reassuming of the desired minimum size. The invitation mechanism into libs-fcp may also be invoked any time the libs-fcp team consists of less than 8 members, should the team agree to do so by consensus.

The initial composition of this team is to be determined jointly by members of the previous libs and libs-api teams by the same process, as if the libs team had constituted a previous libs-fcp team composition.

[facilitator]: https://github.com/rust-lang/leadership-council/blob/main/guides/representative-selection.md

#### Expected activities

Members of libs-fcp should regularly review proposals with outstanding FCP proposals or which are in FCP.

Libs-fcp team members are held to a higher bar of activity than regular libs team members, and are expected to contribute to conversations on changes to the standard library notable enough to be on team meeting agendas and involve themselves in discussion on extant FCPs. Libs-fcp team members are also expected to either regularly attend real-time team meetings or be sufficiently active in async conversations so as not to unduly delay coordination or decisions.

#### Scope of FCP process

An FCP, or Final Comment Period, is required for any change which would either be visible in a stable release or which is seen as a major addition to the standard library. The process is handled by [rfcbot], wherein all those with an *FCP checkbox* may signal agreement with a proposal or lodge an objection. If 10 days pass wherein at most two checkboxes are outstanding and no objections are lodged, the FCP will close automatically in the direction it was proposed (i.e. in favour or against).

If an FCP appears unresolvably deadlocked due to blocking concerns from non-libs-fcp members, libs-fcp consensus may overrule those concerns.

Additionally, it is expected that the broader libs membership is kept up-to-date with ongoing FCPs, such as by a ping when an FCP opens. In the case that only a subset of the libs team wishes to be informed of every new FCP, an ad-hoc ping group or team may be formed.

#### Ad-hoc subteams

It may be the case that the libs-fcp team wishes to separate its activities along various lines, such as distinct areas of the standard library, API design versus implementation, etc.; in such cases, libs-fcp is empowered to delegate its FCP rights in certain cases to any subteam(s) of itself as it sees fit should there be internal consensus to do so. These may take the form of standing subteams with dedicated meetings and agendas, ad-hoc ping groups, topic-specific FCP lists, or anything in between.

#### Last-resort mechanism for libs-fcp reconstitution

Since libs-fcp selects its own successors, ossification of team membership is a possible concern. This section provides a mechanism for the broader libs team to express no-confidence in the membership of libs-fcp and leadership of libs, and forcibly reconstitute it. This section is written in the hopes that it will never be needed, and that the teams will make every possible effort to resolve conflicts without reaching this point.

Any member of the libs team may request to start a no-confidence vote to this effect. Once started and seconded, a 14-day timer to cast votes begins. Other libs team members (excluding those currently serving as part of libs-fcp) may vote in favour, against, or abstain. Votes may be, but are not required to be, accompanied by motivating reasoning.

After 14 days, if the ratio of yes votes to no votes is at least 6:4 and at least 50% of the libs team has voted, the libs-fcp team is automatically dissolved. At this point, the libs team is expected to select two new team leads from amongst themselves. These lead nominees must enjoy at least 2/3rds support from the team as determined by a vote. The new leads will then propose a new libs-fcp team as a subset of the members of the libs team. The libs team has 10 days to object to this new list; if 10 days pass with no objections, the new libs-fcp team is automatically instated.

This mechanism is intentionally not perfect and may be subject to degenerate corner-cases, but it is written with the understanding that such a case likely warrants outside intervention from Rust Project leadership.

[rfcbot]: https://github.com/rust-lang/rfcbot-rs
[breaking]: https://std-dev-guide.rust-lang.org/breaking-changes/summary.html

### Team leads
[team-leads]: #team-leads

The libs team may have, at any given time, at least one and at most two team leads. Once appointed, team leads do not have a set term length and serve until they voluntarily step down or a forced reconstitution of libs-fcp is triggered. However, team leads are encouraged to rotate the lead positions (alternatingly) every few years.

#### Selection process

Should a team lead wish to step down, members of the libs team may self-nominate for the postion. The former team lead may select a candidate for succession from the list of nominees; this process is mandatory if the lead stepping down is the only extant lead. If only one team lead exists at a given time, they are encouraged to trigger the above mechanism to select a second team lead. In either case, approval of a new team lead requires there be no outstanding objection to the nominated new team lead from any member of the libs team for a continuous period of 10 days.

#### Expected activities

There is no special requirement for being a team lead, beyond being held to the same expected level of activity and participation in conversation as members of libs-fcp. However, team leads are expected to act as a first point of contact for the team and act as a moderating voice during team meetings alongside facilitating the team's general functioning and ensuring proper procedure is followed.

For all purposes except having an *FCP checkbox*, everything previously stated that applies to members of libs-fcp also applies to team leads.

## Prior art
[prior-art]: #prior-art

- This structure, much like this document, is heavily inspired by the successful [rework of the compiler team in 2024 as per RFC 3599][rfc3599].

[rfc3599]: https://github.com/rust-lang/rfcs/pull/3599

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None :D
