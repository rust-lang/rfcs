- Feature Name: N/A
- Start Date: 2024-01-19
- RFC PR: [rust-lang/rfcs#3599](https://github.com/rust-lang/rfcs/pull/3599)
- Rust Issue: N/A

# Summary
[summary]: #summary

Re-organise the compiler team:

- Re-define and rename the tiers of membership
- Change how team members and contributors are promoted
- Document expectations of team members and maintainers
- Establish mechanism for scaling additional maintenance activities that
  maintainers take on and recognising these contributions

# Motivation
[motivation]: #motivation

Compiler team contributors were introduced [in 2019 with RFC 2689][rfc2689],
the last significant change to the compiler team's structure. A lot has changed
in the project and compiler team since that time: we receive [approximately
twice as many pull requests each week][review_queue_analysis], there are more
responsibilities that team members choose to take on, and many members of the
team are now employed to work on the project.

Given these increased demands on the team, it is important that the compiler
team's structure can grow while maintaining high-quality output and remaining
sustainable for team members. Ensuring that team members aren't assigned
an untenable number of reviews each week requires that the team onboard new
contributors and team members at a rate which keeps pace with project growth.

Furthermore, the day-to-day operations of the team are composed of more varied
tasks than was the case when RFC 2689 was drafted, now including prioritisation
and issue triage, performance triage, meeting agenda preparation, and review of
major change proposals. Team members who choose to contribute to these efforts
should have those additional activities recognised.

As the team gets larger, our processes need to remain efficient. Final comment
periods (FCPs) have traditionally required sign-off from all team members,
which can become onerous with more team members. As the number of compiler team
members has grown from ~10 to ~15 since RFC 2689, the team has already noticed
scaling issues with our FCP process.

Processes which scale poorly with team size have acted as a unconscious
disincentive to promote compiler team contributors to compiler team members.
Similarly, the team has found that nominations being the primary mechanism for
promotion to compiler team contributor or member tends to result in contributors
falling through the cracks and being considered team members in the minds of the
team but not actually having been nominated for promotion.

Since RFC 2689, the compiler team contributor role's purpose has become
confused. It is often beneficial to be able to grant the infrastructure
and merge permissions to trusted contributors quickly so they can work more
efficiently. However, it is also desirable for the compiler team contributor
role to act as recognition for those contributors who have shown staying power
and that the team would like to recognise. These goals are in tension, adding
new contributors early and regularly improves the efficiency of the compiler
team while watering down the recognition and sense of achievement that the role
would ideally confer.

In addition, as compiler team contributors and members increasingly leverage
their contributions to gain employment/contracts to contribute to the project
full-time (or otherwise), the naming of the compiler team contributor role can
be confusing. An employer unfamiliar with the project may not realise that a
compiler team contributor is a role within the project which recognises regular
contribution and trust rather than just having made a handful of contributions
and thus being a contributor.

[rfc2689]: https://rust-lang.github.io/rfcs/2689-compiler-team-contributors.html
[review_queue_analysis]: https://borrowed.dev/p/on-the-compiler-teams-review-queue

# Definitions
[definitions]: #definitions

There are various permissions/privileges/responsibilities which will be
referenced in later sections of this RFC, defined here:

- **r+**
  - Contributors with *r+* privilege are able to approve pull requests to be
    merged by *bors*. Contributors should not merge their own pull requests (with
    the exception of re-approving their own work on behalf of another contributor
    after a rebase or similarly trivial change). *r+* permissions apply to the
    whole repository, but [it is expected][expectations] that contributors limit
    themselves to only those parts of the *rust-lang/rust* repository that are under
    the purview of the compiler team (unless granted *r+* from other teams too), and
    for subsystems/pull requests that they are confident reviewing.
- **try**
  - Contributors with *try* permissions are able to trigger complete toolchain
    builds for a pull request or commit, which are then used by *rustc-perf* and
    *crater*. *try* permissions aren't available to everyone because try builds
    can pose a security risk: try builds have access to secrets and the resulting
    builds are hosted on `static.rust-lang.org` where we would never want
    malicious code.
- **review rotation**
  - Contributors on the review rotation will be randomly assigned to new pull
    requests submitted to the compiler. Being on the review rotation is one of the
    best ways for contributors to help the compiler team and learn new parts of
    the compiler. Review capacity is one of the most important resources that the
    team has, as it enables our progress in the compiler's continued development
    and maintenance.
- **organization membership**
  - Contributors that are added to the *rust-lang/compiler* team in the GitHub
    organization can be assigned to issues/pull requests, modify labels, receive
    group mentions and receive a "Member" badge next to their name.
- **rustc-perf**
  - Contributors with permissions to use *rustc-perf* can request benchmarking
    of their pull requests (and pull requests they are reviewing). *rustc-perf*
    permissions are useful for regular contributors as it is common to need to
    request benchmarks from contributors with permissions. *rustc-perf* permissions
    only make sense alongside *try* permissions.
- **crater**
  - Contributors with permissions to *crater* can request crater runs to check
    whether their code breaks any public ecosystem code. *crater* permissions only
    make sense alongside *try* permissions.
- **dev desktops**
  - Contributors with access to developer desktops are able to connect to shared
    development servers that they can do their contributions from.
- **triagebot**
  - [triagebot][triagebot] is a GitHub bot that can perform helpful tasks on issues
    and pull requests. Many of its functions are available to everyone, such as issue
    claiming, but some functions may be restricted to project/team members.

[expectations]: https://forge.rust-lang.org/compiler/reviews.html#expectations-for-r
[triagebot]: https://forge.rust-lang.org/triagebot/index.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Contributors start without any particular privileges, permissions or
responsibilities and can contribute whatever they'd like. Contributors can
progress to [Team Members][team-members] and then [Maintainers][maintainers].

## Team Members
[team-members]: #team-members

Being able to grant permissions to trusted contributors quickly is beneficial to
enable them to contribute to the project more efficiently and review and approve
work of their collaborators.

Any contributor can request to become a team members by contacting the
compiler team's leads, or current maintainers and team members can
nominate a contributor. Team leads will check for a reasonable contribution
history, and will check if the current team have any serious concerns related to
contributor conduct (waiting approximately one week).

When evaluating a candidate's contribution history, length of time and
consistency of contributions and interactions with other contributors and
maintainers will be taken into account. It is important to note that many kinds
of contributions will be considered such as code contributions, helping with
issue triage and bisection, running meetings and creating minutes, documentation
contributions for rustc internals or the [Compiler Development Guide], etc.

Team member is a mix of RFC 2689's "working group participant" and
"compiler team contributor" roles. It is explicitly intended to be granted more
liberally to contributors who have demonstrated competence and trustworthiness,
for whom they would be able to work more effectively with these permissions and
can be trusted to use them responsibly. Team members do not need to
have experience with most of the compiler, and can be specialised to specific
subsystems of the compiler.

Team members are granted *r+*, *try*, *triagebot*, *rustc-perf*, and *crater*
permissions; *organization membership*; and *dev desktop* access. Team members
can second major change proposals. Team members are considered members of the
Rust project as a whole, and are automatically eligible for any benefits that
incurs (e.g. invitations to meetups of project members). As representatives of
the Rust project, team members are expected to obey not just the letter
of the [Code of Conduct][coc] but its spirit.

Team members can choose to take on additional maintenance activities, such as
those listed in the [maintenance activities][maintenance-activities] section.
Participating in the team's review rotation is encouraged.

If a team member becomes inactive (the contributor's prior contributions and
other interactions with the project cease) for six months or more, the team
member will be moved into alumni status. At any point in the future, they can
ask to be re-instated at the team member level if they desire.

[Compiler Development Guide]: https://rustc-dev-guide.rust-lang.org/

## Maintainers
[maintainers]: #maintainers

Team members are eligible to become maintainers after they have continued to
contribute actively for a year. Team members can contact team leads or will
be contacted by team leads to enquire about promotion to maintainership. Team
members who are eligible for maintainership do not have to become maintainers.

Unlike team members, maintainers are a subset of the team expected to consider
themselves as exactly that, *maintainers*, of the compiler - put otherwise, to
be invested in the quality of the compiler codebase and overall health of the
compiler team, independent of their own projects. Maintainership is primarily
intended to recognise and encourage participation in activities which are vital
to the success of the compiler team and broader project.

Maintainers are expected to participate in the ongoing maintenance tasks that
the compiler team is responsible for (with all of the expected caveats for
vacation time, mental health breaks, etc) - listed as
[maintenance activities][maintenance-activities] below. However, not all
maintainers need to participate in these responsibilities to an equal degree.
Maintainers should participate in these tasks to the degree that they are able
- volunteers are not expected to participate as much as those employed to work
on the compiler, for example. It is the responsibility of the compiler team
leads to ensure that the ongoing maintenance tasks of the team can be completed
sustainably.

Maintainers aren't expected to make more contributions than team
members or be more active, just participate in
[maintenance activities][maintenance-activities] in addition to regular
contributions.

Like team members, after inactivity for six months or more, a maintainer will be
moved to alumni status. Maintainers who are no longer able to or are not helping
to maintain the compiler but otherwise wish to continue contributing to the
compiler can also be moved to alumni status and retain their team member status.
Alumni can ask to be reinstated in the future.

[coc]: https://www.rust-lang.org/policies/code-of-conduct

### Maintenance activities
[maintenance-activities]: #maintenance-activities

There are various maintenance activities that a maintainer could take on to help the
team.

Maintainers are expected to participate in maintenance activities - if they
are unable to participate in at least one activity then it makes sense to
step back from maintainership and just focus on their contribution. It isn't
possible to put a number on how many activities a maintainer should be
involved in (and this isn't an exhaustive list of activities), it depends on the
contributor. Maintainers ideally wouldn't be just-doing-the-minimum, but instead
acting as a maintainer because they are genuinely invested in the health of the
team and project. For most maintainers, it is anticipated that this will be a
handful of activities that interest them, but that the specifics will vary
with time.

Maintainers can get involved in any of these by contacting the team leads, by
asking maintainers currently involved in these activities, or by asking
in any venue where these activities are conducted (e.g. a Zulip stream).
Team members can participate in activities too - these aren't exclusively
the purview of maintainers.

- Final comment period (FCP) reviewer
    - Final comment periods are the process by which the team signs-off on a
      change before it is made, like stabilizing a feature.

      FCPs have always required whole team to sign-off, but this doesn't scale
      as the team grows. As described above, this acts as a disincentive for the
      team to grow. Furthermore, not all FCPs are relevant to all team members and a
      diffusion of responsibility means that most team members just sanity-check and
      then sign-off. This isn't ideal, as it doesn't guarantee that someone on the
      team has thoroughly considered a FCP.

      Instead, have FCPs require sign-off from maintainers who opt-in to
      being an "FCP reviewer", with the expectation that they will spend time
      reviewing an FCP thoroughly. FCP reviewers should also consider reaching
      out to relevant domain experts and soliciting their opinions whenever
      possible. Any project member can raise concerns with an FCP, which will
      be considered by the FCP reviewers.
      
      To function effectively, it is recommended that there be 4 - 8 FCP
      reviewers at any time, so that there is sufficient diversity of
      perspective. This is not a strict upper bound, as long as FCP reviewers
      are prompt in their reviews and the process isn't unnecessary delayed due
      to the number of reviewers. If less than 4 FCP reviewers are available,
      the compiler team co-leads will act as FCP reviewers until the reviewers
      can be found - this lower bound is necessary to ensure that FCPs are
      reviewed thoroughly.

      FCP reviewers are expected to be able to review FCPs promptly (within
      a couple of weeks) - this could be checking their box, registering a
      concern or just commenting to say they're still working on their review.
      FCP reviewers who consistently aren't able to review FCPs promptly may
      be removed from the FCP reviewer activity - given that the purpose
      of the FCP activity is to ensure that FCPs are thoroughly reviewed
      by those engaged in doing so and that the team's work isn't unnecessarily
      delayed, FCP reviewers who aren't doing this defeat the point of the
      activity existing rather than being something all maintainers do.
      Any reviewer removed can ask to be re-added when they have the bandwidth
      to participate in FCP reviews.

      An FCP can include more of the team (all maintainers or all team members,
      for example) if it makes sense to do so, such as FCPs for changes to the
      team's structure.

- Performance triage
    - There is a rotation of maintainers and other project members who check
      all of the interesting performance benchmarks from the last week to produce a
      report summarizing the improvements and regressions. This is valuable to keep
      track of the compiler's performance over time and make sure that regressions are
      being addressed.

- Issue prioritisation
    - The compiler team has a prioritisation procedure and policy to identify
      and label issues according to their importance. These labels feed into the
      backport procedure (what's worth being backported) and work priorities of
      maintainers.

- Backport reviews
    - On a regular basis, some maintainers participate in a review of pull
      requests which have been nominated for backporting to the beta or stable
      release. This involves a judgement call on the risk of backporting a
      particular fix versus the severity of the issue being addressed.

      Once those maintainers interested in backport reviews are identified,
      this function could be performed in a separate meeting or asynchronously,
      allowing the triage meeting to be streamlined and focused on nominated issues or
      other tasks requiring broader discussion.

      To establish a reasonable quorum of triage members, it is recommended that
      at least 4 members participate in triage meetings. In the event there are not
      enough triage members, the compiler team co-leads will act as triage members
      until additional members are found.

- Review rotation
    - Every week, lots of pull requests are submitted to the compiler which need
      to be reviewed. Being on the review rotation is one of the primary ways that
      maintainers can help keep the wheels turning in the compiler team.

      It is **strongly** encouraged that all maintainers be a part of the
      review rotation.

- Operations
    - There are various *operations* tasks like agenda preparation and taking
      meeting notes which are very useful for the team.

- Tool development
    - There are various tools that the compiler team uses in support of its work, such
      as the performance tracking infrastructure, agenda generation tooling, etc.
      These tools are vital to the ongoing functioning of the team and their continued
      development is useful to the team.

- RFC/MCP participation
    - Participation and review of RFCs and MCPs is important to ensure that
      these proposed changes/features are thoroughly considered.

- Mentoring/working group leadership
    - Mentoring new and experienced contributors in changes is important to help onboard
      team members, retain contributors, and implement new features - keeping our work
      sustainable. 

This list isn't exhaustive, and this RFC shouldn't be considered the canonical
list of these activities. Similarly, this RFC isn't intended to define how
these activities are conducted (in meetings or asynchronously, etc), that
should be decided and documented by those involved in each.

However activity participation is tracked, it should be easy for the team
leads to have visibility into the participation in each to ensure that it is
sustainable.

## Team Leads
[team-leads]: #team-leads

Team leads are defined in [RFC 3262][rfc3262] and is unchanged by this RFC. It is
not required but anticipated that those elected for team leads would be or have been
maintainers.

[rfc3262]: https://rust-lang.github.io/rfcs/3262-compiler-team-rolling-leads.html

# Drawbacks
[drawbacks]: #drawbacks

- Granting permissions earlier may be a risk
    - We haven't had any issues with contributors having staying power to the
      extent that we would trust them with permissions and then having those be used
      inappropriately. We can always revert changes if necessary.
- Expectations of team members
    - This RFC formally establishes expectations which come with team
      membership. Some team members already assume that these expectations are there,
      but this wasn't made explicit when current team members were made team members.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Get better at doing nominations
    - A lot of this proposal's simplification of the way that promotions are
      granted is based on the premise that our current system doesn't work well for us
      - but we could just try and do the current system better.
- Only change review responsibilities
    - We could instead try to increase the number of reviewers on the review
      queue by just amending the current compiler team membership policy to include
      review rotation duty. This does not improve our ability to correctly promote
      contributors or recognize the ways individuals contribute to the maintenance
      of the compiler but could be reasonable if implementing all of the changes
      described here will take too long.

# Prior art
[prior-art]: #prior-art

- [Maintenance activities][maintenance-activities] are similar to [an
  unsubmitted proposal by Niko Matsakis in December 2020 to have "elected
  officers"][officers] within the compiler team responsible for different team
  functions. This RFC shares many of the goals of Niko's earlier proposal,
  but is slightly less formal - activities are loosely-defined groups of
  contributors rather than elected positions, and there are no rotations or term
  limits.

  In this RFC's proposal, it is expected that activites are shared amongst a
  group of team members, and that team members do less of other activities
  so that their workload is sustainable, but this isn't enforced. Team leads are
  instead responsible for ensuring that the team is large enough to perform each
  activity sustainably.

[officers]: https://hackmd.io/S9xqmwJbSI-a9mFdK9yQBA

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None!

# Future possibilities
[future-possibilities]: #future-possibilities

- Maintenance activities could be formalized further - see references in
  [Prior art][prior-art].
- Minimum requirements of maintainers could be elaborated further.
