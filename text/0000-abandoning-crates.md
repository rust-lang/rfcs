- Feature Name: Process for Abandoning Crates
- Start Date: 2016-11-16
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary
Open source development is a fast moving target, with people creating and
abandoning their projects all the time. However, the project names on crates.io
are static and can never be changed.

There is currently no way for admins to forcefully transfer ownership of a
crate, and this is good. Rust should strive to avoid some of the drama that has
inflicted other communities, such as the notorious kik package incident at npm

However, there should be a documented process for a user to willingly give up
ownership of their crate so that the community can recycle the name or find a
new maintainer.

# Motivation
[motivation]: #motivation

Rust is still in it's infancy, so it has not hit many of these issues. However,
as rust's crates ecosystem continues to grow and more and more users try out the
language (and try out publishing
crates) there is going to be a point where a large number (even a majority) of
the crates on crates.io are out of date and abanonded.

Most users, knowing they will no longer maintain a crate or use it's name, will
want to have a method to allow the community to maintain or recycle their name.
We don't like leaving cruft behind, especially not in a place we value. Having a
clearly defined process for doing this administrative work will become more and
more essential as rust matures.

# Detailed design
[design]: #detailed-design

Fortunately, there are no technological changes that need to be made in order to
be able to recycle names right now, only a change in process and documentation.

In the future, cargo itself could make this process easier and more automated.
However, adding this functionality may pose it's own drawbacks and so will not
be detailed in this RFC.

## Github group

The current solution is to simply have a group of volunteers under the umbrella
of a github group do the administrative effort of accepting ownership of
abandoned crates and granting ownership to those who want them. Such a group has
already been created, and I have volunteered myself to lead that group:
https://github.com/rust-crates/abandoned

If the community feels that another should lead, I would be willing to give
ownership of the group to them.

The basic process for abandoning a crate is simple:
- if you want to abandon your crate, you run
    `cargo owner --add github:rust-crates:reclaimers`
    adding the rust-crates reclaimer's group as an owner to your crate
- you then open an issue at https://github.com/rust-crates/abandoned/issues/new
    detailing that you have added the reclaimers as an owner and you wish to
    extinguish your ownership
- a volunteer will remove you as owner, publish a template crate like
    [this one](https://crates.io/crates/rsk) to crates.io and open a branch
    to track the crate.

The process is similar for claiming an abandoned crate:
- open an issue stating which crate you want to claim
- a vounteer will add you as an owner and merge the branch

Because crates.io already supports github groups, this process should be
fairly easy to maintain and require very little oversight. It only requires
a small group of volunteers to keep up with the issue tracker.

Additionally, when the reclaimer team is satisfied with the process that is in
place, most (or possibly even all) of this process can be automated by something
like a jenkins bot, requiring very little manual effort.

## Documentation changes

In addition to the github group being formed, documentation should be added
to the [publishing to crates.io docs](http://doc.crates.io/crates-io.html) near
the section about yanking versions. It makes sense that if someone learns how
to publish they should learn how to unpublish (or at least recycle the name).

# Drawbacks
[drawbacks]: #drawbacks

Except for needing to add a small section to the crates.io documentation,
there are no drawbacks.

The solutions in this RFC were specifically chosen as to have no downsides.
All solutions require no technical changes or change of process and do not
affect the power of crate ownership in any way.

# Alternatives
[alternatives]: #alternatives

There are a few alternatives, such as allowing the crates.io admins
themselves the ability to transfer ownership, or creating an automated
process for "abandoned" crates.

All other options would work outside of the currently defined process
that has served rust well and protects it from issues that have plagued
other communities.

# Unresolved questions
[unresolved]: #unresolved-questions

One possible issue is that the "reclaimers" group could use their admin
privileges of abandoned crates to push security vulnerabilities.
Hopefully a technical solution for this can be found (such as a method
to prevent publishing to a version range, i.e. locking a crate so
it can only publish to versions >= 0.3)

A minimum viable product has already been done at
https://github.com/rust-crates/abandoned, which could be
extended to become the full blown implementation or a different target
could be selected, so most questions are already resolved.

