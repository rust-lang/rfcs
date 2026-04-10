- Feature Name: `rust_foundation_maintainer_fund`
- Start Date: 2026-02-23
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)

# Summary
[summary]: #summary

This RFC defines the relationship between the Rust Foundation Maintainer Fund (RFMF) and the open-source Rust project. The RFMF is a dedicated fund used to support Rust maintenance: open-ended, multiplicative work that improves Rust and its codebase and makes it more accessible.

The Leadership Council has a Project Priorities budget, which is used to fund various initiatives, such as travel grants or program management. RFMF funds will be directed to this budget, but they will be dedicated to activities that direct funding to individual maintainers. This includes the existing program management program and would include the proposed Project Grants Program ([RFC 3919]), which provides modest stipends to recognize and support existing contributors. It will also include a third program, the *Maintainer in Residence program*, proposed by this RFC.

The Maintainer in Residence program is dedicated to hiring long-term maintainers and funding their maintenance work in full. Maintainers' in Residence time is split between priorities guided by the teams they are supporting and priorities of their own choosing within the project.

Selecting Maintainers in Residence is a collaboration between the Foundation and a "Funding team" appointed by the Leadership Council. This Funding team will weigh the set of applications against the project's needs and priorities.

The Funding team is additionally charged with ensuring the program's overall success. When sponsors contribute undirected funding, they are investing in the Rust project as a whole — and the project should meet them in good faith. Project teams receiving support from the program are expected to help the Funding team manage sponsor relations, e.g., by meeting with sponsors or providing other reasonable sponsor benefits.

[RFC 3919]: https://github.com/rust-lang/rfcs/pull/3919

This RFC was jointly written by the [RFMF Design Committee](https://github.com/rust-lang/team/blob/0acc660a6bce2b9166362d4bfcbed872508085a6/teams/rfmf-design-committee.toml).

# Motivation
[motivation]: #motivation

The Rust Foundation is establishing a Maintainer Fund to collect sponsorships and provide long-term funding for Rust maintenance. Funds raised through the RFMF are dedicated to funding Rust maintainers under the supervision of the Leadership Council. The Leadership Council commissioned this RFC to recommend how those funds should be used.

We recommend that the funds be directed into the Project Priorities budget with the restriction that they be used only for programs that fund Rust maintainers to do maintenance work (which includes project management). Dedicating the funds helps ensure that the "sales pitch" is clear: donations given to the RFMF will go directly into a maintainer's pocket.

We further recommend that the Leadership Council create a "Maintainer in Residence" program to augment the existing project management program and the grants program proposed in [RFC 3919]. Maintainers in Residence are maintainers who are paid to work on a full-time or substantial part-time basis to maintain some part of the project.

## Long-term maintenance is the biggest gap we found

In preparing this recommendation, we interviewed team leads across the Project. The message was clear: *"what's needed is people with the focus to drive longer-scale projects."* Volunteer maintenance can keep the lights on, but larger-scale work stalls because nobody has the sustained focus to push it through. As one (volunteer) team lead said, *"All the time that reviewers have goes into reviewing, triaging, and so on, and then the interesting longer-term projects just fall under the table."*

The rust-analyzer and Clippy experiences show what funded presence makes possible, and what happens when it disappears:

* When a funded reviewer was working on r-a, the PR backlog stayed around 10. After that person changed jobs, it climbed to over 110: *"solving the review problem definitely requires money I think, there's no big question there."*
* Short-term grants help but aren't enough. The Clippy team received Foundation grants that let *"one person work almost full time on Clippy, but it was only for 6 months — it was hard to make long-term plans."*

The problems teams describe require sustained, long-term presence.

## Sponsors come in many shapes and sizes

We expect three kinds of support.

First, small-dollar donations from individuals and organizations that value Rust and want to support its long-term health without any particular expectations. The Foundation will help get the word out via funding drives and PR campaigns.

Second, when the Foundation takes in directed funding towards a particular goal, best practice will be to direct a percentage of that work into the RFMF, providing another revenue stream for maintenance work. (See the [Future Possibilities](#future-possibilities) section for more discussion in this direction.)

The third category is companies that employ developers or contributors working full-time to improve Rust. These companies are invested in Rust development, but their contributors' work still needs to be reviewed and landed by experienced maintainers. These companies may also need help resolving upstream bugs or limitations that are being hit internally.

## Larger sponsors want predictability; all sponsors need to show impact

Sponsoring the maintainer fund is a way for companies to ensure the maintenance layer their contributors depend on stays healthy. An alternative is for the company to hire internal staff to do that role, but beyond being more expensive, experience has shown that "in house" maintainer roles at companies are difficult to sustain. Maintenance activities don't advance any single company's goals, so they're hard to justify in a performance review and vulnerable to restructuring when priorities shift. 

For the fund to be sustainable, sponsors also need to be able to report the impact of their contributions. This means the project needs to treat demonstrating impact as a whole-project responsibility, not something that falls on Maintainers in Residence alone. See [the sponsor benefits section](#sponsor-benefits) for concrete details on what we envision.

## The Project has visibility into needs that aren't always apparent from the outside

The fund is structured so that, by default, the Rust project, rather than the sponsors themselves, selects which maintainers to hire and which areas they should focus on. This allows us to aggregate smaller donations and put them to good use. It also means that less visible areas of the project, such as moderation or infrastructure, will be easier to support, as project members are aware of those needs.

## But some sponsors will want to fund maintenance in particular areas, and that's ok too

Although the default is for the project to pick the area of focus for a MiR, we do allow the Funding team the latitude to offer involvement in area selection as a sponsor benefit at higher tiers. The intention is to permit a company that has a strong need on a particular area to fund a maintainer in that area, if they are willing.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The RFMF collects sponsorships from companies and individuals. Funds support project grants, project management, and the Maintainer in Residence program. Maintainers in Residence are experienced, self-directed maintainers who do the work that keeps Rust healthy. They participate in team discussions, review PRs, mentor newcomers, and work on what the team needs.

## Design axioms

### Not one size fits all

Maintainers have a wide variety of needs and no one program will work for everyone. We allow RFMF funds to be used for any kind of program that directly pays maintainers for maintenance work (e.g., project grants from [RFC 3919], the MiR program defined in this RFC, the LC's project management program, or other future programs that may be added).

### The MiR is a collaboration between the Project and the Foundation

Neither the Foundation nor the Project can operate the MiR program on their own. The Foundation has a bank account, legal entity, and operational capacity; the Project has knowledge of team health and needs. The Foundation is the incoming channel by which most sponsors arrive; the Project governs the codebase that sponsors want to support. This RFC proposes that both project members (the Funding team) and Foundation staff jointly make major decisions. This is a partnership, not a handoff.

### The Funding team owns the RFMF program's success, but they can't do it alone

Together with Foundation staff, the Funding team owns sponsor relations and the success of the RFMF program. As the team that selects maintainers and understands project needs, they're best positioned to communicate with sponsors about outcomes and priorities. However, they need support from the broader project, particularly those areas benefiting from a MiR.

### Maintainers are team members

Maintainer in Residence candidates must be established members of the Rust Project who either are already members of the relevant team(s) or have been approved by the team(s) to become a member upon starting their MiR role. They need the permissions required for the work — reviewing PRs, championing goals, and performing actions limited to official team members. This is a hard requirement, not just an expectation. 

For candidates who are not yet members of the target team, the team must confirm they are willing to add the candidate as a member. In cases where a team is defunct, the parent team(s) can invite the candidate to join and help revive the team.

Funded maintainers are not a separate class of contributor — they're project members who can now commit sustained time to team responsibilities.

## Sponsor benefits
[sponsor-benefits]: #sponsor-benefits

RFMF sponsors typically contribute to a general fund and don't direct where the money goes or who gets hired. Every contribution helps fund the sustained maintenance that keeps Rust healthy. All sponsors receive public recognition and visibility into how funds are being used through regular public reports.

To encourage larger contributions or year-over-year commitment, the Funding team can also establish sponsorship tiers where sponsors receive particular benefits.

### Possible benefits associated with higher tiers

This RFC does not specify the precise tiers or benefits associated with those tiers. Instead, we give examples of the *kinds* of benefits we anticipate. The Funding team is free to choose these or other benefits that are similar in kind. A good rule of thumb is *"could the company simply hire a person to do this, presuming they could find someone with the requisite team membership?"* -- if so, it's a reasonable benefit to offer.

* **Sponsor meetings.** The Foundation builds a community of sponsoring organizations that meets with project leadership (Leadership Council, team leads) and Maintainers in Residence a few times a year to discuss project direction, sponsor experiences, and pain points. Project leadership gains insight into the needs of major Rust users; sponsors get visibility into the roadmap and the opportunity to hear from other Rust-adopting companies. The frequency of such meetings may depend on the level of sponsorship.
* **Impact reporting.** Regular reports on what funded maintainers are working on, progress on Project Goals, and how the program is contributing to Rust's health. These reports are prepared with help from the program management team and made publicly available.
* **Prioritized review and bug fixes.** Sponsors can reach out to the Foundation or project contacts about PRs or bugs that need attention, up to a certain frequency that is dependent on funding tier. This provides the sponsor with a form of "insurance" that they will get help resolving priority issues they encounter with Rust; however, this prioritization should be limited to bug fixes or PR reviews, not to larger feature *development*, and is in no way a promise that a PR will be *merged* (simply reviewed).
* **Prioritization for goal championing.** Sponsors may suggest that teams use a MiR on that team as a champion for project goals important to them. Teams and their members are encouraged to consider these suggestions but they are not obligated to take them.
* **Area preferences.** If a sponsor or group of sponsors is willing to fund the entire cost of a MiR but only in a specific area, the Funding team could work with them to find a candidate for that particular area.
    * For example, if a sponsor would specifically like to fund a cargo or rustfmt maintainer, the Funding team could work with them to make that happen. The role would still be a MiR like any other, following the same processes.

### What is not allowed as a benefit

*What sponsors do not get:* the ability to unilaterally direct a maintainer's work, pick who gets added to project teams, or otherwise bypass project processes.

## Selection process is driven by a team within the project, supported by Foundation staff

When funding is available, the Funding team and Foundation put out an open call for applications. Where appropriate, the Funding team may also proactively reach out to potential applicants to encourage them to apply, if they may be a good fit for areas the project needs. The Funding team and Foundation staff review applications, consider the project's needs, and then the Foundation makes offers to the strongest candidates.

## The Funding team owns the project's long-term success

The Funding team owns the program's overall success. They keep up with teams to understand where support is needed and how well the program is working; they can adjust aspects of the program to make it work better over time.

The Leadership Council as well as the teams benefitting from the work of Maintainers in Residence are expected to support the Funding teams' efforts (e.g., by meeting with sponsors upon request or otherwise helping out with the sponsor benefits described above).

## What Maintainers in Residence do

Maintainers in Residence split their time between team priorities and individual priorities of their own choosing within their area of focus. The exact balance varies depending on the individual, their experience, and the needs of the team — the important thing is that both team-directed and self-directed work are expected. This is about "team-directed vs. self-directed," not "maintenance vs. features." The PSF's experience after nearly five years confirms that focusing purely on team-directed needs and multiplicative maintenance can be very draining; giving time for self-directed projects "made all the difference" in satisfaction (see [Prior Art][prior-art]).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## The Funding team's day-to-day responsibilities

The Funding team's role is to keep a pulse on the project and work with the Foundation to select which maintainers to hire. Its core responsibilities are:

1. **Staying in regular contact with teams** — meeting regularly with team leads and members to understand their needs, health, and where support would have the most impact.
    * These meetings can be helpful for purposes other than hiring a MiR: the Funding team may be able to connect the team with Foundation resources to resolve a situation.
2. **Working with the Foundation to select MiR candidates** — when positions become available, evaluating applicants and selecting candidates who'll have the most overall impact based on project needs.
3. **Collecting feedback on how well the program and the MiRs are working** — as the team responsible for selecting which maintainers to hire, the Funding team is also the team responsible for fielding feedback on how well those decisions work out and making adjustments as needed.

The Foundation supports the Funding team with logistics. The Foundation issues contracts or manages employment.

*Unresolved question:* The Funding team may also take on the duties of the Grants team proposed in [RFC 3919]. See the [Unresolved Questions](#unresolved-questions) section.

## The Funding team owns the health of the RFMF program

The Funding team has ownership of the RFMF's long-term health. They need to be responsive to both project needs as well as demonstrating return on investment to sponsors.

## Application and vetting process

The process of hiring a new Maintainer in Residence begins with an open call for applications. Putting out a broad call for applications helps to surface needs and candidates the Funding team might not have identified on its own.

Applicants do not have to be a member of a Rust team. However, to be accepted, the team must be willing to grant the person membership. Therefore, applicants should be people who are experienced contributors to the project and, ideally, either a member of the teams they expect to help as MiR or members of some other Rust team.

Applicants provide (1) their background and experience — both within the Rust project and professionally; (2) their availability (full-time, part-time, etc); and (3) a high-level description of the kind of work they would like to do. This description can be quite general (e.g., "maintain rustfmt") but could also be specific (e.g., "split project `foo` into multiple independent libraries `bar` and `baz`").

The Funding team prioritizes applications based on:

1. conversations with team leads and team members to assess what support is most urgently needed;
2. the applicant's history with the project;
3. any specific work that was proposed in the application;
4. the applicant's availability and whether it suffices for the tasks they expect to perform;
5. the results of interviews or conversations with the applicant; and
6. any information the moderation team chooses to share regarding the applicant's conduct.

The Funding team works with the Foundation to select from the applicant pool and to extend offers. The Funding team is looking for maintainers that have technical depth in the relevant area, community standing and trust within the Project, and sustained work orientation (a track record that suggests they'll thrive in a role focused on reviews, mentorship, unblocking, and the kind of long-term technical work that requires deep context).

## Working arrangements should be substantial and stable

The precise terms of the working arrangement are not defined by this RFC. Whether achieved through contracts, employment, or other means, the goal is to create a stable environment that allows maintainers to focus on their work in a sustained, year-over-year fashion. The important points are that the arrangement is substantial and stable:

* By *substantial*, we mean that the compensated time is either full-time or significant part-time. Some areas may not require a full person's time; it is fine to have one person cover two areas, or two people each contribute part-time to a single area, so long as there is enough concentrated time to build and maintain deep context. This latitude also allows us to accommodate maintainers who are not interested in a full-time role.
* By *stable*, we mean that it is expected to continue (or be renewed) as long as both sides are satisfied and funding is available.

## We recommend a flat pay structure to start

We recommend starting with a single flat rate or small fixed set of flat rates (e.g. 2-3) rather than individually negotiated compensation. Flat rates keep the program simple, avoids the perception that some maintainers are valued more than others for equivalent work, and removes the disadvantage that individual negotiation creates for people who tend to undersell themselves. The rates should be publicly advertised as part of any call for applications.

That said, the Funding team and Foundation may adjust the compensation approach over time as the program learns what works — including moving to multiple bands, cost-of-living adjustments, or individually negotiated rates if that proves necessary. The Funding team and Foundation determine the specific rate(s).

## Publish compensation rate for MiRs

The compensation rate should be published as part of the open call for applications so that prospective applicants and the broader community can see what the program offers. The identities of Maintainers in Residence and the areas they support are also public. Individual totals — hours worked, total compensation received — are not published, though aggregate program spending may be included in impact reports.

## Expectations placed on Maintainers in Residence

Maintainers in Residence are expected to spend 100% of their funded time working to improve the Rust project. That funded time can be split between:

* **Team priorities** — items that are prioritized by the team(s) that the maintainer was specifically hired to contribute to. This includes reviews, mentoring, bug-fixing, triage, and larger development work like refactors or subsystem rewrites.
* **Self-selected items** — work of the individual's choosing.

Experience with the Python Developer-in-Residence program suggests that for long-term satisfaction, it's important that maintainers be given time to pursue self-selected projects. We've also seen over time that maintainers often develop good instincts for what would generally benefit Rust, and thus self-selected "passion projects" can turn into some of Rust's most impactful features.

The split between team priorities and self-selected work will depend on the individual. The Funding team should monitor and make sure that team priorities are being adequately served, while also ensuring that MiRs have the opportunity to pursue self-selected work. If both cannot be done, that likely indicates the need for another MiR in that area.

Maintainers in Residence are also expected to:

* respond to reasonable requests from the Funding team on behalf of sponsors;
* keep records of their activity as needed by the Funding team (see below);
* resolve time-critical issues such as urgent bugs;
* champion Project Goals supported by their respective teams, even if they themselves might not have championed that goal as an individual;
* work with the Project to ensure their work gets regularly reported on;
* remain a member of the Project and relevant teams, in good standing.

## Reporting and impact visibility

The Funding team may request that MiR collect data on their activity so that they can prepare impact reports and other material for sponsors. Whenever possible, however, the Funding team should work with other teams (e.g., the Goals or Content teams) to handle the creation of that material, so that the MiR can focus primarily on maintenance.

The expectation is that a typical MiR could satisfy these requirements by registering project goals for their major initiatives, posting regular updates, and periodic meetings with their manager. Other teams would supplement this work. For example, the Content team may opt to interview a MiR and prepare a blog post covering their work, and the Funding team might gather Github activity automatically to quantify things like PR reviews or issue triage.

## The Funding team collects feedback (positive or negative) on MiR performance

Feedback on MiR performance (positive or negative) can be directed privately to the Funding team. The Funding team will also periodically seek feedback from the project proactively. The Funding team and the manager at the Foundation will convey feedback to the MiR at regular intervals. Feedback, particularly negative feeback, is private and should not be shared publicly without permission from the MiR.

When negative feedback is received, the Funding team will gather context and work with the MiR manager (see next section) to resolve the concern. Typically this means a conversation that brings about a change in behavior. In extreme cases, this might include performance management options like terminating the arrangement or opting not to renew.

## The Manager works with the Funding team to communicate feedback

The *manager* for a MiR is charged with making sure they are well taken care of. They should meet regularly with the MiR, monitor their workload to ensure that it is balanced and they are not being given either too many or too few responsibilities. They should also have performance conversations at regular intervals to give the MiR a sense for whether they are doing well. Finally, if negative feedback has been received, they should have a clear conversation with MiR to set expectations.

If the program grows to large number of MiRs, however, we recommend that the Leadership Council use some portion of the RFMF funds to hire a dedicated manager who would work closely with the Funding team (see [Who manages Maintainers in Residence after they're hired?][who-manages-mirs]).

## Moderation concerns about a MiR

The Moderation team is encouraged to inform the Funding team and/or Foundation about any reported issues relevant to a MiR's conduct.

Code of Conduct violations that result in removal from the Project will make a Maintainer in Residence no longer eligible to continue in their role. Team membership is a hard requirement for the role; a maintainer who is no longer a project member cannot continue as a Maintainer in Residence. From the project's perspective, the role ends immediately; the legal working arrangement however will end later, as described in the next section.

## Terminating a working arrangement

As would be typical in any employment or contracting relationship, a working arrangement might end if:

* there is not adequate funding available to continue the position;
* the maintainer is not performing adequately or has been removed from their role due to Code of Conduct violations; or
* there is an urgent need in another area and funds must be redirected.

Redirecting a well-performing maintainer's position to a different area should not be done lightly. Sustained presence is the core value proposition of the program — startup overhead is real, context takes time to build, and the problems this program targets require continuity. Before redirecting a position, the Funding team should consider whether the existing MiR could help in the new area as well, if the relevant team agrees. The Funding team should strive to grow the program to meet new needs rather than redirecting existing positions.

## Termination period is required

Whatever their structure or the legal limits, working arrangements must include a termination period or severance payment to ease the transition. This is needed to ensure that the moderation team can make decisions without having to account for the impact of someone suddenly losing their funding in a preciptious fashion.

## Team requests on a MiR

Teams should feel free to ask a MiR to take on high-priority work that nobody else can tackle (championing a goal, driving a critical refactor, clearing a review backlog).

However, teams should also bear in mind that MiR do not have infinite capacity. MiR can feel free to decline to work on team priorities past a certain point. The MiR manager and Funding team will help to negotiate this balance as needed.

# Frequently asked questions
[faq]: #faq

## What's the relationship between the Rust Project and the Rust Foundation?

The Rust programming language is developed by the Rust Project, a community of volunteers organized into teams and governed by the Leadership Council. The Rust Foundation is a separate nonprofit organization that supports the Project — it holds funds, employs staff, and handles legal and operational matters, but does not govern the language or its development. This RFC defines how these two entities collaborate to fund maintenance.

## What does it look like to have a Maintainer in Residence on my team?

In one sense, the same as having any other team member. They show up in reviews, participate in design discussions, mentor newcomers, and work on what the team needs. The PSF's experience after nearly five years is that the Developer in Residence does roughly 50/50 maintenance and proactive work, and the role feels like "just another team member" rather than an outside presence (see [Prior Art][prior-art]).

Unlike a volunteer, however, MiR are expected to put some portion of their time towards team priorities. Whereas before, if there was an urgent task, a team had to hope that somebody had the capacity to pick it up, they can now direct sustained effort where it's needed most. This is a new capability.

## Why not a flexible contractor pool instead of long-term maintainers?

Context and trust take time to build. The maintenance problems we heard about in team lead interviews (review backlogs, cross-team blocking, complex refactors that need months of sustained attention) require sustained presence, not project-scoped interventions. Contractors for scoped work is a valid model, but it's a different program solving a different problem.

## Who manages Maintainers in Residence after they're hired?
[who-manages-mirs]: #who-manages-maintainers-in-residence-after-theyre-hired

The Funding team's role ends with the recommendation. Someone needs to synthesize feedback from the Project and make the call on performance. There are two main options:

**Foundation manages (current RFC position).** Management and performance feedback are skills, and the Foundation has professional staff experienced in them. This shouldn't be a heavy lift early on — we're selecting experienced, self-directed maintainers who are unlikely to have significant performance issues. The Foundation gathers input from the Project (team leads, collaborators) and synthesizes it.

**Project manages.** The Project has deeper technical context to evaluate whether work is landing well. But Project-side management means either a volunteer committee handles it (likely unskilled at management, outside the Funding team's competency) or a dedicated person is hired for it (high overhead relative to the program size, especially early on).

In practice, we expect this to be a phased approach. While the program has relatively few maintainers, the Foundation provides management skill and the Funding team provides feedback as an input. As the program grows, a dedicated support role may emerge — someone who meets with MiRs regularly, helps them build a portfolio of their work, and serves as the point of contact when teams have concerns. Whether that role lives within the Foundation, the Project, or somewhere in between is a question that becomes more important at scale and can be revisited as the program learns what it needs.

## What if a Maintainer in Residence underperforms?

See the ["Who manages MiR"][who-manages-mirs] question.

## What about people who only want to work part time?

Maintainers in Residence can work substantial part-time — the key requirement is enough concentrated time to build and maintain deep context, not necessarily a 40-hour week. Some areas may not need a full person's time, and it's fine to have one person cover two areas or two people each contribute part-time to a single area. For contributors who want lighter-touch support, the LC's Project Grants Program ([RFC 3919]) is designed for exactly that. The two programs are complementary: grants support a broad base of contributors; the RFMF funds sustained maintenance work from people with deep context.

## What about sponsors who want to pay for a particular item to get done?

That's outside the scope of the RFMF, which is undirected funding — sponsors contribute to a general fund and the Leadership Council decides how to deploy it. A companion effort, the proposed Project Goals Funding program (see [Future possibilities][#future-possibilities]), is designed for exactly this: sponsors direct funding at specific Project Goals, and the Foundation issues grants to advance that work. Sponsors seeking that level of direction should look there rather than the RFMF.

## Why are RFMF funds dedicated to maintainers?

Sponsors contributing to the RFMF want to know their money is going directly to fund maintainers. Dedicating the funds gives the fund a clear value proposition: every dollar goes to paying people who maintain Rust. Without this restriction, the Leadership Council could use RFMF funds for any purpose — travel grants, event sponsorship, infrastructure — all valuable, but harder to explain to a company evaluating its return on investment. Since money is fungible, dedicating RFMF funds to maintainers frees up budget for other purposes.

The restriction is broad: Maintainers in Residence, project grants, and the program management overhead needed to support them. The Leadership Council determines the specific form. This gives the Council real flexibility while keeping the fund's purpose legible to sponsors.

## What does this RFC deliberately not specify?

This RFC does not define how the RFMF collects sponsorships, the precise tiers or benefits sponsors receive, the precise pay structure, or other fine-grained details. The Funding team and Foundation can work these out.

## What is an example of something that RFMF funds could *not* be used for?

This RFC proposes that RFMF funds are limited to funds that compensate Rust maintainers for time they spend doing maintenance. They could not be used for other Project Priorities budget items such as organizing the Rust All Hands or running a travel grant program.

# Prior art
[prior-art]: #prior-art

## Python Software Foundation: lessons from a mature program

The [PSF Developer in Residence program](https://www.python.org/psf/developersinresidence/) started in 2021. After nearly five years, it now funds three maintainers, each sponsored by a specific company (Meta, Bloomberg, and Vercel). Contracts are for 12 months, renewable based on continued sponsor funding. Maintainers are employees or contractors of the PSF, reporting to both the Executive Director and the Steering Council.

One important lesson that we took from our discussions with the Python Developers in Residence and the Steering Council is the importance of self-directed time. The first Developer in Residence started doing only reviews, backports, and CI, and found that after a year or two, "there's not much joy in that." Allowing feature work alongside maintenance "made all the difference." We've reflected this in the RFMF design: Maintainers in Residence split their time roughly 50/50 between team priorities and individual priorities of their choosing, rather than being assigned purely to maintenance.

One area where we have deviated from Python precedent is in attempting to create a system where most sponsors pool their money into a general fund, rather than funding specific individuals (similar to Zig or the Scala Center, see below). This may be a challenge, as the PSF has found that being able to clearly identify the impact of a sponsors' funding is useful when making the case for renewal. For this reason, we also allow for larger sponsors who wish to sponsor an entire person, rather than putting money into a general pool.

## Django Fellowship: weekly reports and community-focused maintenance

The [Django Fellowship program](https://github.com/django/dsf-working-groups/blob/main/active/fellowship.md) has been running since 2014, predating the PSF program and providing the longest track record of any comparable effort. Fellows are contractors (not employees) who post weekly reports. The work is focused on "housekeeping and community support": monitoring security email, fixing release blockers, reviewing pull requests, and mentoring new contributors.

## Zig Software Foundation: lean and independent

The [ZSF](https://ziglang.org/zsf/) takes a simpler approach: core team members bill hours directly to the foundation. As a 501(c)(3) non-profit founded in 2020, 92% of its spending goes directly to paying contributors, with minimal administrative overhead. It has no big tech companies on its board, an explicit design choice to maintain independence.

The ZSF model is leaner and more informal than PSF or Django, but also smaller in scale and more dependent on individual large donations. It demonstrates that low-overhead models are possible, but the approach may not scale to the number of maintainers Rust needs.

## Scala Center: pool-funded with sponsor engagement

The [Scala Center](https://scala.epfl.ch/), housed at EPFL, takes a pool-funded approach that contrasts with the PSF's per-sponsor-per-position model. Corporate sponsors contribute to a general fund at tiered levels and send representatives to quarterly Advisory Board meetings. The Advisory Board makes non-binding recommendations on priorities; the Center's leadership decides on execution and hiring. Sponsors influence direction through discussion but don't direct specific positions or hires.

Two aspects of the Scala Center model have been particularly influential on this RFC. First, the sponsor meeting structure: sponsors meet regularly with maintainers and with each other, and these meetings have been described as a "big win" for selling the program internally. Having sponsor representatives commit to attend regular meetings makes the program legible to upper management. Second, sponsors often value hearing from their peers — other organizations using the language — as much as from the project itself. We encourage the Funding team to recreate this by creating a community of mid-level sponsors.

## Project Grants Program: a related committee model

The Project Grants Program ([RFC 3919]) proposes a program supporting a handful of contributors with modest monthly stipends. It charters a Grants team (5 members, LC-appointed, organized as a Launching Pad subteam) to select recipients and oversee the program. The RFC explicitly positions itself as "distinct from, but complementary to" the MiR: grants are smaller-scale, flexible, Project-controlled support, while the RFMF targets larger-scale, sustained maintenance.

The Grants team's charter overlaps significantly with the Funding team charter we define here — both involve assessing project needs and selecting candidates. The Leadership Council may choose to extend the Grants team with the Funding team's responsibilities rather than creating a separate body, which would avoid fragmenting the Project's attention across multiple committees with overlapping mandates.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Organizational form of the Funding team
[funding-team-org]: #organizational-form-of-the-funding-team

This RFC defines the Funding team's charter — its responsibilities — but leaves the organizational structure to the Leadership Council. There are two main options:

**Merge the Grants team and the Funding team.** The Grants team proposed in [RFC 3919] already has an LC appointment process, selection infrastructure, and conflict-of-interest policies. We could merge these two teams together. This would mean that serving on the team is significantly more work, but the process of selecting Grant recipients and MiR also share significant overlap.

**Create a new team.** We could also have two distinct teams, where each team has a narrower charter. This might make it easier to staff the two teams since the workload for any individual team is less.

Comparing the two teams:

* The Funding team's charter is broader than selecting recipients — it includes staying in regular contact with teams, connecting them to resources, and understanding project health holistically. MiR selection is only one part of that mandate. 
* There is also a difference in operating cadence: the Grants program has predictable cycles, while the Funding team may need to react promptly when new funding becomes available rather than waiting for the next scheduled round. Current Grants team members may not have signed up for that kind of bandwidth or latency.

# Future possibilities
[future-possibilities]: #future-possibilities

## Extending the vetting service to other funding organizations

This RFC defines the interface between the Rust Project and the RFMF specifically, but nothing about the process is inherently RFMF-specific. The Funding team's core service — assessing where dedicated maintainers would have the most impact, evaluating candidates, and collecting performance feedback — could be offered to other funding organizations that want to hire Maintainers in Residence. The value proposition is the same regardless of who's paying: the Project has visibility into which teams are struggling and which candidates have the trust and context to be effective, and funding organizations benefit from that assessment rather than making hiring decisions without it.

## Recording MiR affiliations

If the Rust Project establishes a mechanism for recording affiliations of team members, Maintainers in Residence could record their RFMF funding as an affiliation. This would make funding relationships visible through the same infrastructure used for employer affiliations.

## Project Goals Funding program

The RFMF provides undirected funding — sponsors contribute to a general fund dedicated to funding Rust maintainers, with the Leadership Council deciding the specific form. There are ongoing plans to propose a Project Goals Funding program that would allow sponsors to direct funding at specific Project Goals. Sponsors would choose goals, roadmaps, or application areas to fund, and the Foundation would issue grants to contributors working on that work. A percentage of directed funding would flow to the Leadership Council's Project Priorities budget, where it can be used to fund maintenance, project management, or other activities. Together, the two programs cover the full spectrum of sponsor needs: undirected funding for those who want to support Rust's overall health, and directed funding for those who want to accelerate specific work.
