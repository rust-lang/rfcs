- Feature Name: crates_io_user_namespaces
- Start Date: 2020-08-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes extending the crates.io package naming format to allow
per-user namespaces. A crates.io user Jane Doe signed in with the GitHub
account `"jdoe"` would be able to publish crates with names prefixed by
`"~jdoe/"` Tooling that operates on crates.io package names, such as docs.rs,
will recognize this syntax and drop the prefix to compute the crate name.

# Motivation
[motivation]: #motivation

crates.io currently places all registered packages in a single flat namespace.
This simplifies some aspects of package discovery, but adds friction for users
who prefer to name packages after their purpose rather than use an opaque
codename. Additionally, due to the large number of registered packages, it is
often difficult to find a name that is short enough to be ergonomic when
used as a symbol in Rust source code.

Introducing per-user namespaces will allow every crates.io user to upload packages without worrying about crate name conflicts.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

TODO: Is a guide-level explanation needed at all? There's already documentation
on how to define dependencies in Cargo where the package name is different from
the crate name, and this isn't adding any new capabilities.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The crates.io package name validation would be relaxed to allow package names
starting with a namespace prefix. This RFC defines the "user" namespace prefix
format. Given a user with a crates.io profile at `"https://crates.io/users/X"`,
their user namespace prefix is `"~X/"`. Existing rules regarding allowed
character set, minimum/maximum crate length, and reserved crate names continue
to apply to the crate name component of a namespaced package name.

The syntax and allowed character set of the username portion is the same as for
crates.io usernames.

If a user requests that their username be changed (e.g. via a support request),
then any existing package uploads under their old user namespace will be
redirected to their new username. This RFC does not specify whether new versions
would be available under the old package name, and recommends the crates.io
team decide that based on expected support load.

# Drawbacks
[drawbacks]: #drawbacks

In previous discussions regarding whether to allow user namespaces, some
members of the community have objected to them on pragmatic, philosophical,
or aesthetic grounds. Common objections include:

* User namespaces are a new concept. Authors of crates.io packages would need
  to decide whether or not they wanted to upload packages with a prefixed name.
	Community tooling such as docs.rs would need to be updated to support
	namespaced package names.

* User namespaces would allow authors to upload crates with "boring" names.
  For example, if Jane Doe wanted to upload her own async HTTP library to share
	with the community, she would be allowed to name it `"~jdoe/async-http"`.
	Without namespaces she might instead choose a fanciful code name such as
	`"hapaxanthous"`, a suffixed name such as `"async-http-37"`, or a globally
	unique identifier such as `"d6a0ebfe-e96b-404b-8611-d0a62abfe3e2"`.

I feel that the value of reducing barriers to package uploads is sufficient
to justify the effort involved in updating tooling and socializing the
acceptance of non-unique crate names.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Team namespaces are out of scope

Previous proposals to add namespacing to crates.io have usually focused on
team or organization namespaces, where a project like Iron could have
the namespace prefix `"@iron/"` to improve discoverability and add a layer of
access control. For this RFC, team namespaces are out of scope -- the new
namespaces are an MVP that covers individual users only.

## Namespaces are optional

This RFC proposes optional namespaces. Authors would not be _required_ to
upload packages with a namespace, because doing so would be disruptive to
the existing crates.io community. It takes no position on the issue of
"name squatting".

# Prior art
[prior-art]: #prior-art

A non-exhaustive set of Rust Internals discussion topics covering crates.io
namespaces:

* [Crates.io package policies](https://internals.rust-lang.org/t/crates-io-package-policies/1041) (steveklabnik, 2014-12-19)
* [[Pre-RFC]: Packages as Namespaces](https://internals.rust-lang.org/t/pre-rfc-packages-as-namespaces/8628) (ethanpailes, 2018-10-20)
* [[Pre-RFC] Domains as namespaces](https://internals.rust-lang.org/t/pre-rfc-domains-as-namespaces/8688) (soc, 2018-10-27)
* [Namespacing on Crates.io](https://internals.rust-lang.org/t/namespacing-on-crates-io/8571) (naftulikay, 2018-12-16)
* [Scoped packages (like in npm)?](https://internals.rust-lang.org/t/scoped-packages-like-in-npm/10223) (dpc, 2019-05-21)
* [[Pre-RFC] [idea] Cratespaces (crates as namespace, take 2… or 3?)](https://internals.rust-lang.org/t/pre-rfc-idea-cratespaces-crates-as-namespace-take-2-or-3/11320) (samsieber, 2019-11-19)
* [[Pre-RFC]: Author Attached Crates-io Names](https://internals.rust-lang.org/t/pre-rfc-author-attached-crates-io-names/11656) (luojia, 2020-01-20)
* [Pre-RFC: User namespaces on crates.io](https://internals.rust-lang.org/t/pre-rfc-user-namespaces-on-crates-io/12851) (jmillikin, 2020-08-07)

Package registries for other languages exist with both namespaced and
non-namespaced formats. It is difficult to determine which option is objectively
better, but I do note that registries without namespacing (most notably NPM and
PyPI) have materially higher levels of name-related community conflicts than
namespaced registries such as Maven.

One language that appears to have completely avoided the issue of packge naming
conflicts is Go, which permits registration of any valid URL in the Go package
index. Many Go package authors use implicit user namespacing via free shared
hosting (e.g. GitHub).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Prefixed usernames

Crates.io does not currently distinguish usernames based on identity provider,
and only supports a single such provider: GitHub. A GitHub user `"@jdoe"`
receives the crates.io user URL `"https://crates.io/users/jdoe"` on signup.

If crates.io were to add support for additional identity providers in the
future, then it would be possible for two different individuals to register
the same account name under different supported identity providers. In that
case the ownership of user URLs and namespaces would be ambiguous.

As an alternative, it would be possible to anticipate a new username format and
use that for user namespaces. A simple solution would be to reuse the prefixes
from teams, so that GitHub user `"@jdoe"` receives the user namespace
`"~github:jdoe/"`. It is recommended, but not mandatory, that the format of
user URLs be updated to match.

## Opaque usernames

A second alternative username format, if complete independence from the identity
provider is desired, uses an opaque identifier for the prefix. In that case
Jane Doe's async HTTP package might be named `"u12345/async-http"`.

In this format package names would be more difficult to remember, but would
be durable against username changes or even large-scale changes to crates.io
identity semantics.

# Future possibilities
[future-possibilities]: #future-possibilities

## Team namespaces

If team namespaces are desired in the future, I recommend they match the syntax
of crates.io team URLs and use `"@"` as the sigil. For example, the team
`"https://crates.io/teams/github:rust-lang:libs"` would be allowed to upload
packages under the namespace `"@github:rust-lang:libs/"`. This is syntatically
similar to other organization- or project-specific namespacing schemes, for
example NPM's "package scopes".

## Special prefixes

It may be desirable to reserve the special prefix `"@/"` to refer unambiguously
to non-namespaced package names.

It may also be desirable to reserve the special prefix `"@rust/"` so that
future additions to the Rust core libraries (`core`, `alloc`, etc) can be
named without any risk of conflicting with existing third-party packages.
