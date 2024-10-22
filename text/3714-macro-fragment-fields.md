- Feature Name: `macro_fragment_fields`
- Start Date: 2024-10-14
- RFC PR: [rust-lang/rfcs#3714](https://github.com/rust-lang/rfcs/pull/3714)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a syntax and mechanism for macros to access "fields" of high-level fragment
specifiers that they've matched, to let macros use the Rust parser for
robustness and future compatibility, while still extracting pieces of the
matched syntax.

# Motivation
[motivation]: #motivation

The macros-by-example system is powerful, but sometimes difficult to work with.
In particular, parsing complex parts of Rust syntax often requires carefully
recreating large chunks of the Rust grammar, in order to parse out the desired
pieces. Missing or incorrectly handling any portion of the syntax can result in
not accepting the same syntax Rust does; this includes future extensions to
Rust syntax that the macro was not yet aware of. Higher-level fragment
specifiers are more robust for these cases, but don't allow extracting
individual pieces of the matched syntax.

This RFC introduces a mechanism to use high-level fragment specifiers while
still extracting individual pieces of the matched syntax.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When writing macros by example, and using certain high-level fragment
specifiers, you can use the syntax `${matched_name.field_name}` to extract
specific "fields" of the matched syntax. This allows you to use the Rust parser
for those high-level fragments, rather than having to recreate parts of the
Rust grammar in order to extract the specific pieces you want. Fields evaluate
to pieces of Rust syntax, suitable for substitution into the program or passing
to other macros for further processing.

For example, the fragment `:adt` parses any abstract data type supported by
Rust: struct, union, or enum. Given a match `$t:adt`, you can obtain the name
of the matched type with `${t.name}`:

```rust
macro_rules! get_name {
    ($t:adt) => { println!("{}", stringify!(${t.name})); }
}

fn main() {
    let n1 = get_name!(struct S { field: u32 });
    let n2 = get_name!(enum E { V1, V2 = 42, V3(u8) });
    let n3 = get_name!(union U { u: u32, f: f32 });
    println!("{n3}{n1}{n2}"); // prints "USE"
}
```

An attempt to access a field that doesn't exist will produce a compilation
error on the macro definition, whether or not the specific macro rule gets
invoked.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Fragment fields may be used in a macro transcriber anywhere the corresponding
fragment name could be used.

Fragment fields typically follow the same rules for repetition handling as the
corresponding fragment (e.g. being used at the same level/kind of repetition).
However, fragment fields that contain multiple items require one additional
level of repetition; see the `param` field of `:fn`, below.

This RFC introduces the following new fragment specifiers, with specified fields:

- `:fn`: A function definition (including body).
  - `name`: The name of the function, as an `ident`.
  - `param`: The parameters of the function, presented as though captured by a
    level of `*` repetition. For instance, you can write `$(${f.param}),*` to
    get a comma-separated list of parameters, or `$(other_macro!(${f.param}))*`
    to pass each parameter to another macro.
  - `return_type`: The return type of the function, as a `ty`. If the function
    has no explicitly specified return type, this will be `()`, with a span of
    the closing parenthesis for the function arguments.
  - `body`: The body of the function, as a block (including the
    surrounding braces).
  - `vis`: The visibility of the function, as a `vis` (may be empty).
- `:adt`: An ADT (struct, union, or enum).
  - `name`: The name of the ADT, as an `ident`.

The tokens within fields have the spans of the corresponding tokens from the
source. If a token has no corresponding source (e.g. the `()` in `return_type`
for a `fn` with no explicitly specified return type), the field definition
defines an appropriate span.

Using a field of a fragment counts as a use of the fragment, for the purposes
of ensuring every fragment gets used at least once at the appropriate level of
repetition.

This extends the grammar of macro metavariable expressions to allow using a dot
and identifier to access a field.

# Drawbacks
[drawbacks]: #drawbacks

This adds complexity to the macro system, in order to simplify macros in the
ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Rather than using field syntax, we could use function-like syntax in the style
of RFC 3086's macro metavariable expressions. However, field syntax seems like
a more natural fit for this concept.

# Prior art
[prior-art]: #prior-art

RFC 3086, for macro metavariable expressions, introduced a similar mechanism to
add helpers for macros to more easily process the contents of fragments.

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC proposes a few obvious useful fields, both for their own sake and to
serve as examples of the concept. There are many more fields we may want to
introduce in the future. This RFC intentionally proposes only a few fields, to
allow evaluating the RFC on the basis of the concept and proposed syntax rather
than every individual field proposal. If any individual proposed field proves
controversial or requires more extensive design, it should be removed and
deferred to a future RFC, rather than complicating this RFC with that more
extensive design.

Some examples of *possible* fields, to be evaluated in the future:
- For `fn`, a field for the ABI. This could be a synthesized `"Rust"` for
  functions without a specified ABI.
- For `adt` and `fn`, fields for the generics and bounds. We may want to
  provide them exactly as specified, or we may want to combine the bounds from
  both generics and where clauses. (This would work well together with a macro
  metavariable expression to generate the appropriate `where` bounds for a
  `derive`.)
- For `adt`, `fn`, and various others, a field for the doc comment, if any.
- For `block`, a field for the statements in the block.
- For `path`, a field for the segments in the path, and a field for the leading
  `::` if any.
- For `lifetime`, a field for the lifetime identifier, without the `'`.

Some examples of *possible* additional fragment specifiers, to be evaluated in
the future:
- `param` for a single function parameter.
- `field` for a single field of a `struct`, `union`, or struct-style enum
  variant.
- `variant` for a single variant of an `enum`
- `fndecl` for a function declaration (rather than a definition), such as in a
  trait or an extern block.
- `trait` for a trait definition, with fields for functions and associated
  types.
- `binop` for a binary operator expression, with fields for the operator and
  the two operands.
- `match` for a match expression, with fields for the scrutinee and the arms.
- `match_arm` for one arm of a match, with fields for the pattern and the body.
- `doc` for a doc comment, with `head` and `body` fields (handled the same way
  rustdoc does).

Some of these have tensions between providing convenient fields and handling
variations of these fragments that can't provide those fields. We could handle
this via separate fragment specifiers for different variations, or by some
mechanism for conditionally handling fields that may not exist. The former
would be less robust against future variations, while the latter would be more
complex.

If, in the future, we introduce fields whose values have fragment types that
themselves have fields, we should support nested field syntax.

We may want to provide a macro metavariable function to extract syntax that has
specific attributes (e.g. derive helper attributes) attached to it. For
instance, a derive macro applied to a struct may want to get the fields that
have a specific helper attribute attached.

If, in the future, we have a robust mechanism for compilation-time execution of
Rust or some subset of Rust, without requiring separately compiled proc macro
crates, we may want to use and extend that mechanism in preference to any
further complexity in the `macro_rules` system. However, such a mechanism seems
likely to be far in the future.
