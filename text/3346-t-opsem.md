- Feature Name: n/a
- Start Date: 2022-11-07
- RFC PR: [rust-lang/rfcs#3346](https://github.com/rust-lang/rfcs/pull/3346)
- Rust Issue: n/a

# Summary

Create an operational semantics team that is tasked with owning the semantics of unsafe code. This responsibility would be transferred from T-types, which had previously been given ownership of this domain. Additionally, this team replaces the Unsafe Code Guidelines working group, which has been doing much of the work in this space.

## Mission and responsibilities

As of this RFC, many of the questions around the rules governing unsafe code in Rust are unanswered. The team is responsible for answering these question by producing an operational semantics that specifies those rules. As a part of this semantics, questions around memory and aliasing models, multi-threading and atomics, and generally "what constitutes undefined behavior" will be answered. This is expected to be a massive undertaking requiring lots of work and collaboration. As such, it is worth calling out that a very important part of T-opsem's responsibility is in the organizational role it plays. The team is responsible for creating a plan, ensuring that all interested parties have a chance to provide input, and ensuring that the end result aligns with the goals and values of the Rust project and T-lang.

Furthermore, the team is responsible for ensuring that while a stable operational semantics does not yet exist for the language, the project remains on track for eventually having one. Concretely, this means that any decisions made by other teams which add new requirements to the operational semantics or make new promises about what is or is not undefined behavior must be approved by T-opsem.

### Scope

It is not possible to precisely define where the scope of the team's responsibilities ends. At minimum, any behavior that is only observable in unsafe code is definitely within scope of T-opsem. However, there are parts of the language that do not satisfy the "only observable in unsafe code" condition, and yet interact very heavily with optimizations, implementability of Miri, and other topics core to T-opsem's interests. As such, T-opsem may at any point come to an agreement with any of the other teams to take (possibly partial) ownership of such questions.

#### Examples

 - **When may a raw pointer be used to write to memory that a unique reference also points to?**

   Writing via a raw pointer requires `unsafe` code, meaning this question is in scope for T-opsem. The answer to this question also has broad implications for the usability of the `unsafe` subset of the Rust language. As such, the lang team will need to approve the high-level answer.
   
 - **Do match guards have semantic meaning?**

   Match guards are inserted by the compiler around match statements to ensure that if guards cannot change the value being matched on. Whether match guards should exist at all primarily affects exhaustiveness checking, and so is a question for T-types and T-lang to answer, not T-opsem.
   
   However, T-opsem is responsible for deciding whether these match guards have semantic meaning at runtime, as that is only observable to `unsafe` code in if-guards.
   
 - **Should an `Unordered` atomic ordering be added to the language?**

   The behavior of an `Unordered` ordering is distinguishable from a `Relaxed` ordering in strictly safe code. However, T-lang should still consult T-opsem on this question, because T-opsem is expected to be the team that is most familiar with and has the most interest in the semantics of atomic memory models.

## Relationships to other teams

**T-lang**: The team is a subteam of T-lang. It has the same relationship to T-lang as T-types has. This means decisions about "details" will be made by the team alone, but decisions around the big picture "direction" will require consultation with T-lang.

**T-types**: As T-types will no longer own semantics questions, the responsibilities of T-opsem and T-types are not expected to overlap. However, like other teams, T-types is expected to consult T-opsem on any changes that require support from the operational semantics. For example, if T-types wants to extend the borrow checker to allow more code patterns, T-opsem must confirm that the code that this permits can be supported by a reasonable operational semantics. Conversely, when T-opsem wants to declare some unsafe code UB, it better be the case that T-types does not have plans to allow the same action to be expressible in safe code. Additionally, T-types and T-opsem are expected to need to collaborate heavily on the syntax and semantics of MIR, since MIR is pivotal to both teams' interests.

**T-compiler**: Unlike T-types, T-opsem is not a subteam of T-compiler as it does not own any implementations. However, T-compiler is still expected to request approval from T-opsem before adding any optimization that depends on new theorems about the operational semantics. T-opsem will ensure that these theorems are expected to be true and are reasonable things for the compiler to depend on now.

## Processes

For most decisions, T-opsem will use a standard FCP process. This includes at least those cases where other teams are asking T-opsem for approval, and internal team decisions that don't affect the language or other teams.

Because of the size and complexity inherent to attempting to stabilize an operational semantics, this RFC does not propose any particular process for achieving that. How an operational semantics is planned, evaluated, and stabilized is an important set of questions that will need to be answered, but requires more work and is sufficiently thorny to deserve its own RFC.

## Membership

New members will be added to the team using a process identical to one already used by the libs and style teams. Specifically:

> Proposed new members of the team are nominated by existing members. All existing members of the team must affirmatively agree to the addition of a member, with zero objections; if there is any objection to a nomination, the new member will not be added. In addition, the team lead or another team member will check with the moderation team regarding any person nominated for membership, to provide an avenue for awareness of concerns or red flags.

When considering someone for membership, the qualifications below will all be taken into account:

 - Is this person **familiar with the current state of operational semantics** work in Rust?
 - Has this person **contributed signifiantly** to the problem space around operational semantics?
    - There is no specific area in which this contribution must have taken place - proposing new designs, preparing a formalized version of the spec, writing libraries that make use of the semantics, writing optimizations that make use of the semantics, contributing to miri and related tooling, or preparing documentation and teaching materials are all possibilities.
 - Does this person have a **good understanding of the tradeoffs** that affect operational semantics work?
    - Have they demonstrated a desire and ability to find solutions that balance and support all of these interests?
 - Is this person **responsible**?
    - When they agree to take on a task, do they either get it done or identify that they are not able to follow through and ask for help?
 - Is this person able to **lead others to a productive conversation**?
    - Are there times when a conversation was stalled out and this person was able to step in and get the design discussion back on track?
    - This could have been by suggesting a compromise, but it may also be by asking the right questions or encouraging the right tone.
 - Is this person able to **disagree productively**?
    - When they are having a debate, do they make an active effort to understand and repeat back others' points of view?
    - Do they "steelman", looking for ways to restate others' points in the most convincing way?
 - Is this person **active**?
    - Are they attending meetings regularly?
    - Either in meeting or elsewhere, do they comment on discussions and otherwise?

The last four bullets are lightly edited versions of a subset of the [T-lang membership qualifications][lang-qualifications].

[lang-qualifications]: https://lang-team.rust-lang.org/membership.html

Like for many teams, membership is kept up to date and team members who are inactive for more than 6 months may be moved to the alumni team.

### Team Leads

Leads are responsible for:

 - Leading and scheduling team meetings
 - Selecting the deep dive meetings
 - Making decisions regarding team membership
 - General "buck stops here"-type decisions

Leads typically serve for 6 months to 1 year, at which point the team will consider whether to rotate.

The initial team leads are Ralf Jung and Jakob Degen. The leads will decide the remaining members after the RFC has been accepted.

## Meetings

The team will have a monthly planning meeting during which the remaining meetings are scheduled.

The majority of the remaining meetings are expected to be deep dive meetings: Someone either presents a problem they have discovered and why it is difficult or they present a proposed solution to a pre-existing problem. For example, most individual issues on the unsafe-code-guidelines repository might be a good candidate for a meeting.

As noted above, the team and certainly all of its members are expected to have interests that extend past the strict scope of the team. Because of this meetings might also be used to hold discussions about topics in the broader problem space. Some possible examples are "how does weak memory modeling work in miri" or "what are common ergonomics problems users face when writing unsafe code."

## Drawbacks and Alternatives

 - This further complicates ownership. There would now be a third team in addition to T-lang and T-types that might be responsible for deciding on a particular language question.

   This is not always necessarily a drawback. It can instead be seen as a concession to the reality that as the language matures, the questions that must be answered require increasingly careful consideration from more than one perspective.
   
 - Unlike T-types, this team does not own any code. As such, there is no procedural processes in place to, for example, ensure that Miri remains in line with the decisions of T-opsem.

   Still, because of the overlapping interests between people working on Miri and T-opsem, it seems unlikely that there is a real risk of divergence.
   
 - One alternative is to maintain the status quo, that is to have T-types continue to be responsible for these decisions.

   Currently, the intersection between the members of WG-unsafe-code-guidelines and T-types is small. This means this option seems non-ideal, as it is unlikely that individuals interested in the topics that remain with T-types after this RFC are the same people who are most interested in working on opsem topics.
