- Feature Name: crates-io-security
- Start Date: 2025-10-27
- RFC PR: [rust-lang/rfcs#3872](https://github.com/rust-lang/rfcs/pull/3872)
- Rust Issue: [rust-lang/crates.io#12507](https://github.com/rust-lang/crates.io/issues/12507)

## Summary

[summary]: #summary

This RFC proposes that crates.io should provide insight into vulnerabilities and unsound
API surface based on the RustSec advisory database.

## Motivation

[motivation]: #motivation

One of the roles that crates.io serves for Rust developers is as a discovery mechanism for library
packages. As such, it is important that users can quickly assess the quality of a given crate,
including security considerations such as unsound code/API or known vulnerabilities.
The RustSec advisory database is a curated database of security advisories for Rust crates,
which tracks known vulnerabilities, unsound code, and maintenance status of crates.

The Rust ecosystem has a culture of having smaller, focused crates with a clear purpose.
As a result, many Rust projects have a large number of dependencies, which increases the
risk of introducing problems in the final artifact via the supply chain of dependencies.
Actively malicious crates (or crate versions) would be one example of these risks; the
crates.io team handles these by deleting them when discovered.

This RFC concerns itself mostly with unintentional vulnerabilities and unsound APIs. An example
from the Java ecosystem is the [Log4Shell] vulnerability in the popular Log4j logging library,
when a widely used package exposed affected services to remote code execution attacks.

The Open Source Security Foundation (OpenSSF) has enumerated [Principles for Package Repository
Security]; while crates.io already addresses many of these, one of these is:

> The package repository warns of known security vulnerabilities in dependencies in the package
> repository UI.

The RustSec advisory database tooling already supports exporting advisories in the OSV format.
Today, crates.io does not display any information about known vulnerabilities or unsound APIs
for a given crate. Devising how best to surface this information across a project dependency
graph is a more complex problem that is outside the scope of this RFC (but see future work).

[Log4Shell]: https://en.wikipedia.org/wiki/Log4j#Log4Shell_vulnerability
[Principles for Package Repository Security]: https://repos.openssf.org/principles-for-package-repository-security.html

## Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

The crates.io website will display information about known vulnerabilities and unsound APIs.
While this information is available today via the RustSec website (including feeds that can
automatically be consumed by tooling), having this information directly on crates.io would
make it accessible and visible to a wider audience.

We want to convey a quick overview of the security status of a crate, and allow users to make informed decisions about whether to use the crate in their projects. Care should be taken to
ensure that the mere existence of past vulnerabilities does not negatively impact the perceived quality of a crate; very popular crates are much more likely to have vulnerabilities reported
against them, simply due to their popularity and the amount of scrutiny.

For example, the UI could be somewhat like this:

> Add a `Security` tab to crate pages. If there are known vulnerabilities for the currently
> selected version, the tab might be highlighted. The Security tab will be there whether there
> are existing advisories for a crate or not. Opening the Security tab for a crate should show
> a list of advisories that affect the crate, including a summary of the issue, a list of
> affected versions, and links to more information.

The way advisories are represented in the crates.io UI will evolve over time based on the
available data and user feedback. This RFC does not mandate a specific UI design.

## Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The RustSec project publishes a number of Rust crates that can be used to parse and query the
advisory database, which can be reused in the crates.io codebase. For now, crates.io will only
display advisories in the UI; we will not be adding API to query RustSec advisories. Downstream
users who want to consume this data can use the RustSec crates and the [advisory-db repository]
directly.

[advisory-db repository]: https://github.com/RustSec/advisory-db

## Drawbacks

[drawbacks]: #drawbacks

The RustSec project is maintained by an independent team of volunteers, so the crates.io Security
tab will be reflecting data that is maintained by what amounts to a kind of third party.
The Leadership Council has an [ongoing discussion] on governance for the Secure Code WG that
governs the RustSec project, which might be relevant to this proposal. Feedback on the RustSec
advisory data can be fed back to the RustSec team via their issue tracker.

Rust developers might be scared off of using crates that have known vulnerabilities, even if
those vulnerabilities are not relevant to their use case, or have been fixed in later versions.
This seems like a reasonable trade-off to me -- we should allow informed users to make decisions
that are best for their projects.

[ongoing discussion]: https://github.com/rust-lang/leadership-council/issues/140

## Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

crates.io is the official package repository for the Rust ecosystem, so sharing important security
context via this interface seems like an effective way to make it accessible to a wide audience.

Widely used tools like [cargo-audit] and [cargo-deny] already provide a way to check for
security-sensitive issues in a Rust project's dependencies, but these tools are opt-in and require
users to be aware of them and to run them. They are also more focused on auditing a project's
existing dependencies rather than helping inform users in the discovery phase.

Alternatively, we might make the RustSec advisory database available directly via cargo. This
seems mostly unrelated to what crates.io does, and seems like an interesting future possibility.

[cargo-audit]: https://crates.io/crates/cargo-audit
[cargo-deny]: https://crates.io/crates/cargo-deny

## Prior art

[prior-art]: #prior-art

Neither npm nor PyPI currently seem to provide support for displaying security advisories.

[lib.rs], the opinionated alternative crate index, does have an [audit page] that shows
both RustSec advisories and reviews from [cargo-crev] and [cargo-vet].

[lib.rs]: https://lib.rs/
[audit page]: https://lib.rs/crates/tokio-tar/audit
[cargo-crev]: https://github.com/crev-dev/cargo-crev
[cargo-vet]: https://github.com/mozilla/cargo-vet

## Unresolved questions

[unresolved-questions]: #unresolved-questions

This seems like a relatively straightforward feature with a limited scope. The main questions
are about the desirability of the feature, the implementation approach, and the governance
of the source data.

## Future possibilities

[future-possibilities]: #future-possibilities

In the future, it would be valuable if lockfile updates exposed open vulnerabilities in a
project's dependency graph in the Cargo CLI, for example on `cargo update` or `cargo check`.
crates.io doesn't necessarily have good access to a project's dependency graph, so a simple
implementation would be limited to direct dependencies, which limits its usefulness.

crates.io could extend its existing API to query advisories for a given crate.

`SECURITY.md` files are often used to communicate a project's security policies. crates.io
could surface the contents of these files on the new Security page. However, `SECURITY.md`
files commonly live in the repository root, which is often a crate workspace, and thus
is not directly associated with a specific crate. Some prerequisite work in Cargo would
probably be needed to associate a crate with the relevant `SECURITY.md` file.
