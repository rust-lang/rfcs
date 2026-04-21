- Feature Name: N/A
- Start Date: 2026-03-13
- RFC PR: [rust-lang/rfcs#3950](https://github.com/rust-lang/rfcs/pull/3950)
- Issue: N/A

## Summary
[summary]: #summary

We adopt a Rust Project contribution policy for AI-generated work.  This applies to all Project spaces.

## Motivation

In the Rust Project, we've seen an increase in unwanted and unhelpful contributions where contributors used generative AI.  These are frustrating and costly to reviewers in the Project.  We need to find ways to reduce the incidence of these and to lower the cost of handling them.

We hope that by stating our expectations clearly that fewer contributors will send us unhelpful things and more contributors will send us helpful ones.  We hope that this policy will make decisions and communication less costly for reviewers and moderators.

## Policy design approach

People in the Rust Project have diverse — and in some cases, strongly opposed — views on generative AI and on its use.  To address the problem in front of us, this policy describes only those items on which Project members agree.

## Normative sections

[Normative sections]: #normative-sections

These sections are normative:

- [Contribution policy for AI-generated work]
- [Definitions, questions, and answers]
- [Normative sections]

Other sections are not normative.

## Contribution policy for AI-generated work

[Contribution policy for AI-generated work]: #contribution-policy-for-ai-generated-work

In all Rust Project spaces:

- Submitting AI-generated work when you weren't in the loop is prohibited.
- Submitting AI-generated work when you haven't checked it with care is prohibited.
- Submitting AI-generated work when you don't have reason to believe you understand it is prohibited.
- Submitting AI-generated work when you can't explain it to a reviewer is prohibited.
- Feeding reviewer questions into an AI tool and proxying the output directly back is prohibited.

## Definitions, questions, and answers

[Definitions, questions, and answers]: #definitions-questions-and-answers

### What is AI-generated work?

Work is AI-generated when agentic or generative machine-learning tools are used to directly create the work.

### What's it mean to be in the loop?

To be in the loop means to be part of the discussion — to be an integral part of the creative back and forth.  You were in the loop if you were there, engaged, and contributing meaningfully when the creation happened.

### What's it mean to check something with care?

To check something with care means to treat its correctness as important to you.  It means to assume that you're the last line of defense and that nobody else will catch your mistakes.  It means to give it your full attention — the way you would pack a parachute that you're about to wear.

### What's it mean to have reason to believe you understand something?

To understand something means that you have a correct mental model of what that thing is, what its purpose is, what it's doing, and how it works.  This is more than we expect.  You're allowed to be wrong.

But you must have *reason* to believe that you understand it.  You must have put in the work to have a mental model and a personal theory of why that model is correct.

It's not enough to just have heard a theory.  If you can close your eyes and map the thing out and why the thing is correct — in a way that you believe and would bet on — then you have reason to believe you understand it.

### What's it mean to be able to explain something to a reviewer?

Reviewers need to build a mental model of their own.  They may want to know about yours in order to help them.  You need to be able to articulate your mental model and the reasons you believe that model to be correct.

### What's it mean to proxy output directly back to a reviewer?

Reviewers want to have a discussion with you, not with a tool.  They want to probe your mental model.  When a reviewer asks you questions, we need the answers to come from you.  If they come from a tool instead, then you're just a proxy.

### Does this policy ban vibecoding?

This policy bans vibecoding.  Andrej Karpathy, who originated the term, [described](https://x.com/karpathy/status/1886192184808149383) *vibecoding* as:

> There's a new kind of coding I call "vibe coding", where you fully give in to the vibes, embrace exponentials, and forget that the code even exists...  I "Accept All" always[;] I don't read the diffs anymore.  When I get error messages I just copy paste them in with no comment [—] usually that fixes it.  The code grows beyond my usual comprehension...  Sometimes the LLMs can't fix a bug so I just work around it or ask for random changes until it goes away...  [I]t's not really coding — I just see stuff, say stuff, run stuff, and copy paste stuff, and it mostly works.

If you didn't read the diffs, then you can't have checked the work with care, you can't have reason to believe you understand it, and you're not in a position to explain it to a reviewer without feeding the questions to the tool and proxying the output back.  If it's grown beyond your comprehension, then even reading the diffs won't help — you don't understand it, won't be able to explain it, and can't say you've checked it with care.

Violating even one of these policy items is enough to violate the policy.

<div id="does-this-policy-ban-slop"></div>

### Does this policy ban AI slop?

This policy goes further than banning AI slop.  *AI slop* is unwanted, **low-quality**, AI-generated work.  This policy does not consider the quality of the work.  High-quality AI-generated work is still prohibited if it fails any item in the policy — e.g., because it was *vibecoded*.  If you weren't in the loop, didn't check the work with care, don't have reason to believe you understand it, or can't explain it to a reviewer, then the contribution is prohibited — regardless of the quality of the work.

### Does this policy ban fully automated AI-generated contributions?

This policy bans fully automated AI-generated contributions.  These are the worst of the unwanted contributions that have come our way, and each item in the policy independently bans these.

If you created the work in a fully automated way, then you weren't in the loop, you can't have checked it with care, you can't have reason to believe you understand it, and you're not in a position to explain it to a reviewer without feeding the questions to the tool and proxying the output back.

Violating even one of these policy items is enough to violate the policy.

### When contributions appear to fall short of this policy, what do reviewers do?

Reviewers may reject any contribution that falls short of this policy without detailed explanation.  Simply link to the policy or paste this template:

> On initial review, unfortunately, this contribution appears to be an AI-generated work that falls short of one or more of our policies:
>
> - Submitting AI-generated work when you weren't in the loop is prohibited.
> - Submitting AI-generated work when you haven't checked it with care is prohibited.
> - Submitting AI-generated work when you don't have reason to believe you understand it is prohibited.
> - Submitting AI-generated work when you can't explain it to a reviewer is prohibited.
> - Feeding reviewer questions into an AI tool and proxying the output directly back is prohibited.
>
> For details, see [RFC 3950](https://github.com/rust-lang/rfcs/pull/3950).
>
> We will not be reviewing this work further.
>
> While we trust that you intended to be helpful in making this contribution, these contributions do not help us.  Reviewing contributions requires a lot of time and energy.  Contributions such as this do not deliver enough value to justify that cost.
>
> We know this may be disappointing to hear.  We're sorry about that.  It pains us to reject contributions and potentially turn away well-meaning contributors.  For next steps you can take, please see:
>
> - *[What should I do if my contribution was rejected under this policy?](https://github.com/rust-lang/rfcs/blob/TC/ai-contribution-policy/text/3950-ai-contribution-policy.md#what-should-i-do-if-my-contribution-was-rejected-under-this-policy)*

### Should reviewers investigate to determine if AI tools were used?

There's no need to investigate to determine if AI tools were used.  If the contribution seems on its face to fall short, then just reject the contribution, link to the policy or paste the template, and, at your discretion, notify the moderators.

### What should I do if my contribution was rejected under this policy?

If your contribution was rejected under this policy, first, step back and honestly evaluate whether your contribution did in fact fall short.  We appreciate people who are honest with themselves about this.  If your contribution failed even one of the policy items above — in letter or spirit — then it fell short of this policy.

If your contribution fell short, reflect on what you could do better.  We need contributors who put heart into their contributions — not just point a tool at our repositories.  If you do want to contribute, then put a lot of care and attention into your next contribution.  If you've already been banned, then reach out to the moderation team and talk about what you've learned and why you want to contribute.

If you're sure that your contribution didn't fall short but you're a new contributor, see the next item.  As a new contributor, it's difficult to use these tools in a way that won't appear to reviewers as falling short.  We encourage you to try again without using generative AI tools, especially for assisting in creation (rather than learning).

In other cases, please understand that we will sometimes make mistakes.  Explain concisely why you believe the contribution to be correct and compatible with this policy; someone will have a look.

### As a new contributor, is it OK to use AI tools?

This policy does not prohibit anyone from using AI tools.  But as a new contributor, it's a good practice to first contribute without using generative AI tools, especially for assisting in creation (rather than learning).  Using these tools correctly is difficult without a firm baseline understanding.  Without this understanding, it's easy to use these tools in a way that will fall short (or appear to reviewers as falling short) of this policy.

### What if I follow the policy but my work sounds like the output of an LLM?

This policy does not prohibit work — that otherwise complies with the policy — from merely *sounding like* the output of an LLM.  But keep in mind that we want to hear from you, not from a tool, so we encourage you to speak in your own voice.  A contribution that sounds like it came from an LLM will, in practice, have a higher risk of being rejected — as a false positive — by a reviewer, even if it complies with this policy.

### What happens to me if my contributions are rejected under this policy?

If your contributions are rejected under this policy and reported to the moderators, the moderators will decide on appropriate next steps that could be as severe as banning you from the Project and all of its spaces.  The moderators will consider the details of each situation when deciding on these next steps.  While this RFC defines what is prohibited, it leaves the handling of violations fully to the discretion of the moderators.

### Does this apply to PRs, issues, proposals, comments, etc.?

This policy applies to pull requests, issues, proposals in all forms, comments in all places, and all other means of contributing to the Rust Project.

### By not banning use of AI tools, does this RFC endorse them?

By not banning use of AI tools, this RFC does not endorse their use.  People in the Project have diverse views on generative AI and on its use.  This RFC takes no position — positive or negative — on the use of these tools beyond forbidding those things the policy prohibits.

### Is this the final policy for contributions or for AI-assisted contributions?

This policy is intended to solve the problem in front of us.  The world is moving quickly at the moment, and Project members are continuing to explore, investigate, learn, and discuss.  Other policies may be adopted later, and this RFC intends to be easy for other policies — of any nature — to build on.

### Does this policy require disclosure of the use of generative AI tools?

This policy does not require disclosure of the use of generative AI tools.  This is a complex question on which Project members have diverse views and where members are continuing to explore, investigate, learn, and discuss.  Later policies may further address this.

### Can teams adopt other policies?

This RFC adopts a policy for shared Project spaces and a baseline policy for all team spaces.  It does not restrict any team from adopting policies for its own spaces that add prohibitions.

At the same time, there is a cost to having different policies across the Project: it risks surprise and confusion for contributors.  By adopting a policy that represents those items on which we have wide agreement and that addresses the concrete problems we're seeing across the Project, we hope to create less need for custom policies and more certainty for contributors.

### What about public communications?

This RFC does not have any policy items focused on the public communications of the Project.  But proposals for Project communications are contributions and must follow this policy.  Later policies may further address this.

### Does this policy make a distinction between new and existing contributors?

New and existing contributors are treated in the same way under this policy.  All contributors — including all Project members — may only make contributions that are compatible with this policy.

At the same time, new contributors face additional challenges in using generative AI tools to produce contributions that reviewers will recognize as compatible with this policy.  It's a good practice for new contributors to first work without using generative AI tools, especially for assisting in creation (rather than learning), to build the baseline understanding required.

## Other questions and answers

### Does accepting AI-generated work risk our ability to redistribute Rust?

What about the copyright situation?  Since this policy does not ban AI-generated work, does that risk our ability to redistribute Rust under our license?  Niko Matsakis [reports](https://nikomatsakis.github.io/rust-project-perspectives-on-ai/feb27-summary.html#the-legality-of-ai-usage):

> On this topic, the Rust Project Directors consulted the Rust Foundation's legal counsel and they did not have significant concerns about Rust accepting LLM-generated code from a legal perspective.  Some courts have found that AI-generated code is not subject to copyright and it's expected that others will follow suit.  Any human-contributed original expression would be owned by the human author, but if that author is the contributor (or the modifications are licensed under an open source license), the situation is no different from any human-origin contribution.  However, this does not present a legal obstacle to us redistributing the code, because, as this code is not copyrighted, it can be freely redistributed.  Further, while it is possible for LLMs to generate code (especially small portions) that is identical to code in the training data, outstanding litigation has not revealed that this is a significant issue, and often such portions are too small or contain such limited originality that they may not qualify for copyright protection.

### Is requiring that contributors take care an acceptable policy item?

To take care is to give something your full attention and treat its correctness as important to you.  That's a meaningful distinction.  As reviewers, we can tell when someone has taken care and when the person has not — there are many signs of this.

At the same time, taking care is just one requirement of the policy.  If a contribution is prohibited by any item in the policy, then it's prohibited by the policy.  A contribution may be rejected under this policy even if we cannot tell whether the person took care.

### Is requiring that contributors have reason to believe they understand an acceptable policy item?

Even the best contributors may sometimes misunderstand their own contributions.  We do not require that people actually understand the things they submit.  But we expect contributors to have *good reason* to expect that they understand what they're submitting to us.  This is reasonable to ask, and it's a prerequisite for a contributor being able to explain the contribution to a reviewer and have a productive conversation.

At the same time, having reason to believe that one understands the contribution is just one requirement of the policy.  If a contribution is prohibited by any item in the policy, then it's prohibited by the policy.  A contribution may be rejected under this policy even if we cannot tell whether the person had good reason for that belief.

### Should the policy require care and attention proportional to that required of reviewers?

An earlier version of the draft that became this RFC stated:

> Submitting AI-generated work without exercising care and attention proportional to what you're asking of reviewers is prohibited.

Is that needed?  In drafting this RFC, it came to feel redundant.  In explaining what it means to check work carefully, we say that this means to check something with care, to treat its correctness as important to you, and to give it your full attention.  That's exactly what it means to exercise care and attention proportional to what's being asked of a reviewer.

## Acknowledgments

Thanks to Jieyou Xu for fruitful collaboration on earlier policy drafts.  Thanks to Niko Matsakis, Eric Huss, Tyler Mandry, Oliver Scherer, Jakub Beránek, Rémy Rakic, Pete LeVasseur, Eric Holk, Yosh Wuyts, David Wood, Jack Huey, Jacob Finkelman, and many others for thoughtful discussion.

All views and errors remain those of the author alone.
