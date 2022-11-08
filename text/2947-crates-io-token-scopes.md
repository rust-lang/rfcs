# crates.io token scopes RFC

- Feature Name: `crates_io_token_scopes`
- Start Date: 2020-06-24
- RFC PR: [rust-lang/rfcs#2947](https://github.com/rust-lang/rfcs/pull/2947)
- crates.io issue: [rust-lang/crates.io#5443](https://github.com/rust-lang/crates.io/issues/5443)

# Summary
[summary]: #summary

This RFC proposes implementing scopes for crates.io tokens, allowing users to
choose which endpoints the token is allowed to call and which crates it's
allowed to affect.

# Motivation
[motivation]: #motivation

While the current implementation of API tokens for crates.io works fine for
developers using the `cargo` CLI on their workstations, it's not acceptable for
CI scenarios, such as publishing crates automatically once a git tag is pushed.

The implementation of scoped tokens would allow users to restrict the actions a
token can do, decreasing the risk in case of automation bugs or token
compromise.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

During token creation, the user will be prompted to select the permissions the
token will have. Two sets of independent scopes will be available: the
endpoints the token is authorized to call, and the crates the token is allowed
to act on.

## Endpoint scopes

The user will be able to choose one or more endpoint scopes. This RFC proposes
adding the following endpoint scopes:

* **publish-new**: allows publishing new crates
* **publish-update**: allows publishing a new version for existing crates the
  user owns
* **yank**: allows yanking and unyanking existing versions of the user's crates
* **change-owners**: allows inviting new owners or removing existing owners
* **legacy**: allows accessing all the endpoints on crates.io except for
  creating new tokens, like tokens created before the implementation of this
  RFC.

More endpoint scopes might be added in the future without the need of a
dedicated RFC.

Tokens created before the implementation of this RFC will default to the legacy
scope.

## Crates scope

The user will be able to opt into limiting which crates the token can act on by
defining a crates scope.

The crates scope can contain a list of crate name patterns the token can
interact with. Crate name patterns can either be regular crate names or they
can end with a `*` character to match zero or more characters.

For example, a crate name pattern of `lazy_static` will only make the token
apply to the corresponding crate, while `serde*` allows the token to act on
any present or future crates starting with `serde` (including `serde` itself),
but only if the user is an owner of those crates.

The crates scope will allow access to all present and future crates matching
it. When an endpoint that doesn't interact with crates is called by a token
with a crates scope, the crates scope will be ignored and the call will be
authorized, unless limited by an endpoint scope (see above).

Tokens created before the implementation of this RFC will default to an empty
crate scope filter (equivalent to no restrictions).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Endpoint scopes and crates scope are two completely separate systems, and can be
used independently of one another.

Token scopes will be implemented entirely on the crates.io side, and there will
be no change necessary to `cargo` or alternate registries.

## Endpoint scopes

The scopes proposed by this RFC allow access to the following endpoints:

| Endpoint                                 | Required scope     |
|------------------------------------------|--------------------|
| `PUT /crates/new` (new crates)           | **publish-new**    |
| `PUT /crates/new` (existing crates)      | **publish-update** |
| `DELETE /crates/:crate_id/:version/yank` | **yank**           |
| `PUT /crates/:crate_id/:version/unyank`  | **yank**           |
| `PUT /crates/:crate_id/owners`           | **change-owners**  |
| `DELETE /crates/:crate_id/owners`        | **change-owners**  |
| everything except `PUT /me/tokens`       | **legacy**         |

Removing an endpoint from a scope or adding an existing endpoint to an existing
scope will be considered a breaking change. Adding newly created endpoints to
an existing scope will be allowed only at the moment of their creation, if the
crates.io team believes the new endpoint won't grant more privileges than the
existing set of endpoints in that scope.

## Crates scope

The patterns will be evaluated during each API call, and if no match is found
the request will be denied. Because it's evaluated every time, a crates scope
will allow interacting with matching crates published after token creation.

The check for the crates scope is separate from crate ownership: having a scope
that technically permits to interact with a crate the user doesn't own will be
accepted by the backend, but a warning will be displayed if the pattern doesn't
match any crate owned by the user.

# Drawbacks
[drawbacks]: #drawbacks

No drawbacks are known at the time of writing the RFC.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

An alternative implementation for endpoint scopes could be to allow users to
directly choose every endpoint they want to allow for their token, without
having to choose whole groups at a time. This would result in more granularity
and possibly better security, but it would make the UX to create new tokens way
more complex (requiring more choices and a knowledge of the crates.io API).

An alternative implementation for crate scopes could be to have the user select
the crates they want to allow in the UI instead of having to write a pattern.
That would make creating a token harder for people with lots of crates (which,
in the RFC author's opinion, are more likely to need crate scopes than a person
with just a few crates), and it wouldn't allow new crates matching the pattern
but uploaded after the token's creation from being accessed.

Finally, an alternative could be to do nothing, and encourage users to create
"machine accounts" for each set of crates they own. A drawback of this is that
GitHub's terms of service limit how many accounts a single person could have.

# Prior art
[prior-art]: #prior-art

The endpoint scopes system is heavily inspired by GitHub, while the rest of the
proposal is similar to nuget. Here is how popular package registries implements
scoping:

* [nuget] (package registry for the .NET ecosystem) implements three endpoint
  scopes (publish new packages, publish new versions, unlist packages), has a
  mandatory expiration and supports specifying which packages the token applies
  to, either by checking boxes or defining a single glob pattern.
  [(documentation)][nuget-docs]
* [npm] (package registry for JavaScript) implements a binary
  "read-only"/"read-write" permission model, also allowing to restrict the IP
  ranges allowed to access the token, but does not allow restricting the
  packages a token is allowed to change. [(documentation)][npm-docs]
* [pypi] (package registry for Python) only implements the "upload packages"
  permission, and allows to scope each token to a *single* package.
  [(documentation)][pypi-docs]
* [rubygems] (package registry for Ruby) and [packagist] (package registry for
  PHP) don't implement any kind of scoping for the API tokens.

[nuget]: https://www.nuget.org/
[nuget-docs]: https://docs.microsoft.com/en-us/nuget/nuget-org/scoped-api-keys
[npm]: https://www.npmjs.com
[npm-docs]: https://docs.npmjs.com/creating-and-viewing-authentication-tokens
[pypi]: https://pypi.org
[pypi-docs]: https://pypi.org/help/#apitoken
[rubygems]: https://rubygems.org/
[packagist]: https://packagist.org/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Are there more scopes that would be useful to implement from the start?
* Is the current behavior of crate scopes on endpoints that don't interact with
  crates the best, or should a token with crate scopes prevent access to
  endpoints that don't act on crates?

# Future possibilities
[future-possibilities]: #future-possibilities

A future extension to the crates.io authorization model could be adding an
optional expiration to tokens, to limit the damage over time if a token ends up
being leaked.

Another extension could be an API endpoint that programmatically creates
short-lived tokens (similar to what AWS STS does for AWS Access Keys), allowing
to develop services that provide tokens with a short expiration time to CI
builds. Such tokens would need to have the same set or a subset of the parent
token's scopes: this RFC should consider that use case and avoid the
implementation of solutions that would make the check hard.

To increase the security of CI environments even more, we could implement an
option to require a separate confirmation for the actions executed by tokens.
For example, we could send a confirmation email with a link the owners have to
click to actually publish the crate uploaded by CI, preventing any malicious
action with stolen tokens.

To remove the need for machine accounts, a future RFC could propose adding API
tokens owned by teams, granting access to all resources owned by that team and
allowing any team member to revoke them.
