- Feature Name: style-evolution
- Start Date: 2022-10-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC defines a mechanism for evolving the default Rust style over time
without breaking backwards compatibility, via the Rust edition mechanism.

# Motivation
[motivation]: #motivation

The current Rust style, as defined in the Rust Style Guide and as implemented
by rustfmt, has some stability expectations associated with it. In particular,
many projects implement continuous integration hooks that verify the style of
Rust code (such as with `cargo fmt --check`), and changes to the style would
break the CI of such projects, in addition to causing churn.

This document proposes to evolve the current Rust style, without breaking
backwards compatibility, by tying style evolution to Rust edition. Code in Rust
2015, 2018, or 2021 will use the existing default style. Code in future
editions (Rust 2024 and onwards) may use a new style edition.

This RFC only defines the mechanism by which we evolve the Rust style; this RFC
does *not* define any specific style changes. Future RFCs or style-guide PRs
will define future style editions. This RFC does not propose or define any
specific future style editions or other formatting changes.

# Explanation
[explanation]: #explanation

`rustfmt`, and `cargo fmt`, will format code according to the default Rust
style. The default Rust style varies by Rust edition. (Not every edition
changes the Rust style, and thus some editions have identical default styles;
Rust 2015, 2018, and 2021 all have the same default style.)

By default, `rustfmt` and `cargo fmt` will use the same edition that the Rust
code itself is configured to use. `cargo fmt` will pass `rustfmt` the edition
specified in `Cargo.toml`; for direct invocation of `rustfmt`,
`rustfmt.toml`/`.rustfmt.toml` can also specify the `edition`.

However, when transitioning between editions, projects may want to separately
make and commit the changes for 1) transitioning to a new Rust edition and 2)
transitioning to a new style edition. To allow for this, `rustfmt` also allows
configuring the style edition directly, via a separate `style_edition`
configuration option, or `--style-edition` command-line option.

Note that rustfmt may not necessarily support all combinations of Rust edition
and style edition; in particular, it may not support using a style edition that
differs by more than one step from the Rust edition.

The current version of the style guide will describe the latest Rust edition.
Each distinct past style will have a corresponding archived version of the
style guide. Note that archived versions of the style guide may not necessarily
document formatting for newer Rust constructs that did not exist at the time
that version of the style guide was archived.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could have a completely separate configuration mechanism, unrelated to
editions. This would increase the development and testing burden of rustfmt,
and seems unlikely to provide commensurate value. This would also increase the
complexity for end users, who would have to understand two separate mechanisms
for handling forwards-compatibility and wonder how they differ. We feel that
since we're providing a mechanism similar to editions, we should make it clear
to users that it works like editions.

We could leave out the separate configuration of style edition, and keep style
edition in lockstep with Rust edition. This would be easier to develop and
test, but would mean larger and noisier commits in projects transitioning from
one edition to another.

We could keep the Rust style static forever, and never change it.

We could evolve the Rust style without a backwards-compatibility mechanism.
This would result in churn in people's repositories if collaborating
developers have different versions of Rust, and would break
continuous-integration checks that check formatting.

# Prior art
[prior-art]: #prior-art

The Rust edition mechanism itself serves as prior art.

# Future possibilities
[future-possibilities]: #future-possibilities

Actual changes to the Rust style for Rust 2024 or future editions.
