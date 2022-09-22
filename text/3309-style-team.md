- Feature Name: N/A
- Start Date: 2022-08-25
- RFC PR: [rust-lang/rfcs#3309](https://github.com/rust-lang/rfcs/pull/3309)
- Rust Issue: N/A

# Summary
[summary]: #summary

This RFC charters the Rust Style Team, responsible for evolving the Rust style over time. This includes styling for new Rust constructs, as well as the evolution of the existing style over the course of Rust editions (without breaking backwards compatibility).

# Motivation
[motivation]: #motivation

RFC 1607 proposed and motivated a process for determining code formatting guidelines and producing a style guide, via a temporary style team. That style guide was published as RFC 2436, and the style team wound up its operation and no longer exists. However, Rust has multiple ongoing needs for new determinations regarding Rust style, such as determining the style of new Rust constructs, and evolving the Rust style over time. Thus, this RFC re-charters the Rust Style Team as a non-temporary subteam.

# Explanation and charter
[explanation]: #explanation

The renewed need for the Rust style team began to arise in discussions of language constructs such as `let`-chaining (RFC 2497) and `let`-`else` (RFC 3137). New constructs like these, by default, get ignored and not formatted by rustfmt, and subsequently need formatting added. The rustfmt team has expressed a preference to not make style determinations itself; the rustfmt team would prefer to implement style determinations made by another team.

In addition, rustfmt maintains backwards compatibility guarantees: code that has been correctly formatted with rustfmt won't get formatted differently with a future version of rustfmt. This avoids churn, and avoids creating CI failures when people use rustfmt to check style in CI. However, this also prevents evolving the Rust style to take community desires into account and improve formatting over time. rustfmt provides various configuration options to change its default formatting, and many of those options represent changes that many people in the community would like enabled by default.

This RFC proposes re-chartering the style team, as originally specified in RFC 1607, to determine the Rust style. This includes:
- Making determinations about styling for new Rust constructs
- Evolving the existing Rust style
- Defining mechanisms to evolve the Rust style while taking backwards compatibility into account, such as via Rust editions or similar mechanisms

## Team structure and membership

The Rust style team will be a subteam of the Rust language team. In addition, the style team will maintain a close working relationship with the rustfmt team.

The initial members of the style team shall be:
- Lead: Caleb Cartwright (@calebcartwright)
- Jane Losare-Lusby (@yaahc)
- Josh Triplett (@joshtriplett)
- Michael Goulet (@compiler-errors)

The Rust style team shall have at least 3 members and at most 8. If the team has fewer than 3 members it shall seek new members as its primary focus.

Members of the style team are nominated by existing members. All existing members of the team must affirmatively agree to the addition of a member, with zero objections; if there is any objection to a nomination, the new member will not be added. In addition, the team lead or another team member will check with the moderation team regarding any person nominated for membership, to provide an avenue for awareness of concerns or red flags.

The style team will have regular synchronous meetings when it has work to do. (The style team may also choose to handle individual agenda items asynchronously.) The style team shall not meet when it does not have work to do, but it shall remain in existence.

The style team will use labels such as `T-style` and `I-style-nominated` on rust-lang repositories, to identify and handle issues requiring style decisions.

The output of the Rust style team shall be modifications to the Rust style guide, and other guidance to the rustfmt team. The style team shall also make determinations regarding changes to the existing style, typically in the form of proposed changes to rustfmt options; such changes shall be applied in new Rust editions or via similar mechanisms, to avoid generating churn and CI failures in existing Rust code.

Note that the Rust style guide will generally match the latest version of the Rust style; the style team does not plan to maintain multiple branches of the style guide for different editions, in part because formatting for new constructs will apply to any edition supporting those constructs.

This RFC proposes to move the Rust style guide to the rust-lang/rust repository, rather than its current location in the RFCs repository. Style work may additionally take place in the `fmt-rfcs` [repository](https://github.com/rust-dev-tools/fmt-rfcs) (which this RFC proposes to revive). Larger style proposals may wish to start there rather than as PRs to the style guide. The style team may choose to change this process.

The style team is empowered to make decisions on Rust style directly. However, the rustfmt team may reject or defer style determinations on the basis of implementation feasibility, providing such feedback to the style team for further revision. The style team may also make non-binding recommendations to the rustfmt team on variations that may warrant rustfmt configuration options, but determination of rustfmt configurability remains the purview of the rustfmt team. The style team may also provide non-binding advice to the language team on aspects of proposed Rust language constructs as they affect Rust style and readability. The style team will take input from the Rust community, though it is not bound to follow determinations of community popularity/majority. The style team may also seek professional advice regarding language readability and learnability.

Style determinations are specifically limited to formatting style guidelines which can be enforced by Rustfmt with its current architecture. Styles that cannot be enforced by Rustfmt without a large amount of work are out of scope.

Whenever possible, style decisions should be made before a new construct is stabilized. However, style decisions shall not be considered a blocker for stabilization.

The Rust style team shall make decisions by consensus, as with other Rust teams. Recognizing that matters of style are *particularly* prone to [bikeshed-painting](https://4682b4.bikeshed.com/) almost by definition, the Rust style team may need to make particular effort to reach amicable consensus.

By way of common understanding, the style team acknowledges that the default style will not and is not expected to satisfy everyone (though it should attempt to take community preferences into account), and that having a single default style is more important than the precise details of that style. The style team may also take into account many other sources of input, including Rust community practice, practice and constructs from other languages, experience with common readings or misreadings of other languages, and research into language learnability and *transfer*.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The rustfmt team could directly determine the Rust style. However, the rustfmt team does not wish to do so, and wouldn't have the capacity even if they did; they would prefer to implement styling but not determine the defaults.
- The Rust language team could determine the styling for new language constructs. This would add more complexity and potential [bikeshed-painting](https://b0c4de.bikeshed.com/) to the language design process, and not all members of the language team are interested in that work. This would also not address the need for evolving the existing style, which would be even further outside the desired scope of the language team.
- The style team could become a joint subteam of both the language team and the rustfmt team. However, several people have expressed a preference for this team to have a single parent team, and in response, the rustfmt team has recommended that this be a lang subteam.

# Prior art
[prior-art]: #prior-art

RFC 1607 already defined the style team; this RFC removes the time bound on its mandate, and expands it to cover style evolution.
