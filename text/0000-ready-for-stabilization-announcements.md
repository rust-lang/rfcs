- Feature Name: ready_for_stabilization_announcements
- Start Date: 2018-08-26
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Announce unstable features close to stabilization or needing community attention
through TWiR.

# Motivation
[motivation]: #motivation

As Rust as a language matures, it is no longer necessary to use nightly features
for a day to day development. This results in less experimentation with features
before they stabilize and in noticeable lack of feedback (or feedback that
arrives after stabilization instead of before).

Actively seeking out nightly features has the downside that one doesn't know to
which ones to pay attention to, as there are too many and it is better to play
with the ready ones.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TWiR (This Week in Rust) shall announce tracking issues which are currently in
final comment period, just like with RFC PRs in FCP.

In addition, it's also possible to announce tracking issues of features where
increased attention and input from community is desirable.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The TWiR newsletter already allows inclusion of „key PRs“ in the „Final Comment
Period“ section.

This RFC asks for inclusion of tracking issues with visible effects in the
„Final Comment Period“ section, and allows also including tracking issues where
there's an explicit call for experimentation and feedback.

Furthermore, it proposes to visually distinguish RFCs from tracking issues and
to explicitly call for experimenting with the features and providing feedback.

When no or very little feedback arrives on a feature, it might be a signal to
either wait longer or consider if the feature is desired by users at all.

# Drawbacks
[drawbacks]: #drawbacks

* It adds more things into TWiR, making it longer and competing more for
  reader's attention.
* People who want to pay attention to things will feel even more overwhelmed
  about everything that happens at each one time.
* It makes the FCP potentially longer, if the relevant team decides to wait for
  more feedback.

However, as TWiR is not subject to the stability promise, it can be reverted if
any of the above turns out to be a large problem.

# Rationale and alternatives
[alternatives]: #alternatives

* Do nothing.
* Choose a different medium, for example the internals forum (or choose both).
* Be even stricter and mandate inclusion of all tracking issues to be posted
  during their FCP and mandate that it can't proceed unless it gets a certain
  amount of feedback and testing ‒ for example two snippets of code actively
  using it in publicly accessible places (where it can reasonably apply).

# Prior art
[prior-art]: #prior-art

* The same is already done for RFCs to bring attention to them at the crucial
  moment of their life.
* Several discussion threads at random places (reportedly more somewhere):
  - https://internals.rust-lang.org/t/idea-mandate-n-independent-uses-before-stabilizing-a-feature/7522
  - https://internals.rust-lang.org/t/fortifying-the-process-against-feature-bloat/7608
* RFCs in many communities mandate certain amount of feedback before proceeding
  to the final stages ‒ for example the IETF RFC process mandates two
  independent implementations that manifest interoperability before letting the
  RFC from experimental to final stage.

# Unresolved questions
[unresolved]: #unresolved-questions

* Exact wording of the text.
* Should there be a hard rule about what is included, or some rule of thumb is
  enough (for example: if it deserves an announcement at release time, it also
  deserves to be seen in TWiR), or should all tracking issues go through this?
* How exactly should be lack of feedback handled?
