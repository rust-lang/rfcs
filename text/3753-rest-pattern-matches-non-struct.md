- Feature Name: `rest_pattern_matches_non_struct`
- Start Date: 2024-12-31
- RFC PR: [rust-lang/rfcs#3753](https://github.com/rust-lang/rfcs/pull/3753)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Special case struct patterns containing only a rest pattern (e.g., `Foo { .. }`)
so that they can match values of any type with the appropriate name, not just
structs (e.g., it could match an `enum Foo` value). This is done so that structs
containing only private fields can be changed to other types without breaking
backwards compatibility.

# Motivation
[motivation]: #motivation

It is common for a library's API to have a public struct with all-private
fields, with the intention of making the type opaque, hiding its contents as
implementation details. For example:
```rust
// A library's initial API
pub struct Foo {
    inner: FooInner,
}
enum FooInner {
    A,
    B,
}
```

In a later version, the library might want to expose the internals of the API.
For example:
```rust
// The library's API is later changed to this.
pub enum Foo {
    A,
    B,
}
```

Intuitively, one might think that this change is backwards-compatible. However,
this is technically not the case, since client code might match the `Foo` type
with a rest pattern (`Foo { .. }`), which currently only matches structs, and
not enums or other types.
```rust
// Client code that uses the library
// Works with the initial API, but doesn't work with the later API.
fn do_something(x: Foo) {
    match x {
        Foo { .. } => {}
    }
}
```

To eliminate this semver hazard, this RFC proposes that the pattern `Foo { .. }`
should match values of any type named `Foo`, not just structs.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As a special case, a struct pattern which contain only a rest pattern, and no
other fields, can match with any value of the appropriate type, even if it is
not a struct.

For example, the pattern `Foo { .. }` can match with any value that have type
`Foo`, even if `Foo` is not a struct (e.g., it might be an enum).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

(The following text is appended to the section on [struct
patterns](https://doc.rust-lang.org/stable/reference/patterns.html#struct-patterns)
in the reference.)

As a special case, if a struct pattern contains only `..` as the fields, and the
path refers to a type (as opposed to an enum variant), then the pattern is an
irrefutable pattern that matches against any value of that type. This applies
even if that type is not a struct type (e.g., it might be an enum type, or it
might be a type alias, etc.)

For example, the pattern `foo::Bar::<Baz> { .. }` can match any value of the
type `foo::Bar<Baz>` (even if this type is not a struct type), or match a value
of the type `foo<Baz>` that contains the enum variant `Bar`.

Formally, this special case applies to the following syntax: *PathInExpression*
`{` *StructPatternEtCetera* `}`

# Drawbacks
[drawbacks]: #drawbacks

This adds a complication to Rust. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* This special case is only applied to `Foo { .. }`, and not `Foo(..)`.
  * This is because, currently, the pattern `Foo(..)` can only match a tuple
  struct whose fields are all public, so the `Foo(..)` pattern does not pose a
  semver hazard.
  * On the other hand, the pattern `Foo { .. }` currently matches any struct
  named `Foo`, including tuple structs, and including type aliases that refer to
  structs. This makes the meaning of the pattern `Foo { .. }` already similar to
  "match any type named `Foo`".
* As an alternative, we could deprecate the pattern `Foo { .. }` (either in all
  cases, or only in cases where `Foo` has no public fields). We could then
  potentially remove this pattern from the language in a future edition.
  * This unfortunately doesn't fix the semver hazard, due to code in older
    editions existing. Additionally, this might be an edge case that macros
    would have to deal with.
* As an alternative, we could have an attribute that marks a type as completely
  opaque, and therefore making it not able to be matched with the pattern `Foo {
  .. }`.
  * Most users are likely to forget to apply this attribute. We could change the
    default over an edition, making a struct with no public fields implicitly
    opaque, but this special case seems rather weird and confusing.

# Prior art
[prior-art]: #prior-art

* `cargo-semver-checks` has [a
lint](https://github.com/obi1kenobi/cargo-semver-checks/issues/954) that checks
specifically if a struct containing only private fields is changed to a
different type.
* [RFC 1506](https://rust-lang.github.io/rfcs/1506-adt-kinds.html) previously
  made the braced-struct patterns match against any struct, including tuple-like
  structs:
  > Permit using tuple structs and tuple variants in braced struct patterns and
  > expressions requiring naming their fields - `TS{0: expr}`/`TS{0: pat}`/etc.
  > While this change is important for consistency, there's not much motivation
  > for it in hand-written code besides shortening patterns like `ItemFn(_, _,
  > unsafety, _, _, _)` into something like `ItemFn{2: unsafety, ..}` and
  > ability to match/construct tuple structs using their type aliases.
  > 
  > However, automatic code generators (e.g. syntax extensions) can get more
  > benefits from the ability to generate uniform code for all structure kinds.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* How much code in the wild currently uses patterns like `Foo { .. }` ?
* Are there any other ways in the language (other than the `Foo { .. }` pattern)
  for client code to depend on a type being specifically a struct?

# Future possibilities
[future-possibilities]: #future-possibilities

N/A