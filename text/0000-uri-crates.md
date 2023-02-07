- Feature Name: `uri_crates`
- Start Date: 2023-02-07
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Introduce crate identification through simplified URIs and introduce crate domains. The URIs are used solely in package configuration files, `Cargo.toml`, and are a limited subset of URIs.

Domains can be registered into crates.io and its ownership is managed by the user. There's no need for graphical user interface in crates.io; just use the `cargo` command for now.

Like crate names, domains have impermanent ownership. So, for example, a crate `jresig_shibuya` can at anytime have its ownership renewed. The same can be said about a domain `www.jresig.com`.

Although URIs can be specified, the crate's standard `name` must still be specified.

# Motivation
[motivation]: #motivation

URIs are an alternative way to clearly specify to what organization or scope a crate belongs. It is **NOT** a way to specify a crate from an external HTTP resource.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A crate package can specify its URI through the `package.uri` field in `Cargo.toml`:

```toml
[package]
uri = "www.jresig.com/shibuya"
```

Once a package is published with an URI, it can be discovered or referred to with an URI:

```toml
[dependencies]
shibuya = { package = "www.jresig.com/shibuya", version = "1.0.0" }
```

When the dependency string contains at least one dot, it is an URI.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

#### URI definition

The `package.uri` field of Cargo.toml is an optional string based in the regular expression `(www.)?[a-z_\-0-9]+(\.[a-z_\-0-9]+)+(/[a-z_\-0-9]+)*` which is similiar to an URI without a scheme, query, hash fragment and `%` octet sequences. In other words, it contains one or more identifiers delimited by dot followed by an optional slash and zero or more identifiers delimited by slash.

The optional "www." prefix is ignored.

#### Dependency

If a dependency string contains at least one dot, the dependency string is an URI. Cargo resolves it as a crate identified by that URI in crates.io

The optional "www." prefix is ignored.

#### Publishing process

An user cannot publish a crate with a specific URI if either its domain does not exist in crates.io, its domain does not belong to the user or the URI is duplicate.

#### crates.io Domains

The `cargo` command allows managing domains from crates.io. For example, it allows:

- searching for existing domains,
- registering domains,
- removing domains,
- transferring domain ownership between users and
- adding other users as owners of a domain.

# Drawbacks
[drawbacks]: #drawbacks

N/A

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Just like crates, domains can be abused in ownership. For example, someone unaffiliated to tokio.rs can register a crate `tokio`. Similiarly with URIs, someone unaffiliated to tokio.rs can register a domain `tokio.rs`.
- Just like crates, domains can change ownership anytime. For example, a crate `foo_bar` can be transferred to another user. Similiarly with URIs, a domain `foo.com` can be transferred to another user.
- URIs are in no way a way to refer to strange or external HTTP resources.
- People have suggested using crates as optional namespaces and allowing crates to have a single slash as separator, where the left name is a namespace. This proposal allows more ellaborated punctuation to refer to a crate and does not affect the existing `package.name` field at Cargo.toml.
- This is not doable in a library or macro, nor does it affect the Rust language. It is a Cargo proposal.

# Prior art
[prior-art]: #prior-art

- Languages like Java support a DNS in their package managers like Maven. They have little similiarity with this proposal, although I'm not entirely familiar with them.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

N/A

# Future possibilities
[future-possibilities]: #future-possibilities

N/A
