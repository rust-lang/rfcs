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
    ($t:adt) => { stringify!(${t.name}) }
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

Fragment fields may be used in a macro transcriber anywhere a fragment name of
the appropriate type could be used.

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
  - `body`: The body of the function, as a `block` (including the surrounding
    braces).
  - `vis`: The visibility of the function, as a `vis` (may be empty).
- `:adt`: An ADT (struct, union, or enum).
  - `name`: The name of the ADT, as an `ident`.
  - `vis`: The visibility of the ADT, as a `vis` (may be empty).

The tokens within fields have the spans of the corresponding tokens from the
source. If a token has no corresponding source (e.g. the `()` in `return_type`
for a `fn` with no explicitly specified return type), the field definition
defines an appropriate span.

Using a field of a fragment counts as a use of the fragment, for the purposes
of ensuring every fragment gets used at least once at the appropriate level of
repetition.

This extends the grammar of macro metavariable expressions to allow using a dot
and identifier to access a field.

Note that future versions of Rust can add new fields to an existing matcher;
doing so is a compatible change.

# Drawbacks
[drawbacks]: #drawbacks

This adds complexity to the macro system, in order to simplify macros in the
ecosystem.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could do nothing, and leave parsing to third-party crates in the ecosystem.
This entails inherently less efficient re-parsing, requires duplicating a Rust
AST/grammar into one or more third-party crates (and keeping it up to date),
pushes people towards proc macros, increases the supply chains of many crates,
and requires macros to update (or update their dependencies) when Rust adds new
syntax.

Rather than using field syntax, we could use function-like syntax in the style
of [RFC 3086](https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html)
macro metavariable expressions. However, field syntax seems like a more natural
fit for this concept.

Rather than synthesizing tokens for cases like `return_type`, we could make a
rule that we *never* provide tokens that aren't in the original source.
However, this would substantially limit usability of these fields in some
cases, and make macros harder to write. This RFC proposes, in general, that we
can synthesize tokens if necessary to provide useful values for fields.

# Prior art
[prior-art]: #prior-art

[RFC 3086](https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html), for
macro metavariable expressions, introduced a similar mechanism to add helpers
for macros to more easily process the contents of fragments.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we synthesize an `()` for `return_type`, or should we treat it as an
optional field?

We could also provide both (e.g. `.return_type` and `.opt_return_type`), or
provide a subfield of `.return_type` that contains only the type as written and
not any synthesized `()`.

Should we develop a lighter-weight process/policy for approving further macro
fragments or fragment fields? Should we delegate it to another team, such as
wg-macros?

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
- For `fn`, one or more fields for qualifiers such as `const` and `async`.
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
- `param` for a single function parameter, with fields for the pattern and the
  type. (This would also need to handle cases like `...` in variadic functions,
  and cases like `self`, perhaps by acting as if it was `self: Self`.)
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

We could handle conditionally available fields by presenting them as though
they have a repetition of `?`, which would allow expansions within `$(...)?`;
that would support simple conditional cases without much complexity, but seems
like an awkward way to write conditionals, and would not handle more complex
cases.

We could handle some other types of conditions by presenting "boolean"-like
fields as fields that expand to no tokens but do so under a repetition of `?`,
to allow writing conditionals like `$(${x.field} ...)?`. This would fit such
conditionals within existing macro concepts, but it may suffer from an unwanted
overabundance of cleverness, and may not be as easy to read as a dedicated
conditional construct.

If, in the future, we introduce fields whose values have fragment types that
themselves have fields, we should support nested field syntax.

We should establish and document a pattern for how to start out by parsing
`$t:adt`, get `${t.name}`, and then handle the case where `$t` is a `struct` vs
the case where `$t` is an `enum`. This would benefit from having better
conditional syntax.

We may want to have a fragment specifier or fields that allow treating a struct
or an enum variant uniformly, not caring whether it is tuple-style or
struct-style. This is another case study in needing synthesized tokens, since
we could present a tuple struct as though it were a struct with fields named
`0`, `1`, etc.

We may want to provide a macro metavariable function to extract syntax that has
specific attributes (e.g. derive helper attributes) attached to it. For
instance, a derive macro applied to a struct may want to get the fields that
have a specific helper attribute attached.

We could have macro metavariable expressions that return structured values with
fields.

We could allow macros to define new macro metavariable functions that can
return structured values. (This has high potential for complexity and would
need to be handled with care.)

If, in the future, we have a robust mechanism for compilation-time execution of
Rust or some subset of Rust, without requiring separately compiled proc macro
crates, we may want to use and extend that mechanism in preference to any
further complexity in the `macro_rules` system. However, such a mechanism seems
likely to be far in the future.
