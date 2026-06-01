- Feature Name: `crate-quarantine`
- Start Date: 2023-07-25
- RFC PR: [rust-lang/rfcs#3464](https://github.com/rust-lang/rfcs/pull/3464)

# Summary
[summary]: #summary

Add an off-by-default quarantine system to crates.io that can be used to prevent crate versions from being immediately published to the crate index based on rules defined by the crates.io team.

# Motivation
[motivation]: #motivation

Packaging ecosystems aren't warm, friendly places any more. PyPI has repeatedly been forced to deal with attacks of spam[^pypi-2021-03][^pypi-2021-06] and malware[^pypi-2023-05]. In March 2023, an analysis indicated that more than half the packages uploaded to NPM were SEO spam[^npm-2023-03], and vendors such as jFrog have previously found significant numbers of malware packages on NPM[^npm-malware].

So far, crates.io has been relatively fortunate to only have isolated incidents[^rustdecimal], but the increasing popularity of Rust only makes it more and more likely that we will also be the target of similar spam and malware attacks.

There are mitigating factors at play: in particular, our reliance on GitHub for authentication, while problematic from purist FOSS and ecosystem diversity perspectives, means that we benefit from GitHub's machinery in terms of detecting bad actors.

Regardless, we need to be able to prevent malicious packages from being uploaded, both in emergency situations (for example, mitigating spam attacks) and in more mundane cases where we can confidently identify a potentially malicious user. This RFC proposes an off-by-default quarantine approach that allows us to examine packages before they are published when it is considered warranted.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This project will add a new admin feature to crates.io that allows crate publishes matching specific properties to be placed into a quarantine state instead of being immediately published.

A version in the quarantined state will have its `.crate` file published to S3 to simplify review, and will appear in crates.io API responses as an unpublished crate, but will _not_ be added to the index — and will therefore _not_ be installable via `cargo` — until it has been reviewed and approved.

`cargo publish` will also be modified to report when an uploaded crate version has been placed into quarantine. 

Once a crate version has been quarantined, then a member of the crates.io team will review the quarantined crate, ideally within 1 week[^sla]. Once reviewed, the crate may be either approved and published to the index, or rejected and deleted. Either way, the crate owner(s) will be notified by e-mail, assuming crates.io has a verified e-mail available for their account.

Should the crate owner(s) disagree with the quarantine action, they may appeal to the moderation team.

## When will a quarantine rule be added?

Ultimately, this will be decided by the crates.io team on a case by case basis. There are two primary scenarios that are currently envisioned where this functionality would be used:

### Large scale spam attack

In the event of a large scale spam attack on the entire site, we may want to either restrict individual users (if the spam is coming from a finite set of users) or quarantine all published versions temporarily (using a `.*` crate name rule) until it subsides, at which point we can analyse which crate versions are actually valid.

### Malicious users

As an alternative to locking or banning a user, when we have high confidence (due to user reports or automated scanning) that a user is uploading malicious crates then a rule may be added for that specific user until the crates.io team is able to communicate with them.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The initial implementation of this RFC will only allow for rules on the crate name or a specific user. Further rule types may be added in the future by the crates.io team as required.

## crates.io

Most changes will need to occur on crates.io. They are described below.

### Database

New tables will need to be added to capture quarantine rules on a per-crate and per-user basis. Crates will be matched by matching their name against a regular expression in each rule; users will be matched by their external user ID[^uid].

The `versions` table will need a new `quarantined` boolean flag, much like the existing `yanked` flag.[^state]

### Publishing

The crates.io database will be expanded to store and capture rules that, if matched, will cause a crate version that is being published to enter the `quarantined` state, rather than being immediately published. These rules will be defined and maintained by crates.io admins.

The publish endpoint will notify the publisher (generally `cargo publish`) of the quarantined version by extending the response[^publish-response] in two ways:

1. A new field will be added to the response called `quarantined`.
2. A human readable warning will be added to the `warnings` field within the response.

When a crate version is quarantined, a support ticket will also be opened within the crates.io help desk, thereby notifying the crates.io team of the newly quarantined version. If additional crates are quarantined for that user or regex, a comment will be added to that ticket, if it is still open. (This prevents the help desk being spammed.)

### API

A new field will be added to API endpoints that return crate versions called `published`. This will only be `false` for crate versions that are in quarantine.

### Admin interface

The crates.io admin console will be extended to provide the following new functionality:

#### Rule definition

A new CRUD area will be provided for rules to be defined and managed.

#### Review

A new review area will be added that lists crate versions currently in quarantine, along with options to download the `.crate` file and manage the version (ie by approving or rejecting the crate version).

## `cargo publish`

The wait loop in `cargo publish`[^wait-loop] currently refreshes the crate index and waits for the new crate version to appear.

This will be extended to check the `quarantined` flag in the response; if present and `true`, `cargo publish` will not wait and will immediately report the warning(s) in the response to the user.

# Drawbacks
[drawbacks]: #drawbacks

## Project team member time

The major drawback here is time: an actual human is going to need to be able to review any quarantined crate versions, which will require both expertise and availability. A crate being quarantined will almost certainly frustrate its owner(s), no matter what communication is provided, and the more time that is required to review the crate, the worse that experience will be.

In the near term, it is unlikely that quarantine will be used in any day-to-day manner. The intention is to provide a break-glass-in-case-of-emergency option more than something that is in continuous use, but if this is used more frequently in normal operation, then workload will quickly become a concern.

## Denial of service vectors

This proposal addresses the needs of a Rust user — who doesn't want to inadvertently download and use a compromised or malicious crate — but does little to prevent denial of service on crates.io: both technically (since most publication steps will still take place for a quarantined crate version), and socially (since a human has to review the crate version).

Overall, this is still an improvement in ecosystem security (since it can be used to prevent the index including malware), but it's not an all-encompassing magic solution to all our ecosystem security woes.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The main alternative approach that would be available would be to, in essence, do nothing. Our current process is to handle malware and spam after crates are published, and if we needed to cut off a deluge of spam, we would be able to do so at the webserver level. It can be argued that this may be sufficient, given the relatively low level of incidents so far.

Were we to keep an approach of dealing with issues ex post facto, we could adopt some of the work PyPI is doing around allowing trusted reporters to flag problematic crates.[^pypi-malware]

Another possible approach would be to require review of all new crates before they are published, but it's unlikely we have the bandwidth to handle that within the Rust Project.

If we decide to accept this RFC and provide a mechanism for attempting to quell malware and spam before crates are published, then there aren't a lot of other ways to implement this — ultimately, that decision has to be made as `cargo publish` is run, and there's no real scenario where this can be done in a purely client-side manner. (Indeed, we could technically implement this purely in crates.io, but the user experience of having `cargo publish` simply time out while waiting for the crate version to appear in the index would be suboptimal.)

# Prior art

Python and Node have both been notably subject to attacks in recent years.

## Python

PyPI has a list of prohibited words that cannot be used in package names[^pypi-prohibited], but no other quarantine or moderation functionality other than ex post facto deletion of packages in response to reports.

## Node

Unlike PyPI with its Warehouse project, the NPM registry isn't open source. There is, however, no indication of a pre-moderation or quarantine system in their public documentation.

# Future possibilities
[future-possibilities]: #future-possibilities

## Allowing trusted reporters to flag problematic users and/or crates

The Python Software Foundation is starting a project to improve malware detection[^pypi-malware], with one of its key pillars being a system where trusted reporters are allowed to make reports that will feed directly into PyPI's triage system.

Depending on the outcome of that project, we may want to consider a similar approach that can feed into quarantine rules.

## Identifying malicious packages automatically

There is considerable existing tooling available — both in the open source and commercial worlds — to identify malware in an automated manner. In the future, we may want to consider adding low latency, low cost checks to the publishing pipeline in the future that could be used to divert potentially malicious crates into quarantine without an existing user or crate rule.

[^npm-2023-03]: https://blog.sandworm.dev/one-in-two-new-npm-packages-is-seo-spam-right-now
[^npm-malware]: https://jfrog.com/blog/malware-civil-war-malicious-npm-packages-targeting-malware-authors/
[^publish-response]: https://github.com/rust-lang/crates.io/blob/63f319a3f9999749be6db924d9f957beec22d3c3/src/controllers/krate/publish.rs#L280-L283
[^pypi-2021-03]: https://www.zdnet.com/article/pypi-gitlab-dealing-with-spam-attacks/
[^pypi-2021-06]: https://heimdalsecurity.com/blog/pypi-repository-deluged-with-spam-packages-and-pirated-movie-links/
[^pypi-2023-05]: https://status.python.org/incidents/qy2t9mjjcc7g
[^pypi-malware]: https://discuss.python.org/t/pypi-malware-detection-project/28222
[^pypi-prohibited]: https://github.com/pypi/warehouse/blob/ffbc61006cb0427c1593eceb81b234092d2ea95d/warehouse/utils/project.py#L102-L110
[^rustdecimal]: https://blog.rust-lang.org/2022/05/10/malicious-crate-rustdecimal.html
[^sla]: 1-2 weeks was suggested. Initially, I chose the more conservative option, but most feedback trended towards a shorter period being more appropriate from the perspective of the crate uploader. We may need to revisit this in the longer term if this is an undue burden on the crates.io team.
[^state]: Or this may be combined into a single state field, but that can be discussed when implementing.
[^uid]: Today, that would be their GitHub user ID, but in the future this could be expanded to other code hosts that are supported for authentication.
