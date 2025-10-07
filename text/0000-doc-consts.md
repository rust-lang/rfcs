- Feature Name: `doc_consts`
- Start Date: 2025-01-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
- Pre-RFC: [Pre-RFC: `#[doc(consts)]` attribute](https://internals.rust-lang.org/t/pre-rfc-doc-consts-attribute/21987)

# Summary
[summary]: #summary

Introduce a `#[doc(normalize::consts = ...)]` attribute controlling how constant expressions are rendered by rustdoc.

# Motivation
[motivation]: #motivation

Different crates and items have conflicting requirements for their constants.
For some, [the exact value of a constant is platform dependant](https://internals.rust-lang.org/t/pre-rfc-doc-consts-attribute/21987/9).
For others, [constant folding obsurces the meaning of values](https://github.com/rust-lang/rust/issues/128347).
Hovever, [showing a constant as written may leak implementation details], 
and in some cases, there is no possible value that would be meaningful to the user of the library.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `#[doc(normalize::consts)]` attribute can be placed on any item to control how contained constant expressions are displayed in rustdoc-generated documentation.

* `#[doc(normalize::consts = "fold")]` will show them in their fully-evaluated state.
* `#[doc(normalize::consts = "expr")]` will show them as-written.
* `#[doc(normalize::consts = "hide")]` will cause constant expressions to be replaced with `_` or not shown at all.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


## The Attribute
The `#[doc(normalize::consts)]` attribute determines how constant expressions (constexprs) are rendered by rustdoc.
When applied to any item (including the top-level module within a crate, or impl blocks), it affects all constexprs within that item, and within all childern of that item.
Whenever multiple such attributes would take effect, the innermost attribute takes priority.

constexprs affected include:
* the RHS of `const` items
* the RHS of `static` items
* const generics in type aliases

### Interaction with `#[doc(inline)]`
When an item is inlined, it is rendered as if it had been defined in the crate it is being inlined into.

This means that if the `doc(normalize::consts)` modes of the source and destination crate do not match, an inlined item will *always* be rendered with the mode from the destination crate.

## The Values

### "fold"
The current default.  Rustdoc will evaluate the constexpr and print it in its fully evaluated form, as if the constexpr was written as a literal.

Numbers will be printed in base 10.

### "expr"
Rustdoc will print the constexpr as-written.

If the constexpr contains private identifiers, they will be exposed, so library authors should take care when using this mode.

### "hide"
This will cause constants and statics to display without any value, as if the value was unrenderable (see [ONCE_INIT](https://doc.rust-lang.org/nightly/std/sync/constant.ONCE_INIT.html)), and will cause other constant expressions–such as generic const parameters–to be rendered as `_`.

<!--This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work. -->

# Drawbacks
[drawbacks]: #drawbacks

Rustdoc does not currently have the ability to show all constants as-written, namely in the case of inlined re-exports from other crates. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

* The attribute is named `consts` and not `const` to avoid using keywords in attributes
* A key-value format is used instead of a directive system like `doc(fold)` to allow multiple states without polluting the doc attribute namespace.
* The `normalize::` prefix is used because of how const normalization paralells type normalization,
  and to improve discoverability via search engines if someone finds it in an unfamiliar codebase.
<!--
- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
-->
# Prior art
[prior-art]: #prior-art

- [RFC 3631](https://github.com/rust-lang/rfcs/pull/3631) for an attribute that affects the rendering of child items in a nesting way.

<!--- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.-->

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should be happen rustdoc cannot format a constant as requested?
- How should structs be handled in `"expr"` mode?
- Are there any other constants that show up in items that this should affect?
- How desirable is the hiding of generic const parameters?
<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
-->

# Future possibilities
[future-possibilities]: #future-possibilities

- `#[doc(normalize::types)]` to control normalization of types.
- Controlling the base of folded integer literals.
- Allowing the attribute on individual constant expressions, such as if a type alias has multible const generics that should be rendered differntly.
- Seperatly specifying the rendering for different categories of constant expressions, such as declaring that only `static` items should have their value hidden.
- Control formatting of expression (collapsing/adding whitespace, etc.)

<!--Think about what the natural extension and evolution of your proposal would
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
The section merely provides additional information. -->
