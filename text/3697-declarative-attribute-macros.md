- Feature Name: `macro_attr`
- Start Date: 2024-09-20
- RFC PR: [rust-lang/rfcs#3697](https://github.com/rust-lang/rfcs/pull/3697)
- Rust Issue: [rust-lang/rust#143547](https://github.com/rust-lang/rust/issues/143547)

## Summary
[summary]: #summary

Support defining `macro_rules!` macros that work as attribute macros.

## Motivation
[motivation]: #motivation

Many crates provide attribute macros. Today, this requires defining proc
macros, in a separate crate, typically with several additional dependencies
adding substantial compilation time, and typically guarded by a feature that
users need to remember to enable.

However, many common cases of attribute macros don't require any more power
than an ordinary `macro_rules!` macro. Supporting these common cases would
allow many crates to avoid defining proc macros, reduce dependencies and
compilation time, and provide these macros unconditionally without requiring
the user to enable a feature.

The [`macro_rules_attribute`](https://crates.io/crates/macro_rules_attribute)
crate defines proc macros that allow invoking declarative macros as attributes,
demonstrating a demand for this. This feature would allow defining such
attributes without requiring proc macros at all, and would support the same
invocation syntax as a proc macro.

Some macros in the ecosystem already implement the equivalent of attribute
using declarative macros; for instance, see
[smol-macros](https://crates.io/crates/smol-macros), which provides a `main!`
macro and recommends using it with `macro_rules_attribute::apply`.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When defining a `macro_rules!` macro, you can prefix some of the macro's rules
with `attr(...)` to allow using the macro as an attribute. The
arguments to the attribute, if any, are parsed by the *MacroMatcher* in the
first set of parentheses; the second *MacroMatcher* parses the entire construct
the attribute was applied to. The resulting macro will work anywhere an
attribute currently works.

```rust
macro_rules! main {
    attr() ($func:item) => { make_async_main!($func) };
    attr(threads = $threads:literal) ($func:item) => { make_async_main!($threads, $func) };
}

#[main]
async fn main() { ... }

#[main(threads = 42)]
async fn main() { ... }
```

Attribute macros defined using `macro_rules!` follow the same scoping rules as
any other macro, and may be invoked by any path that resolves to them.

An attribute macro must not require itself for resolution, either directly or
indirectly (e.g. applied to a containing module or item).

Note that a single macro can have both `attr` and non-`attr` rules. Attribute
invocations can only match the `attr` rules, and non-attribute invocations can
only match the non-`attr` rules. This allows adding `attr` rules to an existing
macro without breaking backwards compatibility.

An attribute macro may emit code containing another attribute, including one
provided by an attribute macro. An attribute macro may use this to recursively
invoke itself.

An `attr` rule may be prefixed with `unsafe`. Invoking an attribute macro in a
way that makes use of a rule declared with `unsafe attr` requires the unsafe
attribute syntax `#[unsafe(attribute_name)]`.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The grammar for macros is extended as follows:

> _MacroRule_ :\
> &nbsp;&nbsp; ( `unsafe`<sup>?</sup> `attr` _MacroMatcher_ )<sup>?</sup>  _MacroMatcher_ `=>` _MacroTranscriber_

The first _MacroMatcher_ matches the attribute's arguments, which will be an
empty token tree if either not present (`#[myattr]`) or empty (`#[myattr()]`).
The second _MacroMatcher_ matches the entire construct the attribute was
applied to, receiving precisely what a proc-macro-based attribute would in the
same place.

Only a rule matching both the arguments to the attribute and the construct the
attribute was applied to will apply. Note that the captures in both
`MacroMatcher`s share the same namespace; attempting to use the same name for
two captures will give a "duplicate matcher binding" error.

An attribute macro invocation that uses an `unsafe attr` rule will produce an
error if invoked without using the `unsafe` attribute syntax. An attribute
macro invocation that uses an `attr` rule will trigger the "unused unsafe" lint
if invoked using the `unsafe` attribute syntax. A single attribute macro may
have both `attr` and `unsafe attr` rules, such as if only some invocations are
unsafe.

This grammar addition is backwards compatible: previously, a _MacroRule_ could
only start with `(`, `[`, or `{`, so the parser can easily distinguish rules
that start with `attr` or `unsafe`.

Attribute macros declared using `macro_rules!` are
[active](https://doc.rust-lang.org/reference/attributes.html#active-and-inert-attributes),
just like those declared using proc macros.

Adding `attr` rules to an existing macro is a semver-compatible change.

If a user invokes a macro as an attribute and that macro does not have any
`attr` rules, the compiler should give a clear error stating that the macro is
not usable as an attribute because it does not have any `attr` rules.

## Drawbacks
[drawbacks]: #drawbacks

This feature will not be sufficient for *all* uses of proc macros in the
ecosystem, and its existence may create social pressure for crate maintainers
to switch even if the result is harder to maintain.

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

This will also give attribute macros access to the `$crate` mechanism to refer
to the defining crate, which is simpler than mechanisms currently used in proc
macros to achieve the same goal.

Macros defined this way can more easily support caching, as they cannot depend
on arbitrary unspecified inputs.

Crates could instead define `macro_rules!` macros and encourage users to invoke
them using existing syntax like `macroname! { ... }`. This would provide the
same functionality, but would not support the same syntax people are accustomed
to, and could not maintain semver compatibility with an existing
proc-macro-based attribute.

We could require the `!` in attribute macros (`#[myattr!]` or similar).
However, proc-macro-based attribute macros do not require this, and this would
not allow declarative attribute macros to be fully compatible with
proc-macro-based attribute macros.

Many macros will want to parse their arguments and separately parse the
construct they're applied to, rather than a combinatorial explosion of both.
This problem is not unique to attribute macros. In both cases, the standard
solution is to parse one while carrying along the other.

We could leave out support for writing a function-like macro and an attribute
macro with the same name. However, this would prevent crates from preserving
backwards compatibility when adding attribute support to an existing
function-like macro.

Instead of or in addition to marking the individual rules, we could mark the
whole macro with `#[attribute_macro]` or similar, and allow having an attribute
macro and a non-attribute macro with the same name.

We could include another `=>` or other syntax between the first and second
macro matchers.

We could use `attribute` rather than `attr`. Rust usually avoids abbreviating
except for the most common constructs; however, `cfg_attr` provides precedent
for this abbreviation, and `attr` appears repeatedly in multiple rules which
motivates abbreviating it.

## Prior art
[prior-art]: #prior-art

We have had proc-macro-based attribute macros for a long time, and the
ecosystem makes extensive use of them.

The [`macro_rules_attribute`](https://crates.io/crates/macro_rules_attribute)
crate defines proc macros that allow invoking declarative macros as attributes,
demonstrating a demand for this. This feature would allow defining such
attributes without requiring proc macros at all, and would support the same
invocation syntax as a proc macro.

Some macros in the ecosystem already implement the equivalent of attribute
using declarative macros; for instance, see
[smol-macros](https://crates.io/crates/smol-macros), which provides a `main!`
macro and recommends using it with `macro_rules_attribute::apply`.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

Is an attribute macro allowed to recursively invoke itself by emitting the
attribute in its output? If there is no technical issue with allowing this, then
we should do so, to allow simple recursion (e.g. handling defaults by invoking
the same rule as if they were explicitly specified).

Are there any places where we currently allow an attribute, but where
implementation considerations make it difficult to allow a `macro_rules`
attribute? (For instance, places where we currently allow attributes but don't
allow proc-macro attributes.)

Before stabilizing this feature, we should make sure it doesn't produce wildly
worse error messages in common cases.

Before stabilizing this feature, we should receive feedback from crate
maintainers, and potentially make further improvements to `macro_rules` to make
it easier to use for their use cases. This feature will provide motivation to
evaluate many new use cases that previously weren't written using
`macro_rules`, and we should consider quality-of-life improvements to better
support those use cases.

## Future possibilities
[future-possibilities]: #future-possibilities

We should provide a way to define `derive` macros declaratively, as well.

We should provide a way for `macro_rules!` macros to provide better error
reporting, with spans, rather than just pointing to the macro.

We may want to provide more fine-grained control over the requirement for
`unsafe`, to make it easier for attribute macros to be safe in some
circumstances and unsafe in others (e.g. unsafe only if a given parameter is
provided).

As people test this feature and run into limitations of `macro_rules!` parsing,
we should consider additional features to make this easier to use for various
use cases.

Some use cases involve multiple attribute macros that users expect to be able
to apply in any order. For instance, `#[test]` and `#[should_panic]` can appear
on the same function in any order. Implementing that via this mechanism for
attribute macros would require making both of those attributes into macros that
both do all the parsing regardless of which got invoked first, likely by
invoking a common helper. We should consider if we consider that mechanism
sufficient, or if we should provide another mechanism for a set of related
attribute macros to appear in any order.

If it turns out many users of attribute macros want to emit new tokens but
leave the tokens they were applied to unmodified, we may want to have a
convenient mechanism for that.
