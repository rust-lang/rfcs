- Feature Name: `crate-attr`
- Start Date: 2025-03-16
- RFC PR: [rust-lang/rfcs#3791](https://github.com/rust-lang/rfcs/pull/3791)
- Rust Issue: [rust-lang/rust#138287](https://github.com/rust-lang/rust/issues/138287)

# Summary
[summary]: #summary

`--crate-attr` allows injecting crate-level attributes via the command line.
It is supported by all the official rustc drivers: Rustc, Rustdoc, Clippy, Miri, and Rustfmt.
Rustdoc extends it to doctests, discussed in further detail below.
It is encouraged, but not required, that external `rustc_driver` tools also support this flag.

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

The third group is the authors of external tools. The [original motivation][impl] for this feature was for Crater, which wanted to enable a rustfix feature in *all* crates it built without having to modify the source code. Other motivations include the currently-unstable [`register-tool`], which with this RFC could be an attribute passed by the external tool (or configured in the workspace), and [build-std], which wants to inject `no_std` into all crates being compiled.

[impl]: https://github.com/rust-lang/rust/pull/52355
[`register-tool`]: https://github.com/rust-lang/rust/issues/66079#issuecomment-1010266282
[doc-url]: https://doc.rust-lang.org/rustdoc/write-documentation/the-doc-attribute.html#at-the-crate-level
[`doc(html_no_source)`]: https://github.com/rust-lang/rust/issues/75497
[build-std]: https://github.com/rust-lang/rfcs/pull/3791#discussion_r1998684847

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `--crate-attr` flag allows you to inject attributes into the crate root.
For example, `--crate-attr=crate_name="test"` acts as if `#![crate_name="test"]` were present before the first source line of the crate root.

To inject multiple attributes, pass `--crate-attr` multiple times.

This feature lets you pass attributes to your whole workspace at once, even if rustc doesn't natively support them as flags.
For example, you could configure `strict_provenance_lints` for all your crates by adding
`build.rustflags = ["--crate-attr=feature(strict_provenance_lints)", "-Wfuzzy_provenance_casts", "-Wlossy_provenance_casts"]`
to `.cargo/config.toml`.

This feature also applies to doctests.
Running (for example) `RUSTDOCFLAGS="--crate-attr='feature(strict_provenance_lints)' -Wfuzzy_provenance_casts" cargo test --doc` will enable `fuzzy_provenance_casts` for all doctests that are run.

(This snippet is adapted from [the unstable book].)

[the unstable book]: https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/crate-attr.html#crate-attr

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Semantics

Any crate-level attribute is valid to pass to `--crate-attr`.

Formally, the expansion behaves as follows:

1. The crate root (initial file given to rustc) is parsed as if `--crate-attr` were not present.
2. The attributes in `--crate-attr` are parsed.
3. The attributes are injected at the top of the crate root (see below for
   relative ordering to any existing attributes).
4. Compilation continues as normal.

As a consequence, this feature does not affect [shebang parsing], nor can it affect nor be affected by comments that appear on the first source line.

Another consequence is that the argument to `--crate-attr` is syntactically isolated from the rest of the crate; `--crate-attr=/*` is always an error and cannot begin a multi-line comment.

`--crate-attr` is treated as Rust source code, which means that whitespace, block comments, and raw strings are valid: `'--crate-attr= crate_name /*foo bar*/ = r#"my-crate"# '` is equivalent to `--crate-attr=crate_name="my-crate"`.

The argument to `--crate-attr` is treated as-if it were surrounded by `#![ ]`, i.e. it must be an inner attribute and it cannot include multiple attributes, nor can it be any grammar production other than an [`Attr`].
In particular, this implies that `//!` syntax for doc-comments is disallowed (although `doc = "..."` is fine).

If the attribute is already present in the source code, it behaves exactly as it would if duplicated twice in the source.
For example, duplicating `no_std` is idempotent; duplicating `crate_type` generates both types; and duplicating `crate_name` is idempotent if the names are the same and a hard error otherwise.
It is suggested, but not required, that the implementation not warn on idempotent attributes, even if it would normally warn that duplicate attributes are unused.

The compiler is free to re-order steps 1 and 2 in the above order if desirable.
This shouldn't have any user-observable effect beyond changes in diagnostics.

## Doctests

`--crate-attr` is also a rustdoc flag. Rustdoc behaves identically to rustc for the main crate being compiled.
For doctests, by default, `--crate-attr` applies to both the main crate and the generated doctest.
This can be overridden for the doctest using `--crate-attr=doc(test(attr(...)))`.
`--crate-attr=doc(...)` attributes never apply to the generated doctest, only to the main crate (with the exception of `doc(test(attr(...)))`, which applies the inner `...` attribute, not the doc attribute itself).

[shebang parsing]: https://doc.rust-lang.org/nightly/reference/input-format.html#shebang-removal
[`Attr`]: https://doc.rust-lang.org/nightly/reference/attributes.html

## Ordering

Attributes are applied in the order they were given on the command line; so `--crate-attr=warn(unused) --crate-attr=deny(unused)` is equivalent to `deny(unused)`.
`crate-attr` attributes are applied before source code attributes.
For example, the following file, when compiled with `crate-attr=deny(unused)`, does not fail with an error, but only warns:

```rust
#![warn(unused)]
fn foo() {}
```

Additionally, all existing `-A -W -D -F` flags become aliases for `--crate-attr` (`allow`, `warn`, `deny`, and `forbid`, respectively). In particular, this implies that the following CLI flag combinations are exactly equivalent:
- `-D unused`
- `-A unused -D unused`
- `--crate-attr=allow(unused) -D unused`

`--force-warn` has no attribute that is equivalent, and is not affected by this RFC.

"Equivalence" as described in this section only makes sense because lint attributes are defined to have a precedence order.
Other attributes, such as doc-comments, define no such precedence. Those attributes have whatever meaning they define for their order.
For example, passing `'--crate-attr=doc = "Compiled on March 18 2025"'` to a crate with `#![doc = "My awesome crate"]` on the first line would generate documentation for the crate root reading:
```
Compiled on March 18 2025
My awesome crate
```

## Spans, modules, and editions

`include!`, `include_str!`, and `module_path!` all behave the same as when
written in source code at the top of the crate root. That is, any module or
path-relative resolution within the `--crate-attr` attribute should be treated
the same as ocurring within the crate root.

`--crate-attr` shares an edition with the crate (i.e. it is affected by `--edition`). This may be observable because `doc` attributes can invoke arbitrary macros. Consider this use of [indoc]:
```
--crate-attr='doc = indoc::indoc! {"
    test
    this
"}'
```
Edition-related changes to how proc-macros are passed tokens may need to consider how crate-attr is affected.

`file!`, `line!`, `column!` *within* the --crate-attr attribute use a synthetic
file (e.g., file might be `<cli-arg>`). This avoids ambiguity for the span
overlapping actual bytes in any existing files on disk, and matches precedent
in other toolchains, e.g., see clang's output for `--include` on the command
line:

```shell
$ touch t.h
$ clang t.h --include foo.h
<built-in>:1:10: fatal error: 'foo.h' file not found
    1 | #include "foo.h"
      |          ^~~~~~~
1 error generated.
```

The line and column will ideally be relative to the individual --crate-attr
command line flag, though this is considered a best-effort detail for quality
of diagnostics.  They will not be affected by the injected `#![` surrounding
the parsed
attribute.

The original source parsing (i.e., the file provided to rustc) is not affected
by the injected attributes, in effect, they are treated as ocurring within 0
bytes at the start of the file.

[indoc]: https://docs.rs/indoc/latest/indoc/

# Drawbacks
[drawbacks]: #drawbacks

It makes it harder for Rust developers to know whether it's idiomatic to use flags or attributes.
In practice, this has not be a large drawback for `crate_name` and `crate_type`, although for lints perhaps a little more so since they were only recently stabilized in Cargo.

Using this feature can make code in a crate dependent on attributes provided through the build system, such that the code doesn't build if reorganized or copied to another project.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- We could require `--crate-attr=#![foo]` instead. This is more verbose and requires extensive shell quoting, for not much benefit. It does however resolve the concern around `column!` (to include the `#![` in the column number), and looks closer to the syntax in a source file.
- We could disallow comments in the attribute. This perhaps makes the design less surprising, but complicates the implementation for little benefit.
- We could apply `--crate-attr` after attributes in the source, instead of before. This has two drawbacks:
    1. It has different behavior for lints than the existing A/W/D flags, so those flags could not semantically be unified with crate-attr. We would be adding yet another precedence group.
    2. It does not allow configuring a "default" option for a workspace and then overriding it in a single crate.
- We could add a syntax for passing multiple attributes in a single CLI flag. We would have to find a syntax that avoids ambiguity *and* that does not mis-parse the data inside string literals (i.e. picking a fixed string, such as `|`, would not work because it has to take quote nesting into account). This greatly complicates the implementation for little benefit. We also already have @file syntax to pass in arguments from a file which provides an escape hatch if this is truly helpful.

This cannot be done in a library or macro. It can be done in an external tool, but only by modifying the source in place, which requires first parsing it, and in general is much more brittle than this approach (for example, preventing the argument from injecting a unterminated block comment, or from injecting a non-attribute grammar production, becomes much harder).

In the author's opinion, having source injected via this mechanism does not make code any harder to read than the existing flags that are already stable (in particular `-C panic` and `--edition` come to mind).

# Prior art
[prior-art]: #prior-art

- HTML allows `<meta http-equiv=...>` to emulate headers, which is very useful for using hosted infra where one does not control the server.
- bash allows `-x` and similar to emulate `set -x` (for all `set` arguments). It also allows `-O shopt_...` for all `shopt ...` arguments.
- tmux config syntax is the same as its CLI syntax (for example `tmux set-option ...` is mostly the same as writing `set-option ...` in `tmux.conf`, modulo some issues around startup order and inherited options).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Is `--crate-name` equivalent to `--crate-attr=crate_name`? As currently implemented, the answer is no. Fixing this is hard; see https://github.com/rust-lang/rust/issues/91632 and https://github.com/rust-lang/rust/pull/108221#issuecomment-1435765434 (these do not directly answer why, but I am not aware of any documentation that does).

# Future possibilities
[future-possibilities]: #future-possibilities

This proposal would make it easier to use external tools with [`#![register_tool]`][`register-tool`], since they could be configured for a whole workspace at once instead of individually; and could be configured without modifying the source code.

We may want to allow [procedural macros at the crate root](https://github.com/rust-lang/rust/issues/54726). At that point we have to decide whether those macros can see `--crate-attr`. I *think* this should not be an issue because the attributes are prepended, not appended, but it needs more research.
