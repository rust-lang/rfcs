- Feature Name: N/A
- Start Date: 2026-03-11
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This RFC proposes adding a number of "reasonable changes" to the current crates.io deletion policy that will allow the deletion of several "reasonable to delete" crates which currently do not qualify.

See the [guide-level explanation](#guide-level-explanation) for the full list.

## Motivation
[motivation]: #motivation

[RFC 3660](https://rust-lang.github.io/rfcs/3660-crates-io-crate-deletions.html) currently details the requirements for deleting a crate which seem very reasonable:

* The crate has been published for less than 72 hours,
* or if all the following conditions are met:
    * The crate has a single owner,
    * The crate is not depended upon by any other crate on crates.io (i.e. it has no reverse dependencies),
    * The crate has been downloaded less than 100 times for each month it has been published.

However, there are a few obvious exceptions that could be added to this, based upon the RFC author's experience with attempting to delete some very old crates that should meet these requirements.

Specifically, those crates are:

* [bow](https://crates.io/crates/bow), which has been broken due to an incompatability hazard for almost a decade
* [value](https://crates.io/crates/value), which has a single reverse-dependency through a yanked version that almost certainly used it due to a typo

The additions from this RFC are directly inspired by these two examples, although they are also designed to apply as broadly as possible.

## Guide-level explanation (Proposal)
[guide-level-explanation]: #guide-level-explanation

The crate deletion criteria receives the following change, specifically regarding reverse dependencies which are considered by the deletion criteria:

* Yanked versions which are over a year old are not considered as valid reverse dependencies. If the depended crate passes the deletion criteria and gets deleted, these yanked versions may also get deleted.
* If a version of a crate has been failing to compile on the latest stable for over 4 years (~1 edition cycle), it may be yanked without the approval of the crate author. Since these versions are necessarily at least four years old, they also satisfy the requirements for the above criteria.

The full deletion criteria becomes:

* The crate has been published for less than 72 hours,
* or if all the following conditions are met:
    * The crate has a single owner,
    * The crate has been downloaded less than 100 times for each month it has been published,
    * The crate is not depended upon by any other crate on crates.io (i.e. it has no reverse dependencies):
        * The only crate versions that directly depend on the to-be-deleted crate (including dev and build dependencies) were published at least a year ago and have been yanked for at least a month
        * Versions of crates which have failed to compile on stable Rust for approximately four years may be yanked without their authors' approval to help satisfy this requirement

These changes effectively allow for long-broken crates and reverse dependencies due to mistakes to be still valid under the deletion criteria.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section, for lack of anything better, will include extra comments on how these criteria will be implemented in practice.

In particular, the "not compiling on latest stable" requirement is effectively a catch-all for when a crate author is unavailable to yank a version, and would *always* be implemented as a manual override by crates.io staff. Since this kind of requirement would effectively only be used for passing the deletion criteria, it is very likely that the number of requests for this would be low; the requests will likely be even lower now that newer versions of cargo require that the crate compile before publishing, to avoid obvious mistakes.

The "yanked, older than a year" requirement should be easily doable with a PostgreSQL index, and since these reverse-dependents queries are run infrequently, it should be fine to implement into the existing code.

## Drawbacks
[drawbacks]: #drawbacks

The largest drawback is that this complicates the deletion criteria and thus makes it more difficult to understand.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The biggest motivation for this change is that it's simple and relatively uncontroversial. crates.io allows yanking but not unpublishing versions to help be a long-term archive of crate versions: if code was able to work before, it should be able to work now too, even if a version was yanked. However, the deletion criteria was developed explicitly to allow deleting "not very useful" crates, and it comes into obvious friction with yanked versions: a single accidental dependency can ruin a crate's opportunity to delete itself, and similarly, the crate versions that use these dependencies clearly did so by accident.

Cluttering the crates.io namespace with old, undesired crates helps nobody, and while there are obvious downsides to letting anyone delete whatever they want, hopefully, allowing a few more extra cases that preserve intent will be an overall improvement.

## Prior art
[prior-art]: #prior-art

* [`incoherent_fundamental_impls` lint](https://github.com/rust-lang/rust/issues/46205), which was directly linked to `bow` crate as an incompatibility. The crate published an empty latest version to avoid having to keep it on the crater exceptions list.
* [crates.io issue filed for `value` crate](https://github.com/rust-lang/crates.io/issues/12881), where the RFC author learned how a single yanked version depending on your crate can stop deletion entirely.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

## Future possibilities
[future-possibilities]: #future-possibilities

These are definitely not the only possible exceptions to the deletion criteria, although the emphasis on yanking can allow for more deletion opportunities if a creator explicitly yanks a version of a crate to allow this.
