- Feature Name: N/A
- Start Date: 2024-01-19
- RFC PR: [rust-lang/rfcs#3599](https://github.com/rust-lang/rfcs/pull/3599)
- Rust Issue: N/A

Summary
=======
[summary]: #summary

Re-organise the compiler team:

- Re-define and rename the tiers of membership
- Change how team members and contributors are promoted
- Document expectations of team members
- Establish mechanism for scaling additional responsibilities that team members
  take on and recognising these contributions

Motivation
==========
[motivation]: #motivation

Compiler team contributors were introduced [in 2019 with RFC 2689][rfc2689],
the last significant change to the compiler team's structure. A lot has changed
in the project and compiler team since that time: we receive [approximiately
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
should have those additional responsibilities recognised.

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

Definitions
-----------
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
    *crater*.
- **review queue**
  - Contributors on the review queue will be randomly assigned to new pull
    requests submitted to the compiler. Being on the review queue is one of the
    best ways for contributors to help the compiler team and learn new parts of
    the compiler. Review capacity is one of the most important resources that the
    team has, as it enables our progress in the compiler's continued development
    and maintenance.
- **organization membership**
  - Contributors that are added to the *rust-lang/compiler* team in the GitHub
    organisation can be assigned to issues/pull requests, modify labels, receive
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

[expectations]: https://forge.rust-lang.org/compiler/reviews.html#expectations-for-r

Guide-level explanation
=======================
[guide-level-explanation]: #guide-level-explanation

Contributors start without any particular privileges, permissions or
responsibilities and can contribute whatever they'd like. Contributors can
progress to [Trusted Contributors][trusted-contributors] and then [Team Members]
[team-members].

Trusted Contributors
--------------------
[trusted-contributors]: #trusted-contributors

Being able to grant permissions to trusted contributors quickly is beneficial to
enable them to contribute to the project more efficiently and review and approve
work of their collaborators.

Any contributor can request to become a trusted contributor by contacting the
compiler team's leads, or current team members and trusted contributors can
nominate a contributor. Team leads will check for a reasonable contribution
history, and will check if the current team have any serious concerns related to
contributor conduct (waiting approximately one week).

When evaluating a candidate's contribution history, length of time and
consistency of contributions and interactions with other contributors and team
members will be taken into account. It is important to note that many kinds
of contributions will be considered such as code contributions, helping with
issue triage and bisection, running meetings and creating minutes, documentation
contributions for rustc internals or the [Compiler Development Guide], etc.

Trusted contributor is a mix of RFC 2689's "working group participant" and
"compiler team contributor" roles. It is explicitly intended to be granted more
liberally to contributors who have demonstrated competence and trustworthiness,
for whom they would be able to work more effectively with these permissions and
can be trusted to use them responsibily. Trusted contributors do not need to
have experience with most of the compiler, and can be specialised to specific
subsystems of the compiler.

Trusted contributors are granted *r+*, *try*, *triagebot*, *rustc-perf*, and
*crater* permissions; *organisation membership*; and *dev desktop* access.
Trusted contributors are considered members of the Rust project as a whole, and
are automatically eligible for any benefits that incurs (e.g. invitations to
meetups of project members). As representatives of the Rust project, trusted
contributors are expected to obey not just the letter of the [Code of Conduct]
[coc] but its spirit.

Trusted contributors can choose to take on additional responsibilities, such as
those listed in the [responsibilities][responsibilities] section. Participating
in the team's review queue is encouraged.

If a trusted contributor becomes inactive (the contributor's prior contributions
and other interactions with the project cease) for longer than a year, the
trusted contributor will be moved into alumni status. At any point in the
future, they can ask to be re-instated at the trusted contributor level if they
desire.

[Compiler Development Guide]: https://rustc-dev-guide.rust-lang.org/

Team Members
------------
[team-members]: #team-members

Trusted contributors are eligible to become team members after they have
continued to contribute actively for a year. Trusted contributors can contact
team leads or will be contacted by team leads to enquire about promotion to team
membership. Trusted contributors who are eligible for team membership do not
have to become team members.

Unlike trusted contributors, team members are expected to consider themselves
as *maintainers* of the compiler - put otherwise, to be invested in the quality
of the compiler codebase and overall health of the compiler team, independent
of their own projects. Team membership is primarily intended to recognise and
encourage participation in activities which are vital to the success of the
compiler team and broader project.

Team members are expected to participate in the ongoing maintainance tasks
that the compiler team is responsible for (with all of the expected caveats
for vacation time, mental health breaks, etc) - listed as [responsibilities]
[responsibilities] below. However, not all contributors need to participate
in these responsibilities to an equal degree. Contributors should participate
in these tasks to the degree that they are able - volunteers are not expected
to participate as much as those employed to work on the compiler, for example.
It is the responsibility of the compiler team leads to ensure that the ongoing
maintenance tasks of the team can be completed sustainably.

Team members have all of the same permissions and access as trusted
contributors. Like trusted contributors, team members are considered members of
the Rust project as a whole and are expected to follow the spirit of the [Code
of Conduct][coc].

Like trusted contributors, after inactivity for longer than a year, a
contributor will be moved to alumni status. Members who are no longer able to
help maintain the compiler but otherwise wish to continue contributing to the
compiler can also be moved to alumni status and retain their trusted contributor
status. Alumni can ask to be reinstated in future.

[coc]: https://www.rust-lang.org/policies/code-of-conduct

Responsibilities
----------------
[responsibilities]: #responsibilities

There are various responsibilities that a team member could take on to help the
team. All team members must participate in at least one activity.

Team members can get involved in any of these by contacting the team leads, by
asking team members currently involved in these responsibilities, or by asking
in any venue where these responsibilities are conducted (e.g. a Zulip stream).

- Final comment period (FCP) reviewer
    - Final comment periods are the process by which the team signs-off on a
      change before it is made, like stabilizing a feature.

      FCPs have always required whole team to sign-off, but this doesn't scale
      as the team grows. As described above, this acts as a disincentive for the
      team to grow. Furthermore, not all FCPs are relevant to all team members and a
      diffusion of responsibility means that most team members just sanity-check and
      then sign-off. This isn't ideal, as it doesn't guarantee that someone on the
      team has thoroughly considered a FCP.

      Instead, have FCPs require sign-off from team members who opt-in to being
      an "FCP reviewer", with the expectation that they will spend time reviewing
      an FCP thoroughly. FCP reviewers should also consider reaching out to relevant
      domain experts and soliciting their opinions whenever possible. Any team or
      project member can raise concerns with an FCP, which will be considered by the
      FCP reviewers.
      
      To function effectively, it is recommended that there be 4 - 8 FCP
      reviewers at any time. If less than 4 FCP reviewers are available, the compiler
      team co-leads will act as FCP reviewers until the reviewers can be found.

- Performance triage
    - There is a rotation of team members and other project members who check
      all of the interesting performance benchmarks from the last week to produce a
      report summarizing the improvements and regressions. This is valuable to keep
      track of the compiler's performance over time and make sure that regressions are
      being addressed.

- Issue prioritisation
    - The compiler team has a prioritisation procedure and policy to identify
      and label issues according to their importance. These labels feed into the
      backport procedure (what's worth being backported) and work priorities of team
      members.

- Backport reviews
    - Each week, some team members participate in the weekly triage meeting
      to review pull requests which have been nominated for backporting to the beta
      or stable release. This involves a judgement call on the risk of backporting a
      particular fix versus the severity of the issue being addressed.

      Once those team members interested in backport reviews are identified,
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
      team members can help keep the wheels turning in the compiler team.

      It is **strongly** encouraged that all team members be a part of the
      review rotation.


- Operations
    - There are various *operations* tasks like agenda preparation and taking
      meeting notes which are very useful for the team.

      This list isn't exhaustive, and this RFC shouldn't be considered the canonical
      list of these responsibilities. Similarly, this RFC isn't intended to define how
      these responsibilities are conducted (in meetings or asynchronously, etc), that
      should be decided and documented by those involved in each.

While this RFC doesn't aim to be authorative with respect to how team members
who take on additional responsibilities are recognised, one way would be for
team members who take on additional responsibilities to record this in the
*rust-lang/team* metadata (using the `roles` key), e.g.

```toml
[people]
members = [
    { github = "davidtwco", roles = ["compiler-backport"] }
]

[[roles]]
id = "compiler-backport"
description = "Backport Reviewer"
```

By tracking responsibility participation in the team repository, it is easier
for the team leads to have visibility into the participation in each to ensure
that it is sustainable. Participants will be recognised for the additional
responsibilities they participate in on the compiler team's page on the Rust
website.

Drawbacks
=========
[drawbacks]: #drawbacks

- Granting permissions earlier may be a risk
    - We haven't had any issues with contributors having staying power to the
      extent that we would trust them with permissions and then having those be used
      inappropriately. We can always revert changes if neccessary.
- Expectations of team members
    - This RFC formally establishes expectations which come with team
      membership. Some team members already assume that these expectations are there,
      but this wasn't made explicit when current team members were made team members.

Rationale and alternatives
==========================
[rationale-and-alternatives]: #rationale-and-alternatives

- Get better at doing nominations
    - A lot of this proposal's simplification of the way that promotions are
      granted is based on the premise that our current system doesn't work well for us
      - but we could just try and do the current system better.
- Only change review responsibilites
    - We could instead try to increase the number of reviewers on the review
      queue by just amending the current compiler team membership policy to include
      review queue duty. This does not improve our ability to correctly promote
      contributors or recognize the ways individuals contribute to the maintenance
      of the compiler but could be reasonable if implementing all of the changes
      described here will take too long.

Prior art
=========
[prior-art]: #prior-art

- [Responsibilities][responsibilities] are similar to [an unsubmitted proposal
  by Niko Matsakis in December 2020 to have "elected officers"][officers] within
  the compiler team responsible for different team functions. This RFC shares
  many of the goals of Niko's earlier proposal, but is slightly less formal -
  responsibilities are loosely-defined groups of contributors rather than elected
  positions, and there is no rotations or term limits.

  In this RFC's proposal, it is expected that responsibilities are shared
  amongst a group of team members, and that team members do less of other
  responsibilities so that their workload is sustainable, but this isn't enforced.
  Team leads are instead responsible for ensuring that the team is large enough to
  perform each responsibility sustainably.

[officers]: https://hackmd.io/S9xqmwJbSI-a9mFdK9yQBA

Unresolved questions
====================
[unresolved-questions]: #unresolved-questions

None!

Future possibilities
====================
[future-possibilities]: #future-possibilities

Responsibilities could be formalized further - see references in [Prior art]
[prior-art].
