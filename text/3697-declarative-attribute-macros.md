- Feature Name: `declarative_attribute_macros`
- Start Date: 2024-09-20
- RFC PR: [rust-lang/rfcs#3697](https://github.com/rust-lang/rfcs/pull/3697)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Support defining `macro_rules!` macros that work as attribute macros.

# Motivation
[motivation]: #motivation

Many crates provide attribute macros. Today, this requires defining proc
macros, in a separate crate, typically with several additional dependencies
adding substantial compilation time, and typically guarded by a feature that
users need to remember to enable.

However, many common cases of attribute macros don't require any more power
than an ordinary `macro_rules!` macro. Supporting these common cases would
allow many crates to avoid defining proc macros, reduce dependencies and
compilation time, and provide these macros unconditionally without requiring a
the user to enable a feature.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When defining a `macro_rules!` macro, you can prefix some of the macro's rules
with `attribute(...) =>` to allow using the macro as an attribute. The
arguments to the attribute, if any, are parsed by the *MacroMatcher* in the
first set of parentheses; the second *MacroMatcher* parses the entire construct
the attribute was applied to. The resulting macro will work anywhere an
attribute currently works.

```rust
macro_rules! main {
    attribute() => ($func:item) => { make_async_main!($func) };
    attribute(threads = $threads:literal) => ($func:item) => { make_async_main!($threads, $func) };
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

Note that a single macro can have both attribute and non-attribute rules.
Attribute invocations can only match the attribute rules, and non-attribute
invocations can only match the non-attribute rules.

For simplicity, an attribute macro may not recursively invoke its attribute
rules; to recurse, invoke a non-attribute rule or another macro.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The grammar for macros is extended as follows:

> _MacroRule_ :\
> &nbsp;&nbsp; ( `attribute` _MacroMatcher_ `=>` )<sup>?</sup>  _MacroMatcher_ `=>` _MacroTranscriber_

The first _MacroMatcher_ matches the attribute's arguments, which will be an
empty token tree if not present. The second _MacroMatcher_ matches the entire
construct the attribute was applied to, receiving precisely what a
proc-macro-based attribute would in the same place.

This grammar addition is backwards compatible: previously, a _MacroRule_ could
only start with `(`, `[`, or `{`, so the parser can easily distinguish the
identifier `attribute`.

Attribute macros declared using `macro_rules!` are
[active](https://doc.rust-lang.org/reference/attributes.html#active-and-inert-attributes),
just like those declared using proc macros.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Adding this feature will allow many crates in the ecosystem to drop their proc
macro crates and corresponding dependencies, and decrease their build times.

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

We could use `attr` rather than `attribute`. Rust usually avoids abbreviating
except for the most common constructs; however, this can occur repeatedly in
multiple rules, so it may make sense to abbreviate it.

# Prior art
[prior-art]: #prior-art

We have had proc-macro-based attribute macros for a long time, and the
ecosystem makes extensive use of them.

The [`macro_rules_attribute`](https://crates.io/crates/macro_rules_attribute)
crate defines proc macros that allow invoking declarative macros as attributes,
demonstrating a demand for this. This feature would allow defining such
attributes without requiring proc macros at all, and would support the same
invocation syntax as a proc macro.

# Future possibilities
[future-possibilities]: #future-possibilities

We should provide a way to define `derive` macros declaratively, as well.
