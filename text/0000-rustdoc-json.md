- Feature Name: `rustdoc_json`
- Start Date: 2020-06-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC describes the design of a JSON output for the tool `rustdoc`, to allow tools to
lean on its data collection and refinement but provide a different front-end.

# Motivation
[motivation]: #motivation

The current HTML output of `rustdoc` is often lauded as a key selling point of Rust. Using this
ubiquitous tool, you can easily find nearly anything you need to know about a crate. However,
despite its versatility, the use of this specific output has its drawbacks:

- Viewing this output requires a web browser, with (for some features of the output) a JavaScript
  interpreter.
- The HTML output of `rustdoc` is explicitly not stabilized, to allow `rustdoc` developers the
  option to tweak the display of information, add new information, etc. However, this also means
  that converting this HTML into a different output is infeasible.
- As the HTML is the only available output of `rustdoc`, its integration into centralized,
  multi-language, documentation browsers is difficult.

In addition, `rustdoc` had JSON output in the past, but it failed to keep up with the changing
language and [was taken out][remove-json] in 2016. With `rustdoc` in a more stable position, it's
possible to re-introduce this feature and ensure its stability. This [was brought up in 2018][2018-discussion]
with a positive response and there are [several][2019-interest] [recent][rustdoc-infopages] discussions
indicating that it would be a nice feature to have.

In [the draft RFC from 2018][previous-rfc] there was some discussion of utilizing `save-analysis` to
provide this information, but with [RLS being replaced by rust-analyzer][RA-RLS] it's possible that
the feature will be eventually removed from the compiler. In addition `save-analysis` output is just
as if not more unstable than the current HTML output of `rustdoc`, so a separate feature is preferable.

[remove-json]: https://github.com/rust-lang/rust/pull/32773
[2018-discussion]: https://internals.rust-lang.org/t/design-discussion-json-output-for-rustdoc/8271/6
[2019-interest]: https://github.com/rust-lang/rust/issues/44136#issuecomment-467144974
[rustdoc-infopages]: https://internals.rust-lang.org/t/current-state-of-rustdoc-and-cargo/11721
[previous-rfc]: https://github.com/QuietMisdreavus/rfcs/blob/rustdoc-json/text/0000-rustdoc-json.md#unresolved-questions
[RA-RLS]: https://github.com/rust-lang/rfcs/pull/2912

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(*Upon successful implementation/stabilization, this documentation should live in The Rustdoc
Book.*)

In addition to generating the regular HTML, `rustdoc` can create JSON files based on your crate.
These can be used by other tools to take information about your crate and convert it into other
output formats, insert into centralized documentation systems, create language bindings, etc.

To get this output, pass the `--output-format json` flag to `rustdoc`:

```console
$ rustdoc lib.rs --output-format json
```

This will output a JSON file in the current directory (by default). For example, say you have the
following crate:

```rust
//! Here are some crate-level docs!

/// Here are some docs for `some_fn`!
pub fn some_fn() {}

/// Here are some docs for `SomeStruct`!
pub struct SomeStruct;
```

After running the above command, you should get a `lib.json` file like the following (indented for
clarity):

```json
{
TODO
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

(*Upon successful implementation/stabilization, this documentation should live in The Rustdoc
Book.*)

When you request JSON output from `rustdoc`, you're getting a version of the Rust abstract syntax
tree (AST), so you could see anything that you could export from a valid Rust crate. The following
types can appear in the output:

TODO

You also get a collection of mappings between items such as all the types that implement a certain
trait and vice versa. The structure of those mappings is as follows:

TODO

(*This documentation is deliberately left incomplete; filling it out will happen during the design  process.*)

(*Complete documentation information is deferred to final design and implementation work.*)

# Drawbacks
[drawbacks]: #drawbacks

- By supporting JSON output for `rustdoc`, we should consider how much it should mirror the internal
  structures used in `rustdoc` and in the compiler. Depending on how much we want to stabilize, we
  could accidentally stabilize the internal structures of `rustdoc`.

- Even if we don't accidentally stabilize `rustdoc`'s internals, adding JSON output adds *another*
  thing that must be kept up to date with language changes, and another thing for compiler
  contributors to potentially break with their changes. Because the HTML output is only meant for
  display, it requires less vigilant updating when new language features are added.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- **Status quo.** Keep the HTML the way it is, and make users who want a machine-readable version of
  a crate parse it themselves. In the absence of an accepted JSON output, the `--output-format` flag in rustdoc
  remains deprecated and unused.
- **Alternate data format (XML, Bincode, CapnProto, etc).** JSON was selected for its ubiquity in
  available parsers, but selecting a different data format may provide benefits for file size,
  compressibility, speed of conversion, etc. If the implementation leans on serde then this may be a
  non-issue as it would be trivial to switch serialization formats.
- **Alternate data structure.** Massage the data so that it echoes something closer to user
  perception, rather than the internal `clean` AST that they're currently modeled after. Such a
  refinement can be provided in a future RFC, as a potential alternate data format to output, if
  necessary.

# Prior art
[prior-art]: #prior-art

A handful of other languages and systems have documentation tools that output an intermediate
representation separate from the human-readable outputs:

- [PureScript] uses an intermediate JSON representation when publishing package information to their
  [Pursuit] directory. It's primarily used to generate documentation, but can also be used to
  generate `etags` files.
- [Doxygen] has an option to generate an XML file with the code's information.
- [Haskell]'s documentation tool, [Haddock], can generate an intermediate representation used by the
  type search engine [Hoogle] to integrate documentation of several packages.
- [Kythe] is a "(mostly) language-agnostic" system for integrating documentation across several
  langauges. It features its own schema that code information can be translated into, that services
  can use to aggregate information about projects that span multiple languages.
- [GObject Introspection] has an intermediate XML representation called GIR that's used to create
  langauge bindings for GObject-based C libraries. While (at the time of this writing) it's not
  currently used to create documentation, it is a stated goal to use this information to document
  these libraries.

[PureScript]: http://www.purescript.org/
[Pursuit]: https://pursuit.purescript.org/
[Doxygen]: https://www.doxygen.nl/
[Haskell]: https://www.haskell.org/
[Haddock]: https://www.haskell.org/haddock/
[Hoogle]: https://www.haskell.org/hoogle/
[Kythe]: http://kythe.io/
[GObject Introspection]: https://gi.readthedocs.io/en/latest/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What is the stabilization story? As langauge features are added, this representation will need to
  be extended to accommodate it. As this will change the structure of the data, what does that mean
  for its consumers?
- How will intra-doc links be handled? Supporting `struct.SomeStruct.html` style links is pretty
  infeasible since it would tie alternative front-ends to `rustdoc`'s file/folder format. With the
  nightly intra-rustdoc link syntax it's debatable whether we should resolve those to HTML links or
  leave that up to whatever consumes the JSON.
- How do we represent types, and allow people to properly collect type information from places like
  struct fields, function signatures, etc? `rustdoc`'s own `clean::Type` enum is large and recursive
  and represents a lot of primitives, in addition to ultimately deferring the lookup to a DefId.
- The `id` field is basically a copy of DefId from inside the compiler; is there a better way to
  represent it? How necessary is it to have?
- Where should we store impls?
  - In `rustdoc`, trait impls are pooled in the crate root (or placed in the module they're declared
    in), but before rendering, the information is copied into two maps: one mapping traits to their
    implementors, and one mapping types to all their impls (inherent or trait).
  - The HIR copies all trait impls into a map connecting traits to their implementors, though
    they're also available in the location they're defined if you iterate over the HIR.
  - However, while trait impls are unburdened by scope rules for visibility, *inherent* impls are.
    Currently, if `--document-private-items` is passed, the methods defined in an impl are all
    pooled into a struct, and any `pub(restricted)` scopes link to their respective modules.
    However, private methods are just shown as private, without any information connecting them to
    where they're allowed.
  - This leads to wanting to pool impls on their type (and copying them in to their trait for trait
    impls), and leaving the visibility fix for a later PR.


# Future possibilities
[future-possibilities]: #future-possibilities

- Since refactoring has to be done to support both the HTML and JSON backends to `rustdoc`, future
  work to add other output formats such as pure markdown should be relatively simple after this.
- Once the JSON output is added, a Rust library for parsing it back into useful structs that lives
  outside the compiler would be helpful to allow people to easily use this representation.
