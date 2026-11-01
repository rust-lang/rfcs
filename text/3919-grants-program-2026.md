- Feature Name: N/A
- Start Date: 2026-01-31
- RFC PR: [rust-lang/rfcs#3919](https://github.com/rust-lang/rfcs/pull/3919)
- Rust Issue: N/A

# Project Grants Program 2026

This RFC charters the Grants team and establishes a grants program for 2026, funded from the council's Project Priorities budget, to support Rust Project contributors.

## Summary

This RFC allocates $100,800 from the council's budget to support six contributors at $1,400 per month for twelve months.  A Grants team of five people, appointed by the council, selects recipients and oversees the program.  The program expects light reporting from grantees, coordinated through our program managers, to ensure we can demonstrate success and continue to improve.

This program is distinct from, but complementary to, the Rust Foundation Maintainers Fund (RFMF).  Where the RFMF focuses on sustainable, corporate-funded support for maintainers through the Foundation, this program gives the Project the ability to directly support contributors whose situations or contributions don't fit that model.

## Motivation: Philosophy and goals

### What we're trying to do

The grants program has a simple purpose: to recognize people who are already doing valuable work for the Rust Project and to make it easier for them to continue.

This is not a salary.  We're not trying to employ anyone.  Nor is this purely a retroactive "thank you" for past work, though gratitude is certainly part of it.  The grants acknowledge that someone has been contributing meaningfully to Rust and provide modest support so that person can keep doing so.

Think of it as: "Thank you for what you've done.  We'd like to help you keep going."

We've heard from people on academic tracks who have turned down job offers to keep working on Rust.  We've heard from students and from contributors in countries where a modest stipend makes a real difference.  We've heard from retirees who want to justify the time they spend on Rust.  A little money, directed to the right people, can help them — and us — a lot.

### What we're not trying to do

This program cannot solve the broader problem of sustainable funding for open source maintainers — not with $100,000.  We hope to learn from this program and grow it over time, and we hope the RFMF will succeed in creating more substantial funding pathways.  But for now, we're starting with what we have and what we can responsibly manage.

This program is not meant to:

- Replace salaries or provide full-time employment.
- Fund specific feature work (that's better suited to contracts or Project Goals).
- Compete with or duplicate the RFMF.
- Address the need for larger-scale contracting (that's RFMF's domain).
- Create a hierarchy where grant recipients are seen as more esteemed than other contributors.

There's a real need for larger-scale contracting that small grants can't effectively address.  If we were to try to press small grants into service for that purpose, they would do it poorly.  This program is modest because we can only make a small investment right now.  Larger-scale funding for specific work is better suited to the RFMF or to contracts.  If we try to do everything with $100,000 in small grants, we'll do nothing well.

The point about not creating a hierarchy deserves emphasis.  Many contributors cannot accept money for various reasons — employment policies, tax situations, personal preference, or simply not needing it.  The grants are not a recognition of superior contribution.  The grants are support for people who can benefit from it and who we believe will continue contributing.  The value of a contributor's work is not measured by whether that contributor receives a grant.

### The model we're advancing

After listening to many people across the Project and holding preliminary discussions to elicit the values of council members, we've converged on a model that sits between two extremes:

- **Purely retrospective grants** would reward past work without any expectation of future contribution.  If someone gets the money and disappears, that's fine — that person already did good work.
- **Contract-style funding** would specify deliverables and expect regular progress on defined objectives.

Our model is neither.  We select people based on demonstrated track records — that's the retrospective element.  But we provide support with the expectation of continued contribution — that's the forward-looking element.  We don't require specific deliverables, and we don't treat this as purchasing time.  We're investing in people who have demonstrated value and making it easier for them to keep doing what they're doing.

This model respects the point that if someone has already done good work, supporting that person is worthwhile even without guarantees.  It also respects the observation that we're not just saying "thank you" — we're acknowledging people who are actively working and hoping that work will continue.  Both perspectives inform the design.

We select people based on what they've already demonstrated.  We expect they'll continue doing valuable work, but we don't specify what that work must be.  We ask for light reporting, but that's to help us learn and communicate successes, not to hold people accountable for specific deliverables.

If someone receives a grant and circumstances change such that the grantee can't continue contributing, we have a conversation.  We don't demand refunds.  We don't punish people for life changes.  But we also don't ignore the situation — we talk about it and figure out together whether the grant should continue.

This middle position has tradeoffs.  It's not as clean as pure retrospective "thank you" (which would require no follow-up at all) or as accountable as contracts (which would specify deliverables and milestones).  But we believe it's the right balance for what we're trying to do: support people we believe in without turning this into a job.

### Why the council runs this program

Running this program from the council's budget rather than through a separate vehicle is a deliberate choice.  With the council directly responsible and accountable for this spend, we have every incentive to learn from our experiences and improve the program over time.

The previous grants program struggled not because the grants themselves were bad — by many accounts, they provided real value to recipients and to the Project.  What wasn't working was that we weren't getting timely feedback to the Project members managing the program.  Without closing that loop, there was no opportunity to course correct, no way to see if teams could help struggling recipients succeed, and no way to learn from earlier choices and make better ones.

This time, we have human infrastructure we didn't have before: program management.  With program managers actively supporting the program, we can get the feedback we need without burdening grantees with heavy reporting requirements.  The program managers can help collect updates, flag concerns early, and ensure that what we learn feeds back into how we run the program.

This is also why some degree of reporting is necessary — not because we want paperwork, but because without feedback, we can't learn and improve.  And without evidence that the program is working, we can't maintain the perception that it's a good use of resources.  The council needs to show that it's investing prudently.  Doing so helps us attract more funding, which we can use to support programs like this one.  Light reporting, supported by program management, closes the loop.

## Program structure

### Budget allocation

This RFC allocates **$100,800** for this program in 2026.  This leaves approximately $28,000 in reserve council funds for other potential needs.

Starting conservatively makes sense.  The previous grants program had communication issues, and we want to restart carefully.  Better to support fewer people well, learn from the experience, and scale up than to overcommit and struggle to manage the program effectively.

### Grant amounts

Each grant provides **$1,400 per month**, paid quarterly ($4,200 per quarter) for a **twelve-month term** ($16,800 per year).

At this rate, $100,800 funds six grants for the full year.

Why $1,400?  This is similar to the $1,500 rate used for the October 2024 fellowships.  It's designed to be enough that someone can justify spending meaningfully more time contributing to Rust — for example, by allowing a person to avoid taking on a second job or by making it easier to turn down extra freelance work.  We're not trying to replace a salary.  We're trying to remove a barrier.  We acknowledge that the purchasing power of this amount varies considerably depending on where someone lives, and that's acceptable.

### Eligibility

Grant recipients are typically:

- Active contributors to the Rust Project with a demonstrated track record.
- People for whom the grant would make a meaningful difference in the ability to continue contributing.
- People who are not already receiving substantial funding for Rust work through other means (e.g., full-time employment focused on Rust, RFMF support).

We intentionally keep eligibility criteria flexible.  The Grants team considers each situation individually.  Someone might be eligible even while having a full-time job, if that job doesn't involve Rust and the grant would help maintain volunteer contributions.  Someone might not be the right fit even if technically eligible, if the situation suggests other support mechanisms would be more appropriate.

#### Hard requirements

Some requirements are not flexible:

- **Able to receive payment**: Recipients must be able to receive payments from the Rust Foundation, which is a U.S.-based 501(c)(6) nonprofit.  This generally excludes individuals in countries subject to U.S. sanctions (e.g., Russia, Crimea, Cuba, Iran, North Korea, Syria).  We acknowledge this constraint doesn't always produce ideal outcomes and have compassion for contributors who are affected by circumstances beyond their control, but this is a legal requirement we cannot waive.

- **Age**: Recipients must be at least 18 years old, in compliance with applicable labor laws.

- **Language**: Recipients need a reasonable working proficiency in English.  This isn't about native fluency — many excellent contributors work in English as a second or third language — but recipients will need to interact with teams, respond to program managers, and provide written updates.

#### Tax responsibility

Grant recipients are responsible for any tax obligations arising from the grants in their respective jurisdictions.  Recipients should understand that this income may be taxable and should consult with tax professionals if unsure.  This has been a source of concern in past programs, and we want to be clear about it upfront.

#### Legal agreement

The Foundation may require grant recipients to sign a legal agreement covering terms such as use of funds, intellectual property, reporting expectations, code of conduct adherence, and other standard provisions.  The details will be determined in coordination with the Foundation, but applicants should expect some form of written agreement.

### Project membership

We don't formally restrict grants to Project members, but given the selection criteria — particularly the emphasis on contribution history — we expect that in practice the recipients will be Project members.

There is one scenario worth mentioning: sometimes we're slow to add someone to a team even when the person has been doing great work for quite some time.  If someone is clearly in the process of becoming a Project member, or it's obvious that the person will be soon, the Grants team should have the flexibility to consider the whole situation rather than being blocked by a technicality.

Given the budget constraints for this first round, this probably won't come up.  But we don't want to create an artificial barrier that prevents the Grants team from making a sensible decision in edge cases.

### Grant terms

- **Duration**: Twelve months, with the possibility of renewal.
- **Payment**: Quarterly, via the Foundation's existing payment infrastructure.
- **Renewal**: Not guaranteed.  Grantees reapply each year.  But existing grants that have been effective have a natural advantage — a demonstrated track record reduces uncertainty, which matters when making selection decisions.

We'll discuss what happens when circumstances change in more detail below.

### Why quarterly payments?

Quarterly payments minimize payment overhead for the Foundation, align with our expected quarterly updates, and keep everything synchronized with the Grants team's review cadence.

When we describe grant amounts as "$1,400 per month," we mean this as a rate — $4,200 paid each quarter, not twelve separate monthly payments.

### Why twelve months?

A six-month term would also be reasonable and has its merits: it would force regular reviews of each grant and allow grants that aren't working out to be rotated more quickly.

We chose twelve months for several reasons:

- **Certainty for recipients**: A longer term gives grantees more stability and reduces anxiety about renewal.
- **Lower team overhead**: Fewer renewal decisions means less work for the Grants team.
- **Expected stability**: For this first round, we expect to choose contributors with established track records who are likely to remain engaged throughout the year.

Future iterations of the program might revisit this choice, especially if we find that shorter terms would better serve the program's goals.

### Why not shorter terms to support more people?

One could argue for shorter grant terms — say, six months with explicit non-renewal as the default — to spread the same money across more people over time.  With a fixed budget, this would let us support more contributors sequentially rather than fewer contributors for longer.

We've chosen not to do this because our expectation is that the Grants team selects grants based on what they estimate will produce the highest value for the Project.  If a grant is delivering that value, the team shouldn't have to choose a lower-value grant simply to spread money around more evenly.

If we wanted maximum distribution, we could simply ask which Project members are willing to take money and divide it up evenly.  That's not the goal.  Preferring to continue grants that are delivering expected value adds certainty for everyone and lets us invest more deeply in what is working.

We want to support more grantees by bringing in more money and growing the program, not by doing a round-robin with limited funds.

## Selection process

### Selection criteria

The Grants team considers:

- **Contribution history**: What has this person done for the Project?  How long has this person been involved?  What's the nature and impact of the work?
- **Continued engagement**: Is this person actively contributing?  Does the person seem likely to continue?
- **Benefit from support**: Does this grant make a meaningful difference in the Project's ability to retain this contributor?  Are there other funding sources that would be more appropriate for the situation?
- **Diversity of support**: We want to support people across different areas of the Project.  The Grants team should consider whether we're reaching different teams and types of contribution.

The Grants team does not require detailed financial disclosure.  A brief explanation of how the grant would help is sufficient.  We're not conducting means testing; we're making judgment calls about where modest support would do the most good for the Project.

### Application process

We envision a lightweight application process:

1. **Self-application**: Anyone can apply via a simple form.
2. **Support gathering**: Applicants may request support from teams or members (see "Expressing support" below).
3. **Grants team review**: The Grants team reviews applications and makes selections.

We intend to issue as many grants as we can fund upfront — not hold back money to spread it across quarters.  The quarterly cadence is for two purposes: reviewing new applications (if funding becomes available through new money or canceled grants) and reviewing the expected quarterly updates from current grantees to ensure the program is working and to provide feedback or take action if needed.

Applications ask:

- Who are you and what do you work on in the Rust Project?
- How long have you been contributing?
- How would this grant help you continue contributing?  (E.g., do you currently receive any funding to work on Rust?)
- Is there anything else you'd like us to know?

That's it.  No lengthy proposals.  No detailed project plans.  We're investing in contributors based on track records, not funding proposals.

### Expressing support

While applications must be self-submitted, Project members and teams can express support for applicants:

- Any Project member can voice support for an existing application, explaining why the work is valuable for the Project to support.
- Teams can formally endorse applications, indicating that the team sees value in the work and wants to see it continue.

This support is not a requirement — many excellent contributors may not think to ask for endorsements, and the Grants team should evaluate applications on their merits regardless.  But when teams or members do express support, it provides valuable signal about the impact of an applicant's work.

Applicants who want to request support from teams or members should do so themselves.  This keeps the process simple and ensures that applicants are ready, willing, and able to participate.

### Conflicts of interest

The Grants team needs clear rules about conflicts of interest:

- Team members may not vote on applications from themselves.
- Team members should recuse themselves from applications where they have a close relationship with the applicant outside of the Project — for example, if they work at the same company or have a close personal relationship.
- All recusals should be documented.

We don't require recusal simply because a Grants team member happens to work on the same Project team as an applicant.  For many team members, the union of their teammates across the Project would be a substantial fraction of the eligible population.  Recusal should be reserved for situations where there's a genuine conflict of interest, not merely professional familiarity.

We expect that some excellent candidates may have relationships with Grants team members.  That's fine — it just means those individuals recuse themselves, not that the candidate is ineligible.

### Moderation check

Before finalizing selections, the Grants team will provide the names of applicants under serious consideration to the moderation team for feedback.  This is a confidential check, not a veto — an open moderation matter doesn't necessarily prevent a grant from being issued.

Moderation operates at many levels, and until an accusation has been fully investigated and resolved, a person deserves some presumption of innocence.  However, the Grants team should have full context to consider.  If there's a serious ongoing concern, the Grants team can delay a decision pending resolution or can decide to proceed while aware of the situation.

This check also works in the other direction: if the moderation team removes someone from the Project, the moderation team will inform the Grants team so that any active grant can be reviewed.

## Expectations of grantees

### What we expect

We expect grantees to:

- **Continue contributing to the Rust Project** in a manner consistent with what they were doing when selected.  We're not specifying deliverables, but we expect continued engagement.
- **Provide brief quarterly updates** describing work done and any changes to the grantee's situation.  A few paragraphs is fine.  (Project Goals updates satisfy this requirement; simply point the Grants team to those.)
- **Be responsive to program managers** who will check in periodically to help collect updates and ensure reporting happens smoothly.
- **Consider participating in Project Goals** where relevant.  We won't require this, and it won't affect selection, but grantees whose work aligns with a goal may benefit from the structure and support that goals provide.
- **Consider working with the Content team**.  We also encourage, but don't require, grantees to be responsive to the Content team when asked to help communicate successes.  The earlier grants program required grantees to write blog posts at year's end; we're deliberately asking less.  But part of what sustains a program like this is being able to show what it accomplishes, and grantees who are willing to share their stories help ensure the program can continue.

### What we don't expect

We don't expect:

- **Timesheets or hourly accounting**.  We're not buying hours.
- **Specific deliverables or milestones**.  This isn't a contract.
- **Detailed progress reports**.  Brief qualitative updates are sufficient.
- **Any particular level of availability or responsiveness** beyond what's normal for a Project contributor.

The reporting requirement is lighter than what we'd expect from contractors or employees.  It's meant to be low-friction while still giving us what we need to demonstrate the program's value.

### Why we need some reporting

Some council members have expressed concern about reporting being too onerous.  We've tried to design the requirements to be minimal while still serving essential purposes.

This is worth explaining, because there's appeal in the simpler model of giving money without strings attached.  If someone has already done good work, why not just say "thank you" and trust that the work will continue?

The answer comes from our experience with the previous program.  The grants themselves weren't the problem — by many accounts, they provided real value to recipients and to the Project.  What wasn't working was that we weren't getting timely feedback.  Without that feedback:

- There was no opportunity to course correct when things weren't going well.
- Teams couldn't step in to help struggling recipients succeed.
- The Grants team couldn't learn from earlier choices and make better ones.
- When the program was paused, it was hard to justify restarting it because we couldn't point to clear successes.

Great work got done, but it wasn't reported back to the Project or the public.  The program couldn't be justified — not because it failed, but because we couldn't show that it succeeded.  That's a failure of communication, not of the underlying idea.

Light reporting serves several purposes:

1. **Learning and improving**: Without feedback, we can't get better at selecting recipients, supporting them, or running the program.  It's difficult to maintain the perception that a program is working if we have no evidence that it is.
2. **Catching problems early**: If a grantee's situation changes or the grantee becomes disengaged, we'd rather know sooner so we can have a conversation about it — and perhaps help the grantee succeed.
3. **Communicating successes**: The Foundation, potential sponsors, and the broader community benefit from hearing about what the program enables.  This is essential for the program's sustainability.

The program managers will actively support grantees in meeting these expectations.  We're not asking grantees to navigate bureaucracy alone.  The goal is to close the feedback loop without burdening the people we're trying to support.

## Support for grantees

### Program management

The program management team will help grantees succeed.  This includes:

- Checking in proactively to gather updates.
- Helping grantees who find written reporting challenging.
- Ensuring communication flows smoothly between grantees and the Grants team.
- Flagging any concerns early so they can be addressed.

We've invested substantially in program management precisely so that programs like this can function without overwhelming volunteers with administrative burden.

### Content team coordination

One of the biggest issues with the previous program was visibility.  Great work got done, but nobody heard about it.  Without stories to tell, it was hard to justify the program's continuation — not because it failed, but because we couldn't show what it accomplished.

The Content team can help fix this.  When grantees accomplish something noteworthy, the Content team can help communicate it.  This might mean a blog post, an interview, a podcast appearance, or other visibility.  The goal is to celebrate successes in ways that benefit the grantee, the program's sustainability, and the broader perception that investing in contributors pays off.

We're not asking grantees to write blog posts or create marketing materials themselves — the earlier program required this, and we've reduced the ask.  We're asking them to be open to sharing what they're working on with the Content team, who can then craft the story.  The reporting requirement feeds directly into this: brief updates give the Content team something to work with.

That said, some grantees may prefer not to participate in public visibility.  That's OK — it's encouraged, not required.  We recognize that not everyone is comfortable with public attention, and we won't penalize anyone for declining.  But grantees who are willing to share their stories help ensure the program can demonstrate value and continue.

This visibility serves everyone's interests:

- Grantees get recognition for their work.
- The program demonstrates value, supporting future funding.
- The broader community sees that investing in contributors matters.
- Potential future funders can see what their money could accomplish.

### Project Goals integration

We encourage grantees to engage with the Project Goals process where it makes sense for their work.  Grantees participating in Project Goals would satisfy the program's reporting requirements through their goal updates.

This isn't a requirement, and the Grants team does not prioritize applications based on Project Goals involvement.  Some excellent maintenance work doesn't map neatly to a goal, and that's fine.  Applicants will not improve their chances by forcing work that isn't appropriate into a Project Goal — the Grants team selects based on expected value to the Project regardless of whether the work fits the goals framework.

Where alignment does exist, though, we want to facilitate it.  Project Goals provide structure, visibility, and often additional support.  They let goal owners request dedicated bandwidth from teams, receive support from program managers, and gain greater visibility for their work.  The goals process is meant to be supportive — not a "Project asks" system where external parties request work from already-overloaded volunteers.

## When circumstances change

### The basic principle

Circumstances change.  Someone might get a job.  Someone might need to step back from contributing for personal reasons.  Someone's focus might shift to a different area.

When this happens, we communicate.  The Grants team's job is to allocate funds in the best interest of the Project, but that doesn't mean surprising people.  If someone's situation changes in a way that affects the grant, we talk about it.

### Specific scenarios

#### A grantee gets a job doing Rust work full-time

If someone is now being paid well to work on Rust, it probably makes sense to redirect grant funds to someone else.  But we'd have a conversation first.  Maybe the new job covers different areas than what the person was doing as a volunteer, and continued support makes sense.  Maybe not.  We'd figure it out together.

#### A grantee gets a job unrelated to Rust

If the grantee is still contributing to Rust in remaining time, the grant might continue.  The question is whether the support is still making a meaningful difference in the ability to contribute.

#### A grantee becomes less engaged

If the quarterly updates suggest declining engagement, the Grants team should reach out.  Maybe there are reasons and the recipient will soon return to more active contribution.  Maybe not.  If someone has effectively stopped contributing, continuing the grant isn't the best use of funds — but we'd communicate about this before making changes.

#### A grantee receives RFMF support

If someone is selected for RFMF funding, that would typically replace the grant.  RFMF support would generally be more substantial.  We'd coordinate to ensure a smooth transition.

#### A grantee stops responding

If a grantee stops responding to outreach from program managers or the Grants team, the grant will eventually be canceled.  This was a real problem in previous programs — staff spent considerable time chasing unresponsive recipients.

We'd make reasonable efforts to reach out through multiple channels before taking action.  Life happens, and there might be good reasons for temporary unresponsiveness.  But if someone has genuinely disappeared without explanation, continuing to send money doesn't serve the program's purposes.

#### A grantee is removed from the Project

If the moderation team removes someone from the Project, the moderation team will inform the Grants team.  In most cases, the grant will be rescinded.

The specific circumstances matter.  Removal might happen for many reasons, and the Grants team should understand the context before acting.  But generally, someone who has been removed from the Project is no longer in a position to do the work the grant was meant to support.

#### A grantee voluntarily leaves the Project

If a grantee voluntarily steps back from the Project — e.g. due to changing circumstances or interests — we expect the grantee to notify us.

The Grants team would have a conversation to understand the situation.  Sometimes people leave teams but continue doing valuable work in the ecosystem.  Sometimes they're taking a break but expect to return.  Sometimes they're moving on entirely.

If we expect valuable work to continue despite the formal departure, the grant might continue.  If the work is ending, the grant generally would too.  We'd figure this out together.

### Expectations about continuation

We want to be clear with grantees: renewal is not guaranteed.

Grants that are working well — where the grantee remains engaged and the support is clearly adding value — have a natural advantage at renewal time.  A demonstrated track record reduces uncertainty and provides evidence that new applicants can't.  But the program operates within budget constraints, and if the council's funding situation changes, or if we need to redistribute support, even effective grants might not continue.

We'll communicate early and often about this.  If we anticipate not renewing a grant, we'd tell the grantee as soon as we know, not at the last minute.  We want people to be able to plan.

### The renewal process

How does renewal actually work?

- **Reapplication required**: Grantees seeking renewal submit a new application each year.  Much of the content may be similar to the previous year, but the Grants team needs current information.
- **Considered alongside new applicants**: Renewal applications are evaluated alongside new applications.  There's no separate track or guaranteed renewal.
- **No shame in not renewing**: If someone has done good work but the Grants team decides to support others instead, that's not a negative judgment.  Budget is limited, and excellent contributors may not receive grants in any given year.

#### The natural advantage of effective grants

The Grants team's job is to maximize expected value to the Project with each selection.  This might suggest strict neutrality between renewal and new applicants.  But effective grants have a natural advantage: **reduced uncertainty**.

When we've funded someone for a year and seen how that person operates under the program — how the person reports, whether the work continued, whether the support made a difference — we have real data.  A new applicant, no matter how promising, is a bet on future performance.  An effective current grantee is a known quantity.

This isn't formal priority.  The Grants team won't continue a grant that isn't working just because it exists.  And a sufficiently compelling new applicant could displace a satisfactory renewal.  But it would be strange to cancel a grant when we've been getting what we wanted.  The track record matters.

So while we don't adopt a formal preference for renewals over new applicants, effective grants will often continue simply because they've demonstrated value.  The uncertainty is gone.  That's a form of evidence that new applicants can't provide.

In past programs, there was a slight preference for awarding grants to people who hadn't received them before.  We're not adopting that either.  The Grants team should make the best decisions they can about where support will do the most good, informed by whatever evidence is available — including the track record of existing grants.

## Coordination with RFMF

### Different programs, complementary purposes

The Rust Foundation Maintainers Fund and this grants program serve related but distinct purposes:

**RFMF**:

- Funded by corporate sponsors through the Foundation.
- Focused on sustainable, ongoing support for maintainers.
- Likely to involve larger amounts and longer-term commitments as it grows.
- Administered by the Foundation with input from a committee that includes Project and funder representatives.

**This program**:

- Funded from the council's Project Priorities budget.
- Focused on smaller-scale, flexible support.
- Controlled entirely by the Project through the council.
- Useful for people who don't fit the RFMF model or as bridge support.

The programs complement each other.  The grants program can reach people the RFMF doesn't (yet) serve.  It can provide quick, flexible support while RFMF develops its processes.  And because this program is run by and for the Project, the Grants team and the council learn directly from the experience — what works, what doesn't, and how to improve.

### Coordination mechanisms

To avoid duplication and ensure the programs work well together:

- The Grants team should be aware of who's receiving RFMF support.
- If someone applies to both programs, the Grants team and RFMF should communicate about which is the better fit.
- If someone transitions from a grant to RFMF support, the programs should coordinate timing.
- Regular informal communication between the programs helps keep everyone aligned.

We should not create a situation where someone receives both a grant and RFMF support unless there's a specific reason that makes sense.

## Implementation and timeline

### Timeline

- **February 2026**: Council discusses and refines this RFC.
- **Late February/Early March 2026**: FCP on establishing the program and appointing the Grants team.
- **March 2026**: Committee forms, finalizes application process.
- **April 2026**: Open applications and nominations.
- **May 2026**: First selections; payments begin.
- **August 2026**: First quarterly check-ins.
- **Q4 2026**: Committee reviews program operation, recommends any adjustments for 2027.

### Next steps

Once this RFC is accepted:

1. Identify candidates for the Grants team.
2. Develop the application form and process details.
3. Begin the timeline above.

## Grants team

This RFC establishes the **Grants team** to manage the program.  The team consists of five members appointed by the council.  Five is a good size — large enough to represent views from across the Project, small enough to make decisions efficiently.  Members may be from the council or from the broader Project, depending on who's available and interested.

The team designates a liaison to the council (see "Council liaison" below).  While this will often be a council member, we don't require it — this handles the case where someone leaves the council but remains on the Grants team and can continue serving as liaison.

### Grants team charter

This charter establishes the Grants team as a subteam of the Launching Pad team within the Rust Project governance structure.  As a Launching Pad subteam, the Grants team is a pseudo top-level team — directly accountable to the council and tasked with crosscutting matters that affect the Project as a whole.

#### Mission

The Grants team administers the Project Grants Program, selecting grant recipients, overseeing the program's operation, and ensuring grants achieve their intended purpose of adding value to the Project by targeted support of Project contributors doing valuable work.

#### Goals

- Select grant recipients who will benefit the Project.
- Ensure the program runs smoothly and achieves its goals.
- Maintain accountability and transparency in how grant funds are used.
- Learn from experience and recommend improvements to the council.

#### Delegated responsibilities

The Grants team has authority to:

- Review applications and select grant recipients within the budget allocated by the council.
- Set application deadlines and manage the application process.
- Communicate with grantees about expectations, check-ins, and any changes.
- Make decisions about grant continuation or early termination when circumstances change.
- Recommend policy adjustments to the council based on experience.

The team does not have authority to:

- Change the budget allocation (requires council action).
- Change eligibility criteria or core program parameters (requires council action).
- Bind the council to future commitments.

#### Duration

This is a standing team for as long as the program continues.  The council should review the team's charter and performance annually when reviewing the program overall.

#### Parent team

The team is a subteam of the Launching Pad.

#### Contact, communication, and workspace

- **Zulip stream**: `#grants`.
- **GitHub**: Issues, policies, minutes, recusals, and other team records are stored in the `rust-lang/grants-team` repository.

#### Process

- The team coordinates primarily via Zulip and team calls.
- Decisions about grant selections require consensus among non-recused members.
- The team meets at least quarterly to review applications, existing grants, and program status.
- The team provides updates to the council through the liaison member.

### Membership and selection

The council appoints the initial team membership and manages changes to it going forward.

Council members who are currently receiving a grant or who have an outstanding application should recuse themselves from decisions about team membership.  As is normal council process, the team that member represents can send an alternate for such decisions.  A member who is merely planning to apply at some point in the future does not need to recuse.

Most Project teams manage their own membership internally — the team controls the admission of new members.  That model isn't right here.  Because the Grants team allocates money, it needs to remain representative of the Project overall.  Having the council manage membership ensures accountability and prevents the team from becoming insular over time.

If the program grows substantially, it might make sense to evolve toward a model where teams directly appoint representatives, similar to how they appoint council members.  But for a program of this size, having the council manage membership is simpler and provides appropriate oversight.

### Member expectations

Serving on the Grants team is a moderate time commitment.  Members should expect:

- **Selection periods**: A few hours during selection periods (likely once or twice per year initially) to review applications, discuss candidates, and make decisions.
- **Quarterly reviews**: A few hours each quarter to review reports collected by program managers, assess how grants are proceeding, and identify any concerns.
- **Ad hoc meetings**: Occasional meetings to address situations that arise — a grantee whose circumstances change, a concern about engagement, a question about policy.
- **Working with teams**: When a grantee is struggling, working with relevant team leads to see if additional support could help the grantee succeed.
- **Program improvement**: Contributing to discussions about lessons learned and how to improve the program over time.

The total time commitment is estimated at 5–10 hours per quarter during normal operation, with somewhat more during initial selection and end-of-year reviews.  Program managers handle day-to-day coordination, so Grants team members focus on decision-making and oversight rather than administrative tasks.

### Council liaison

The team designates one member to serve as liaison to the council.  The liaison:

- Attends council meetings when grants-related topics are on the agenda.
- Provides regular updates to the council about program status.
- Brings policy questions to the council when the team needs guidance or approval.
- Ensures the council has visibility into how the program is operating.

While the liaison will often be a council member (since they're already attending meetings), this isn't required.  If someone leaves the council but remains on the Grants team, that person can continue as liaison if the team wishes.  The key is that someone is consistently bridging communication between the team and the council.

## Prior art

### The earlier Rust Foundation grants programs

The Rust Foundation previously administered a comprehensive Community Grants Program with several distinct tracks.  Understanding what that program offered — and where it struggled — informs our design choices.

#### Fellowship program

The Fellowship program offered three types of fellowships, all at $1,500 per month — a rate similar to what we're proposing:

**Project Fellowships** (12 months): For Rust Project Team and Working Group members to support contributions serving team goals.  Included a $4,000 combined travel, hardware, and training allowance (up to $1,500 for hardware).

**Community Fellowships** (12 months): For people building Rust communities outside of Western Europe and North America — organizing communities and events, creating content and training materials.  Same $4,000 allowance as Project Fellowships.

**Project Goal Fellowships** (6 months): For anyone working on agreed Rust Project Goals.  Applicants didn't need to be Project members but needed relevant skills.  Included a $2,000 allowance.

All fellowships expected an average of 20 hours per month, though Fellows mapped their own schedules.  Reporting consisted of brief quarterly reports plus participation in catch-up calls with Foundation staff.  At year end, Fellows wrote a blog post about their experience.

#### The 2022 Fellowship program and Associate Fellows

The earliest iteration of the Fellowship program in 2022 differed from later versions in ways worth noting.  The 2022 program offered a lower stipend ($1,000 per month, later increased to $1,500) but introduced a two-tier system:

**Fellows** were experienced, active Project members with the skills and experience to work largely independently without additional support.  The majority of the cohort was expected to be Fellows.

**Associate Fellows** were community members with Rust programming experience but less (or no) experience with the Rust Project.  These were people keen to develop skills with a view to becoming active team members, working group participants, or other maintainers in the future.  The program expected 4–6 Associate Fellows in its first cohort, with placement dependent on capacity from Project teams to provide mentorship support.

The 2022 program also included explicit philosophy about its purpose:

> "We are not expecting [Fellows] to spend 20 *more* hours a month supporting the community, rather to reward them for the time they are already contributing to help improve their work-life balance and reduce the risk of burnout."

This anti-burnout framing — the explicit statement that Fellows were NOT expected to increase their hours — is philosophically aligned with our RFC's approach of supporting people to continue doing what they're already doing, rather than purchasing additional time.

The Associate Fellows tier was apparently not continued in later iterations of the program, likely because it required significant mentorship capacity from teams.  However, if the council wishes to support contributors who are earlier in their journey toward becoming maintainers, a mentorship-oriented track like Associate Fellows could be considered in a future expansion of this program.

#### Project grants

Separate from fellowships, the Foundation offered Project Grants of $2,500 to $15,000 for specific packages of work to be completed within six months.  These were open to individuals and teams, whether Project members or not, including non-coding education and outreach work.

The work fell into three categories: Maintenance and Support (code review, documentation, issue triage, bug fixes), Development and Infrastructure (new features, performance, CI/releases), and Community (moderation, communication, education, mentoring, event organizing).

#### Other programs

The Foundation also ran **Hardship Grants** ($500–$1,000, rolling applications, decisions within 5 days) for Project contributors facing urgent financial insecurity, and **Event Support Grants** ($100–$500, quarterly budget of $15,000) for Rust community events.

#### What worked and what didn't

By many accounts, the grants themselves provided real value.  Recipients appreciated the support, and valuable work was done.  However, the program struggled in several ways:

- **Feedback loop**: The team managing the program didn't receive timely feedback about how grantees were doing.  Without this, there was no opportunity to course correct or learn from earlier choices.
- **Visibility**: Great work was done, but it wasn't communicated back to the Project or the public.  When the program was paused, it was hard to justify restarting it because we couldn't point to clear successes.
- **Program complexity**: Multiple tracks with different eligibility criteria, durations, and allowances created administrative overhead and potential confusion.

#### How this RFC responds

This RFC draws on these lessons.  We've designed:

- **Light reporting requirements** with program manager support to close the feedback loop without burdening grantees.
- **Content team coordination** to ensure successes are communicated.
- **A simpler, unified structure** rather than multiple distinct tracks — we can add complexity later if needed.
- **Council accountability** so that the Project directly learns from and improves the program.

We've chosen not to include travel or training allowances, hardship grants, or event support in this initial program.  Those are good ideas that we might revisit as the program grows (see "Future possibilities"), but starting simpler lets us focus on getting the core program right.

### Grants programs in other open-source communities

Other open-source projects have experimented with similar support mechanisms:

**Python Software Foundation Grants Program**: The PSF has run grants supporting Python community development.  Their guiding principles emphasize being impactful, reliable, equitable, transparent, and sustainable.  Notably, the program has at times needed to pause when reaching funding caps — a reminder that sustainability requires ongoing attention.

**Linux Foundation fellowships**: Various fellowships have supported kernel maintainers and other critical contributors.  These tend to be more substantial commitments than what we're proposing, but they demonstrate the value of investing in people who maintain critical infrastructure.

These examples suggest that modest, flexible support for contributors is a recognized need across open-source communities.  Our program is one approach, designed to fit the Rust Project's structure and resources.

## Rationale

Throughout council discussions, various concerns and questions have been raised.  This section addresses them.

### On reporting requirements

#### "Reporting will be too onerous."

We've designed the reporting requirements to be minimal: brief quarterly updates supported by program managers.  A few paragraphs about what someone worked on is not onerous.  We won't ask for timesheets or detailed proposals.  If a grantee participates in the Project Goals program and provides regular updates through that process, those updates satisfy our reporting requirements.

If someone finds even this challenging, the program managers can help.  We're not trying to create paperwork; we're trying to demonstrate that the program accomplishes something.

#### "Why not just give money without strings attached?"

This is a legitimate perspective.  We select based on demonstrated track record — that's retrospective.  We expect continued engagement but not specific deliverables.  If someone has done good work, we're acknowledging that, while also expecting — gently, without specific deliverables — that the work will continue.

The light reporting is necessary to close the feedback loop, as discussed in the motivation section.  Without it, we can't learn, improve, or demonstrate value.

### On circumstances and continuation

#### "What happens when someone's situation changes?"

We talk about it.  The Grants team communicates with grantees.  If circumstances change — a new job, declining engagement, other funding — we have a conversation about whether the grant should continue.  No surprises.

#### "Grant continuation expectations are unclear."

We're being explicit: renewal is not guaranteed, but effective grants have a natural advantage.  A track record of successful performance under the program provides evidence that new applicants can't.  The Grants team maximizes expected value, and reduced uncertainty counts.  That said, budget constraints and changing needs may require changes.  We'll communicate early if a grant won't be renewed.

### On conflicts of interest

#### "There are conflicts of interest."

Grants team members recuse themselves from decisions involving themselves or where they have a close relationship outside the Project (e.g., same employer, close personal relationship).  We don't require recusal for mere Project teammates — that would exclude too many applicants.  We document recusals.  This doesn't preclude excellent candidates who happen to know team members — it just means the right people recuse themselves when appropriate.

### On hierarchy and esteem

#### "This will make grant recipients seem more esteemed."

We're being explicit that grants are not a measure of contribution quality.  Many excellent contributors can't or don't want to receive money.  The grants are support for people who can benefit, not recognition of superior work.

Whether this actually prevents hierarchical perceptions is partly in our hands — in how we communicate about the program — and partly not.  But we're naming the concern and designing with it in mind.

### On workload and sustainability

#### "This adds work to already-overloaded volunteers."

The Grants team distributes the work across a small group.  Program managers handle operational coordination.  The council delegates selection decisions to the Grants team while retaining oversight through the liaison member and control over policy.  We've tried to design the program so that the work is manageable and distributed appropriately.

#### "The previous program had issues."

The previous grants program suffered from unclear accountability, inconsistent reporting, and poor communication when it was paused.  We've learned from that:

- Clear Grants team oversight.
- Defined (light) reporting expectations.
- Program manager support.
- Commitment to communicate early about changes.

#### "This won't solve the bigger problem."

It won't.  $100,000 cannot sustainably fund open source maintenance.  We're doing what we can with what we have, learning from it, and hoping to grow.  The RFMF and other efforts are pursuing larger-scale solutions.  This program is one piece of a larger picture.

### On program design

#### "We should fund specific projects or goals instead."

That's a different program.  Contracts for specific work make sense sometimes.  Project Goals provide a framework for directed effort.  This program is about investing in contributors based on track records, not funding proposals.  Both approaches have value.

#### "Why call them 'grants'?"

We chose the term "grants" deliberately.

- **Neutral connotation**: "Grants" is a flexible, widely understood term that doesn't imply special status or rank.  Unlike "fellowships" or "scholarships," which can carry connotations of academic achievement or prestige, "grants" simply describes financial support for a purpose.

- **Avoiding hierarchy**: We want to avoid creating a perception that grant recipients are more esteemed than other contributors.  Many excellent contributors can't or don't want to receive money.  The term "grants" helps frame this as practical support rather than an honor or award.

- **Familiar and appropriate**: In open source and nonprofit contexts, "grants" commonly describes funding that supports work without turning it into employment.  The term accurately reflects what we're doing: providing financial support to enable continued contribution.

- **Consistency with similar programs**: Other open source communities use "grants" for similar programs (e.g., PSF grants).  Using the same terminology makes our program immediately understandable to people familiar with those models.

Alternative terms like "stipends," "fellowships," or "awards" each carry different connotations that we wanted to avoid.  "Stipends" suggests smaller, more incidental payments.  "Fellowships" implies a selective honor.  "Awards" suggests recognition of achievement.  "Grants" is the most neutral term that accurately describes what we're providing.

#### "Why $1,400 per month specifically?"

This is similar to the $1,500 rate used for the October 2024 fellowships.  It's designed to be enough that someone can justify spending meaningfully more time contributing to Rust — for example, by making it possible to avoid taking on a second job.  We're not trying to replace a salary; we're trying to remove a barrier.  We acknowledge that purchasing power varies globally.

#### "Why twelve-month terms?"

One year is long enough to provide stability and let people plan but short enough that we can reassess annually.  It's renewable if circumstances warrant.

#### "Can council members receive grants?"

Yes.  Council members who aren't on the Grants team can apply without special procedures for that application.  If a council member who also serves on the Grants team applies for a grant, the standard recusal rules apply — that member recuses from discussions about the application.

However, council members who are currently receiving a grant or who have an outstanding application should recuse themselves from decisions about Grants team membership.  Their team can send an alternate for such decisions.

#### "Can people apply who haven't been contributing long?"

The Grants team considers the whole picture.  Someone newer but doing substantial, high-quality work might be a good candidate.  Someone who just started probably isn't.  There is no hard cutoff.

#### "What about people who receive other funding for Rust work?"

It depends on the nature of that funding.  Someone paid full-time to work on Rust probably doesn't need a grant.  Someone with a part-time contract or limited scope might still benefit.  The Grants team evaluates each situation.

#### "How do we measure success?"

Qualitatively more than quantitatively.  Did the grantees continue contributing?  Can we point to work they did?  Did the program run smoothly?  Do we feel good about doing it again?

#### "What if we get many more applications than we can fund?"

The Grants team prioritizes.  Not everyone can be funded.  We try to be fair, consider diversity of support across areas, and communicate respectfully with people we can't support this round.

#### "What if someone stops contributing after getting a grant?"

We'd try to catch this early through check-ins.  If someone disengages, we'd reach out.  If it becomes clear they've moved on, we'd have a conversation about whether the grant should continue.  This isn't about punishment — it's about directing limited resources where they'll do good.

#### "How does this relate to the Foundation's work?"

The Foundation provides the payment infrastructure and includes these expenditures in its financial management.  The program itself is controlled by the council through the Grants team.  It's Project-directed, Foundation-facilitated.

#### "What if the council's budget is cut?"

We'd reassess.  We might need to reduce the number of grants or not renew some.  We'd communicate early with affected grantees.  This is why we maintain reserves and don't overcommit.

## Future possibilities

### What success looks like

A year from now, we'd like to be able to say:

- The program supported several contributors who continued doing valuable work.
- We learned how to run such a program effectively.
- We can point to specific outcomes enabled by the grants.
- The administrative burden was manageable.
- We're ready to do it again, and perhaps to grow.

### Growing the program

If this program succeeds, how might it grow?

- **Demonstrated value attracts funding**: Success stories help the Foundation make the case for additional Project Priorities funding.  "Here's what the Project accomplished with $100k" is a compelling argument for allocating more to the council's budget.
- **RFMF growth**: Our experience here informs how RFMF might scale.  Lessons learned transfer.
- **External interest**: Companies who benefit from Rust might be interested in contributing to a program with a proven track record.  This isn't likely in year one, but could become possible as we demonstrate results.

We won't grow the program until we've shown it works.  But we're designing it with growth in mind.

### Sustainability concerns

Some have raised concerns about sustainability.  What if funding doesn't continue?

This is a real risk.  The council's budget depends on Foundation allocations and donations.  If those decline, we might not be able to maintain the program at its current level.

We mitigate this by:

- Starting conservatively, so we're not overcommitting.
- Keeping reserve funds rather than allocating everything.
- Being clear with grantees that continuation isn't guaranteed.
- Designing the program to demonstrate value, improving our chances of continued funding.

We can't eliminate uncertainty.  But we can be honest about it and manage it responsibly.

### Longer-term evolution

If the program grows substantially over time, several evolutions might make sense:

- **Team structure**: A larger program might warrant a team where top-level teams directly appoint representatives, similar to how they appoint council members.
- **Grant tiers**: Different grant amounts for different situations — smaller amounts for lighter support, larger amounts for contributors who might otherwise take non-Rust jobs.
- **Term flexibility**: Shorter or longer terms depending on the grantee's situation and the nature of the work.
- **Integration with RFMF**: As RFMF develops, the programs might evolve their relationship — perhaps this program becomes a "feeder" for RFMF, or they formalize coordination mechanisms.

These are possibilities, not plans.  We'll learn from experience and adapt.

### Elements from earlier programs we might readopt

The earlier Foundation grants programs included several elements we've chosen not to include in this initial RFC but that might be worth revisiting as the program matures:

**Travel and training allowances**: The earlier fellowships provided $2,000–$4,000 for travel, hardware, and training.  However, these allowances historically did not receive much uptake — many fellows did not fully use them.  Additionally, Project members receiving grants will already be eligible for travel grants through the council's existing travel budget.  If we find that training support or hardware allowances would significantly benefit grantees in ways not covered by existing programs, we could revisit this.

**Hardship grants**: The Foundation ran a separate, fast-track mechanism for contributors facing urgent financial need — smaller amounts ($500–$1,000) with quick decisions (within 5 days).  This served a different purpose from ongoing support: emergency assistance for people in crisis.  A hardship track could complement the main grants program for situations that can't wait for the normal selection cycle.

**Event support grants**: Small grants ($100–$500) supported Rust community events, with a quarterly budget cap of $15,000.  This is adjacent to but distinct from contributor support — it invests in community infrastructure rather than individuals.  Such a program might be better run separately from the grants program or might fit under the council's other budget categories.

**Community-building focus**: The earlier Community Fellowships specifically supported work building Rust communities, with a geographic focus on regions outside Western Europe and North America.  As Rust grows globally, targeted investment in underserved regions could help build sustainable local communities.  This might make sense as a distinct track or selection criterion.

**Project Goals integration**: The earlier program had a dedicated 6-month Project Goal Fellowship track tied to the goals calendar.  We've chosen to encourage but not require Project Goals participation.  If we find that closer integration would be valuable — for example, if grantees working on goals consistently have better outcomes — we might formalize this connection.

**Dedicated Project Goals funding**: A related but distinct possibility is acting as a matchmaker between goal owners and ecosystem companies who want to support specific work.  Companies might provide earmarked funds for particular goals; we would help connect those funds to goal owners pursuing that work.  This is different from general grants — it's targeted support for specific projects — but could complement the grants program and help companies invest in work they care about.  The Project Goals framework provides the alignment mechanism: a company can see what goals exist, understand the value, and fund accordingly.  Some portion of this funding would support general infrastructure such as program management.  By sharing infrastructure in this way between programs, we get some economies of scale.

We're starting simple because we want to get the core program right before adding complexity.  These elements represent natural directions for growth once we've demonstrated that the basic model works.
