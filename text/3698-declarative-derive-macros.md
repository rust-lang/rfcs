- Feature Name: `declarative_derive_macros`
- Start Date: 2024-09-20
- RFC PR: [rust-lang/rfcs#3698](https://github.com/rust-lang/rfcs/pull/3698)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Support implementing `derive(Trait)` via a `macro_rules!` macro.

# Motivation
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

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can define a macro to implement `derive(MyTrait)` by defining a
`macro_rules!` macro with the `#[macro_derive]` attribute. Such a macro can
create new items based on a struct, enum, or union. Note that the macro can
only append new items; it cannot modify the item it was applied to.

For example:

```rust
trait Answer { fn answer(&self) -> u32; }

#[macro_derive]
macro_rules! Answer {
    // Simplified for this example
    (struct $n:ident $_:tt) => {
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

A derive macro may also define *helper attributes*. These attributes are
[inert](https://doc.rust-lang.org/reference/attributes.html#active-and-inert-attributes),
and exist for the derive macro to parse and act upon. Note that
they're visible to *all* macros, not just the one that defined them; macros
should ignore any attributes not meant for them.

To define helper attributes, put an attributes key in the `macro_derive`
attribute, with a comma-separated list of identifiers for helper attributes:
`#[macro_derive(attributes(helper))]`. The derive macro can process the
`#[helper]` attribute, along with any arguments to it, as part of the item the
derive macro was applied to.

If a derive macro emits a trait impl for the type, it may want to add the
[`#[automatically_derived]`](https://doc.rust-lang.org/reference/attributes/derive.html#the-automatically_derived-attribute)
attribute, for the benefit of diagnostics.

If a derive macro mistakenly emits the token stream it was applied to
(resulting in a duplicate item definition), the error the compiler emits for
the duplicate item should hint to the user that the macro was defined
incorrectly, and remind the user that derive macros only append new items.

# Drawbacks
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

# Rationale and alternatives
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

We could allow `macro_rules!` derive macros to emit a replacement token stream;
however, that would be inconsistent with the restriction preventing proc macros
from doing the same.

We could allow directly invoking a `macro_rules!` derive macro as a
function-like macro. This has the potential for confusion, given the
append-only nature of derive macros versus the behavior of normal function-like
macros. It might potentially be useful for code reuse, however.

# Prior art
[prior-art]: #prior-art

We have had proc-macro-based derive macros for a long time, and the ecosystem
makes extensive use of them.

The [`macro_rules_attribute`](https://crates.io/crates/macro_rules_attribute)
crate defines proc macros that allow invoking declarative macros as derives,
demonstrating a demand for this. This feature would allow defining such derives
without requiring proc macros at all, and would support the same invocation
syntax as a proc macro.

The `macro_derive` attribute and its `attributes` syntax are based on the
[existing `proc_macro_derive` attribute for proc
macros](https://doc.rust-lang.org/reference/procedural-macros.html#derive-macros).

# Future possibilities
[future-possibilities]: #future-possibilities

We should provide a way for `macro_rules!` macros to provide better error
reporting, with spans, rather than just pointing to the macro.

We may want to support error recovery, so that a derive can produce an error
but still provide enough for the remainder of the compilation to proceed far
enough to usefully report further errors.

As people test this feature and run into limitations of `macro_rules!` parsing,
we should consider additional features to make this easier to use for various
use cases.

We may want to provide a means to namespace helper attributes or detect
collisions between them. This would apply to both proc macros and
`macro_rules!` macros.
