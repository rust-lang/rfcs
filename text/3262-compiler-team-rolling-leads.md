- Feature Name: n/a
- Start Date: 2022-05-09
- RFC PR: [rust-lang/rfcs#3262](https://github.com/rust-lang/rfcs/pull/3262)
- Rust Issue: n/a

# Summary
[summary]: #summary

The Rust Compiler Team has used a co-leadership model since late 2019. This RFC
codifies the expectations the team has for its leads and the time and effort we
expect to be necessary to meet those expectations. It also specifies a
succession plan, via which a team member can rotate through junior and senior
leadership positions.

Note: this RFC is adapted from a longer
[document](https://hackmd.io/2dnAg2SNS5CbRkljqLHaeg?view) by pnkfelix. That
draft was the subject of compiler team steering meeting ([compiler-team#506][])
and has also been circulated amongst project leadership. In other words: No
surprises here.

[compiler-team#506]: https://github.com/rust-lang/compiler-team/issues/506

# Motivation
[motivation]: #motivation

We want to enable rolling leadership, to prevent burnout for the leads
themselves, and to encourage new leaders to step up and push the team towards
new unexpected directions.

To enable such rolling leadership, we need to establish a shared vision for
what our expectations are for our leaders, as well as the vision for what
succession planning looks like.

The expected outcome is that we have healthy team whose leads will expect to
only serve in that role for a limited time (on the order of 2 to 5 years), and
whose members can have opportunities to take on that leadership role themselves.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The Rust Compiler Team uses a rolling co-leadership model of governance, with a
senior lead and a junior lead each serving the team as team representatives and
decision owners, for a total of two to five years.

Roughly every one to two years, a new junior lead is selected, the current
junior promoted to senior, and the current senior returns to being a normal
compiler team member.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The Rust Compiler Team uses a rolling co-leadership model of governance, with a
senior lead and a junior lead each serving the team as team representatives and
decision owners, for a total of two to five years.

## Team Representative

The leads represent the compiler team by speaking with Rust users, individual
contributors, or organizations seeking to support development of Rust; they also
represent the team by engaging with other teams in the Rust project.

A team lead must have a rough understanding (at least) of what tasks fall under
the remit of the compiler team, and which are better suited for another one of
the Rust teams.

A team lead should be aware of what large scale initiatives are happening within
the compiler, so that they can speak in an informed manner about what issues in
the compiler are being addressed, and which issues are not likely to be
addressed in the near term.

A team lead should be aware of what painpoints (technical, social, etc) that the
compiler team is suffering from most. The leads should be prepared to provide
advice on how others can provide support, and the leads should be prepared to
reach out on behalf of the team to identify external stakeholders to the team
who could drive progress forward on resolving issues that face the team.

The leads' representation of the team should manifest itself via structured
communication, such as blog posts on https://blog.rust-lang.org/inside-rust/

## Decision Owner

The leads own decisions on behalf of the compiler team.

Most choices made by the team are consensus-driven by the usual "FCP all-but-two
with no concerns" process.

The leads own making decisions about urgent issues or ones with a specific
deadline. For example, deciding what to do about a critical release-blocking bug
should happen before the release, preferably long before. Likewise,
beta-backport decisions need to be made in time for the backport to happen
before the beta is lifted to stable.

Finally, when adverse events happen, the leads are responsible for reviewing
what decisions or processes led to the event, and taking action to prevent
future occurrences of the same event. Examples of this include the incr-comp bug
that plagued the 1.52.0 release, which led to a 1.52.1 release four days later
(and three steering meetings as follow-up).

## Time Commitment, Expectations, and Competencies

Let us recall that any compiler team member is allowed to:

* drive progress on backlogged work,
* draft steering meeting proposals, and often write the associated steering meeting document to drive the meetings,
* solicit individuals to form working groups to address important problems,
* take on the resolution of unassigned or abandoned P-critical or P-high issues, and
* drive larger initiatives related to the compiler.

None of that is exclusively the domain of team leads, though the team leads are
expected to take part in such activities as time permits.

The compiler team leads need to do the following as well:

* issue “unilateral approval” for decisions (such as beta backports) that are either urgent or are trivial enough to not require team discussion,
* drive the two weekly meetings (Thursday triage, and Friday steering),
* engage in asynchronous zulip conversations amongst Rust leadership,
* author communication on behalf of the team (such as the 1.52.1 blog post and the 2022 ambitions blog post),
* coordinate with each other as co-leads, either in an on-demand manner, or via periodic "sync-up" meetings.

We expect these leadership related duties may consume 8 hours per week, on
average, with high variance. That’s in addition to whatever time one might spend
on actual development work on Rust itself.

Any member of the T-compiler already has the technical competencies necessary to
be a lead for the team. (For example, they need to build the compiler and run
its test suite, bisect the git history, and post pull requests, especially ones
that revert existing changes.)

A person who leads the team *also* needs enough social connection with the
other T-compiler team members to feel comfortable reaching out for one-on-one
communication when necessary.

Thus, the main prerequisites to be a candidate for T-compiler leadership are
"membership in the T-compiler team" and "regularly attends the Thursday and
Friday T-compiler meetings." (A record of leadership on one or more project
groups or working groups is probably a good thing to have as well, but is not a
strict requirement.)

## Term Length and Leader Selection

> *"Choose your leaders with wisdom and forethought.*
> *To be led by a coward is to be controlled by all that the coward fears.*
> *To be led by a fool is to be led by the opportunists who control the fool.*
> *To be led by a thief is to offer up your most precious treasures to be stolen.*
> *To be led by a liar is to ask to be told lies.*
> *To be led by a tyrant is to sell yourself and those you love into slavery."*
> -- Octavia E. Butler

The Rust Compiler Team uses rolling co-leadership model of governance, with a
senior lead and a junior lead. After serving in their positions for one to two
years, the leaders, with input from the team, select a teammate who is not a
current lead, and that teammate becomes the new junior lead. The old junior lead
becomes the new senior lead, and the old senior lead is again a normal compiler
team member.

The specific term length is left variable since the timing for when a shift in
leadership makes sense will depend on context.

In code:

```
enum Level { Senior, Junior }
struct Member { lead: Option<Level>, ... }

fn roll(curr_senior: &mut Member, curr_junior: &mut Member, incoming: &mut Member) {
    assert_eq!(curr_senior.lead, Some(Level::Senior));
    assert_eq!(curr_junior.lead, Some(Level::Junior));
    assert_ne!(curr_senior, incoming);
    assert_ne!(curr_junior, incoming);
    curr_senior.lead = None;
    curr_junior.lead = Some(Level::Senior);
    incoming.lead = Some(Level::Junior);
}
```

### Selection process

When the senior co-lead decides that they are ready to step down, and have
confirmed that the junior co-lead feels ready to take on the senior co-lead
role, then the two tell the T-compiler team privately about the intention to
have a rollover in leadership.

Then the T-compiler team members can nominate their teammates to serve as the
new junior co-lead. We here follow the model of our FCP process: The leads
should provide a ten-day window for nominations to come in, unless they get
confirmation that the set of nominations is complete.

After nomination is completed, the outgoing senior and junior co-leads discuss
the set of nominees, and also, if desired, have short discussions with the
nominees. Then, the senior and junior co-leads select the new co-lead from the
set of nominees. And that’s it! Then current leads just need to publish a blog
post saying that the leadership is scheduled to roll over, who the new junior
lead is, and the date that it takes effect.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

Committing to specific term lengths puts pressure on the leads to identify new
leaders earlier than they might otherwise. Note that if the leads fail to identify 
any suitable candidates, then we will have hit a (hopefully exceptional) situation 
where we will need to ask the current leadership to stay on board for longer than
expected. At that point, the leads' ongoing goals **must** include the proactive 
seeking of the next generation of leaders.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rolling, not Rotation

We specify here a rolling leadership process, where we will see a shift in
leads, but not every team member is required to serve as a leader at any point.
An alternative model is a true *rotating leadership*, where every team member
will eventually become a lead.

Our rationale for this is that we do not think every team member is interested
in becoming a lead. If we adopted a true rotation and forced someone to become a
lead who did not really want the role, then that probably be bad for that
individual, and probably would be bad for the team as well.

## Selection, Not Election

We specify here that the new junior lead is *selected* (from a set of
individuals nominated by the compiler team) by the current leads. An obvious
alternative would be a pure democracy where the electorate (either the compiler
team, or some superset thereof) gets to vote for who the new junior lead will
be.

When it comes to co-leadership, the two leaders need to be able to work
together effectively; we believe they need compatible working styles
and complementary sets of skills. Therefore, we currently are choosing a
system where the
current leaders have final say on who the next junior lead will be, in order to
optimize for healthy intra-leader communications.

## Do Nothing?

If we stick with the status quo, where no protocol is specified at all, that
would not be the end of the world. We can certainly *emulate* any model we want;
the [original doc](https://hackmd.io/2dnAg2SNS5CbRkljqLHaeg?view#A-%E2%80%9CNew%E2%80%9D-Process)
argues that the process described here matches what the team has already informally employed.

However, there is value in setting down formal expectations. It is *healthy* for
us to tell our teammates: We want each of you to have a chance to perform in
this same role, if that appeals to you, and we want it to happen in a time frame
that is within sight, not some far off future.


# Prior art
[prior-art]: #prior-art

Obviously the Rust governance RFC specified aspects of project leadership:
https://rust-lang.github.io/rfcs/1068-rust-governance.html

pnkfelix isn't sure what other Programming Languages or Projects have adopted a
formal structure for rolling or rotating leadership. Many use a
[BDFL](https://en.wikipedia.org/wiki/Benevolent_dictator_for_life) model
instead.

Python did have a formal [abdication][python-xfer] of BDFL from Guido van Rossum,
but it explicitly chose not to establish a successor.

[python-xfer]: https://mail.python.org/pipermail/python-committers/2018-July/005664.html


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?

Are the term-lengths anywhere near appropriate?

Wesley asked this question on an earlier draft of this:
> Given the flexible time commitment, a tenure of this length basically requires
> the lead to be somebody that works on Rust as part of their job. I'm not sure
> we want to limit our pool of candidates to just those people.

# Future possibilities
[future-possibilities]: #future-possibilities

Should other large teams look into adopting this model?

Many small teams do not have sufficiently large membership to justify two
co-leads; can this same system work fine there, and just rely on calibrating new
leaders amongst all the participating members (which, since we're talking about
small teams, would be a relatively small set of people)?
