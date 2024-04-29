- Feature Name: N/A
- Start Date: 2024-04-18
- RFC PR: [rust-lang/rfcs#3614](https://github.com/rust-lang/rfcs/pull/3614)
- Rust Issue: N/A

# Summary
[summary]: #summary

This RFC proposes to run an experimental goal program during the second half of 2024 with nikomatsakis as owner/organizer. This program is a first step towards an ongoing Rust roadmap. The proposed outcomes for 2024 are (1) select an initial slate of goals using an experimental process; (2) track progress over the year; (3) drawing on the lessons from that, prepare a second slate of goals for 2025 H1. This second slate is expected to include a goal for authoring an RFC proposing a permanent process.

# Motivation
[motivation]: #motivation

The Rust project last published an annual roadmap in 2021. Even before that, maintaining and running the roadmap process had proved logistically challenging. And yet there are a number of challenges that the project faces for which having an established roadmap, along with a clarified ownership for particular tasks, would be useful:

* Focusing effort and avoiding burnout:
    * One common contributor to burnout is a sense of lack of agency. People have things they would like to get done, but they feel stymied by debate with no clear resolution; feel it is unclear who is empowered to "make the call"; and feel unclear whether their work is a priority.
    * **Having a defined set of goals, each with clear ownership, will address that uncertainty.**
* Helping direct incoming contribution:
    * Many would-be contributors are interested in helping, but don't know what help is wanted/needed. Many others may wish to know how to join in on a particular project. 
    * **Identifying the goals that are being worked on, along with owners for them, will help both groups get clarity.**
* Helping the Foundation and Project to communicate
    * One challenge for the Rust Foundation has been the lack of clarity around project goals. Programs like fellowships, project grants, etc. have struggled to identify what kind of work would be useful in advancing project direction.
    * **Declaring goals, and especially goals that are desired but lack owners to drive them, can be very helpful here.**
* Helping people to get paid for working on Rust
    * A challenge for people who are looking to work on Rust as part of their job -- whether that be full-time work, part-time work, or contracting -- is that the employer would like to have some confidence that the work will make progress. Too often, people find that they open RFCs or PRs which do not receive review, or which are misaligned with project priorities. A secondary problem is that there can be a perceived conflict-of-interest because people's job performance will be judged on their ability to finish a task, such as stabilizing a language feature, which can lead them to pressure project teams to make progress.
    * **Having the project agree before-hand that it is a priority to make progress in an area and in particular to aim for achieving particular goals by particular dates will align the incentives and make it easier for people to make commitments to would-be employers.**

For more details, see

* [Blog post on nikomatsakis's blog about project goals](https://smallcultfollowing.com/babysteps/blog/2023/11/28/project-goals/)
* [Blog post on nikomatsakis's blog about goal ownership](https://smallcultfollowing.com/babysteps/blog/2024/04/05/ownership-in-rust/)
* [nikomatsakis's slides from the Rust leadership summit](https://github.com/nikomatsakis/team-goals-2024)
* [Zulip topic in #council stream](https://rust-lang.zulipchat.com/#narrow/stream/392734-council/topic/Project.2Fteam.20goals). This proposal was also discussed at the leadership council meeting on 2024-04-12, during which meeting the council recommended opening an RFC.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The plan for 2024

The plan is to do a "dry run" of the process in the remainder of 2024. The 2024 process will be driven by nikomatsakis; one of the outputs will be an RFC that proposes a more permanent process for use going forward. The short version of the plan is that we will

* **ASAP (April):** Have a ~2 month period for selecting the initial slate of goals. Goals will be sourced from Rust teams and the broader community. They will cover the highest priority work to be completed in the second half of 2024.
* **June:** Teams will approve the final slate of goals, making them 'official'.
* **Remainder of the year:** Regular updates on goal progress will be posted
* **October:** Presuming all goes well, the process for 2025 H1 begins. Note that the planning for 2025 H1 and finishing up of goals from 2024 H2 overlap.

## The "shiny future" we are working towards

We wish to get to a point where

* it is clear to onlookers and Rust maintainers alike what the top priorities are for the project and whether progress is being made on those priorities
* for each priority, there is a clear owner who
    * feels empowered to make decisions regarding the final design (subject to approval from the relevant teams)
* teams cooperate with one another to prioritize work that is blocking a project goal
* external groups who would like to sponsor or drive priorities within the Rust project know how to bring proposals and get feedback

More concretely, assuming this goal program is successful, we would like to begin another goal sourcing round in late 2024 (likely Oct 15 - Dec 15). We see this as fitting into a running process where the project evaluates its program and re-establishes goals every six months.

## Design axioms

* **Goals are a contract.** Goals are meant to be a *contract* between the owner and project teams. The owner commits to doing the work. The project commits to supporting that work. 
* **Goals aren't everything, but they are our priorities.** Goals are not meant to cover all the work the project will do. But goals do get prioritized over other work to ensure the project meets its commitments.
* **Goals cover a problem, not a solution.** As much as possible, the goal should describe the problem to be solved, not the precise solution. This also implies that accepting a goal means the project is committing that the **problem** is a priority: we are not committing to accept any particular solution.
* **Nothing good happens without an owner.** Rust endeavors to run an open, participatory process, but ultimately achieving any concrete goal requires someone (or a small set of people) to take ownership of that goal. Owners are entrusted to listen, take broad input, and steer a well-reasoned course in the tradeoffs they make towards implementing the goal. But this power is not unlimited: owners make proposals, but teams are ultimately the ones that decide whether to accept them.
* **To everything, there is a season.** While there will be room for accepting new goals that come up during the year, we primarily want to pick goals during a fixed time period and use the rest of the year to execute.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The long-term vision is to create a sustainable goals process for the project. Per the axiom that **goals cover a problem, not a solution**, this RFC does not propose a specific process. **Rather, the goal is to devise the process.** To help us get going quickly, the intent is that the goal owner will design and drive an experiental process, including (a) selecting a slate of goals that will be confirmed by the teams they affect; (b) monitoring and reporting on progress towards those goals; and (c) developing infrastructure to support that monitoring and lessen the load. Experience from that will be used to shape an RFC that describes the process to use for the future (assuming the experiment is a success). 

## Ownership and other resources

**Owner:** nikomatsakis

* nikomatsakis can commit 20% time (avg of 1 days per week) to pursue this task, which he estimates to be sufficient.

### Support needed from the project

* Project website resources to do things like
    * post blog posts on both Inside Rust and the main Rust blog;
    * create a tracking page (e.g., `https://rust-lang.org/goals`);
    * create repositories, etc.
* For teams opting to participate in this experimental run:
    * they need to meet with the goal committee to review proposed goals, discuss priorities;
    * they need to decide in a timely fashion whether they can commit the proposed resources

## Outputs and milestones

### Outputs

There are three specific outputs from this process:

* A **goal slate for the second half (H2) of 2024**, which will include
    * a set of goals, each with an owner and with approval from their associated teams
    * a high-level write-up of why this particular set of goals was chosen and what impact we expect for Rust
    * plan is to start with a smallish set of goals, though we don't have a precise number in mind
* **Regular reporting** on the progress towards these goals over the course of the year
    * monthly updates on Inside Rust (likely) generated by scraping tracking issues established for each goal
    * larger, hand authored updates on the main Rust blog, one in October and a final retrospective in December
* A **goal slate for the first half (H1) of 2025**, which will include
    * a set of goals, each with an owner and with approval from their associated teams
    * a high-level write-up of why this particular set of goals was chosen and what impact we expect for Rust
    * (probably) a goal to **author an RFC with a finalized process** that we can use going forward

### Milestones

Key milestones along the way (with the most impactful highlighted in bold):

| Date         | 2024 H2 Milestone                                               | 2025 H1 Milestones                                                         |
| ------------ | --------------------------------------------------------------- | -------------------------------------------------------------------------- |
| **Apr 26**   | **Kick off the goal collection process**                        |                                                                            |
| May 24       | Publish draft goal slate, take feedback from teams              |                                                                            |
| June 14      | Approval process for goal slate begins                          |                                                                            |
| **June 28**  | **Publish final goal slate**                                    |                                                                            |
| July         | Publish monthly update on Inside Rust                           |                                                                            |
| August       | Publish monthly update on Inside Rust                           |                                                                            |
| September    | Publish monthly update on Inside Rust                           |                                                                            |
| **Oct 1**    | **Publish intermediate goal progress update on main Rust blog** | **Begin next round of goal process, expected to cover first half of 2025** |
| November | Publish monthly update on Inside Rust                           | Nov 15: Approval process for 2025 H1 goal slate begins                             |
| **December** | **Publish retrospective on 2024 H2**                            | **Announce 2025 H1 goal slate**                                            |

### Process to be followed

The owner plans to author up a proposed process but rough plans are as follows:

* Create a repository rust-lang/project-goals that will be used to track proposed goals.
* Initial blog post and emails soliciting goal proposals, authored using the same format as this goal.
* Owner will consult proposals along with discussions with Rust team members to assemble a draft set of goals
* Owner will publish a draft set of goals from those that were proposed
* Owner will read this set with relevant teams to get feedback and ensure consensus
* Final slate will be approved by each team involved:
    * Likely mechanism is a "check box" from the leads of all teams that represents the team consensus

It is not yet clear how much work it will be to drive this process. If needed, the owner will assemble a "goals committee" to assist in reading over goals, proposing improvements, and generally making progress towards a coherent final slate. This committee is not intended to be a decision making body.

# Drawbacks
[drawbacks]: #drawbacks

None identified.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Frequently asked questions

### Is there a template for project goals?

This RFC does not specify details, so the following should not be considered normative. However, you can see a preview of what the project goal process would look like at the [nikomatsakis/rust-project-goals](https://github.com/nikomatsakis/rust-project-goals) repository; it contains a [goal template](https://nikomatsakis.github.io/rust-project-goals/2024h2/0000-TEMPLATE.html). This RFC is in fact a "repackaged" vesion of [2024's proposed Project Goal #1](https://nikomatsakis.github.io/rust-project-goals/2024h2/0001-Project-goal-slate.html).

### Why is the goal completion date targeting end of year?

In this case, the idea is to run a ~6-month trial, so having goals that are far outside that scope would defeat the purpose. In the future we may want to permit longer goal periods, but in general we want to keep goals narrowly scoped, and 6 months seems ~right. We don't expect 6 months to be enough to complete most projects, but the idea is to mark a milestone that will demonstrate important progress, and then to create a follow-up goal in the next goal season.

### How does the goal completion date interact with the Rust 2024 edition?

Certainly I expect some of the goals to be items that will help us to ship a Rust 2024 edition -- and likely a goal for the edition itself (presuming we don't delay it to Rust 2025).

### Do we really need a "goal slate" and a "goal season"?

Some early drafts of project goals were framing in a purely bottom-up fashion, with teams approving goals on a rolling basis. That approach though has the downside that the project will *always* be in planning mode which will be a continuing time sink and morale drain. Deliberating on goals one at a time also makes it hard to weigh competing goals and decide which should have priority.

There is another downside to the "rolling basis" as well -- it's hard to decide on next steps if you don't know where you are going. Having the concept of a "goal slate" allows us to package up the goals along with longer term framing and vision and make sure that they are a coherent set of items that work well together. Otherwise it can be very easy for one team to be solving half of a problem while other teams neglect the other half.

### Do we really need an owner?

[Nothing good happens without an owner][owners]. The owner plays a few important roles:

[owners]: https://smallcultfollowing.com/babysteps/blog/2024/04/05/ownership-in-rust/

* Publicizing and organizing the process, authoring blog posts on update, and the like.
* Working with individual goal proposals to sharpen them, improve the language, identify milestones.
* Meeting with teams to discuss relative priorities.
* Ensuring a coherent slate of goals.
    * For example, if the cargo team is working to improve build times in CI, but the compiler team is focused on build times on individual laptops, that should be surfaced. It may be that its worth doing both, but there may be an opportunity to do more by focusing our efforts on the same target use cases.

### Isn't the owner basically a BDFL?

Simply put, no. The owner will review the goals and ensure a quality slate, but it is up to the teams to approve that slate and commit to the goals.

### Why the six months horizon?

Per the previous points, it is helpful to have a "season" for goals, but having e.g. an annual process prevents us from reacting to new ideas in a nimble fashion. At the same time, doing quarterly planning, as some companies do, is quite regular overhead. Six months seemed like a nice compromise, and it leaves room for a hefty discussion period of about 2 months, which sems like a good fit for an open-source project.

# Prior art
[prior-art]: #prior-art

## The Rust roadmap process

Proposed in RFC 1728, the Rust roadmap process resembled the project goal process in a number of ways.
The intention was to publish an annual roadmap RFC with regular tracking updates.
Several such roadmaps were published between 2018 and 2021.
The project goal process proposed here can be considered a successor to that roadmap process,
incorporating several lessons:

* The roadmap process did not assign *owners* to goals and therefore did not explicitly account for available resources.
* The roadmap process didn't have an effective mechanism for tracking progress.
* The roadmap process didn't account for the work of authoring the roadmap -- no explicit owner was assigned. The process also lacked mechanisms for sourcing goals from teams. The core team attempted on some occasions to drive such work but without an explicit owner it (in this RFC author's opinion) floundered.

## Corporate planning processes

There are a variety of planning processes around corporate goals. This proposed goal process (6 month horizon, outcome-oriented, but qualitative and not quantitative) is modeled on those processes but intentionally simplified to account for Rust project needs.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None: this RFC does not specify an explicit process but that will come as a follow-up RFC.

# Future possibilities
[future-possibilities]: #future-possibilities

See [earlier section](#the-shiny-future-we-are-working-towards).
