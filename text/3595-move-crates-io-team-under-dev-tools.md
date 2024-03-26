- Feature Name: move-crates-io-team-under-dev-tools
- Start Date: 2024-03-25
- RFC PR: [rust-lang/rfcs#3595](https://github.com/rust-lang/rfcs/pull/3595)
- Rust Issue: N/A

# Summary

This RFC proposes merging the Crates.io team into the Dev tools team as a subteam. The membership of the Crates.io and Dev tools teams proper remain the same.[^subteam]

[^subteam]: Note: Members of subteams are not automatically direct members of their parent team. So Crates.io members will be part of the wider Dev tools team family but *not* direct members of the team proper. In practical terms this means, among other things, that Crates.io team members would not have checkbox authority associated with direct Dev tool team membership, but Crates.io team members could serve as the Leadership Council representative for Dev tools.

# Motivation

The Crates.io team has a much smaller membership base than other teams when both top-level members and the number of subteams are taken into account. It is the only team without any subteams[^subteam-requirement].

As of 2024-03-19:

| Team | # of top-level members | # of subteams/WGs/PGs [^count] |
|--|--|--|
| Crates.io | 8 | 0 |
| Compiler | 15 | 31 |
| Dev tools | 6 | 11 |
| Infrastructure | 6 | 4 |
| Language | 5 | 19 |
| Library | 6 | 7 |
| Moderation[^mods] | 2 | 2 |

[^count]: As calculated by doing a search in the [Teams repo](https://github.com/rust-lang/team) for `subteam-of = "team-id"`
[^mods]: The Moderation team is a special case where being a member demands high community trust and performing difficult work, and the work that the Moderation team does always needs a seat at the Leadership Council table, even though they are also small.

Additionally, out of the small number of crates.io team members, many either do not have bandwidth to serve on the Leadership Council or would have perceived conflicts of interest (namely, being employed by the Rust Foundation) that don't make them the best candidate for Leadership Council representative.

[^subteam-requirement]: This RFC is not proposing any sort of requirement such as "top-level teams must have subteams", necessarily (proposing such a requirement is left as an exercise for a future RFC). Pointing out the Crates.io team is the only top-level team without subteams is merely one signal that the Crates.io team isn't comparable to the other top-level teams.

[RFC 3392](https://github.com/rust-lang/rfcs/blob/master/text/3392-leadership-council.md#top-level-teams) outlines what typically qualifies a team as "top-level". While one could make the argument that the Crates.io team fits these points, there are arguably two aspects where it does not neatly fit:

* "Have a purview that not is a subset of another team's purview": this is hard to argue exactly as most teams don't have well defined purviews, but one could argue that Crates.io's purview is a subset of multiple teams' (see the "Alternatives" section for discussion on this point).

* "Be the ultimate decision-makers on all aspects of that purview": Many decisions involving the crates.io team are ultimately one or more of:
  * legal decisions or funding work done by the Foundation
  * hosting decisions made by Infrastructure
  * capabilities that interface with Cargo

  While the Crates.io team is certainly involved in those decisions and in executing them, it's arguable whether the Crates.io team is the ultimate decision-maker of all aspects of running crates.io.

In the past, whether a team is "top-level" or not has not been of huge consequence. However, this is no longer true since [RFC 3392](https://github.com/rust-lang/rfcs/pull/3392) introduced the Leadership Council whose representation is based on top-level team status. RFC 3392 specifically called out the need for re-examination of which teams are top-level, and this proposal is the second attempt at such a re-examination, after [RFC 3533] that moved the Release team under the Infrastructure team.

Currently, the representation burden is not productive or fair: by virtue of the Crates.io team being small, more of the teams' collective time is being spent on being a representative of the Leadership Council. Additionally, the Crates.io Leadership Council representative is speaking for fewer people than other teams' representatives are.

For the purposes of actual decision making, the Crates.io subteam retains all decision-making power with regard to crates.io related issues (i.e., this proposal does not change who makes any particular decision and is purely a change in Council representation). This may change over time should the Dev tools team choose to structure itself in a different way.

# Practicalities

Once this proposal is accepted, the Crates.io team will move to be a subteam of Dev tools. The Dev tools team does not change its top-level form.

The Dev tools team's Council representative would continue to serve on the Council while the Crates.io representative would immediately stop counting as a representative for all purposes.[^plan]

[^plan]: It is currently the unofficial plan that Carol Nichols will step down in her role as the Crates.io representative, and Eric Huss would take over as the rep, but this would be made official after the merger through internal Dev tools team process.

# Alternatives

## Merge Crates.io into Infrastructure

Crates.io uses infrastructure such as Fastly, CloudFront, S3, Heroku, GitHub, and other services that are managed by the Infrastructure team. There's certainly an argument to be made that the Crates.io team belongs there, however, the docs.rs team is in a similar situation and they are a subteam of Dev tools.

## Be a subteam of both Dev tools and Infra

The Types team provides precedence for this; technically the Types team is a subteam of both the Compiler and Lang teams. However, the teams repo doesn't really support multiple inheritance. The Crates.io team would like to cultivate a closer relationship with the Cargo team especially, and thinks the relationship with the Infra team could be continued in the current manner.

## Creating a new team

There are aspects of the Crates.io Team's purview that are more policy decisions than they are implementation details, such as what information Crates.io should surface about each crate, how to handle different crate ownership situations, or what browsers the site should support.

There could be a new team dedicated to policy questions such as these and other questions that have currently come up to the Leadership Council. People who enjoy thinking about and discussing policies might enjoy being on this team and not being responsible for implementing the policies. People implementing the policies might enjoy not being responsible for creating the policies.

However, it isn't clear if a policy team would be feasible and desirable. If so, this division could be done as a future enhancement; this RFC does not prevent such.

# Prior Art

Many thanks to Ryan Levick's [RFC 3533], much of which was copy-pastaed into this one. ❤️

[RFC 3533]: https://github.com/rust-lang/rfcs/pull/3533
