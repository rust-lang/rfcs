- Feature Name: cfg_attr_multi
- Start Date: 2018-11-24
- RFC PR: 
- Rust Issue: 

# Summary
[summary]: #summary

Allow multiple attributes inside attribute containers (`#[]` and `#![]`)
delimited by commas.

# Motivation
[motivation]: #motivation

It lets us express our intent more clearly when attributes added for the same
reason are grouped together. It aligns with [RFC 2539] doing the same for the
`cfg_attr` attribute. It matches C#'s attribute syntax that we borrowed in the
first place.

# Background Information
[background-information]: #background-information

## Extension of RFC 2539

In [RFC 2539], we allowed `cfg_attr(predicate, any, number, of attributes)` as
expanding to those any number of attributes when the predicate is true. This
was the first place to allow multiple attributes together with commas. This
RFC is the second place. Outside of arbitrary macros, this RFC suggests doing
so in the only other place we allow multiple attributes together.

## Attribute Syntax

In [RFC 2539], we defined the attribute syntax and a restriction on attributes
so that commas cannot end up in the top level of attribute syntax. This means
that attribute syntax is as follows:

An attribute can look like:

* `name`,
* ``name(`TokenStream`)``
* ``name = `TokenTree` ``

where `TokenStream` is a sequence of tokens that only has the restriction that
delimiters match and `TokenTree` is a single identifer, literal, punctuation
mark, or a delimited `TokenStream`.

The following cannot ever be parsed as attributes:

* `name, option`
* `name = some, options`

Arguably, we could allow `(name, option)`, but we shouldn't.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Attribute containers may contain multiple attributes, delimited by commas. They
are resolved as if each attribute was in its own attribute brace. Furthermore,
attribute containers allow a trailing comma and may contain zero attributes.

There is no behavior difference between putting each attribute in its own
container and putting the attributes in the same container. For example, given
three attributes, `attr1`, `attr2`, and `attr3`, the following will all behave
identically:

* `#[attr1] #[attr2] #[attr3]`
* `#[attr1, attr2] #[attr3]`
* `#[attr1] #[attr2, attr3]`
* `#[attr1, attr2, attr3]`

For a real example of multiple attributes in the same container, we have a
test that is ignored:

```rust
#[test, ignore]
fn should_not_panic() {
    panic!("Oh nooooooooo");
}
```

## Attribute Macro Input `TokenStream` Changes
 
Before this RFC, when an attriute macro is executed, it is passed two
`TokenStream`s. The first is the input passed to the attribute itself. That is
unchanged by this RFC. The second is the `TokenStream` of the syntactic element
the attribute is attached to without the attribute macro. Because the attribute
macro was the only attribute in the attribute container, this `TokenStream`
did not have the attribute container at all. This behavior continues as is,
but we add that in the case of the attribute container containing multiple
attributes, only the attribute macro and its following comma, if it exists, is
removed. This is easy to see by example:

Source | Macro Attribute `TokenStream` as String
--- | ---
`#[macro_attr] thing` | `"thing"`
`#[attr1] #[macro_attr] thing` | `"#[attr1] thing"`
`#[attr1, macro_attr] thing` | `"#[attr1, ] thing"`
`#[macro_attr, attr1] thing` | `"#[attr1] thing"`
`#[attr1, macro_attr, attr2] thing` | `"#[attr1, attr2] thing"`
`#[] #[macro_attr] thing` | `"#[] thing"` or `"thing"` (see unresolved questions)

Other active attributes (currently `test`, `cfg`, and `cfg_attr`) behave
similarly.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The attributes on a syntactic element are those inside the outer and inner
attribute containers attached to the syntactic element. Attributes are ordered
from their order in the source. The attribute containers contain a list of
attributes separated by commas. The final attribute in a container may have
a comma following it.

## Reference Documentation Changes

Change the syntax to be as follows:

> **<sup>Syntax</sup>**\
> _AttributeContainer_ :\
> &nbsp;&nbsp; _InnerAttribute_ | _OuterAttribute_
>
> _InnerAttribute_ :\
> &nbsp;&nbsp; `#![` _AttributeList_<sup>?</sup> `]`
>
> _OuterAttribute_ :\
> &nbsp;&nbsp; `#[` _AttributeList_<sup>?</sup> `]`
>
> _AttributeList_ :\
> &bnsp&nbsp; MetaItem (`,` MetaItem)<sup>*</sup> `,`<sup>?</sup>

The changes are as follows:

* _Attribute_ becomes _AttributeContainer_ (Or removed? Is it actually used?)
* _InnerAttribte_ and and _OuterAttribute_ take an _AttributeList_ instead of
  a MetaItem.
* _AttributeList_ is added.

The section on active and inert attributes will be more specific on what it
means to remove the active attribute. Something like:

Active attributes are removed from the thing they are on during processing. If
the attribute is the only attribute in its container, the entire container is
removed. Otherwise, just the metaitem and the following comma if it is there is
removed.

## Lint Changes

This RFC allows `#[]`. This is so that macros can generate it. Having it in the
source text emits an `unused_attributes` warning.

Any lints that point to an attribute must make sure their span is the attribute
metaitem and not the attribute container.

# Drawbacks
[drawbacks]: #drawbacks

The standard drawbacks to allowing more syntax exists. It's yet another thing to
learn. It's only a small win over just having multiple attribute blocks.

This also gives us more to argue about for the style of Rust programs. Some
might argue that all attributes should be in the same attribute block while
others might argue that only related attributes should be and yet others will
argue that this was a mistake and one attribute per attribute container. Beyond
that, there's also the whitespace questions that arise with multiple attributes.
Do we write `#[attr1, attr2]` or `#[\n\tattr1,\n\tattr2\n]`. And then there's
the question of trailing commas, although that's well trodden ground.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

We could require that multiple attributes must be within in a delimiter to make
it so that it's always two arguments at the top level. E.g.,
`#[cfg_attr(predicate, [attr, attr])]`. While this could increase clarity, it
mostly seems like it would just add noise. In the multiline case, it already
reads pretty clear with the predicate on the first line and each attribute
indented.

The default alternative of not doing this is a possibility. It would just mean
that conditionally including attributes is slightly less ergonomic than it
could be.

We could change attribute container syntax to allow multiple attributes and then
state that `cfg_attr` takes the attribute container syntax without the `#[]`
part. While this could be a good final state, it's a more ambitious change that
has more drawbacks. There are legitimate reasons we'd want `cfg_attr` to take
multiple attributes but not the attribute container. As such, I would like to
go with the conservative change first.

The original draft of this RFC only allowed one or more attributes and did not
allow the trailing comma. Because it helps macros and fits the rest of the
language, it now allows those.

# Prior art
[prior-art]: #prior-art

Both [`GNU C`] and [`C#`] allow multiple attributes separated by commas in the same
container.

From the [`GNU C`] docs:

> An attribute specifier is of the form `__attribute__ ((attribute-list))`. An
> attribute list is a possibly empty comma-separated sequence of attributes,
> where each attribute is one of the following: 

[Microsoft's documentation][`C#`] shows that it allows multiple attributes in
the same square brackets (their attribute syntax is just square brackets):

> More than one attribute can be placed on a declaration as the following
> example shows:
>
> ```C#
> void MethodA([In][Out] ref double x) { }
> void MethodB([Out][In] ref double x) { }
> void MethodC([In, Out] ref double x) { }
> ```

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should we include empty attribute containers in the `TokenStream` for macro
attributes and derive attributes?

# Future possibilities

None.

[`GNU C`]: https://gcc.gnu.org/onlinedocs/gcc/Attribute-Syntax.html
[`C#`]: https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/concepts/attributes/
[RFC 2539]: https://github.com/rust-lang/rfcs/blob/master/text/2539-cfg_attr-multiple-attrs.md