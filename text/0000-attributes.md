- Feature Name: scoped_attributes
- Start Date: 2016-09-22
- RFC PR:
- Rust Issue:


# Summary
[summary]: #summary

This RFC specifies custom attributes, which can be used by both external tools
and by compiler plugins of various kinds. It also specifies the interaction of
custom attributes, built-in attributes, and attribute-like macros.

Tools may use scoped attributes. Each crate must declare which attribute
prefixes may be used using `#![attribute(...)]`. Scoped attributes which are not
declared in this way are treated as macros, and will result in name resolution
errors if a corresponding macro does not exist.

# Motivation
[motivation]: #motivation

Attributes are a useful, general-purpose mechanism for annotating code with
meta-data. They are used in the language (from `cfg` to `repr`), for macros
(e.g, `derive` (a built-in procedural macro), and for user-supplied attribute-
like macros), and by external tools (e.g., `rustfmt_skip` which instructs
Rustfmt not to format an AST node). Attributes could also be used by compiler
plugins such as lints.

Currently, custom attributes (i.e., those not known to the compiler, e.g.,
`rustfmt_skip`) are unstable. There is a future compatibility hazard with custom
attributes: if we add `#[foo]` to the language, than any users using a `foo`
custom attribute will suffer breakage.

There is a potential problem with the interaction between custom attributes and
attribute-like macros. Given an attribute, the compiler cannot tell if the
attribute is intended to be a macro invocation or an attribute that might only
be used by a tool (either outside or inside the compiler). Currently, the
compiler tries to find a macro and if it cannot, ignores the attribute (giving a
stability error if not on nightly or the `custom_attribute` feature is not
enabled). However, if the user intended the attribute to be a macro, silently
ignoring the missing macro error is not the right thing to do. The compiler
needs to know whether an attribute is intended to be a macro or not.


# Detailed design
[design]: #detailed-design

[RFC 1561](https://github.com/rust-lang/rfcs/blob/master/text/1561-macro-naming.md)
proposed allowing paths instead of identifiers in the names of
attributes in order to allow scoped macros in attribute position. E.g.,
`#[foo::bar]` looks up a macro named `bar` in a module named `foo`. This RFC
extends that mechanism to allow paths to be used for custom attributes. For
example, we might use `#[rustfmt::skip]`, `#[rustc::deprecated]`, or
`#[awesome_test::fail::panics]`.

Crates must opt-in to attributes. This is done by using a new attribute:
`#![attribute(rustfmt)]` opts-in to attributes starting with `rustfmt`. This
includes `#[rustfmt::skip]` and `#[rustfmt::foo::bar]`, but not `#[rustfmt]`. A
crate can opt in to more deeply scoped names too, e.g.,
`#![attribute(foo::bar)]` which opts-in to `#[foo::bar::baz]`,
`#[foo::bar::baz::qux]` and so forth. The use case here is to allow name-
spacing, so a crate can opt-in to both `BigCo::test` and
`domain_specific::test`.

The only un-scoped attributes that are allowed are those that are built-in to
the language. Top-level attributes and scoped attributes are independent: for
example, `test` is built-in to the language, but a program could still opt-in to
scoped attributes beginning with test: `#![attribute(test)]`. In the code,
`#[test]` is the built-in attribute; `#[test::foo]` is a scoped attribute. This
is necessary for backwards compatibility.

During macro expansion, when faced with an attribute, the compiler first checks
if the attribute matches any of the declared or built-in attributes. If not, it
tries to find a macro using the [macro name resolution rules](https://github.com/rust-lang/rfcs/blob/master/text/1561-macro-naming.md).
If this fails, then it reports a macro not found error. The compiler *may*
suggest mistyped attributes (declared or built-in).

The `rustc` namespace is reserved for the compiler. No opt-in is required for
these attributes. We will add aliases for all existing `rustc_` attributes so
that `rustc_foo` has the same meaning as `rustc::foo`. Attributes added by the
compiler for later use should be in the `rustc_internal` namespace. It is not
possible to opt-in to this namespace using `#[attribute(...)]`, and so users can
never use these attributes in source code. This will be checked just before
macro expansion (i.e., these attributes may be emitted by macros).

How attributes are read or used is not restricted by the compiler (since they
are often handled by external tools, I see no way to enforce anything). It is
best practice for a given tool to only use attributes scoped for that tool, e.g.,
`rustfmt` should only read attributes in the `rustfmt::` namespace.

The compiler will preserve attributes into the expanded AST and HIR (where
possible). It is considered best practice (but unenforceable) for macros to
preserve all attributes except those in their own namespace (see note below).

## Staging

If this RFC is accepted and implemented, then the `custom_attibute` feature
should be deprecated immediately. When this RFC is stabilised, the
`custom_attribute` feature should be removed.

Likewise, all `rustc_` attributes should be deprecated in favour of their
`rustc::` aliases if this RFC is accepted; they probably cannot be removed for
the sake of backwards compatibility.

## A note on macro attributes

Sometimes a procedural macro wants to use attributes to supply extra
information. For example, a macro on a struct may require attributes on
significant fields. In this case, there are no hard restrictions on the
attribute used - the macro will process the struct before the compiler sees the
attribute on the field, and can strip the attribute before the compiler checks
it.

We may in the future add API for macros to mark an attribute that should be
preserved and not checked by the compiler, but this is left for later.

We should decide on some conventions for naming such attributes. As a first
proposal, I suggest that the absolute, qualified name of the macro, starting
with the macro's crate (i.e., not the relative path from the use site) is used
as the namespace. E.g., if a macro named `foo` is in the root of crate `bar`,
then we could write:

```
use foo::bar;

#[bar(...)]
struct A {
    f: String,
    #[foo::bar::significant]
    g: String,
    h: i32,
}
```

Alternatively, we could allow using the relative path to a macro, however, we
would need to provide an API for the macro to match attributes relative to the
use-site.


# Drawbacks
[drawbacks]: #drawbacks

The proposed scheme does not allow tools or macros to use custom top-level
attributes.


# Alternatives
[alternatives]: #alternatives

This proposal could be tweaked in numerous ways. Some ideas include:

* could require `::*` in `attribute` declarations. This would bring the syntax
  closer to that of imports (where glob imports have similar semantics to
  `attribute` declarations). However, this is a bit misleading since non-glob
  syntax is not supported, therefore I think it is syntactic noise.
* could require opt-in per-module, rather than per-crate. I prefer per-crate,
  since I would expect tools to be run over a whole crate.

A bigger change would be to allow scoped attributes without opt-in and warn (or
silently ignore) missing attribute-like macros.


# Unresolved questions
[unresolved]: #unresolved-questions

Do we need a way to opt-out of `rustc::` attributes?

Some top-level attributes are part of the language in some sense, but could also
be replaced by alternate implementations (e.g., `test`). Do these need special
treatment?

Some possible extensions:
* an import mechanism so that attributes can be used with a less qualified name.
* APIs for tools and macros to access 'their' attributes.
* Guarantees about preserving attributes into the MIR for lints or compiler
  plugins that work there.
* Generalising the `rustc_internal` machinery so macros can add attributes which
  are not allowed in source text.
