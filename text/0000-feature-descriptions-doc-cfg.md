- Feature Name: feature-descriptions-doc-cfg
- Start Date: 2023-04-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC has three simple goals:

1. Allow adding descriptions to features in `Cargo.toml`
2. Allow specifying additional `rustdoc` configuration (favicon URL, playground
   URL, etc) in either `Cargo.toml` or a new `rustdoc.toml`
3. By combining the two, allow `rustdoc` will be able to document cargo
   features, and have room to expand its configuratino options.

# Motivation
[motivation]: #motivation

Currently, <docs.rs> provides a simple view of available feature flags on a
rather simple page: for example, <https://docs.rs/crate/tokio/latest/features>.
It is helpful as a quick overview of available features, but means that users
must manually maintain a feature table if they want them to be documented
somehow.

The second problem is that `rustdoc` has some per-crate configuration settings,
such as relevant URLs, that are awkward to define in Rust source files using
attributes. It is expected that there may be further configuration options in
the future.

This RFC provides a way to solve both problems: it will give `rustdoc` the
ability to document features, and provide room to grow with more configration...

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Usage is simple: features will be able to be specified in a table (inline or
separate) with the keys `doc`, `public`, and `requires`. Sample `Cargo.toml`:

```toml
# Cargo.toml

[features]
# current configuration
foo = []
# Add a description to the feature
bar = { requires = ["foo"], doc = "simple docstring here"}
# `public` indicates whether or not the feature should be visible in
# documentation, and defaults to true
baz = { requires = ["foo"], public = false }

# Features can also be full tables if descriptions are longer
[features.qux]
requires = ["bar", "baz"]
doc = """
# qux

This could be a longer description of this feature
"""
```

This RFC will also enable a `[tools.rustdoc]` table where existing configuration
can be specified

```toml
# Cargo.toml

[tools.rustdoc]
html-logo-url = "https://example.com/logo.jpg"
issue-tracker-base-url = "https://github.com/rust-lang/rust/issues/"
```

For projects that do not use cargo or want separate configuration, these options
can also be specified in a `rustdoc.toml` file

```toml
# rustdoc.toml containing the same information as above

html-logo-url = "https://example.com/logo.jpg"
issue-tracker-base-url = "https://github.com/rust-lang/rust/issues/"

[features]
# current configuration
foo = []
# Add a description to the feature
bar = { requires = ["foo"], doc = "simple docstring here"}

# (baz and qux features clipped)
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

What exactly `rustdoc` does with the information is TBD. There are two options 

## JSON Configuration

`rustdoc` will gain a `--json-config` argument that allows passing a
JSON-serialized string of the TOML configuration. It is likely that this is what
Cargo can use when it invokes `rustdoc`: all that is needed is to parse the
`features` and `tools.rustdoc` table from `Cargo.toml`, serialize to JSON, and
pass as an argument.

```sh
rustdoc --argfoo --argbar . --json-config '{"html-logo-url":
"https://example.com/logo.jpg","issue-tracker-base-url":
"https://github.com/rust-lang/rust/issues/","features":{"foo":[],"bar":{"doc":
"simple docstring here","requires":["foo"]},"baz":{"public":false,"requires":
["foo"]},"qux":{"doc":"# qux\n\nThis could be a longer description of this feature\n"
,"requires":["bar","baz"]}}}'
```

This sort of format has two distinct advantages:

1. Build systems other than `cargo` can easily make use of the configuration
2. `rustdoc` does not need to be aware of `cargo`, workspaces, etc.

- Question: could/should this work from stdin?
- Note: there is a possible precedent to set here that could make it easy for
  other tools. `cargo-foobar` could receive the JSON-serialized string of the
  `[tools.foobar]` section.

## Configuration file argument

`rustdoc` will gain the `--config-file` argument that can point to a
`rustdoc.toml` formatted file. The name `rustdoc.toml` is not required.

If argument length would be exceeded with the `--json-config` option, `Cargo`
can create a temporary `rustdoc.toml` in the target build folder and point
`rustdoc` to it.

The arguments `--json-config` and `--config-file` can be specified more than
once, later invocations will just overwrite previous configuration.

- Question: should rustdoc look for config if it isn't specified?


**rest of this RFC todo**

---

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

- It is work

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Anything special for workspaces?

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
