- Feature Name: i-am-become-death-destroyer-of-apis
- Start Date: 2015-07-16
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Adds a proper path for APIs to migrate from unstable to stable, including a
full shepherd and final comment period. Also establishes guidelines for
how RFCs and PRs should be handled and when RFCs or PRs should be preferred.




# Motivation

Now that the trains and subteams have been running for a while, we have enough
data to flesh out the details on the *Life and Death of an API*. So as to keep
us on the same page, this RFC posits the following premises:

* RFCs are heavyweight:
    * RFCs generally take at minimum 2 weeks from posting to land. In
      practice it can be more on the order of months for particularly
      controversial changes.
    * RFCs are a lot of effort to write; especially for non-native speakers or
      for members of the community whose strengths are more technical than literary.
    * RFCs may involve pre-RFCs and several rewrites to accommodate feedback.
    * RFCs require a dedicated shepherd to herd the community and author towards
      consensus.
    * RFCs require review from a majority of the subteam, as well as an official
      vote.
    * RFCs can't be downgraded based on their complexity. Full process always applies.
      Easy RFCs may certainly land faster, though.
    * RFCs can be very abstract and hard to grok the consequences of (no implementation).

* PRs are low *overhead* but potentially expensive nonetheless:
    * Easy PRs can get insta-merged by any rust-lang contributor.
    * Harder PRs can be easily escalated. You can ping subject-matter experts for second
      opinions. Ping the whole team!
    * Easier to grok the full consequences. Lots of tests and Crater to save the day.
    * PRs can be accepted optimistically with bors, buildbot, and the trains to guard
      us from major mistakes making it into stable. The size of the nightly community
      at this point in time can still mean major community breakage regardless of trains,
      however.
    * HOWEVER: Big PRs can be a lot of work to make only to have that work rejected for
      details that could have been hashed out first. *This is the motivation for
      having RFCs*.

* RFCs are *only* meaningful if a significant and diverse portion of the community actively
  participates in them. The official teams are not sufficiently diverse to establish
  meaningful community consensus by agreeing amongst themselves.

* If there are *tons* of RFCs -- especially trivial ones -- people are less likely to
  engage with them. Official team members are super busy. Domain experts and industry
  professionals are super busy *and* have no responsibility to engage in RFCs. Since
  these are *exactly* the most important people to get involved in the RFC process,
  it is important that we be maximally friendly towards their needs. We cannot build
  a robust language using only the opinion of hobbyists and students with all the time
  in the world.





# Detailed Design

The overarching philosophy of this RFC is: *do whatever is easiest*. If an RFC
would be less work than an implementation, that's a good sign that an RFC is
necessary. That said, if you anticipate controversy, you might want to short-circuit
straight to an RFC. For instance new APIs almost certainly merit an RFC. Especially
as `std` has become more conservative in favour of the much more agile cargoverse.

Here is the general flow for *The Life and Death of an API*:

* Someone wants to make a change:
    * **Submit a PR** if the change is a:
        * Bugfix
        * Docfix
        * Obvious API hole patch, such as adding an API from one type to a symmetric type.
          e.g. `Vec<T> -> Box<[T]>` clearly motivates adding `String -> Box<str>`
        * Minor tweak to an unstable API (renaming, generalizing)
        * Implementing an "obvious" trait like Clone/Debug/etc
    * **Submit an RFC** if the change is a:
        * New API
        * Semantic Change to a stable API
        * Generalization of a stable API (e.g. how we added Pattern or Borrow)
        * Deprecation of a stable API
        * Nontrivial trait impl (because all trait impls are insta-stable)
    * **Do the easier thing** if uncertain. (choosing a path is not final)

* If a PR is submitted, someone reviews it. After normal review:
    * **Close it** if clearly not acceptable:
        * Disproportionate breaking change (small inference breakage may be acceptable)
        * Unsound
        * Doesn't fit our general design philosophy around the problem
        * Better as a crate
        * Too marginal for std
        * Significant implementation problems
    * **Merge as Unstable** with a fresh feature gate and associated tracking issue
      if good to go. Note that trait impls and docs are insta-stable and thus have
      no tracking issue. This may imply requiring a higher level of scrutiny for such
      changed.
    * **Ping @rust-lang/libs** if the change merits greater scrutiny.
    * **Block on an RFC** if consensus can't be reached.

* If a change makes it through the RFC process:
  * **Create a tracking issue**
  * **Find someone to implement**

HOWEVER: an accepted RFC is not a rubber-stamp for merging an implementation PR.
Nor must an implementation PR perfectly match the RFC text. Implementation details
may merit deviations, though obviously they should be justified. The RFC may be
amended if deviations are substantial, but are not generally necessary. RFCs should
favour immutability. The RFC + Issue + PR should form a total explanation of the
current implementation, though.

* Once something has been merged as unstable, a shepherd should be assigned
  to promote and obtain feedback on the design.

* Once the API has been unstable for at least one full cycle (6 weeks),
  the shepherd (or any libs team member really) may nominate an API for a
  *final comment period* of another cycle. Feedback and other comments should be
  posted to the tracking issue. This should be publicized

* After the final comment period, an API should ideally take one of two paths:
  * **Stabilize** if the change is desired, and consensus is reached
  * **Deprecate** is the change is undesired, and consensus is reached
  * **Extend the FCP** is the change cannot meet consensus
    * If consensus *still* can't be reached, consider requiring a new RFC or
      just deprecating as "too controversial for std".

* If any problems are found with a newly stabilized API during its beta period,
  *strongly* favour reverting stability in order to prevent stabilizing a bad
  API. Due to the speed of the trains, this is not a serious delay (~2-3 months
  if it's not a major problem).




# Drawbacks

Less gutfeels, moar process?




# Alternatives

Nothing in particular. This is largely what the libs team has been drifting towards.
There are infinite subtle modifications that can be made.




# Unresolved Questions

* Can soundness fixes short-circuit the RFC process?

* Precisely what merits a PR vs an RFC is not exactly an empirical fact.

* 6 weeks may be overkill for an API FCP, but this seems justified for two reasons:
    * it aligns everything with the trains
    * API stabilization is almost absolutely the last step for *forever* being
      stabilized as part of the standard library. As such it merits higher
      scrutiny than an RFC.

* Due to strong overlap, this RFC may want to establish policy for rust-lang
  crates. In particular, a standard API stabilization path involving establishing
  a new rust-lang crate which can be iterated on separate from normal language
  versioning process.


