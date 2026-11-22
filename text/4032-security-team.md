- Feature Name: `security_team`
- Start Date: 2026-11-20
- RFC PR: [rust-lang/rfcs#4032](https://github.com/rust-lang/rfcs/pull/4032)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC establishes a new `security` team under `launching-pad`, with three subteams (`security-response`, `project-security`, `ecosystem-security`) and a private `security-advisors` pool. It does so by defining the charters of each team, including a charter for the existing `security-response` team, which does not currently have one.

## Motivation
[motivation]: #motivation

Today, we have `wg-security-response`. They have a good handle on incident response. However, they are focused on reactive security and are not interested in expanding scope to proactive security or broader ecosystem security. As we encounter urgent security incidents (for instance: [arrayref](https://blog.rust-lang.org/2026/08/20/supply-chain-attack-on-arrayref/)), there is a risk of excessive strain on the security response team. It does not operate an oncall or make specific guarantees on response time. To keep that approach practical, we need to make build our capacity in other ways and make investments in our forward security posture.

On the capacity-building front, security engineers with the Rust Foundation are already doing some interesting and promising things on the proactive security front: typosquatting checks, dependency graph analysis, and various detection systems. This is not as visible to the Project as it could be, nor is it especially easy to build on top of. Moving this work into a Project team will make it more accessible to collaborators.

More broadly, we could use a "front door" for security-oriented collaborators. It is not appropriate for them to join the "trust circle" of response groups that operate under embargo during incidents. But, there is plenty of system-building, guidance-writing, training, and other work that is not as sensitive. By creating Project teams tackling this work, we improve our ability to attract new contributors and build trust with them.

Related to trust and forward posture, [we have goals to apply crates.io security controls](https://goals.rust-lang.org/2026/crates-io-supply-chain-hardening.html) that will ultimately rely on some amount of consensus since they make opinionated choices. And, some of these choices will need to be semi-private given the "cat and mouse" of security controls. So, having a team of people that can be delegated to make these decisions, under a more broadly agreed-upon framework, seems important.

Because this is such a multi-faceted set of goals, covering different areas of focus and levels of trust, this RFC proposes creating multiple groups and relationships between the groups. Refer to the "Alternatives" section for a deeper discussion of this structure.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The guide provides brief descriptions of the proposed teams. Refer the "Reference" section for full charters per team.

### `security`

Coordinate the Rust Project's security work across its subteams, act as an entry point for people who want to get involved in security, and keep track of the big picture of the Project's overall security posture (reactive, proactive, and ecosystem-facing).

This group is mostly a thin coordination layer. It doesn't do hands on work itself, besides owning the threat model and integrating the subteams' contributions to it. It will highlight gaps in what its subteams are covering and propose charter expansions or new subteams.

In practice, this means holding synchronous or asynchronous discussions on some cadence and generally providing a venue for discussing security incidents. Think of this as along the lines of the #security-chat Zulip stream but that more actively recommends new work if there is general interest.

This group also would author RFCs proposing major shifts in charters, as needed, in collaboration with impacted subteams.

### `security-response`

Triage and respond to incoming vulnerability reports for the Rust Project, coordinate the development and disclosure of fixes, and maintain the Project's ability to respond to security incidents in a timely, confidential, and trustworthy manner. It handles reports that arrive at `security@rust-lang.org` and drives them to resolution and public disclosure.

The primary contact point for this team is the `security@rustlang.org` inbox, shared with trusted members of `ecosystem-security`.

The team is *not* responsible for work around forward security posture. It is largely not responsible for responding to vulnerabilities on ecosystem crates, besides ones owned directly by the Rust Project. Those are the scope of the sibling `project-security` and `ecosystem-security` teams under the `security` umbrella.

### `project-security`

Proactively protect Project-owned repositories and infrastructure and improve the forward security posture of Rust Project contributors. Success for this team means that fewer incidents ever reach `security-response`.

The team produces guidance, tooling, training, and threat-model input that Project teams and contributors are encouraged to adopt, and works alongside t-infra to harden the Project's own resources.

This is the complement to `security-response`: response is reactive, project-security is preventive. Both concern the Project's own systems (as opposed to `ecosystem-security`, which focuses primarily on the wider crates.io ecosystem).

### `ecosystem-security`

Improve the security posture of the wider Rust ecosystem, including crates published on crates.io, and other non-Rust Project software.

This spans proactive work (tooling, detection, information feeds, community guidance) and response work (coordinating action on malicious or vulnerable crates).

The primary contact point for this team for ecosystem-scope response work is the `security@rustlang.org` inbox, shared `security-response`.

To accomplish these goals, the team will consider the health of ecosystem maintainers and burn-out avoidance both of team members and ecosystem maintainers as a key strategic consideration in all approaches.

### `security-advisors`

A marker team of trusted people in a private Zulip channel, `security-advisors`. The channel allows the security subteams to loop in relevant contributors across different Rust Project teams and the broader ecosystem to advise on security responses. For instance, if there is an issue in the standard library, representatives of the `t-libs` will be in the channel to consult or to route responders to specific experts in the team.

The channel also offers an opportunity to privately seek feedback and a thumbs up on whether security measures meet Project norms and published policies. This is necessary because security efforts involve some amount of cat-and-mouse tactics where public discussion of methods undermines efficacy.

The pool is modeled on [`funding-advisors`](https://github.com/rust-lang/team/blob/master/teams/funding-advisors.toml).

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Significant changes to the charters that would impact the scope of other teams, or go beyond the spirit of the initial RFC, will require a new RFC directed to `t-leadership-council`. Other changes to the charters, including creating new subteams that do not impact the scope of other teams, will be handled internal to `security`.

### Security Team Charter
[security-team-charter]: #security-team-charter

#### Mission

Coordinate the Rust Project's security work across its subteams, act as an entry point for people who want to get involved in security, and keep track of the big picture of the Project's overall security posture (reactive, proactive, and ecosystem-facing).

This group is mostly a thin coordination layer. It doesn't do hands on work itself, besides owning the threat model and integrating the subteams' contributions to it. It will highlight gaps in what its subteams are covering and propose charter expansions or new subteams.

If major changes to charters are needed that impact the broader project, this group writes the RFCs, in consultation with impacted subteams.

#### How does this further the Rust Project goals?

The Project benefits from a team that can coordinate security concerns across different front-line teams. It also offers an on-ramp for new contributors.

#### Responsibilities & Commitments

##### Decision Maker

The top-level team is the decision maker on:

- Scope and responsibilities across subteams within `security`, including new subteams when further division is helpful
- The necessary scope and detail of the Project threat model. Subteams provide inputs for their scope of responsibility, but `security` integrates them and owns the full model.

It is NOT the decision-maker for work and infrastructure owned by other teams, such as `crates.io` and `infra`.

##### Accountable for results

- Making sure that subteams are able to coordinate effectively
- Offering a way for new contributors to get involved with security work, both asynchronous and synchronous
- Keeping the Project's threat model current, including addressing newly discovered gaps

The team is NOT responsible for outcomes of subteams' work, such as success of specific vulnerability responses. It can identify process gaps of subteams and suggest alternatives, but it is up to subteams to accept and implement these suggestions. 

##### Responsible for work

- Holding meetings and other coordination points between subteams
- Tracking #security-chat and other security-related channels to highlight ideas and trends that would benefit from direct discussion
- Identifying and proposing a home for new security-related work that is not currently being handled
- Surfacing security related needs, such as funding or new coordination, to external parties such as the Leadership Council, the Rust Foundation, or other Project teams
- Regularly reviewing the Project threat model and making changes, most commonly integrating new inputs from subteams
- Offering spokespeople on general Rust Project security matters 

##### Consulted for input

The team should be consulted on:

- Decisions with broad security implications if it is not clearly in scope of a specific subteam
- Proposals to fund security work  in the Project

##### Informed for awareness

The team should be informed about significant security-related developments encountered by subteams. This will generally be post-disclosure or otherwise involving non-embargoed information, unless members are consulted as part of `security-advisors` discussions.

#### How does this team make decisions?

Generally by consensus, but via RFC for changes that impact the broader Rust process or would benefit from explicit subteam approval.

#### What work is not specifically part of this team's remit?

- Line work on addressing any proactive or reactive security issues, which is owned by subteams

#### Team Processes

##### Contact Point

The `seurity-chat` channel is the primary contact point. Team lead(s) are also available for direct outreach.

`security@rustlang.org` is not intended as a contact point, but relevant communications to this inbox will be routed to `security` by `security-response` and `ecosystem-security` reviewers. 

##### Team Membership

Team members are expected to:

- Regularly provide input into a discussions of the group, asynchronous or synchronous
- Engage with any RFCs directed to `security` in a timely way
- Actively route interested contributors to relevant workstream and public channels, to grow the Project's security contributor base

##### How is team membership managed?

The team reaches out to and invites new members based on internal consensus.

##### Where does this team work?

Primarily the `security-chat` channel, with synchronous meetings announced in this channel. For extended discussions, the team uses subchannels.

#### security-advisors

A marker team of trusted people in a private Zulip channel, `security-advisors`. The channel allows the security subteams to loop in relevant contributors across different Rust Project teams and the broader ecosystem to advise on security responses. For instance, if there is an issue in the standard library, representatives of the `t-libs` will be in the channel to consult or to route responders to specific experts in the team.

The channel also offers an opportunity to privately seek feedback and a thumbs up on whether security measures meet Project norms and published policies. This is necessary because security efforts involve some amount of cat-and-mouse tactics where public discussion of methods undermines efficacy.

The pool is modeled on [`funding-advisors`](https://github.com/rust-lang/team/blob/master/teams/funding-advisors.toml).

#### How does this team make decisions?

This team generally does not make decisions, besides its own membership, which is discussed in a following section.

#### What work is not specifically part of this team's remit?

This is a marker team and does not own any work directly.

#### Team Processes

##### Contact Point

The team communicates with the public via `#security-chat` or via direct outreach to the team's leads(s).

##### Team Membership
Members are expected to:

- Represent their subteam's status and needs in coordination, and carry information back.
- Help route interested contributors and surface cross-cutting needs.

##### How is team membership managed?

New members are nominated by current members and security subteams, and approved by the team's lead(s).

##### Where does this team work?

Full-team communication happens in a private #security-advisors channel. Additional private channels may be created to communicate amongst a subset of the team for embargoed incident responses. Though, more commonly, embargoed communication happens in other private channels where subteams work, with certain security advisors added as needed.

### Security Response Team Charter
[security-response-team-charter]: #security-response-team-charter

#### Mission

Triage and respond to incoming vulnerability reports for the Rust Project, coordinate the development and disclosure of fixes, and maintain the Project's ability to respond to security incidents in a timely, confidential, and trustworthy manner. It handles reports that arrive at `security@rust-lang.org` related to Project-impact security concerns and drives them to resolution and public disclosure. It routes communications that do not require a reactive Project-scope security response to relevant teams (`ecosystem-security` or `security`, generally).

The team is *not* responsible for work around forward security posture. It is largely not responsible for responding to vulnerabilities on ecosystem crates, besides ones owned directly by the Rust Project. Those are the scope of the sibling `project-security` and `ecosystem-security` teams under the `security` umbrella.

#### How does this further the Rust Project goals?

The Rust Project's repositories should not contain vulnerabilities that put maintainers and consumers at risk. Security researchers need a way to report vulnerabilities.

#### Responsibilities & Commitments

##### Decision Maker

- How quickly incoming vulnerability reports are triaged and other process-related guarantees besides that reports will eventually be looked
- Whether a given report merits remediation or disclosure
- The disclosure plan for a given report (for instance: embargo timeline, disclosure date, timing of release, coordination with downstream distributors)
- The content and timing of security advisories published through Project channels (for instance: the RustLang blog, GitHub Security Advisories)
- The primary contents of the incident response related portions of the Project's threat model, though not the threat model itself

The team does NOT decide the actual fixes to vulnerabilities. It also does not override the determination of the team that owns code about whether it is worth fixing.

##### Accountable for results

- Every report sent to `security@rust-lang.org` is ultimately acknowledged and driven to resolution (which includes closing it as not actionable or rerouting it)
   - This does not involve a specific timeline for this triage and remediation
- If a coordinated disclosure is agreed upon, it is carried out as planned
- The incident response content for the threat model is kept current
- The security response process is documented and functional, including custody of the security team's security key and access to the shared secrets store

##### Responsible for work

- Monitoring and triaging the `security@rust-lang.org` inbox (including decrypting GPG-encrypted reports).
- Flagging `security@rust-lang.org` reports related to ecosystem incidents for `ecosystem-security` processes.
- Setting up per-incident private Zulip streams when a fix is deemed actionable
- Coordinating remediation, arranging point releases, and distributing any signed patchfiles
- Authoring and publishing advisories and announcements across the required channels
- Writing content for the incident response portion of the Project threat model

##### Consulted for input

The team should be consulted on:

- Changes to Project infrastructure, release processes, repository policies, or security scope that would materially affect the Project's ability to respond to or contain a security incident
- Sibling `security` subteams' proactive work where it intersects with incident handling (e.g. `project-security` guidance on repo hardening, `ecosystem-security` handling of malicious/vulnerable crates that may generate reports into this team's inbox).
- Designs of security-relevant features by owning teams (e.g. crates.io trusted publishing, cargo build-script sandboxing) where the response team's incident experience is relevant.
- ecosystem-security handling of security incidents for projects outside of `rust-lang`
    - This is a desired state, but initially, security-response is responsible for leading the handling of high-impact ecosystem vulnerabilities according to its existing processes, until ecosystem-security has the process and capacity to handle them.

##### Informed for awareness

The team should be informed of:

- Security-relevant changes shipping in `rust-lang`/`rust-lang-nursery` repositories significant enough to warrant mention in release notes or advisories.
- Ecosystem incidents handled by `ecosystem-security` (e.g. malicious-crate takedowns) so that response and ecosystem work stay coordinated even when the response team is not the one acting.

#### How does this team make decisions?

By consensus, usually in private per-response incident channels.

#### What work is not specifically part of this team's remit?

- Proactive security for Project resources (GitHub Actions usage policy, CI hardening recommendations, publishing hygiene, spearphishing training)
- Wider ecosystem security (RustSec/advisory-db operations, scanning and outreach for crates the Project does not itself depend on, malicious-crate response on crates.io)
    - This is the desired end state, but initially security-response remains responsible for high-impact ecosystem incidents, until `ecosystem-security` has the process and capacity to handle them
    - The exception to this is if a report in ecosystem-security scope happens to arrive through the security-response inbox, in which case security-response is responsible for handing the incident off to ecosystem-security but not following it to completion.
- Building security tooling or detection systems

#### Team Processes

##### Contact Point

Reports and contact go through security@rust-lang.org per the [Rust security policy](https://www.rust-lang.org/policies/security).

##### Team Membership

Team members are expected to:

- Be reachable for incident response according to team-determined processes and share the load of monitoring/triaging the `security@` inbox.
- Handle reports confidentially and follow the documented process (including key handling and coordinated-disclosure obligations).

##### How is team membership managed?

The team reaches out to and invites new members based on internal consensus.

##### Where does this team work?

Process documentation lives in [rust-lang/wg-security-response](https://github.com/rust-lang/wg-security-response).

Live incident handling happens in private, per-incident Zulip streams on the [rust-lang Zulip instance](https://rust-lang.zulipchat.com), named `sec/<incident>`.

General (non-incident) coordination happens in the `#security-chat` Zulip channel.

### Project Security Team Charter
[project-security-team-charter]: #project-security-team-charter

#### Mission

Proactively protect Project-owned repositories and infrastructure and improve the forward security posture of Rust Project contributors. Success for this team means that fewer incidents ever reach `security-response`.

The team produces guidance, tooling, training, and threat-model input that Project teams and contributors are encouraged to adopt, and works alongside t-infra to harden the Project's own resources.

This is the complement to `security-response`: response is reactive, project-security is preventive. Both concern the Project's own systems (as opposed to `ecosystem-security`, which focuses primarily on the wider crates.io ecosystem).

#### How does this further the Rust Project goals?

The Rust Project's own repos, CI, and publishing flows are high-value targets.

A dedicated team that develops shared best-practice guidance, training, and enforcement tooling raises the baseline security of Project resources while respecting each owning team's autonomy.

#### Responsibilities & Commitments

##### Decision Maker

The team is the decision maker on:

- The content of its recommendations, guidance, and training (e.g. safe GitHub Actions usage policy, crates.io publishing hygiene, CI hardening advice, "you are a high-value target" anti-spearphishing training).
- The design of any tooling it builds to help teams audit or enforce those recommendations.
- Contents of the Project threat model related to proactive security

The team does NOT have a mandate to require other teams to adopt its recommendations. This mirrors how t-infra operates with the teams whose systems it touches.

##### Accountable for results

The team is accountable for:

- The existence and upkeep of a set of proactive-security recommendations for Project properties
- Contents of the Project threat model related to proactive security are kept up to date

##### Responsible for work

The team members do the work of:

- Writing and maintaining Rust Project guidance/policies for GitHub Actions usage, CI, and publishing hygiene, and building audit/enforcement tooling for them.
- Developing and delivering trainings (anti-spearphishing, high-value-target awareness) and recruiting/mentoring per-team security points of contact.
    - The Rust Foundation will offer funding and logistics support for this work.
- Authoring contents of the Project threat model related to proactive security
- Collaborating with t-infra on proactive hardening initiatives
- 
##### Consulted for input

The team should be consulted on:

- Changes to Project CI, repository settings, or publishing flows where a security perspective is valuable.
- Designs of security-relevant Project-infrastructure features by owning teams (t-infra, crates.io, cargo).
- Decisions about moving critical tooling with security implications into rust-lang (for instance: widely-used GitHub Actions)

##### Informed for awareness

The team should be informed of:

- Notable incidents handled by `security-response` (after the fact or otherwise respecting embargo)
- Infrastructure changes across the Project with security relevance

#### How does this team make decisions?

Generally by consensus, but via RFC for recommendations or infrastructure changes that impact the broader Rust process or would benefit from explicit team approval.

#### What work is not specifically part of this team's remit?

- Reactive incident response: belongs to security-response
- Ecosystem-facing security (crates.io-wide scanning, RustSec, malicious-crate response)
- Operating other teams' infrastructure
    - Any hands-on operation of Project repos/CI/publishing happens only under explicit, revocable delegation from owning team (primarily t-infra, and the relevant repo/org owners for a given repository)
- Building major missing security features on crates.io or other Project infrastructure. The team might contribute this work, but it will be via the other teams' processes.

#### Team Processes

##### Contact Point

The team communicates with the public via `#security-chat` or via direct outreach to the team's leads(s).

`security@rustlang.org` is not intended as a contact point, but relevant communications to this inbox will be routed to `project-security` by `security-response` and `ecosystem-security` reviewers. 

##### Team Membership

Members are expected to:

- Contribute to guidance, tooling, training, or threat-model work on a recurring basis.
- Engage with owning teams (especially t-infra) collaboratively, respecting their authority.
- Engage with any RFCs directed to `project-security` in a timely way

##### How is team membership managed?

Existing team members may nominate new members, who are approved by team consensus.

##### Where does this team work?

Discussion in the `#security-chat` Zulip channel, with subchannels for directed work as needed.

If synchronous meetings are desired, they will be announced in `#security-chat`.

The team will additional publish recommendations, tooling, and other durable records, to `rustlang/project-security` on GitHub. 

### Ecosystem Security Team Charter
[ecosystem-security-team-charter]: #ecosystem-security-team-charter

#### Mission

Improve the security posture of the wider Rust ecosystem, including crates published on crates.io, and other non-Rust Project software.

This spans proactive work (tooling, detection, information feeds, community guidance) and response work (coordinating action on malicious or vulnerable crates).

To accomplish these goals, the team will consider the health of ecosystem maintainers and burn-out avoidance both of team members and ecosystem maintainers as a key strategic consideration in all approaches.

#### How does this further the Rust Project goals?

Rust has a "thin" standard library and defers a large amount of functionality to external ecosystem crates. If important crates experience supply chain attacks or contain significant vulnerabilities, this hurts trust in Rust. It also impacts the Rust Project itself, since it itself relies on ecosystem crates.

The Project is not responsible for the contents of ecosystem crates, but it is in its interest to make efforts to broadly improve the security posture of the ecosystem.

#### Responsibilities & Commitments

##### Decision Maker

- The design and roadmap of ecosystem-security tooling and information feeds it owns or maintains (e.g. scanning pipelines, detection heuristics, CI tooling).
- Its own internal division of labor between "response" (handling of malicious or vulnerable crates) and "proactive" (scanning, tooling, feeds).
- The approaches taken for handling of malicious crates, which may include author outreach or publishing advisors.
- The content and publication of security advisories for ecosystem crates (RustSec / advisory-db), once RustSec lives here (contingent on a separate proposal)
- The operations, particularly the operational security practices, of its response subgroup.

The team is **not** the decision maker on crates.io or cargo behavior. Actions taken on crates.io (e.g. removing a malicious crate, acting on ownership) are done only under explicit delegation by the crates.io team, with access granted and revocable by them.

##### Accountable for results

- Ecosystem-security tooling and feeds it commits to remaining maintained and useful to consumers.
- Ecosystem security incidents have an owner and any remediation is carried to completion.
    - This is a desired state, but initially, `security-response` is responsible for this outcome, until `ecosystem-security` has the process and capacity to handle it

##### Responsible for work

The team members do the work of:

- Providing content related to the ecosystem component of the Project threat model
- Building and running detection/scanning/mitigation systems to reduce the incidence of malware on crates.io.
- Producing consumer-facing security tooling/guidance (e.g. nicer GitHub Actions around cargo audit/deny/vet-style checks)
    - Note that formal Project adoption of external tools is a separate proposal.
- Monitoring and triaging the `security@rust-lang.org` inbox (including decrypting GPG-encrypted reports).
  - Flagging `security@rust-lang.org` reports related to Project incidents for `security-response` processes.
  - This is a desired state, but `security-response` will initially be responsible for this work, until `ecosystem-security` as the process, capacity, and trust model to handle it.


##### Consulted for input

The team should be consulted on:

- crates.io / cargo designs with ecosystem-security implications (e.g. trusted publishing, yanking/quarantine semantics, publish-time policy).
- Project decisions about adopting ecosystem security tooling.
- Project-internal incidents from security-response that have ecosystem spillover, for instance crates that are owned by `rust-lang`.

##### Informed for awareness

The team should be informed of:

- Relevant Foundation security engineering work

#### How does this team make decisions?

By team consensus. Broader Project decisions, for instance substantial contributions to crates.io, or significant decisions that benefit from explicit team consensus, follow the standard RFC process.

#### What work is not specifically part of this team's remit?

- Hardening of Rust Project repositories and infrastructure (Project repos/CI/publishing)
- Reactive response to vulnerabilities in Project-owned code
- Deciding crates.io / cargo policy or operating their systems outside delegated actions
- Formal Project adoption of external tooling (cargo audit/vet/deny, etc.) as governed by Rustlang

#### Team Processes

##### Contact Point

General discussion happens in the `#security-chat` Zulip channel. The team lead(s) are also available for direct contact.

Ecosystem-scoped security reports requiring a reactive response are routed to `security@rustlang.org`. This inbox is shared with `security-response`, with coordination across groups to ensure that the relevant team is driving actionable reports.

##### Team Membership

Members are expected to:

- Contribute recurring work to advisory operations, tooling/detection, or ecosystem response.
- When acting on crates.io, operate strictly within crates.io-delegated authority and access.
- Engage with any RFCs directed to `ecosystem-security` in a timely way

##### How is team membership managed?

Existing team members may nominate new members, who are approved by team consensus.

##### Where does this team work?

Primarily async, on Zulip, in the `#security-chat` Zulip channel. Subchannels will be created for directed discussions as needed.

Synchronous meetings are announced in `#security-chat`, with a preference for breakouts rather than full-group meetings.

The team will additional publish recommendations, tooling, and other durable records, to `rustlang/ecosystem-security` on GitHub. 

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Alternative: broaden `security-response` instead of creating a new team

Rather than a new umbrella team, the existing `security-response` working group could take on proactive and ecosystem security work as well. But, as we see an increase in the number and severity of security incidents, there is a risk of excessive strain on the security response team. As @Manishearth put it, "don't make the people driving security response manage all security work"; security-response shouldn't _become_ T-security.

This leaves the obvious question of why not just bring more people into `security-response`. At the end of the day, `security-response` didn't sign up for this, and incident response is a different way of working than proactive measures. People can already do cross-cutting things in `security`, so let's not mess up what's working by trying to refactor the existing team.

### Alternative: a single flat `security` team

We could have one big team rather than a parent team for coordination and subteams. But, we want to optimize for doing work, and grouping related efforts allows higher communication bandwidth among the people working on them. Not to mention, `security-response` already exists and is working well, so we don't want to change that unnecessarily.

### Why a private `security-advisors` pool rather than public discussion?

Security efforts involve some amount of cat-and-mouse tactics where public discussion of methods undermines efficacy. They also involve individual people, in some cases making mistakes around security, who are not appropriate to bring into the public view. The channel allows operators to pull people into private response threads quickly, without widening an embargo to a whole team. It also offers an opportunity to privately seek feedback and a thumbs up on whether security measures meet Project norms and published policies.

### Why `launching-pad` rather than a top-level team?

Adding a top-level team has broader implications around governance, including Council representation. We want to wait to see how the team operates in practice, and what level of participation we see, before raising that question. Notably, there might be further changes, such as incorporating RustSec, that meaningfully change the team's scope. We propose to revisit this question in 6 months.

### Why not include RustSec now?

RustSec / advisory-db lives outside the Rust Project, with its maintainers coordinating partially through wg-secure-code. We think it should live in `ecosystem-security` or a new subteam, but we first need to discuss moving RustSec inside the Project at all. That deserves its own RFC discussion rather than overloading this one.

### Why does `project-security` make recommendations rather than requirements?

Existing teams own these areas and are the subject matter experts on them. The Rust Project does not charter teams that reduce other teams' scope without explicit consensus on both sides. We're best off operating in an advisory way, and perhaps seeing over time what a team might prefer to hand off.

### Why not start off with crates.io delegating security work to ecosystem-security?

Finding the right delegation model takes care and consensus seeking, even if the crates.io team is not interested in being the frontline for security issues in the long term. The team therefore starts with no delegated authority on crates.io.

### Why does security-response initially keep primary responsibility for ecosystem incidents?

security-response is handling high-impact ecosystem incidents today, and we need to transition mindfully to avoid dropping the ball. security-response therefore remains responsible for them until ecosystem-security has the process and capacity to handle them.

### Why does the top-level team own the threat model?

Threat models work best when there is one owner. The top-level team is also responsible for creating new subteams, which will be informed by the threat model.

### Why an open top-level team alongside confidential subteams?

We care about having an accessible front door for people who want to get involved in Rust security, and a place for external sponsors and collaborators to plug in to. The top-level team shouldn't be discussing sensitive topics, so we optimize for openness and transparency until there is a reason to segment. Embargoed incident details, in-progress advisories, and any secrets remain confined to the relevant subteam's access-controlled channels and are never shared into the coordination team or `#security-chat`.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

The following are left open in the charters and are expected to be resolved by the initial members after the teams exist, rather than through this RFC:

- Lead / point person for `project-security`.
- How membership is managed for `project-security` and `ecosystem-security` (the latter in coordination with RustSec maintainers).
- Decision-making process for `ecosystem-security`, to be confirmed with initial members and with RustSec maintainers.
- RustSec: we think it should live in `ecosystem-security` or a new subteam, but whether to move RustSec inside the Project at all will be discussed in a separate RFC which @djc intends to drive following the chartering of `security` and `ecosystem-security`.
- wg-secure-code: almost certainly belongs under `ecosystem-security`, but we want to have further discussions before moving forward.
- Whether the existing `wg-security-response` repo and Zulip presence are renamed to `security-response`. No rename is proposed here unless current team members prefer it.

## Future possibilities
[future-possibilities]: #future-possibilities

- Making `security` a top-level team depending on level of engagement.
- An opt-in ecosystem-security oncall operating under crates.io-delegated access. This needs more design and is explicitly *not* part of the initial chartering request.
- The crates.io team may choose to delegate some operational authority to `ecosystem-security`. This requires more discussion and would need to be explicitly scoped and revokable.
- Formal Project adoption of external tooling (cargo audit/vet/deny, etc.). This team may raise proposals for adoption of tools to the broader Project.
