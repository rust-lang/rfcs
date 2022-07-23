# RFC: Make the authors field optional

- Feature Name: `optional_authors_field`
- Start Date: 2021-01-07
- RFC PR: [rust-lang/rfcs#3052](https://github.com/rust-lang/rfcs/pull/3052)
- Rust Issue: [rust-lang/rust#83227](https://github.com/rust-lang/rust/issues/83227)

# Summary
[summary]: #summary

This RFC proposes to make the `package.authors` field of `Cargo.toml` optional.
This RFC also proposes preventing Cargo from auto-filling it, allowing crates
to be published to crates.io without the field being present, and avoiding
displaying its contents on the crates.io and docs.rs UI.

# Motivation
[motivation]: #motivation

The crates.io registry does not allow users to change the contents of already
published versions: this is highly desirable to ensure working builds don't
break in the future, but it also has the unfortunate side-effect of preventing
people from updating the list of crate authors defined in `Cargo.toml`'s
`package.authors` field.

This is especially problematic when people change their name or want to remove
their name from the Internet, and the crates.io team doesn't have any way to
address that at the moment except for deleting the affected crates or versions
altogether. We don't do that lightly, but there were a few cases where we were
forced to do so.

The contents of the field also tend to scale poorly as the size of a project
grows, with projects either making the field useless by just stating "The
$PROJECT developers" or only naming the original authors without mentioning
other major contributors.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

crates.io will allow publishing crates without the `package.authors` field, and
it will stop showing the contents of the field in its UI (the current owners
will still be shown). docs.rs will also replace that data with the crate
owners.

`cargo init` will stop pre-populating the field when running the command, and
it will not include the field at all in the default `Cargo.toml`. Crate authors
will still be able to manually include the field before publishing if they so
choose.

Crates that currently rely on the field being present (for example by reading
the `CARGO_PKG_AUTHORS` environment variable) will have to handle the field
being missing (for example by switching from the `env!` macro to
`option_env!`).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this RFC spans multiple parts of the Rust project:

## Cargo

Cargo will stop fetching the current user's name and email address when running
`cargo init`, and it will not include the field in the default template for
`Cargo.toml`.

## crates.io

crates.io will allow publishing versions without the field and with the field
empty. The Web UI will remove the authors section, while retaining the current
owners section.

The API will continue returning the `authors` field in every endpoint which
currently includes it, but the field will always be empty (even if the crate
author manually adds data to it). The database dumps will also stop including
the field.

## docs.rs

docs.rs will replace the authors with the current owners in its UI.

# Drawbacks
[drawbacks]: #drawbacks

Cargo currently provides author information to the crate via
`CARGO_PKG_AUTHORS`, and some crates (such as `clap`) use this information.
Making the authors field optional will require crates to account for a missing
field if they want to work out of the box in projects without the field.

This RFC will make it harder for third-party tools to query the author
information of crates published to crates.io.

By design, this RFC discourages adding the metadata allowing to know historical
crate authors and makes it harder to retrieve it. In some cases, crate authors
may have wanted that information preserved. After this RFC, crate authors who
want to display historical authors who are not current crate owners will have
to present that information in some other way.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC reduces the problems related to changing the names in the authors
field significantly, as people will now have to explicitly want to add that
data instead of it being there by default.

We could do nothing, but that would increase the support load of the crates.io
team and would result in more crates being removed from the registry due to
this issue.

# Prior art
[prior-art]: #prior-art

* **JavaScript:** `package.json` has an optional `authors` field, but it's not
  required and the interactive `npm init` command does not prepopulate the
  field, leaving it empty by default. The npm Web UI does not show the contents
  of the field.
* **Python:** `setup.py` does not require the `authors` field. The PyPI Web UI
  shows its contents when present.
* **Ruby:** `*.gemspec` requires the `authors` field, and the RubyGems Web UI
  shows its contents.
* **PHP:** `composer.json` has an optional `authors` field. While it's not
  required, the interactive `composer init` command allows you to choose
  whether to pre-populate it based on the current environment or skip it. The
  Packagist Web UI does not show the contents of the field.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* What should we do about the metadata in already published crates?

# Future possibilities
[future-possibilities]: #future-possibilities

The `package.authors` field could be deprecated and removed in a future
edition.

A future RFC could propose separating metadata fields that could benefit from
being mutable out of `Cargo.toml` and the crate tarball, allowing them to be
changed without having to publish a new version. Such RFC should also propose a
standardized way to update and distribute the extracted metadata.
