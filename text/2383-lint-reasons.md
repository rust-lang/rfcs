- Feature Name: `lint_reasons`
- Start Date: 2018-04-02
- RFC PR: [rust-lang/rfcs#2383](https://github.com/rust-lang/rfcs/pull/2383)
- Rust Issue: [rust-lang/rust#54503](https://github.com/rust-lang/rust/issues/54503)

# Summary
[summary]: #summary

Rust has a number of code lints, both built into the compiler and provided
through external tools, which provide guidelines for code style. The linter
behavior can be customized by attaching attributes to regions of code to allow,
warn, or forbid, certain lint checks.

The decision for allowing, warning on, or forbidding, specific lints is
occasionally placed in a comment above the attribute or, more often, left
unstated. This RFC proposes adding syntax to the lint attributes to encode the
documented reason for a lint configuration choice.

# Motivation
[motivation]: #motivation

The style guide for a project, team, or company can cover far more than just
syntax layout. Rules for the semantic shape of a codebase are documented in
natural language and often in automated checking programs, such as the Rust
compiler and Clippy. Because the decisions about what rules to follow or ignore
shape the codebase and its growth, the decisions are worth storing in the
project directly with the settings they affect.

It is common wisdom that only the text the environment can read stays true; text
it ignores will drift out of sync with the code over time, if it was even in
sync to begin. Lint settings should have an explanation for their use to explain
why they were chosen and where they are or are not applicable. As they are text
that is read by some lint program, they have an opportunity to include an
explanation similar to the way Rust documentation is a first-class attribute on
code.

The RFC template asks three questions for motivation:

- Why are we doing this?

We are adding this behavior to give projects a mechanism for storing human
design choices about code in a manner that the tools can track and use to
empower human work. For example, the compiler could use the contents of the
lint explanation when it emits lint messages, or the documenter could collect
them into a set of code style information.

- What use cases does it support?

This supports the use cases of projects, teams, or organizations using specific
sets of code style guidelines beyond the Rust defaults. This also enables the
creation and maintenance of documented practices and preferences that can be
standardized in a useful way. Furthermore, this provides a standardized means of
explaining decisions when a style guide must be violated by attaching an
overriding lint attribute to a specific item.

- What is the expected outcome?

The expected outcome of this RFC is that projects will have more information
about the decisions and expectations of the project, and can have support from
the tools to maintain and inform these decisions. Global and specific choices
can have their information checked and maintained by the tools, and the Rust
ecosystem can have a somewhat more uniform means of establishing code guidelines
and practices.

I expect Clippy will be a significant benefactor of this RFC, as Clippy lints
are far more specific and plentiful than the compiler lints, and from personal
experience much more likely to want explanation for their use or disuse.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When a linting tool such as the compiler or Clippy encounter a code span that
they determine breaks one of their rules, they emit a message indicating the
problem and, often, how to fix it. These messages explain how to make the linter
program happy, but carry very little information on why the code may be a
problem from a human perspective.

These lints can be configured away from the default settings by the use of an
attribute modifying the code span that triggers a lint, or by setting the linter
behavior for a module or crate, with attributes like `#[allow(rule)]` and
`#![deny(rule)]`.

It is generally good practice to include an explanation for why certain rules
are set so that programmers working on a project can know what the project
expects from their work. These explanations can be embedded directly in the lint
attribute with the `reason = "Your reasoning here"` attribute.

For example, if you are implementing `Ord` on an enum where the discriminants
are not the correct ordering, you might have code like this:

```rust
enum Severity { Red, Blue, Green, Yellow }
impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        use Severity::*;
        use Ordering::*;
        match (*self, *other) {
            (Red, Red) |
            (Blue, Blue) |
            (Green, Green) |
            (Yellow, Yellow) => Equal,

            (Blue, _) => Greater,
            (Red, _) => Less,

            (Green, Blue) => Less,
            (Green, _) => Greater,

            (Yellow, Red) => Greater,
            (Yellow, _) => Less,
        }
    }
}
```

The ordering of the left hand side of the match branches is significant, and
allows a compact number of match arms. However, if you're using Clippy, this
will cause the `match_same_arms` lint to trip! You can silence the lint in this
spot, and provide an explanation that indicates you are doing so deliberately,
by placing this attribute above the `match` line:

```rust
#[allow(match_same_arms, reason = "The arms are order-dependent")]
```

Now, when the lints run, no warning will be raised on this specific instance,
and there is an explanation of why you disabled the lint, directly in the lint
command.

Similarly, if you want to increase the strictness of a lint, you can explain why
you think the lint is worth warning or forbidding directly in it:

```rust
#![deny(float_arithmetic, reason = "This code runs in a context without floats")]
```

With a warning or denial marker, when a linting tool encounters such a lint trap
it will emit its builtin diagnostic, but also include the reason in its output.

For instance, using the above Clippy lint and some floating-point arithmetic
code will result in the following lint output:

```text
error: floating-point arithmetic detected
reason: This code runs in a context without floats
 --> src/lib.rs:4:2
  |
4 |     a + b
  |     ^^^^^
  |
note: lint level defined here
 --> src/lib.rs:1:44
  |
1 | #![cfg_attr(deny(float_arithmetic, reason = "..."))]
  |                  ^^^^^^^^^^^^^^^^
  = help: for further information visit ...
```

## `expect` Lint Attribute

This RFC adds an `expect` lint attribute that functions identically to `allow`,
but will cause a lint to be emitted when the code it decorates ***does not***
raise a lint warning. This lint was inspired by Yehuda Katz:

> [@ManishEarth](https://twitter.com/ManishEarth) has anyone ever asked for
> something like #[expect(lint)] which would basically be like #[allow(lint)]
> but give you a lint warning if the problem ever went away?
>
> I basically want to mark things as ok while doing initial work, but I need to
> know when safe to remove
>
> — Yehuda Katz ([@wycats](https://twitter.com/wycats))
>
> [March 30, 2018](https://twitter.com/wycats/status/979742693378019329)

When the lint passes run, the `expect` attribute suppresses a lint generated by
the span to which it attached. It does not swallow any other lint raised, and
when it does not receive a lint to suppress, it raises a lint warning itself.
`expect` can take a `reason` field, which is printed when the lint is raised,
just as with the `allow`/`warn`/`deny` markers.

This is used when prototyping and using code that will generate lints for now,
but will eventually be refactored and stop generating lints and thus no longer
need the permission.

```rust
#[expect(unused_mut, reason = "Everything is mut until I get this figured out")]
fn foo() -> usize {
    let mut a = Vec::new();
    a.len()
}
```

will remain quiet while you're not mutating `a`, but when you do write code that
mutates it, or decide you don't need it mutable and strip the `mut`, the
`expect` lint will fire and inform you that there is no unused mutation in the
span.

```rust
#[expect(unused_mut, reason = "...")]
fn foo() {
    let a = Vec::new();
    a.len()
}
```

will emit

```text
warning: expected lint `unused_mut` did not appear
reason: Everything is mut until I get this figured out
 --> src/lib.rs:1:1
  |
1 | #[expect(unused_mut, reason = "...")]
  |   -------^^^^^^^^^^-----------------
  |   |
  |   help: remove this `#[expect(...)]`
  |
  = note: #[warn(expectation_missing)] on by default
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC adds a `reason = STRING` element to the three lint attributes. The
diagnostic emitter in the compiler and other lint tools such as Clippy will need
to be made aware of this element so that they can emit it in diagnostic text.

This RFC also adds the `expect(lint_name, reason = STRING)` lint attribute. The
`expect` attribute uses the same lint-suppression mechanism that `allow` does,
but will raise a new lint, `expectation_missing` (name open to change), when the
lint it expects does not arrive.

The `expectation_missing` lint is itself subject to
`allow`/`expect`/`warn`/`deny` attributes in a higher scope, so it is possible
to suppress expectation failures, lint when no expectation failures occur, or
fail the build when one occurs. The default setting is
`#![warn(expectation_missing)]`.

That’s pretty much it, for technical details.

## OPTIONAL — Yet Another Comment Syntax

A sugar for lint text MAY be the line comment `//#` or the block comment
`/*# #*/` with `U+0023 # NUMBER SIGN` as the signifying character. These
comments MUST be placed immediately above a lint attribute. They are collected
into a single string and collapsed as the text content of the attribute they
decorate using the same processing logic that documentation comments (`///` and
`//!` and their block variants) currently use. Example:

```rust
//# Floating Point Arithmetic Unsupported
//#
//# This crate is written to be run on an AVR processor which does not have
//# floating-point capability in hardware. As such, all floating-point work is
//# done in software routines that can take a significant amount of time and
//# space to perform. Rather than pay this cost, floating-point work is
//# statically disabled. All arithmetic is in fixed-point integer math, using
//# the `FixedPoint` wrapper over integer primitives.
#![deny(float_arithmetic)]
```

The `#` character is chosen as the signifying character to provide room for
possible future expansion – these comments MAY in the future be repurposed as
sugar for writing the text of an attribute that declares a string parameter that
can accept such comments.

This comment syntax already pushes the edge of the scope of this RFC, and
extension of all attributes is certainly beyond it.

Implementing this comment syntax would require extending the existing transform
pass that replaces documentation comments with documentation attributes.
Specifically, the transform pass would ensure that all lint comments are
directly attached to a lint attribute, and then use the strip-and-trim method
that the documentation comments experience to remove the comment markers and
collapse the comment text, across multiple consecutive comment spans, into a
single string that is then inserted as `reason = STRING` into the attribute.

Given that this is a lot of work and a huge addition to the comment grammar, the
author does not expect it to be included in the main RFC at all, and is writing
it solely to be a published prior art in case of future desire for such a
feature.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

Possibly low value add for the effort.

# Rationale and alternatives
[alternatives]: #alternatives

- Why is this design the best in the space of possible designs?

    Attributes taking descriptive text is a common pattern in Rust.

- What other designs have been considered and what is the rationale for not
    choosing them?

    None.

- What is the impact of not doing this?

    None.

# Prior art
[prior-art]: #prior-art

The `stable` and `unstable` attributes both take descriptive text parameters
that appear in diagnostic and documentation output.

# Unresolved questions
[unresolved]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process
    before this gets merged?

    The name of the `reason` parameter.

- What parts of the design do you expect to resolve through the implementation
    of this feature before stabilization?

    The use sites of the `reason` parameter.

- What related issues do you consider out of scope for this RFC that could be
    addressed in the future independently of the solution that comes out of this
    RFC?

    The means of filling the `reason` parameter, or what tools like `rustdoc`
    would do with them.
