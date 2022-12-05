- Feature Name: style-evolution
- Start Date: 2022-10-26
- RFC PR: [rust-lang/rfcs#3338](https://github.com/rust-lang/rfcs/pull/3338)
- Rust Issue: [rust-lang/rust#105336](https://github.com/rust-lang/rust/issues/105336)

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

Direct invocations of `rustfmt` obtain the edition used for parsing Rust code
from the `edition` option in its configuration file (`rustfmt.toml` or
`.rustfmt.toml`), or via the `--edition` command-line option; `cargo fmt`
obtains the edition from the `edition` option in `Cargo.toml` and passes it to
`rustfmt`. By default, `rustfmt` and `cargo fmt` will use the same edition for
style as the Rust edition used for parsing.

However, when transitioning between editions, projects may want to separately
make and commit the changes for 1) transitioning to a new Rust edition and 2)
transitioning to a new style edition. Keeping formatting changes in a separate
commit also helps tooling ignore that commit, such as with git's
`blame.ignoreRevsFile`.

To allow for this, `rustfmt` also allows configuring the style edition
directly, via a separate `style_edition` configuration option, or
`--style-edition` command-line option. `style_edition` or `--style-edition`, if
set, always overrides `edition` or `--edition` for the purposes of styling,
though `edition` or `--edition` still determines the edition for the purposes
of parsing Rust code.

Note that rustfmt may not necessarily support all combinations of Rust edition
and style edition; in particular, it may not support using a style edition that
differs by more than one step from the Rust edition. Similarly, rustfmt need
not support every existing configuration option in new style editions.

New style editions will be initially introduced as nightly-only, to make them
available for testing; such nightly-only editions will produce an error if
requested in stable rustfmt. Nightly versions of style editions are subject to
change and do not provide stability guarantees. New style editions will get
stabilized contemporaneously with the corresponding Rust edition.

The current version of the style guide will describe the latest Rust edition.
Each distinct past style will have a corresponding archived version of the
style guide. Note that archived versions of the style guide may not necessarily
document formatting for newer Rust constructs that did not exist at the time
that version of the style guide was archived. However, each style edition will
still format all constructs valid in that Rust edition, with the style of those
constructs coming from the first subsequent style edition providing formatting
rules for that construct.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could have a completely separate configuration mechanism, unrelated to
editions. This would increase the development and testing burden of rustfmt,
and seems unlikely to provide commensurate value. This would also increase the
complexity for end users, who would have to understand two separate mechanisms
for handling forwards-compatibility and wonder how they differ. We feel that
since we're providing a mechanism similar to editions, we should make it clear
to users that it works like editions.

We could allow style edition to vary completely independently of Rust edition.
This would, for instance, allow projects to stay on old style editions
indefinitely. However, this would substantially increase the development and
testing burden for formatting tooling, and require more complex decisions about
how old style editions format constructs that didn't exist in the corresponding
Rust edition. In general, while the Rust edition mechanism allows projects to
stay on old Rust editions, and projects doing so can similarly stay on the
corresponding old style editions, the style edition mechanism does not exist to
facilitate staying on old styles *indefinitely* while still moving forward to
newer Rust editions.

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

The Rust edition mechanism itself serves as prior art, as does the mechanism of
nightly features remaining subject to change until stabilization.

`rustfmt` has a still-unstable option `version = "Two"` to opt into new
formatting, though the exact changes this makes are not documented.

`rustfmt`'s stability guarantees are documented in [RFC
2437](https://github.com/rust-lang/rfcs/pull/2437/).

# Future possibilities
[future-possibilities]: #future-possibilities

Actual changes to the Rust style for Rust 2024 or future editions.
