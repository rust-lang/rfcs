- Feature Name: N/A
- Start Date: 2026-07-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: N/A

## Summary
[summary]: #summary

Adopt [guidelines] for using AI tools, along with other AI policy items that apply to the whole Project.

## Motivation
[motivation]: #motivation

In the Rust Project, we've seen an increase in unhelpful contributions, especially ones made with AI tools.  These are frustrating and costly for reviewers.  We need to find ways to reduce how many of these we get and to lower the cost of handling them.

The [guidelines] in this RFC set clear expectations.  We hope that, as a result, fewer contributors will send us unhelpful things and more contributors will send us helpful ones.  We hope that this will make decisions and communication less costly for reviewers and moderators.

At the same time, the *process* of agreeing on an AI policy has been costly.  Project members have many reasonable and diverse views about AI tools and about what role these tools should have in society, in open source communities, and in the Project.

This RFC aims to adopt an intersection of agreement: the points on which we agree.  The goal is to reduce the most serious harms that we face right now.

## Reference-level and guide-level explanation
[reference-level explanation]: #reference-level-explanation

The [guidelines] below form Rust's policy on the use of AI tools, and applies to all contributors to the Rust Project. The guidelines may be cited or copied in any place where we guide contributors on what we expect.

The [enforcement and modifications][meta-policy] section describes how will the policy be enforced and how it can be extended in specific Project places. It is most relevant to Project members.

## Guidelines for using AI tools
[guidelines]: #guidelines-for-using-ai-tools

### Be in the loop
[be in the loop]: #be-in-the-loop

As the author, you are fully responsible for anything you send us (for example, code, documentation, pull requests, issues, reports, proposals, or comments).  Before you submit something, you *must* review it, understand it, and be able to explain it.  You *must* be the human in the loop.  You *must not* send us fully autonomous contributions (that is, contributions made by an AI tool alone, with no human in the loop).

Consider describing how your contribution was produced and disclosing usage of AI; this helps reviewers give better feedback.

### Communication
[communication]: #communication

In comments (and messages, posts, etc.) on platforms such as GitHub or Zulip, we want to hear from *you*.  You *must not* let AI communicate on your behalf or just copy AI output when you comment or reply.

It is allowed to use AI tools for translating comments.  In that case consider including your original message, or saying that you used automated translation.

It is allowed to include raw AI output in an attachment, for example in a security report, if you disclose that it was produced by AI.  But you *must* discuss, in your own words, that output and how it was produced.  Consider saying why it is useful and how much you have reviewed it.

## Enforcement and modifications
[meta-policy]: #meta-policy

Rust teams may extend the [guidelines] above for the repositories (or their subsets) that they maintain, provided that all relevant teams who also maintain that same area agree with it. However, teams may not relax the restrictions concerning [communication].

Teams may not alter the policies cited in the rest of this section below.

The *must* and *must not* items are the normative part of the guidelines.  If someone has been told about these items and still repeatedly fails to follow them, that is disruptive behavior.  It is also a form of the "other attention-stealing behavior" described in our [Code of Conduct].  When this causes conflict or tension, consider reporting it to the moderation team.

A maintainer may decline to review a contribution for any reason.  That includes declining to review a contribution because it appears to have been made with an AI tool and the maintainer does not want to review such work.

[Code of Conduct]: https://rust-lang.org/policies/code-of-conduct/

## Drawbacks
[drawbacks]: #drawbacks

When we adopt any policy for contributions, we ask more from our contributors.  On the other hand, this policy might help those contributors by making it clear what we expect.

This policy does not completely ban the use of AI tools.  Some people may see that as an implicit endorsement of these tools.  This policy is not an endorsement, but it's not always possible to stop people from getting the wrong idea.

## Rationale and alternatives
[rationale and alternatives]: #rationale-and-alternatives

This policy encourages disclosure (saying that you used AI tools), but it does not *demand* disclosure.  Some feel strongly that disclosure should be demanded.  Others feel strongly that demanding it causes harms.  This RFC adopts the intersection of agreement.

This policy reduces the autonomy of teams in some places and preserves it in others.  Where teams keep their autonomy, they can fit their practices to their own unique work and circumstances, but the Project can feel more uneven.  Where the policy restricts autonomy, the Project becomes more uniform, but the policy may interfere with teams that have unique needs.  This RFC tries to balance the two.

## Prior art
[prior art]: #prior-art

Python adopted a [similar policy][Python AI policy] in [python/devguide#1778].  The Crates.io team adopted the Python policy in [rust-lang/crates.io#13726].  This RFC also draws ideas from the [Astral AI policy] (adopted by [regex] in [rust-lang/regex#1369]), [LC#273], [RFC 3936], [RFC 3950], and from the discussion in [rust-lang/rust-forge#1040].

Many other open-source projects have adopted various policies, and within the Project, various policies have been proposed or adopted.  To keep this section brief, we have not tried to survey all of those here.

[Astral AI policy]: https://github.com/astral-sh/.github/blob/main/AI_POLICY.md
[Python AI policy]: https://devguide.python.org/getting-started/ai-tools/
[python/devguide#1778]: https://github.com/python/devguide/pull/1778
[regex]: https://github.com/rust-lang/regex/
[rust-lang/crates.io#13726]: https://github.com/rust-lang/crates.io/pull/13726
[rust-lang/regex#1369]: https://github.com/rust-lang/regex/pull/1369
[rust-lang/rust-forge#1040]: https://github.com/rust-lang/rust-forge/pull/1040
[LC#273]: https://github.com/rust-lang/leadership-council/issues/273
[RFC 3936]: https://github.com/rust-lang/rfcs/issues/3936
[RFC 3950]: https://github.com/rust-lang/rfcs/pull/3950

## Unresolved questions
[unresolved questions]: #unresolved-questions

None.

## Future possibilities
[future possibilities]: #future-possibilities

This RFC aims to reduce the harms that we face right now.  In the future, we may decide other things.  The world is moving quickly.  Let's adopt an intersection of agreement today and then see what happens later.

## Acknowledgments

@Kobzol coauthored this policy with me; thanks to him for that fruitful collaboration.  Thanks to the authors of the [Python AI policy] and the [Astral AI policy] for helpful ideas.  Thanks to all those who left useful feedback on the earlier policy drafts.

All errors remain mine alone.
