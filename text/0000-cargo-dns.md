- Feature Name: `cargo_dns`
- Start Date: 2023-11-10
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC adds support for a organization Domain Name System (DNS) to Cargo.

# Motivation
[motivation]: #motivation

Packages published at a certain domain, either in the form `org::q::p` or in the form `com::q::p`, clarify that `q` is an organization and not a project.
It is predictable that the authors of `com` and `org` are unlikely to introduce their own subpackages to these packages.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The top-level `org` and `com` package namespaces at Cargo are extended to allow organizations to introduce their own crates to them,
and Cargo packages are allowed to contain multiple `::` delimiters in their names. Packages originating from organization namespaces are referred to as *domain-tied* packages.

Names that conflict with existing items of the existing crates `org` and `com` at crates.io result in a compile time error.

Publishing domain-tied packages currently requires that the GitHub user is a member of the domain's organization. The domain's organization
is taken from the manifest's repository URL. Building domain-tied packages locally does not require authentication.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This proposal adds the following functionality to Cargo:

* A package's name is allowed to contain multiple `::` delimiters. The subsequent segment in the name is a subcrate of the previous segment's crate.
* Any user is allowed to publish domain-tied packages; that is, packages belonging the namespace of the existing packages `org` and `com`.
  * The publishing process deduces the domain's organization from the manifest's repository URL.
  * It is a publishing error if the manifest's repository URL does not specify an organization-authored repository but rather a personal-authored repository.
  * It is a publishing error if the authenticated GitHub user is not a member of the organization.

This proposal has the following subtle effects on the language:

* An organization's name that conflicts with an item of the super crate (either `org` or `com`) may result in an ambiguous item resolution if the super crate is in scope.

# Drawbacks
[drawbacks]: #drawbacks

It is unpredictable whether an organization's name might conflict with the existing crates `org` and `com`. Despite this, a conflict is unlikely to happen due to:

* `org` or `com` is not always in scope for every project;
* `org` or `com` export a small number of modules.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* This design may easily integrate with the existing *Packages as (optional) Namespaces* feature.
* The symbol `@` as an alternative to `com::` and `org::` introduces extra changes to the filesystem structure of the recipes directory.
* This is not an urgent or necessary feature currently.
* This proposal applies to Cargo and interacts with the *Packages as (optional) Namespaces* feature.

# Prior art
[prior-art]: #prior-art

* Domains are common in the Java and ActionScript languages. The Maven package manager supports a domain name system as well.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

N/A

# Future possibilities
[future-possibilities]: #future-possibilities

* Author domains such as `me::john` (based on `https://john.me`).
