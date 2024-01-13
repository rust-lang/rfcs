- Feature Name: combine-infra-and-release
- Start Date: 2023-11-23
- RFC PR: [rust-lang/rfcs#3533](https://github.com/rust-lang/rfcs/pull/3533)
- Rust Issue: N/A

# Summary

This RFC proposes merging the Release team into the Infrastructure team as a subteam. The membership of the Infrastructure and Release teams proper remain the same.[^subteam]

[^subteam]: Note: Members of subteams are not automatically direct members of their parent team. So Release members will be part of the wider Infra team family but *not* direct members of the team proper. In practical terms this means, among other things, that Release team members would not have checkbox authority associated with direct Infra team membership, but Release team members could serve as the Leadership Council representative for Infra.

# Motivation

Historically the Release team has had a much smaller scope than other teams. It typically only functions to execute on releases. While this is not a trivial amount of work, it is definitely much smaller than other top-level teams' purviews (e.g., the compiler team's purview to develop the Rust compiler).

[RFC 3392](https://github.com/rust-lang/rfcs/blob/master/text/3392-leadership-council.md#top-level-teams) outlines what typically qualifies a team as "top-level". While one could make the argument that the Release team fits these points, there are arguably two aspects where it does not neatly fit:
* "Have a purview that is foundational to the Rust Project": while Rust releases are obviously extremely important, it is hard to argue that they are "foundational". Just like there is no CI team (despite CI also being extremely important), the release process is not a foundational piece of what makes Rust, Rust.
* "Have a purview that not is a subset of another team's purview": this is hard to argue exactly as most teams don't have well defined purviews, but one could argue that the Infrastructure team's purview is roughly "to maintain and administer all logistical activities for the continuing operation of the Rust project" which would then be a superset of what the Release team's purview is. The Infrastructure team's purview already contains user-facing components (e.g.,the Rust Playground and crates.io CDN infrastructure) so adding additional user-facing concerns is not a categorical expansion of that purview.

In the past, whether a team is "top-level" or not has not been of huge consequence. However, this is no longer true since [RFC 3392](https://github.com/rust-lang/rfcs/pull/3392) introduced the Leadership Council whose representation is based on top-level team status. This RFC specifically called out the need for re-examination of which teams are top-level, and this proposal is the first attempt at such a re-examination. The most immediate practical reason for being strict with the definition of "top-level" is to balance "the desire for a diverse and relatively shallow structure while still being practical for productive conversation and consent."

With this proposal the Infrastructure team's purview does not grow considerably and remains at a level similar to other existing top-level teams such as a language and compiler.

For the purposes of actual decision making, the Release subteam retains all decision-making power with regard to release related issues (i.e., this proposal does not change who makes any particular decision and is purely a change in Council representation). This may change over time should the Infra team choose to structure itself in a different way.

# Practicalities

Once this proposal is accepted, the Release team will move to be a subteam of Infrastructure. The Infrastructure team does not change its top-level form.

The Infrastructure team's Council representative would continue to serve on the Council while the Release representative would immediately stop counting as a representative for all purposes.[^plan]

As part of this change, wg-triage will move out from under the Release team and move to be a part of Launching Pad until a more appropriate home for the working group can be found.

[^plan]: It is currently the unofficial plan that Ryan Levick will step down in his role as the Infrastructure representative, and Mark Rousskov would take over as the rep, but this would be made official after the merger through internal Infrastructure team process.

# Alternatives

## Combining Infra and Release into a new team

Instead of merging the Release team into Infrastructure, a new team is formed consisting of Infrastructure and Release as the two subteams.

This option is much more complex logistically speaking for the following reasons:
* It is unclear what this team's purview would actually be (i.e., we end up in the same situation as the status quo - one team with a larger purview than the other - but now within this new team)
* Some process would need to be created to decide who the team leads would be.

It is not clear that there are any benefits to this alternative that are worth the cost of the above complexities.

# Future Possibilities

## Merge part of crates.io into Infrastructure

Crates.io is arguably the other team next to Release with the smallest purview compared to other teams. We may want to merge at least part of the team (i.e., the part responsible for running the crates.io infra) into Infrastructure.

This is not included in this proposal, because there are more open questions in this case than in the case of the Release team (e.g., where would the other part of the crates.io team - in charge of crates.io policy - go?).
