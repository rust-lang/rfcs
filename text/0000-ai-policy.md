- Feature Name: N/A
- Start Date: 2026-07-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: N/A

## Summary
[summary]: #summary

Adopt [guidelines] for using AI tools, along with other AI policy items that apply to the whole Project.

## Motivation
[motivation]: #motivation

In the Rust Project, we've seen more and more unhelpful contributions, especially ones made with AI tools.  These are frustrating and costly for reviewers.  We need to find ways to reduce how many of these we get and to lower the cost of handling them.

The [guidelines] in this RFC say clearly what we expect.  We hope that, as a result, fewer contributors will send us unhelpful things and more contributors will send us helpful ones.  We hope that this will make decisions and communication less costly for reviewers and moderators.

At the same time, the *process* of agreeing on an AI policy has been costly.  Project members have many reasonable and diverse views about AI tools and about what role these tools should have in society, in open source communities, and in the Project.

This RFC aims to adopt the points we agree on.  The goal is to reduce the most serious harms that we face right now.

## Reference-level and guide-level explanation
[reference-level explanation]: #reference-level-explanation

The [guidelines] below are Rust's policy on the use of AI tools.  These apply to all contributions to the Rust Project.  The guidelines may be referred to in (or copied to) any place where we guide contributors on what we expect.

The [enforcement and extension][meta-policy] section describes how the policy will be enforced and how it can be extended in specific Project spaces.  It is most relevant to Project members.

## Guidelines for using AI tools
[guidelines]: #guidelines-for-using-ai-tools

### Human in the loop
[human in the loop]: #human-in-the-loop

As the author, you are fully responsible for anything you send us (for example, code, documentation, pull requests, issues, reports, proposals, or comments).  Before you submit something, you *must* review it, understand it, and be able to explain it.  You *must* be the human in the loop.  You *must not* send us fully autonomous contributions (that is, contributions made by an AI tool alone, with no human in the loop).

Consider describing how your contribution was made, including whether you used AI tools (and how).  This helps reviewers give better feedback.

### Communication
[communication]: #communication

In comments (and messages, posts, and so on) on platforms such as GitHub and Zulip, we want to hear from *you*.  You *must not* let AI communicate on your behalf or just copy AI output when you comment or reply.

It is allowed to use AI tools for translating comments.  In that case consider including your original message.  Or consider saying that you used automated translation.

It is allowed to include raw AI output in an attachment, for example, in a security report.  But you *must* discuss, in your own words, that attachment and how it was made, including that it was made by AI.  Consider saying why it is useful and how much you have reviewed it.

## Enforcement and extension
[meta-policy]: #enforcement-and-extension

Rust teams may extend the [guidelines] above for the repositories (or the subsets of those repositories) and other spaces that they maintain, provided that all relevant teams that also maintain those same spaces agree to the extension.  But teams may not relax the restrictions concerning [communication].

Teams may not change the policies in the rest of this section.

The *must* and *must not* items are the normative parts of the guidelines.  If someone has been told about these items and still repeatedly fails to follow them, that is disruptive behavior.  It is also a form of the "other attention-stealing behavior" described in our [Code of Conduct].  When this causes conflict or tension, consider reporting it to the moderation team.

A maintainer may decline to review a contribution for any reason.  That includes declining to review a contribution because it seems to have been made with an AI tool and the maintainer does not want to review such work.

[Code of Conduct]: https://rust-lang.org/policies/code-of-conduct/

## Drawbacks
[drawbacks]: #drawbacks

When we adopt any policy for contributions, we ask more from our contributors.  On the other hand, this policy might help those contributors by making it clear what we expect.

This policy does not completely ban the use of AI tools.  Some people may think this means we approve of these tools.  It does not, but it's not always possible to stop people from getting the wrong idea.

## Rationale and alternatives
[rationale and alternatives]: #rationale-and-alternatives

This policy encourages you to say when you have used AI tools, but it does not *demand* that you say so.  Some feel strongly that we should demand this.  Others feel strongly that demanding it causes harms.  This RFC adopts the points we agree on.

This policy reduces the autonomy of teams in some places and keeps it in others.  Where teams keep their autonomy, they can fit their practices to their own unique work and situation, but the Project can feel more uneven.  Where the policy limits autonomy, the Project becomes more uniform, but the policy may get in the way of teams that have unique needs.  This RFC tries to balance the two.

## Prior art
[prior art]: #prior-art

Python adopted a [similar policy][Python AI policy] in [python/devguide#1778].  The Crates.io team adopted the Python policy in [rust-lang/crates.io#13726].  This RFC also takes ideas from the [Astral AI policy] (adopted by [regex] in [rust-lang/regex#1369]), [LC#273], [RFC 3936], [RFC 3950], and from the discussion in [rust-lang/rust-forge#1040].

Many other open-source projects have adopted various policies, and within the Project, various policies have been proposed or adopted.  To keep this section short, we have not tried to list them all here.

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

This RFC aims to reduce the harms that we face right now.  In the future, we may decide other things.  The world is moving quickly.  Let's adopt the points we agree on today and then see what happens later.

## Acknowledgments

@Kobzol coauthored this policy with me; thanks to him for that fruitful collaboration.  Thanks to the authors of the [Python AI policy] and the [Astral AI policy] for helpful ideas.  Thanks to all those who left useful feedback on the earlier policy drafts.

All errors remain mine alone.
