# Rationale and alternatives

The design space for governance is quite large. This section only attempts to address the largest and most consequential alternatives to the design decisions presented in this RFC. This section presents each such alternative along with justifications for why they were not selected.

## Broader governance changes in this RFC

We considered doing *more* in this RFC to set up initial governance structures and improve existing governance structures. In particular, we considered changes to the existing set of top-level teams.

However, we felt strongly that anything that *could* be deferred to the Council should be, and that this RFC should focus on defining and delimiting the Council itself and its interactions with the rest of the Project. We felt it would go beyond the mandate of the transitional leadership structure to do much more than just architecting long-term leadership.

We also felt that further incremental evolutions would become much easier with the structures proposed by this RFC in place.

We recognize that changes to the set of top-level teams will prove especially difficult. However, we felt that the interim leadership group (including top-level team leads) would have that problem in common with the Council. Furthermore, we found that many members and leads of top-level teams were if anything *enthusiastic* about potential systematic improvements in this area, rather than resistant to them, even when such changes involved their own teams.

Apart from that, developing and building consensus on this RFC already represented a massive time investment by many people, and making it larger would make it take even longer.

## Alternative Council structures and non-representative Council members

As an alternative to Council representatives exclusively being the representatives of top-level teams, we extensively considered other structures, whether in addition or in place of that. For instance, the Council could appoint additional members, or appoint successors, or some or all Council representatives could be elected by the Project. Such approaches could potentially make it easier to represent aspects or constituencies of the Project not *yet* represented by existing top-level teams, before even the nascent structures of those teams started to take shape.

Specific variants we decided not to pursue:

### Non-representative Council structures

Alternative structures in which Council members are not representatives of top-level teams would have various drawbacks:
- Any structure that does not guarantee each team a representative would provide less comprehensive and balanced representation for existing teams.
- A structure not based on team-appointed representatives would make it harder to change representatives quickly and easily in a pinch, such as in response to changes in personal circumstances, or changes in a representative's affiliations that cause a violation of the limits placed on shared affiliations.
- Some variants of this (such as Council-appointed additional members or Council-appointed successors) would steer the Council towards a more self-perpetuating nature that we wanted to avoid.

Ultimately, we addressed part of this issue by instead allowing the Council to easily create provisional teams (so as to introduce additional *representatives* on the Council), and then made room for the Council to further evolve its structure in the future by consent.

### Elections

Any structure involving elections would raise additional problems:
- Accurately determining the electorate: who precisely qualifies as being "part of the Rust Project"?
  - Many people have intuitive ideas about this, and such intuitions don't currently cause problems because we don't tie much of substance to that status. However, such intuitive definitions cause serious issues if membership in the Project determines eligibility to vote.
- The usual problems of popularity contests: not all folks doing organizational/coordinative work are especially visible/glamorous/popular, and those doing visible/glamorous/popular work may serve the Project better doing that work rather than reallocating their time towards organizational/coordinative work.
- Elections motivate some form of campaigning.
- A robust election system would introduce more process complexity, both directly for the voting process, indirectly by making it harder to rotate/replace candidates in a pinch or supply alternates/backups.
- Elections would introduce more difficult challenges when needing to change representatives quickly and easily in a pinch, such as in response to changes in personal circumstances, or changes in affiliation that run into the limits upon shared affiliations. The voters will have chosen candidates, and it's harder to go back to the electorate for new candidates, so there would have to be (for example) careful rules for selecting backup candidates based on the next lower number of votes.
- Elections, no matter what voting system they use, inherently ignore the consent of many constituents.
- Simpler election structures would not guarantee teams a representative, and would thus provide less comprehensive and balanced representation for existing teams. Providing more comprehensive/proportional representation of teams would add even more complexity to the election system.
  - In particular, if the people in the project fall into teams in a vaguely Pareto-style structure (a small number of teams contain a large number of people), a simple election structure may result in many teams having *no* representation.

We felt that we could better improve people's routes to be heard and taken into account by ensuring all governance structures and all Project members are connected through parent teams, and thus that every member of the Project has at least one representative on the Council.

## Referendums

We considered introducing a full-fledged referendum system, by which proposals could be introduced, supported, and voted on by the Project as a whole. This would sidestep issues of ensuring proposals get considered and added to the Council's agenda, and would make it easier to make substantial changes not aligned with the existing Council (for better or for worse); it would also serve as an addition check and balance on the Council.

However:
- This would have all the problems mentioned above about determining constituency in the Project.
- This would also be a *complex* new structure introduced entirely in this RFC (rather than later by the Council).
  - This mechanism and its eligibility and corner cases would need to be *very* precisely specified, as it would often be invoked in situations with high tension and a need for a timely and authoritative decision.
- Voting mechanisms, no matter what voting system they use, inherently ignore the consent of many constituents.
  - Voting mechanisms trend towards picking winners and losers, rather than consensus-seeking and finding ways to meet *everyone's* needs.
- If such a mechanism were trivial to initiate, it could become a dysfunctional pseudo-"communication" mechanism in its own right, substituting for healthier communication and more consent-driven actions. It would, effectively, escalate problems into public dirty laundry, making it *harder* to resolve smaller problems. In addition, reporting on such events can generate unwarranted news like "Rust considers X" even if X has no meaningful support.
- If such a mechanism were not trivial to initiate, the type of grassroots organizing required to successfully raise and pass such a referendum would produce better effects by working through teams, when the Project is well-aligned.
- Conversely, if the Project has substantial issues aligning with its leadership, making *individual* decisions doesn't solve the underlying problem with Project health.

We chose to instead provide extensive checks on the Council itself, and mechanisms to ensure feedback and alignment between the Council and the Project, as well as a last-resort mechanism, rather than providing an ongoing mechanism to make or override *individual* Project-wide decisions.

## Alternative checks and balances between the Leadership Council and the Project

We considered many structures for additional checks and balances between the Leadership Council and the Project:
- We considered "vote of no confidence" mechanisms, but these would have many of the same problems as referendums, including determining the electorate, being either too difficult or too easy to initiate, and tending towards escalation rather than resolution.
- We considered arrangements in which members of teams could directly raise objections to Council RFCs. However, this added complexity for something that the consent decision-making mechanism *should* make redundant.
- We considered more formal feedback systems that could provide checks on *individual* Council decisions. However, any such mechanisms would also make it difficult to make timely decisions, and the blocking mechanisms would cause problems if they were either too easy or too difficult to initiate.

## Alternative checks and balances between the Leadership Council and the moderation team

We went through substantial tuning on the checks and balances between the Leadership Council and the moderation team:
- We considered making audits not automatically granted, and instead having the Council decide whether to grant an audit request. However, this would raise fairness questions for how the Council decides when to grant an audit based on limited information, as well as motivating procedural delays to give time for such an evaluation. We also felt that automatic audits (at least initially) would provide an opportunity to thoroughly test and evaluate the audit process.
- We also considered structures using separate auditors rather than using the "contingent moderators" as auditors, but this raised *severe* trust issues with sharing private moderation information with those auditors.

## Launching pad alternatives

We considered other alternate structures apart from the "launching pad", for handling existing teams that aren't attached to the rest of the team structure. For instance, we considered attaching such teams directly to the Council; however, this would have required special-case handling for representation that would start to look a lot like the launching pad, but with more coordination work attached to the Council.

We also considered options in which we *didn't* connect those teams, and permitted "disconnected" working groups and similar. This would require less transition, but would leave many Project members unrepresented and disenfranchised.

We felt that we could best improve people's routes to be heard and taken into account by ensuring all governance structures and all Project members are connected through parent teams.

We considered giving additional purviews to the launching pad, such as contributing to team organization and structure, best practices, or processes. However, the launching pad is already the one exception in which this RFC creates a new team, and we already have concerns about successfully staffing that team; we don't want to add further complexity beyond that in this RFC. The Council has the option of changing the launching pad's purview in the future.

We considered the name "landing pad" (a place for unattached teams to land) instead of "launching pad", but we felt that "launching pad" better conveyed the idea of incubating teams and helping them thrive rather than just serving as a holding area.

## Double-linking

We considered adopting a "double-linking" structure between the Council and top-level teams, in which teams have two representatives on the Council, one more responsible for connecting team-to-Council and the other more responsible for connecting Council-to-team. Such redundancy could provide a firmer connection between the Council and teams, making it *much* less likely that concerns from teams would fail to propagate to the Council and vice versa. However:
- Such a structure would require either an unmanageable number of Council members or far fewer than our current number of top-level teams (and we did not want to change the number of top-level teams in this RFC, or limit the number of top-level teams that strongly).
- This would require substantial changes to the structures of top-level teams themselves to permit such linking, and such structural changes would go beyond the remit of this RFC.
- Some models of double-linking would have one of the representatives determined by the team and the other by the Council; such a model would add complexity to the membership of the Council, such that members were not exclusively the representatives of top-level teams, which would have many of the downsides of such variations mentioned above, notably giving the Council a more self-perpetuating nature.
