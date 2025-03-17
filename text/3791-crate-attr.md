- Feature Name: `crate-attr`
- Start Date: 2025-03-16
- RFC PR: [rust-lang/rfcs#3791](https://github.com/rust-lang/rfcs/pull/3791)
- Rust Issue: [rust-lang/rust#138287](https://github.com/rust-lang/rust/issues/138287)

# Summary
[summary]: #summary

`--crate-attr` allows injecting crate-level attributes via the command line.

# Motivation
[motivation]: #motivation

There are three main motivations.

1. CLI flags are easier to configure for a whole workspace at once.
2. When designing new features, we do not need to choose between attributes and flags; adding an attribute automatically makes it possible to set with a flag.
3. Tools that require a specific attribute can pass that attribute automatically.

Each of these corresponds to a different set of stakeholders. The first corresponds to developers writing Rust code. For this group, as the size of their code increases and they split it into multiple crates, it becomes more and more difficult to configure attributes for the whole workspace; they need to be duplicated into the root of each crate. Some attributes that could be useful to configure workspace-wide:
- `no_std`
- `feature` (in particular, enabling unstable lints for a whole workspace at once)
- [`doc(html_{favicon,logo,playground,root}_url}`][doc-url]
- [`doc(html_no_source)`]
- `doc(attr(...))`

Cargo has features for configuring flags for a workspace (RUSTFLAGS, `target.<name>.rustflags`, `profile.<name>.rustflags`), but no such mechanism exists for crate-level attributes.

Additionally, some existing CLI options could have been useful as attributes. This leads to the second group: Maintainers of the Rust language. Often we need to decide between attributes and flags; either we duplicate features between the two (lints, `crate-name`, `crate-type`), or we make it harder to configure the options for stakeholder group 1.

The third group is the authors of external tools. The [original motivation][impl] for this feature was for Crater, which wanted to enable a rustfix feature in *all* crates it built without having to modify the source code. Other motivations include the currently-unstable [`register-tool`], which with this RFC could be an attribute passed by the external tool (or configured in the workspace), and [custom test frameworks].

[impl]: https://github.com/rust-lang/rust/pull/52355
[`register-tool`]: https://github.com/rust-lang/rust/issues/66079#issuecomment-1010266282
[doc-url]: https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html#at-the-crate-level
[`doc(html_no_source)`]: https://github.com/rust-lang/rust/issues/75497
[custom test frameworks]: https://github.com/rust-lang/rust/pull/52355#issuecomment-405037604

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `--crate-attr` flag allows you to inject attributes into the crate root.
For example, `--crate-attr=crate_name="test"` acts as if `#![crate_name="test"]` were present before the first source line of the crate root.

To inject multiple attributes, pass `--crate-attr` multiple times.

This feature lets you pass attributes to your whole workspace at once, even if rustc doesn't natively support them as flags.
For example, you could configure `strict_provenance_lints` for all your crates by adding
`build.rustflags = ["--crate-attr=feature(strict_provenance_lints)", "-Wfuzzy_provenance_casts", "-Wlossy_provenance_casts"]`
to `.cargo/config.toml`.

(This snippet is adapted from [the unstable book].)

[the unstable book]: https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/crate-attr.html#crate-attr

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Any crate-level attribute is valid to pass to `--crate-attr`.

Formally, the expansion behaves as follows:

1. The crate is parsed as if `--crate-attr` were not present.
2. The attributes in `--crate-attr` are parsed.
3. The attributes are injected at the top of the crate root.
4. Macro expansion is performed.

As a consequence, this feature does not affect [shebang parsing], nor can it affect nor be affected by comments that appear on the first source line.

Another consequence is that the argument to `--crate-attr` is syntactically isolated from the rest of the crate; `--crate-attr=/*` is always an error and cannot begin a multi-line comment.

`--crate-attr` is treated as Rust source code, which means that whitespace, block comments, and raw strings are valid: `'--crate-attr= crate_name /*foo bar*/ = r#"my-crate"# '` is equivalent to `--crate-attr=crate_name="my-crate"`.

The argument to `--crate-attr` is treated as-if it were surrounded by `#![ ]`, i.e. it must be an inner attribute and it cannot include multiple attributes, nor can it be any grammar production other than an [`Attr`].

If the attribute is already present in the source code, it behaves exactly as it would if duplicated twice in the source.
For example, duplicating `no_std` is idempotent; duplicating `crate_type` generates both types; and duplicating `crate_name` is idempotent if the names are the same and a hard error otherwise.
It is suggested, but not required, that the implementation not warn on idempotent attributes, even if it would normally warn that duplicate attributes are unused.

[shebang parsing]: https://doc.rust-lang.org/nightly/reference/input-format.html#shebang-removal
[`Attr`]: https://doc.rust-lang.org/nightly/reference/attributes.html

# Drawbacks
[drawbacks]: #drawbacks

It makes it harder for Rust developers to know whether it's idiomatic to use flags or attributes.
In practice, this has not be a large drawback for `crate_name` and `crate_type`, although for lints perhaps a little more so since they were only recently stabilized in Cargo.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could require `--crate-attr=#![foo]` instead. This is more verbose and requires extensive shell quoting, for not much benefit.
- We could disallow comments in the attribute. This perhaps makes the design less surprising, but complicates the implementation for little benefit.
- We could add a syntax for passing multiple attributes in a single CLI flag. We would have to find a syntax that avoids ambiguity *and* that does not mis-parse the data inside string literals (i.e. picking a fixed string, such as `|`, would not work because it has to take quote nesting into account). This greatly complicates the implementation for little benefit.

This cannot be done in a library or macro. It can be done in an external tool, but only by modifying the source in place, which requires first parsing it, and in general is much more brittle than this approach (for example, preventing the argument from injecting a unterminated block comment, or from injecting a non-attribute grammar production, becomes much harder).

In the author's opinion, having source injected via this mechanism does not make code any harder to read than the existing flags that are already stable (in particular `-C panic` and `--edition` come to mind).

# Prior art
[prior-art]: #prior-art

- HTML allows `<meta http-equiv=...>` to emulate headers, which is very useful for using hosted infra where one does not control the server.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How should this interact with doctests? Does it apply to the crate being tested or to the generated test?

# Future possibilities
[future-possibilities]: #future-possibilities

This proposal would make it easier to use external tools with [`#![register_tool]`][`register-tool`], since they could be configured for a whole workspace at once instead of individually; and could be configured without modifying the source code.
