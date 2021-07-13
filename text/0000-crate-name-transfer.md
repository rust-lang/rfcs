- Feature Name: crate_name_transfer
- Start Date: 2018-12-15
- RFC PR: [rust-lang/rfcs#2614](https://github.com/rust-lang/rfcs/pull/2614)
- Rust Issue:

# Summary

This experimental RFC proposes a process by which a designated team could
reassign ownership of a crate name on crates.io. The aim is to identify the
absolute smallest, most conservative step we can feasibly make from the status
quo of crate names that can only be transferred by a current owner. The proposal
is intended to address *only the absolute most clear-cut situations* in which a
crate name would need to be transferred. It is not intended to address the
situation of name squatting on crates.io, nor as a gateway to eventual more
aggressive forced exchange of crate names.

# Motivation

It is my humble belief that there exists such a thing as a clear-cut situation
in which transferring a crate name from one person to another makes sense.

The following situation describes a real crate (all of the following describe
the same crate):

- Crate was last updated 5 years ago,
- Not a single version of the crate compiles on any Rust compiler 1.0.0 or
  newer,
- In the most recent version of the crate, the author has indicated that the
  entire crate is deprecated,
- Zero other crates depend on the crate,
- Crate has a desirable name that a different user is eager to use for a library
  that is all ready to be published,
- Reasonable efforts to contact the author are unsuccessful.

This eRFC introduces an opportunity for a small team to be entrusted in
volunteering their time and their reputation to improve the experience of using
crates.io by arbitrating similarly clear-cut cases.

# Experiment

This RFC proposes evaluating the process outlined below as an experiment that
can be abandoned at any time with or without cause.

Any Rust team (core team, or subteam with representation on the core team) is
welcome to call off the experiment at any time if they perceive undesired
fallout from the experiment, or for any reason whatsoever. Such a decision would
be arrived at by that team's ordinary decision-making / consensus-reaching
process. The safeword is as follows:

> The \[your-team-name\] team believes it would be better to discontinue this
> experiment and would like to stop fulfilling crate transfer requests effective
> immediately.

No justification needs to be given; the activities described in this RFC will
terminate without further discussion.

This termination condition is intended as a lightweight way to avert or contain
unforeseen impact of adopting this experimental RFC.

The experiment will continue for 18 months unless before that point it is
terminated as described here or amended by another RFC. Beyond 18 months
everything continues the same except that teams other than the crates.io team
lose the ability to terminate the experiment; they would need to convince the
crates.io team to do so.

# Reference-level explanation

The crates.io team will designate a team of 3-5 people known collectively as the
Responsible Party. Crate transfers happen by group decision of the Responsible
Party.

Membership is at the discretion of the crates.io team with no other
restrictions. In particular, members need not already be part of another Rust
team. It is reasonable to expect that the crates.io team will select reputable
individuals.

The Responsible Party is expected to follow the consensus protocol that the Rust
project largely follows: in order to transfer a crate, a majority of the team
should approve and there should be zero objections. This gives flexibility if
some team members are not available for all decisions. The team should seek to
all enthusiastically approve but it is understood that this is not always
feasible.

The Responsible Party will be considered a subteam of the crates.io team. One
member should be available to attend crates.io team meetings to discuss the
subteam's activity. Not necessarily the same member every time, and not
necessarily every week; we can pin this down once things are up and running and
we get a sense of the scale of incoming requests. Around the 6 month mark if the
experiment has made it that far, the crates.io team should meet with the full
Responsible Party to check in and dig into how our processes can be improved.

To reiterate: crate transfers are to happen purely by group decision of the
Responsible Party. They are not bound by any checklist of criteria specified by
this eRFC. The Responsible Party are entrusted with upholding the intent of the
eRFC in performing transfers in only the most clear-cut cases according to their
judgement.

When a user requests ownership of a crate, the two common responses by the
Responsible Party will be:

1. All right, we have yanked the existing releases and set you as the owner!
   Please ensure that the next version you publish is semver-incompatible with
   any of the old yanked releases.

2. Hi! Thanks for the request. We believe this case is less clear-cut than the
   ones we are currently willing to grant. We would recommend picking a
   different crate name for now, but we can leave this request open if you want
   and revisit the decision in the future.

The Responsible Party should feel comfortable reaching these decisions
privately, but are to maintain some sort of public log of how many requests were
made, how many were rejected, and some brief discussion of the rationale of the
accepted requests (and they must be brief, because it's supposed to be clear
cut). The general principle is to strive to be as transparent as possible
without interfering with the team's ability to get work done.

When accepting a request, the Responsible Party will reach out to an operations
person on crates.io who will perform the transfer. There is a possibility that
this could be automated later but this is not required for the eRFC to go
forward; some design and implementation and policy work would be involved.

# Guide-level explanation

The details of requesting ownership of a crate name are to be figured out over
time, but a viable way to begin would be as follows.

- Set up a repository under https://github.com/rust-lang called
  "crate-transfers".

- Write a readme outlining the process. Ensure that the readme conveys that only
  the most clear-cut requests will be granted, so not to expect too much.

- Requests are made by filing a GitHub issue.

- Include a list of "up for grabs" crate names in the readme. Authors that no
  longer wish to use a crate name that they own can add it to the list by filing
  an issue or sending a pull request. A crate that is "up for grabs" trivially
  meets the "clear-cut situation" requirement if ownership of that name is ever
  requested.

- Decision to terminate the experiment may be delivered by filing an issue
  containing the safeword message above. A pre-prepared note will be added to
  the readme and the repository will be [archived].

[archived]: https://help.github.com/en/github/creating-cloning-and-archiving-repositories/about-archiving-repositories

# Blockers

Some design and implementation work needs to happen before the eRFC could go
into effect.

- Implement the appropriate [audit trail in crates.io][audit] to provide ground
  truth if a dispute should arise.

- Implement a restriction against publishing of semver compatible versions of a
  transferred crate.

- Add a banner on the package page indicating when a crate has been recently
  transferred.

[audit]: https://github.com/rust-lang/crates.io/issues/1548

# Drawbacks

### What if people get mad at the Responsible Party?

People are welcome to get mad at any Rust team member at any time, although this
is not encouraged. The Responsible Party needs to be willing to stake their
reputation on their ability to make sound decisions, and must be willing to take
the fall if things go bad. Some experiments fail.

Checks and balances exist, particularly as any Rust team may terminate the
experiment with or without cause.

### What recourse is available if somebody is unhappy with a decision?

The usual:

- Complain on Twitter.
- Blog about it.
- Strategically upvote and downvote on Reddit.
- Email the mods.
- File lawsuit.
- ...

None of this is different from any other team decision that may make a person
unhappy -- the libs team breaking your code, or the moderation team deleting
your flame war for example.

### This RFC does not go far enough.

This eRFC is not intended as a solution to the name squatting situation on
crates.io. This eRFC does not denounce the behavior of registering crate names
without the intention of using them.

By taking this step, it may be perceived as a decision *against* ever addressing
the name squatting situation more decisively. That is not the intention of this
eRFC.

It may be perceived as a decision *against* adopting a namespacing scheme or
other mechanism of shrinking the occurrence of package name contention. Likewise
not the intention of this eRFC. I would encourage anyone believing strongly in
namespacing to pursue it in a different RFC. Namespacing touches on an
overlapping set of concerns but is orthogonal to this RFC in that we may opt to
do neither, one or the other, or both.

By design, this eRFC proposes making the smallest and most conservative possible
step from the status quo.

### The volume of requests may be overwhelming.

If the volume of requests is overwhelming, that would be sufficient reason for
a team or the Responsible Party to terminate the experiment.

Rather than "let's not try this experiment because there is a possibility there
will be too many requests," I would recommend that you please see it the other
way around: "let's try this experiment because there is a possibility that there
will *not* be too many requests."

If there are overwhelmingly many requests, this RFC puts us under no obligation
to hire some kind of large paid support staff to work through them. A
lightweight termination condition returns us to the status quo, after which we
can revisit in another RFC better informed by our newfound knowledge.

### Why would anyone volunteer to be a Responsible Party?

At least three existing Rust team members and at least two reputable community
members have already expressed willingness. (Disclosure: one of them is the RFC
author.) I believe this is sufficient to move forward with the RFC.

If in the future it becomes impossible to find a willing qualified candidate,
this would be sufficient reason for a team to terminate the experiment, which
immediately returns us to the status quo. Note that a gap in coverage does not
automatically terminate the experiment. Termination would be a team decision
based on the negatives of having the process continue to exist without a team
behind it, as well as the perceived likelihood of qualified candidates turning
up in time.

# Rationale and alternatives

### Alternative: provide a checklist of criteria

"The crate must be X years old, must not have crates depending on it, must be
below version 1.0, must ..."

I believe it is better to leave the decision entirely up to judgement of the
Responsible Party with no checklist. Similarly, the lang team does not have a
checklist that determines what language changes are made ("must use no more than
two new keywords, ...").

Under this RFC, crate transfers happen at the sole discretion of the Responsible
Party. Purely as a place to maintain record of the bikeshed on possible
criteria, here is a non-exhaustive list of factors that one might expect could
influence the decision:

- How many years since last release was published.
- How long since owner has engaged in any public way with the repository.
- Whether there exists any stable or nightly Rust compiler 1.0.0 or newer that
  can successfully compile the crate.
- Statement by the owner that they no longer want the crate name.
- Announcement by the owner that the crate is abandoned.
- Code that couldn't possibly be useful to anyone for any purpose.
- Reasonable efforts to contact the owner have been unsuccessful.
- Number of dependent crates.
- Prominence of dependent crates.
- Reputation of requester.
- Requester has code ready to be published.
- Requester is the chronologically first person to have requested the name.

It is not necessary to debate this list because this list has no bearing on the
proposal of this RFC, which is that crate transfers happen at the sole
discretion of the Responsible Party. Let's debate the list only after deciding
that there should be a list.

### Alternative: membership fewer than 3 people

This eRFC specifies membership of 3-5 people for the Responsible Party. We are
hesitant to begin the experiment with fewer than 3 volunteers in order to:

- Avoid conflict of interest when a member of the Responsible Party theirself
  wants to request a crate;
- Reduce perception that a request has been denied because of personal grievance
  with the requester, or other social factor;
- Reduce perception that a request has been granted because of personal
  acquaintance with the requester, or other social factor;
- Achieve more continuous coverage when a person needs time off;
- Reduce the possibility of simply overlooking some important factor;
- Ease some pressure from a high-pressure role.

In the event that we find ourselves without at least 3 qualified volunteers in
the future, the crates.io team may terminate the experiment, or may choose to
continue provisionally with a team of 2 since we expect that a 3-member team
would have quorum of 2 anyway.

### Alternative: membership greater than 5 people

For the sake of consensus seeking, scheduling, and consistency, we are hesitant
to begin this experiment with a team larger than 5 people.

The crates.io team may opt to raise the size of the Responsible Party beyond 5
people later at their discretion.

### Alternative: allow publishing semver compatible updates

The eRFC proposes implementing a restriction against publishing of semver
compatible versions of a transferred crate. All versions published by the new
owner must be semver incompatible with version published by the old owner(s).

The use case of transferring actively depended upon packages to a new owner for
continued maintenance largely falls outside the scope of "clear cut" under this
eRFC.

Relaxing this proposed restriction would need to be proposed in a separate RFC.

### Alternative: transfers by consent of the owner only

The crates.io team currently is happy to reach out to a crate owner on someone's
behalf, and if they receive consent to transfer, they will do so (but this is
not widely known about).

As an alternative to this eRFC we could put effort toward streamlining this
consensual transfer process. Perhaps crate owners who no longer want a name
could mark it as up for grabs, which makes it automatically available to any new
owner without another human in the loop.

We should do both! Streamlining these types of transfers could potentially
significantly bring down the number of crate transfer requests requiring human
evaluation. Beginning the experiment would be a good way to gauge how much
effort it's worth putting toward streamlining the consensual transfer process.

### Alternative: do nothing

The ol' do-nothing alternative. Crate names, and maybe even good crate names,
might not be a scarce enough resource yet.

See the last paragraph under [Prior art: npm](#npm) for my view on this.

# Prior art

### npm

https://www.npmjs.com/policies/disputes

- Name squatting is against the Terms of Use. Squatted packages (as assessed by
  a human) are eligible for immediate transfer to another user who asks for it.

- Outside of squatting, users wanting to request transfer of a package name are
  required to contact the existing author with cc to support@npmjs.com.

- In case of no amicable resolution within 4 weeks, support staff arbitrate the
  dispute entirely at their discretion and judgement.

Like in this RFC, resolution is ultimately at the discretion of staff and not
governed (at least not publicly) by a criteria checklist.

We know from @ashleygwilliams (source: she worked at npm for 3 years) that npm
has a large paid support staff that spends 80+% of its time handling package
disputes. Manually handling package disputes is a huge drain on their time. Some
may hold this as a reason not to pursue this RFC. I see it as the opposite. If
"do nothing" were a viable plan, npm would surely have thought of it and be
saving their money instead. The service provided by their dispute resolution is
so valuable to the health of their ecosystem that it is worth paying so much
for.

### PyPI

https://www.python.org/dev/peps/pep-0541/

PEP 541 "Package Index Name Retention" was [accepted] in March 2018 but a
precise workflow is still being developed. Implementation is tracked in
[pypa/warehouse#1506].

Under this policy, name squatting is disallowed and squatted packages are
removed on sight.

Outside of name squatting, transfer requests are only granted for abandoned
packages. The staff does not participate in disputes around active packages. A
project is considered abandoned when all of the following are met:

- owner not reachable across 3 attempts over 6 weeks
- no releases within the past 12 months
- no activity from the owner on the project's home page

If a candidate appears willing to continue maintenance on an abandoned project,
ownership of the name is transferred when all of the following are met:

- candidate demonstrates failed attempts to contact the existing owner
- candidate demonstrates improvements on their own fork of the project
- candidate justifies why a fork under a different name is not an acceptable
  workaround
- the maintainers of the Package Index have no additional reservations

An abandoned project can be transferred to a new owner for purposes of reusing
the name when all of the following are met:

- candidate demonstrates failed attempts to contact the existing owner
- candidate demonstrates that the project suggested to reuse the name already
  exists and meets notability requirements
- candidate justifies why selecting a different name is not an acceptable
  workaround
- download statistics indicate that the existing package is not being used
- the maintainers of the Package Index have no additional reservations

[accepted]: https://mail.python.org/pipermail/distutils-sig/2018-March/032089.html
[pypa/warehouse#1506]: https://github.com/pypa/warehouse/issues/1506

### Other

I am interested to hear experiences from any other package ecosystems!

# Unresolved questions

<!-- Let me know! -->

### Legality

Legal counsel has not yet been consulted about this policy.

Our vague understanding is that publishing a crate can constitute a trademark on
that name, but the Rust team does not have enough legal expertise to be sure
which crate authors have a reasonable claim to a trademark on which names, nor
whether reassigning a name could violate a trademark and expose the new owner or
the Rust team to legal risk.

It should be fine to grant only consensual transfers for now until we get
feedback from legal counsel. That is, transfers in which the owner has
previously made it known that they do not want the name.

If anyone has contacts within npm or PyPI or another package team that transfers
names, we would love to hear from them on this aspect.

### Role within crates.io team

As a subteam of the crates.io team, do members of the Responsible Party
participate in decisions that are the responsibility of the crates.io team?

Is it better to increase cohesion with the crates.io team by sharing in these
responsibilities, or to minimize workload for the Responsible Party by not
sharing in them?

# Future possibilities

This eRFC would like to avoid, as much as possible, any association with a name
squatting policy. Such a policy should be a largely orthogonal RFC.

As much as this is not the objective of this eRFC, the way this experiment plays
out will likely inform any future policy on squatting.
