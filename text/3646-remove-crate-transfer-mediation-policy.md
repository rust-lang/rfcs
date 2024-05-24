- Feature Name: `remove_crate_transfer_mediation_policy`
- Start Date: 2024-05-24
- RFC PR: [rust-lang/rfcs#3646](https://github.com/rust-lang/rfcs/pull/3646)
- Rust Issue:

# Summary
[summary]: #summary

The [crates.io package ownership policies currently state](https://crates.io/policies#package-ownership):

> If you want to take over a package, we require you to first try and contact the current owner
> directly. If the current owner agrees, they can add you as an owner of the crate, and you can
> then remove them, if necessary. If the current owner is not reachable or has not published any
> contact information the crates.io team may reach out to help mediate the process of the ownership
> transfer.

The crates.io team would like to remove the final sentence in this paragraph and stop attempting to
mediate ownership transfer of crates.

# Motivation
[motivation]: #motivation

As the number of crates on crates.io grows, so do the number of effectively abandoned crates, and
so do the number of support requests we get asking us to attempt to contact a crate owner to see if
they would be willing to transfer their crate. Managing these requests take time, and they aren't
even usually successful. The crates.io team would like to spend their time working on the site
rather than providing this crate mediation service.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

If someone wants a crate name that is currently in use, and their efforts to either find contact
information for or get a response from the current owner have been unsuccessful, they will need to
pick a different name for their crate. Any requests to the crates.io team to mediate will be
declined.

# Drawbacks
[drawbacks]: #drawbacks

Some crate transfers that would have happened with the help of the crates.io team will not happen,
which could lead to churn in the ecosystem of finding and switching to a new crate that could have
been evolution of an existing crate. It is unclear if the number of successful transfers is an
amount that is significant enough to justify the time spent by the crates.io team.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Request ownership directly through the crates.io website

Alternatively, crates.io could build a mechanism into crates.io to allow one user to request a
crate from another user without exposing email addresses. However, this would require significant
design and complex implementation to prevent abuse such as a mob of people all requesting transfer
of the same crate as a harassment vector. That engineering effort is best spent elsewhere.

It's also unclear if current users have consented to be contacted by anyone who uses crates.io.
[The privacy policy](https://foundation.rust-lang.org/policies/privacy-policy/#crates.io) currently
states:

> We [the Rust Foundation and the crates.io team] will only use your email address to contact you
> about your account.

Given that ambiguity, we feel that any contact feature would need to be opt-in, limiting the
possible utility even further.

## Separate committee for crate ownership adjudication

[eRFC #2614](https://github.com/rust-lang/rfcs/pull/2614) proposed to establish a separate
committee to make decisions regarding crate ownership, which eventually would face the same
problems of bandwidth and burnout as the number of requests increases.

# Prior art
[prior-art]: #prior-art

- [PyPI](https://pypi.org/) has policies under [PEP 541](https://peps.python.org/pep-0541/) and [they are not able to keep up with the requests](https://github.com/pypi/support/issues?q=is%3Aissue+is%3Aopen+pep+541).
- [npm has a dispute resolution process](https://docs.npmjs.com/policies/disputes) but it is ["not available for dispute requests due to lack of activity related to a specific name"](https://docs.npmjs.com/policies/disputes#when-not-to-use-this-process).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- None known at this time

# Future possibilities
[future-possibilities]: #future-possibilities

- None known at this time
