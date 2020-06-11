- Feature Name: N/A
- Start Date: 2020-05-27
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

* Introduce a new step, a **major change proposal** (MCP), that is taken before an RFC is created. 
    * **Major change proposals** are created by opening an issue on the lang-team repository. The issue includes a short summary of an idea with a focus on motivation and a de-emphasis on the details of the solution.
    * A Zulip topic is also created to discuss the proposal.
* MCPs can be resolve in a few ways:
    * If the proposal is small and approved, then a PR against rust-lang/rust can be created and FCP'd.
    * If the proposal is larger, and there is a willing lang-team liaison, then a project group can be chartered to draft an RFC.
    * Finally, the proposal may be closed, along with an explanation as to why. This may simply be that there is no available bandwidth to move the proposal forward right now.
* To help in tracking what the lang-team is up to, as well as the set of bandwidth available for each member, we will track (and publicly communicate)
    * Active project groups, grouped by liasion
    * Topics of interest that each member may wish to pursue
    * A [design notes] section that can be used to log exploration or discussions that yielded interesting results but didn't ultimately lead to a project group at this time.

[design notes]: https://lang-team.rust-lang.org/design_notes.html
[project group]: https://github.com/rust-lang/rfcs/pull/2856

# Motivation
[motivation]: #motivation

The RFC system is one of Rust's great strengths, but we have found over time that there are also some shortcomings. The hope is that the "proposal" process will allow us to better focus our energies and create a more collaborative environment. Some of the benefits we foresee are as follows.

## More collaboration throughout the design process

Many people have reported that they have ideas that they would like to pursue, but that they find the RFC process intimidating. For example, they may lack the detailed knowledge of Rust required to complete the drafting of the RFC. 

Under the proposal system, people are able to bring ideas at an early stage and get feedback. If the idea is something we'd like to see happen, then a group of people can gather to push the RFC through to completion. The lang-team liaison can provide suggestions and keep the rest of the lang-team updated, so that if there are concerns, they are raised early.

Furthermore, we hope that project groups will help to shift the focus and way that design iteration happens, so that contentious issues can be discussed and settled in the group. RFC threads will be shorter-lived and used more to gather feedback and points of concern that can then be ironed out within the group and the greater team. (Note that, while a project group is driving the design, the team members are the ones making the ultimate decision.)

## More focus on RFCs that reflect current priorities

The RFC repository currently contains far too many RFCs for anyone to keep up with -- especially members of the lang-team. Some of these RFCs are unlikely to be accepted, either because the ideas are too early stage, or because they don't align well with the project's current goals. Other RFCs may be very solid indeed, but they can be lost in the noise. 

We expect that after this change, the only RFCs that will be open are those that have already seen iteration in a project group, with the aid of a project liaison and general approval of the team. This makes the RFC repository a good place to monitor for ideas that have traction.

Meanwhile, the lang-team proposals will still contain quite a mix of ideas. However, unlike the RFC repository, we do not intend to allow proposals to stay open indefinitely. Proposals that do not receive a liaison or have active discussion will be closed, so that the issue list remains a good indication of ideas that are under active consideration.

## More staging of discussion

Building on the previous point, we believe that this new process will allow us to better separate the different "stages" of the design and implementation process:

* **Very early design discussion** starts on internals or other spaces, as today.
* **Promising ideas** can be formed into a proposal and discussed with the lang-team on Zulip.
* For ideas that are accped, **design iteration and RFC authorship** takes place in the [project group], typically on Zulip or in lang team design meetings.
* Finally, **polished designs** are brought to the RFC repository for feedback.

## Swifter resolution of RFCs, with design iteration taking place in project groups

Because RFCs will be the end result of more discussion, we can expect to resolve them faster. If the discussion uncovers critical flaws or new considerations, the RFC may be withdrawn and edited, and a new RFC posted instead. This can also happen if there is simply a lot of feedback, even if the design doesn't change -- in those cases, the new RFC will be updated to include a summary of the major points as well as responses to them. When working on amended RFCs, we can focus the discussion on the outstanding questions and avoid rehashing older debates.

This should help with the problem that longer RFCs threads can be quite hard to follow. This is compounded by the fact that the RFC evolves over the course of the discussion, sometimes changing dramatically. Getting "up to speed" on the state of an RFC thread involves reading a mix of stale comments on older drafts that have long since been addressed, thoughts on motivation, details of the design, and other things, and trying to sort them into an order.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Hello! If you would like propose a change to the Rust language, you've come to the right place. The process can be summarized as follows (but check out the [flowchart] for a graphical version):

* Open a **major change proposal** (MCP) issue on the lang-team repository
    * The issue should summarize the motivation and sketch out some details of your idea.
    * You don't have to have everything figured out at this stage, and it's more important to emphasize the problems you are solving and why they're important than the nitty gritty details of how to solve them.
    * When you create a proposal, we will also automatically make a Zulip stream for discussion. The issue thread is reserved for summaries, announcements, or procedural updates.
* If the MCP is simple enough, the team may direct you to go straight to implementation.
    * In that case, you can create a PR against the rust-lang/rust repository.
* Otherwise, the proposal will require chartering a project group and drafting an RFC. This step requires a **lang-team liaison** who is willing and able to help the idea move forward.
    * The first step will be chartering a project group, which the liaison will help you with.
    * The charter is very similar to the MCP itself, and describes the motivation and other considerations that may have come up.
* Of course, many proposals will not be accepted. This can occur for a number of reasons:
    * There are no lang team members who have available bandwidth to serve as a liaison right now.
    * The problem being solved doesn't seem important enough or is not a match for Rust's current priorities.
    * The proposal has flaws in its design that don't seem like they can be solved.
    
## Flowchart

You can also view the [flowchart in graphical form][flowchart]. This
is derived from a [mermaid document][flowchart-source] and would be
posted on the lang-team website.

[flowchart]: https://is.gd/nMrsVE
[flowchart-source]: https://is.gd/LdRVWk

## Reasons to accept or decline a proposal

In the proposal stage, we are looking for proposals that have

* high motivation -- this is a good fit for the project's priorities
* available bandwidth -- there are lang team members who want to work on it as well as (if appropriate) members of other Rust teams

The focus is primarily on the motivation and secondarily on the solution(s) proposed. If we feel that the problem is worth solving, but the solutions have flaws that seem surmountable, it still makes sense to accept the proposal.

Reasons to decline a proposal, or to oppose the creation of a project group, include the following:

* The team has grave doubts about the feasibility of solving this problem.
    * When possible, though, it is better not to block a proposal on technical grounds. Instead, those grounds can be noted as something that must be resolved by the project group during the design process.
* There isn't available bandwidth to tackle the problem right now.
    * In particular, it may require input from particular members who have the requisite expertise, and they may be too busy with other things.
* The proposed solutions disproportionate to the scale of the problem.
    * For example, the solutions may have fundamental flaws, be too invasive, or be too much work to implement.
* The proposal is incomplete or is proposing a direction that seems wrong.
    * The team may ask for the proposal to include more potential solutions, or to be rewritten to emphasize a particular direction that we think would be better. If the author of the proposal disagrees with that direction, they should make the case for the existing solutions, but of course the proposal may not ultimately be accepted if the lang team doesn't find that case persuasive

## When are proposals discussed

We will look at pending proposals during our weekly triage meetings and try to
post updates or questions promptly. Since the intent of a proposal is not to
iterate on a design, but rather to determine if there is an available and
interested lang team liaison, we should be able to move much more quickly in
deciding whether to assign a proposal or not. The intent is to avoid the kind of
limbo that we've seen in the past with pending RFCs.

## Simple MCPs can be implemented directly

In some cases, MCPs are sufficiently simple that we can just move straight to implementation. In this case, someone from the lang-team will leave a command indicating that we'd be happy to see a PR implementing this proposal and close the issue, and you can go ahead and create a PR against the rust-lang/rust repository directly. In that PR, you can cc the lang team and we will `fcp merge` the PR to make the final, official decision.

## Chartering a project group

For more complex MCPs, we will instead charter a [project group]. The goal of the group is to create an RFC. Chartering a project group is a lightweight process and a group doesn't necessarily imply a lot of people, it might just be one or two. But every project group always includs a liaison with the lang team. The job of a liaison is to help move the idea through the process. In some cases, they may actively help with the RFC and discussion in other cases they serve more as a bridge.

[project group]: https://github.com/rust-lang/rfcs/pull/2856

Mechanically, chartering a project group involves creating a PR against the rust-lang/lang-team repository with the charter text. The liaison will initiate an `rfcbot fcp merge` command, so that lang-team members can check their boxes to indicate they agree with chartering the new project group. This also initiates a Final Comment Period that is advertised to the broader community.

Writing a charter is not meant to be a big job. Often, it will suffice to copy over the MCP itself, perhaps with some light edits. However, in cases where there is a lot of discussion, charters should summarize the key points made during that discussion, and hence they can in some rare cases be longer.

## Declining a proposal

Proposals that are not accepted are called "declined". Proposals may be declined for many reasons and it does not mean that the proposal is not a good idea. Before closing issues, we try to leave comments laying out the reasoning behind the decision, but we also close issues in an automated fashion if they have not received comments from lang-team members after a certain period of time (this indicates a general lack of bandwidth to take on the proposal). Like any issue, closing an issue doesn't mean that that the proposal can never be re-opened, particularly if circumstances change in some way that invalidates the reasoning behind closing in the first place.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Template for lang-team MCPs

We will create a template for lang-team proposals that has the following sections:

* Summary
    * a few bullet points to capture the high-level concept
* Motivation and use-cases
    * examples of the problem being solved and why we care
* Links and related work
    * links to internals threads, other languages, etc
* Solution sketches
    * sketch of one or more ways to solve the problem

The intent is that proposals do not have to be exhaustively researched. They should contain enough details to understand the idea at a high level, but they don't have to have all the details worked out yet.

The intent is also that proposals will largely be accepted or declined "as is" -- if major changes are required, it is better to close and open a fresh proposal.

## Automation

We will have a bot that monitors proposals. The bot will take the following automated actions (though the details here are subject to change as we experiment).

* When an issue is opened with the Major Change Proposal template, it will be tagged as a "draft proposal".
    * An automated bot will create a dedicated Zulip topic in the `#t-lang/major changes` stream (same as the compile team process).
* (Optional) After N days without a comment from a lang-team member, an automated bot will indicate that the issue will be closed soon due to inactivity. After N more days, the issue is autoclosed.

## Transparency around bandwidth

As part of this process, the lang-team plans to organize and expose more clearly which project groups each member is currently involved with (likely using a Github project to expose details and other information). Similarly, we plan to communicate more clearly the sorts of proposals we are looking for at any period of time via blog posts or information on the [lang-team website].

[lang-team website]: https://lang-team.rust-lang.org/

## Transitioning to the new system

There are a number of existing RFCs, many of which have received quite a lot of work, that have to be transitioned to the new system. We do not want to just "auto close" all of those RFCs because in many cases they represent a significant amount of effort and "near consensus".

Our plan is to first create the new system and encourage folks to create proposals who wish to. Presuming our experience with the new system is positive, and once we have "ironed out the kinks", we will begin a process to port over the existing RFCs. 

The precise plans will be determined then, but they will likely include:

* A review of the RFCs to try and find those lang-team liaisons where possible.
* Announcing a date when we will complete the migate and posting a notice on existing RFCs. This notice would encourage others to try out the new process. After that date expires, existing RFCs would be closed.
* An alternative might be to "auto-migrate" all existing proposals via automation to create proposals, perhaps spaced out in groups.

# Drawbacks
[drawbacks]: #drawbacks

* There is an additional step to prepare an RFC, which indicates more overhead and procedures to be followed.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* We could have proposals be opened on the RFC repository
    * Why? Central place for all proposals across teams.
    * Why not? The RFC repository is no longer a "high signal" place to watch for "changes that have traction".
* We could try this for all teams at once.
    * Why? Uniform process is good.
    * Why not? Not sure how it will work, and each team has somewhat different needs.
* We could have MCPs be opened as a PR
    * In earlier versions of this RFC, we made the MCP proposas be a PR against the lang-team repo that contained the proposal.
    * The idea here was that it would be useful to have the text of the proposal as something we can merge as "not accepted" in order to have a better record.
    * We ultimately moved to an issue for a few reasons:
        * YAGNI: The tooling and workflow seemed more esoteric. The idea of opening first to a "not accepted" directory and then merging just felt a bit "unexpected" and harder to explain and envision.
        * It is hard to collaborate actively on a PR; a hackmd or other such document is better for this purpose.
        * Capturing the initial version of a proposal *but not the associated discussion thread* is pretty incomplete. There's a decent chance that the proposal didn't go forward because of critiques or concerns that surfaced early on and people could easily overlook that.
        * The current workflow does allow us to capture and explicitly choose to postpone proposals where not needed.
* We could require chartering project groups to be done as an RFC
    * In the past, we used RFCs to charter project groups, but the sense is that this process was a bit more heavy-weight than was really necessary, and that it would be better to reserve RFCs for actual proposals.

# Prior art
[prior-art]: #prior-art

The compiler-team has **[major change proposals]** as well and they operate in a similar way: a lightweight proposal is prepared that must be seconded, and that proposal can either lead to a PR (or series of PRs) or a project group and even RFCs. The major difference is that language team changes are much more likely to wind up as project groups.

[major change proposals]: https://forge.rust-lang.org/compiler/mcp.html

This proposal grew out of a long-running conversation on integrating [staging into our RFC process][staging]. It has echoes of the [TC39 process](https://tc39.es/process-document/). an MCP corresponds roughly to TC39's "Stage 1 proposal", and the TC39 notion of a "champion" corresponds to our term of "liaison".

[staging]: http://smallcultfollowing.com/babysteps/blog/2018/06/20/proposal-for-a-staged-rfc-process/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* None at this time.

# Future possibilities
[future-possibilities]: #future-possibilities

We may wish to add additional steps towards "staged RFCs" as a follow-up to this work or to refine our process around project groups.
