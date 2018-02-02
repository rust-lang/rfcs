- Feature Name: Adopt Ferris as the Official Mascot/Epoch Costumes
- Start Date: 2018-02-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Officially adopt Ferris as the mascot for the current epoch and have a new
costume/design for each epoch as it arrives.

# Motivation
[motivation]: #motivation

Ferris has been a part of the community for many years and used as an unofficial
mascot for Rust. Our nickname Rustacean is a reference to Ferris the Crustacean.
We've [made plushies], [art work], [had multiple] [books], reference it on the cover,
but no move has been made to officially adopt our lovable friend in an official capacity.
This RFC seeks to formalize the relationship we have with Ferris and making it the mascot
for the Rust community.

Branding is an important thing when selling your technology to others, including
the non-technical or project managers in charge of what gets used. As things
change though we also want to be able to convey that to users who might not have
picked up or used Rust since the last epoch. In this way we can have Ferris wear
a new costume since it's a new epoch. It also conveys what we mean by [what an epoch is]:
Each epoch plays nicely with the previous ones despite having breaking changes. In the same
way that Ferris has multiple costumes they can all be worn, or mixed and matched, and are
able to play with each other nicely. On top of this they can act as a marker for embodying the
accomplishments of the community after each epoch begins.

# Detailed design
[design]: #detailed-design

How the branding would work and how to market the different costumes Ferris
would wear are details best left outside of this RFC and deliberated to the
relevant teams/working groups/etc. when the time comes. What this RFC lays out
is a method in which we choose our mascots:

- Upon a new epoch being announced the Community Team begins setting up
  a contest, whereby community members can submit ideas, drawings etc. to
  have potential mascots be considered
- These can then be filtered out for junk submissions (we don't want ones
  that violate our CoC for instance) and let's the community vote on the costume
  they want to choose.
- Community members would choose their top three picks for the outfit and that
  information will be used to calculate the chosen costume
- After being chosen the Community Team announces the results and work begins on
  integrating the new costume with previous ones as well as artwork and designs
  for the epoch release.

The idea is that the community has a way to have their ideas be given a chance
to mark the passing of a big milestone as a community. Even if one doesn't
submit an idea they would have a choice in the matter and can feel ownership of
the new look for Ferris. Resubmission of non chosen costumes for future epochs
are also fine!

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Unlike the more technical RFCs this is one where there would be tribal knowledge
on the matter and not one that would be documented in places like `TRPL`, or the
`Nomicon` for instance. Instead it would be taught through engagement with the
community, having events with the different versions of Ferris, as well as things
like conference T-Shirts.

# Drawbacks
[drawbacks]: #drawbacks

* Splits up the branding amongst multiple costumes rather than just one.

# Alternatives
[alternatives]: #alternatives

* We do nothing at all and continue the course as is without any mascot at all.
* We adopt multiple mascots, one for each epoch.
* We adopt only one mascot that isn't Ferris

# Unresolved questions
[unresolved]: #unresolved-questions

[made plushies]: https://www.kickstarter.com/projects/1702984242/ferris-the-small-squishable-rustacean-rust-mascot
[art work]: http://www.rustacean.net/
[had multiple]: http://shop.oreilly.com/product/0636920040385.do
[books]: https://nostarch.com/Rust
[what an epoch is]: https://github.com/rust-lang/rfcs/blob/master/text/2052-epochs.md#the-basic-idea
