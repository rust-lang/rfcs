- Feature Name: `macro_derive`
- Start Date: 2024-09-20
- RFC PR: [rust-lang/rfcs#3698](https://github.com/rust-lang/rfcs/pull/3698)
- Rust Issue: [rust-lang/rust#143549](https://github.com/rust-lang/rust/issues/143549)

## Summary
[summary]: #summary

Support implementing `derive(Trait)` via a `macro_rules!` macro.

## Motivation
[motivation]: #motivation

Many crates support deriving their traits with `derive(Trait)`. Today, this
requires defining proc macros, in a separate crate, typically with several
additional dependencies adding substantial compilation time, and typically
guarded by a feature that users need to remember to enable.

However, many common cases of derives don't require any more power than an
ordinary `macro_rules!` macro. Supporting these common cases would allow many
crates to avoid defining proc macros, reduce dependencies and compilation time,
and provide these macros unconditionally without requiring the user to enable a
feature.

The [`macro_rules_attribute`](https://crates.io/crates/macro_rules_attribute)
crate defines proc macros that allow invoking declarative macros as derives,
demonstrating a demand for this. This feature would allow defining such derives
without requiring proc macros at all, and would support the same invocation
syntax as a proc macro.

The derive feature of the crate has [various uses in the
ecosystem](https://github.com/search?q=macro_rules_attribute%3A%3Aderive&type=code).

`derive` macros have a standard syntax that Rust users have come to expect for
defining traits; this motivates providing users a way to invoke that mechanism
for declarative macros. An attribute or a `macro_name!` invocation could serve
the same purpose, but that would be less evocative than `derive(Trait)` for
the purposes of making the purpose of the macro clear, and would additionally
give the macro more power to rewrite the underlying definition. Derive macros
simplify tools like rust-analyzer, which can know that a derive macro will
never change the underlying item definition.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can define a macro to implement `derive(MyTrait)` by defining a
`macro_rules!` macro with one or more `derive()` rules. Such a macro can create
new items based on a struct, enum, or union. Note that the macro can only
append new items; it cannot modify the item it was applied to.

For example:

```rust
trait Answer { fn answer(&self) -> u32; }

macro_rules! Answer {
    // Simplified for this example
    derive() (struct $n:ident $_:tt) => {
        impl Answer for $n {
            fn answer(&self) -> u32 { 42 }
        }
    };
}

#[derive(Answer)]
struct Struct;

fn main() {
    let s = Struct;
    assert_eq!(42, s.answer());
}
```

Derive macros defined using `macro_rules!` follow the same scoping rules as
any other macro, and may be invoked by any path that resolves to them.

A derive macro may share the same path as a trait of the same name. For
instance, the name `mycrate::MyTrait` can refer to both the `MyTrait` trait and
the macro for `derive(MyTrait)`. This is consistent with existing derive
macros.

If a derive macro emits a trait impl for the type, it may want to add the
[`#[automatically_derived]`](https://doc.rust-lang.org/reference/attributes/derive.html#the-automatically_derived-attribute)
attribute, for the benefit of diagnostics.

If a derive macro mistakenly emits the token stream it was applied to
(resulting in a duplicate item definition), the error the compiler emits for
the duplicate item should hint to the user that the macro was defined
incorrectly, and remind the user that derive macros only append new items.

A `derive()` rule can be marked as `unsafe`:
`unsafe derive() (...) => { ... }`.
Invoking such a derive using a rule marked as `unsafe`
requires `unsafe` derive syntax:
`#[derive(unsafe(DangerousDeriveMacro))]`

Invoking an unsafe derive rule without the unsafe derive syntax will produce a
compiler error. Using the unsafe derive syntax without an unsafe derive will
trigger an "unused unsafe" lint. (RFC 3715 defines the equivalent mechanism for
proc macro derives.)

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The grammar for macros is extended as follows:

> _MacroRule_ :\
> &nbsp;&nbsp; ( `unsafe`<sup>?</sup> `derive` `(` `)` )<sup>?</sup>  _MacroMatcher_ `=>` _MacroTranscriber_

The _MacroMatcher_ matches the entire construct the attribute was
applied to, receiving precisely what a proc-macro-based attribute
would in the same place.

(The empty parentheses after `derive` reserve future syntax space
for derives accepting arguments, at which time they'll be replaced
by a second _MacroMatcher_ that matches the arguments.)

A derive invocation that uses an `unsafe derive` rule will produce
an error if invoked without using the `unsafe` derive syntax. A
derive invocation that uses an `derive` rule (without `unsafe`)
will trigger the "unused unsafe" lint if invoked using the `unsafe`
derive syntax. A single derive macro may have both `derive` and
`unsafe derive` rules, such as if only some invocations are unsafe.

This grammar addition is backwards compatible: previously, a _MacroRule_ could
only start with `(`, `[`, or `{`, so the parser can easily distinguish rules
that start with `derive` or `unsafe`.

Adding `derive` rules to an existing macro is a semver-compatible change,
though in practice, it will likely be uncommon.

If a user invokes a macro as a derive and that macro does not have any `derive`
rules, the compiler should give a clear error stating that the macro is not
usable as a derive because it does not have any `derive` rules.

## Drawbacks
[drawbacks]: #drawbacks

This feature will not be sufficient for *all* uses of proc macros in the
ecosystem, and its existence may create social pressure for crate maintainers
to switch even if the result is harder to maintain. We can and should attempt
to avert and such pressure, such as by providing a post with guidance that
crate maintainers can link to when responding to such requests.

Before stabilizing this feature, we should receive feedback from crate
maintainers, and potentially make further improvements to `macro_rules` to make
it easier to use for their use cases. This feature will provide motivation to
evaluate many new use cases that previously weren't written using
`macro_rules`, and we should consider quality-of-life improvements to better
support those use cases.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Adding this feature will allow many crates in the ecosystem to drop their proc
macro crates and corresponding dependencies, and decrease their build times.

This will also give derive macros access to the `$crate` mechanism to refer to
the defining crate, which is simpler than mechanisms currently used in proc
macros to achieve the same goal.

Macros defined this way can more easily support caching, as they cannot depend
on arbitrary unspecified inputs.

Crates could instead define `macro_rules!` macros and encourage users to invoke
them using existing syntax like `macroname! { ... }`, rather than using
derives. This would provide the same functionality, but would not support the
same syntax people are accustomed to, and could not maintain semver
compatibility with an existing proc-macro-based derive. In addition, this would
not preserve the property derive macros normally have that they cannot change
the item they are applied to.

A mechanism to define attribute macros would let people write attributes like
`#[derive_mytrait]`, but that would not provide compatibility with existing
derive syntax.

We could allow `macro_rules!` derive macros to emit a replacement token stream.
That would be inconsistent with the restriction preventing proc macros from
doing the same, but it would give macros more capabilities, and simplify some
use cases. Notably, that would make it easy for derive macros to re-emit a
structure with another `derive` attached to it.

We could allow directly invoking a `macro_rules!` derive macro as a
function-like macro. This has the potential for confusion, given the
append-only nature of derive macros versus the behavior of normal function-like
macros. It might potentially be useful for code reuse, however.

### Syntax alternatives

Rather than using `derive()` rules, we could have `macro_rules!` macros use a
`#[macro_derive]` attribute, similar to the `#[proc_macro_derive]` attribute
used for proc macros.

However, this would be inconsistent with `attr()` rules as defined in RFC 3697.
This would also make it harder to add parameterized derives in the future (e.g.
`derive(MyTrait(params))`).

## Prior art
[prior-art]: #prior-art

We have had proc-macro-based derive macros for a long time, and the ecosystem
makes extensive use of them.

The [`macro_rules_attribute`](https://crates.io/crates/macro_rules_attribute)
crate defines proc macros that allow invoking declarative macros as derives,
demonstrating a demand for this. This feature would allow defining such derives
without requiring proc macros at all, and would support the same invocation
syntax as a proc macro.

The derive feature of the crate has [various uses in the
ecosystem](https://github.com/search?q=macro_rules_attribute%3A%3Aderive&type=code).

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Before stabilizing this feature, we should ensure there's a mechanism macros
can use to ensure that an error when producing an impl does not result in a
cascade of additional errors caused by a missing impl. This may take the form
of a fallback impl, for instance.

Before stabilizing this feature, we should make sure it doesn't produce wildly
worse error messages in common cases.

Before stabilizing this feature, we should receive feedback from crate
maintainers, and potentially make further improvements to `macro_rules` to make
it easier to use for their use cases. This feature will provide motivation to
evaluate many new use cases that previously weren't written using
`macro_rules`, and we should consider quality-of-life improvements to better
support those use cases.

Before stabilizing this feature, we should have clear public guidance
recommending against pressuring crate maintainers to adopt this feature
rapidly, and encourage crate maintainers to link to that guidance if such
requests arise.

## Future possibilities
[future-possibilities]: #future-possibilities

We should provide a way for derive macros to invoke other derive macros. The
`macro_rules_attribute` crate includes a `derive_alias` mechanism, which we
could trivially implement given a means of invoking another derive macro.

We should provide a means to perform a `derive` on a struct without being
directly attached to that struct. (This would also potentially require
something like a compile-time reflection mechanism.)

We could support passing parameters to derive macros (e.g.
`#[derive(Trait(params), OtherTrait(other, params))]`). This may benefit from
having `derive(...)` rules inside the `macro_rules!` macro declaration, similar
to the `attr(...)` rules proposed in RFC 3697.

In the future, if we support something like `const Trait` or similar trait
modifiers, we'll want to support `derive(const Trait)`, and define how a
`macro_rules!` derive handles that.

We should provide a way for `macro_rules!` macros to provide better error
reporting, with spans, rather than just pointing to the macro.

We may want to support error recovery, so that a derive can produce an error
but still provide enough for the remainder of the compilation to proceed far
enough to usefully report further errors.

As people test this feature and run into limitations of `macro_rules!` parsing,
we should consider additional features to make this easier to use for various
use cases.

We could provide a macro matcher to match an entire struct field, along with
syntax (based on macro metavariable expressions) to extract the field name or
type (e.g. `${f.name}`). This would simplify many common cases by leveraging
the compiler's own parser.

We could do the same for various other high-level constructs.

We may want to provide simple helpers for generating/propagating `where`
bounds, which would otherwise be complex to do in a `macro_rules!` macro.

We may want to add a lint for macro names, encouraging macros with derive rules
to use `CamelCase` names, and encouraging macros without derive rules to use
`snake_case` names.

### Helper attribute namespacing and hygiene

We should provide a way for derive macros to define helper attributes ([inert
attributes](https://doc.rust-lang.org/reference/attributes.html#active-and-inert-attributes)
that exist for the derive macro to parse and act upon). Such attributes are
supported by proc macro derives; however, such attributes have no namespacing,
and thus currently represent compatibility hazards because they can conflict.
We should provide a namespaced, hygienic mechanism for defining and using
helper attributes.

For instance, could we have `pub macro_helper_attr! skip` in the standard
library, namespaced under `core::derives` or similar? Could we let macros parse
that in a way that matches it in a namespaced fashion, so that:
- If you write `#[core::derives::skip]`, the macro matches it
- If you `use core::derives::skip;` and `write #[skip]`, the macro matches it
- If you `use elsewhere::skip` (or no import at all) and write `#[skip]`, the
  macro *doesn't* match it.

We already have *some* interaction between macros and name resolution, in order
to have namespaced `macro_rules!` macros. Would something like this be feasible?

(We would still need to specify the exact mechanism by which macros match these
helper attributes.)
