- Feature Name: `inline_const`
- Start Date: 2020-04-30
- RFC PR: [rust-lang/rfcs#2920](https://github.com/rust-lang/rfcs/pull/2920)
- Rust Issue: [rust-lang/rust#76001](https://github.com/rust-lang/rust/issues/76001)

# Summary
[summary]: #summary

Adds a new syntactical element called an "inline `const`", written as
`const { ... }`, which instructs the compiler to execute the contents of the
block at compile-time. An inline `const` can be used as an expression or
anywhere in a pattern where a named `const` would be allowed.

```rust
use std::net::Ipv6Addr;

fn mock_ip(use_localhost: bool) -> &'static Ipv6Addr {
    if use_localhost {
        &Ipv6Addr::LOCALHOST
    } else {
        const { &Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0) }
    }
}

const MMIO_BIT1: u8 = 4;
const MMIO_BIT2: u8 = 5;

fn main() {
    match read_mmio() {
        0 => {}
        const { 1 << MMIO_BIT1 } => println!("FOO"),
        const { 1 << MMIO_BIT2 } => println!("BAR"),

        _ => unreachable!(),
    }
}
```

# Motivation
[motivation]: #motivation

Rust has `const` items, which are guaranteed to be initialized at compile-time.
Because of this, they can do things that normal variables cannot.  For example,
a reference in a `const` initializer has the `'static` lifetime, and a `const`
can be used as an array initializer even if the type of the array is not
`Copy` (with [RFC 2203]).

[RFC 2203]: https://github.com/rust-lang/rfcs/pull/2203

```rust
fn foo(x: &i32) -> &i32 {
    const ZERO: &'static i32 = &0;
    if *x < 0 { ZERO } else { x }
}


fn foo() -> &u32 {
    const RANGE: Range<i32> = 0..5; // `Range` is not `Copy`
    let three_ranges = [RANGE; 3];
}
```

Writing out a `const` declaration every time we need a long-lived reference or
a non-`Copy` array initializer can be annoying. To improve the situation,
[RFC 1414] introduced rvalue static promotion to extend lifetimes, and
[RFC 2203] extended the concept of promotion to array initializers.
As a result, the previous example can be written more concisely.

[RFC 1414]: https://github.com/rust-lang/rfcs/pull/1414

```rust
fn foo(x: &i32) -> &i32 {
    if *x < 0 { &0 } else { x }
}

fn foo() -> &u32 {
    let three_ranges = [0..5; 3];
}
```

However, the fact that we are executing the array initializer or expression
after the `&` at compile-time is not obvious to the user. To avoid violating
their assumptions, we are very careful to promote only in cases where the user
cannot possibly tell that their code is not executing at runtime. This means a
[long list of rules][prom-rules] for determining the promotability of expressions, and it
means expressions that call a `const fn` or that result in a type with a `Drop`
impl need to use a named `const` declaration.

[prom-rules]: https://github.com/rust-lang/const-eval/blob/master/promotion.md#promotability

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This proposal is a middle ground, which is less verbose than named constants
but more obvious and expressive than promotion. In expression context, it
behaves much like the user had written the following, where `Ty` is the
inferred type of the code within the inline `const` expression (represented by
the ellipsis):

```rust
{ const UNIQUE_IDENT: Ty = ...; UNIQUE_IDENT }
```

With this extension to the language, users can ensure that their code executes
at compile-time without needing to declare a separate `const` item that is only
used once.

```rust
fn foo(x: &i32) -> &i32 {
  if *x < 0 { const { &4i32.pow(4) } } else { x }
}

fn foo() -> &u32 {
    let three_ranges = [const { (0..=5).into_inner() }; 3];
}
```

## Patterns

Patterns are another context that require a named `const` when using complex
expressions.  Unlike in the expression context, where promotion is sometimes
applicable, there is no other choice here.

```rust
fn foo(x: i32) {
    const CUBE: i32 = 3.pow(3);
    match x {
        CUBE => println!("three cubed"),
        _ => {}
    }
}
```

If that `const` is only used inside a single pattern, writing the code using an
inline `const` block makes it easier to scan.

```rust
fn foo(x: i32) {
    match x {
        const { 3.pow(3) } => println!("three cubed"),
        _ => {}
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC extends the [grammar for expressions] to be,

[grammar for expressions]: https://doc.rust-lang.org/stable/reference/expressions.html#expressions

> ```
> ExpressionWithBlock :
>    OuterAttribute*†
>    (
>         BlockExpression
>       | AsyncBlockExpression
>       | UnsafeBlockExpression
>       | ConstBlockExpression // new
>       | LoopExpression
>       | IfExpression
>       | IfLetExpression
>       | MatchExpression
>    )
>
> ConstBlockExpression: `const` BlockExpression // new
> ```

This RFC extends the [grammar for patterns] to be,

[grammar for patterns]: https://doc.rust-lang.org/stable/reference/patterns.html

> ```
> Pattern :
>      LiteralPattern
>    | IdentifierPattern
>    | WildcardPattern
>    | RangePattern
>    | ReferencePattern
>    | StructPattern
>    | TupleStructPattern
>    | TuplePattern
>    | GroupedPattern
>    | SlicePattern
>    | PathPattern
>    | MacroInvocation
>    | ConstBlockExpression // new
>
> RangePatternBound :
>      CHAR_LITERAL
>    | BYTE_LITERAL
>    | -? INTEGER_LITERAL
>    | -? FLOAT_LITERAL
>    | PathInExpression
>    | QualifiedPathInExpression
>    | ConstBlockExpression // new
> ```

In both the expression and pattern context, an inline `const` behaves as if the
user had declared a uniquely named constant in the containing scope and
referenced it.

## Generic Parameters

For now, inline `const` expressions and patterns cannot refer to in-scope
generic parameters. As of this writing, the same restriction applies to array
length expressions, which seem like a good precedent for this RFC. As far as I
know, this is only a temporary restriction; the long-term goal is to allow
array length expressions to use generic parameters. When this happens, inline
`const` expressions and patterns will also be allowed to refer to in-scope
generics.

```rust
fn foo<T>() {
    let x = [4i32; std::mem::size_of::<T>()];   // NOT ALLOWED (for now)
    let x = const { std::mem::size_of::<T>() }; // NOT ALLOWED (for now)
}
```

## Containing `unsafe`

At present, containing `unsafe` blocks do not apply to array length expressions inside:

```rust
fn bar() {
    let x = unsafe {
        [4i32; std::intrinsics::unchecked_add(2i32, 3i32)] // ERROR
    };
}
```

I find this somewhat strange, but consistency is important, so inline `const`
expressions should behave the same way. The following would also fail to
compile:

```rust
fn bar() {
    let x = unsafe {
      const { std::intrinsics::unchecked_add(2i32, 3i32) } // ERROR
    };
}
```

If [#72359] is considered a bug and resolved, that change would also apply to
inline `const` expressions and patterns.

[#72359]: https://github.com/rust-lang/rust/issues/72359

# Drawbacks
[drawbacks]: #drawbacks

This excludes other uses of the `const` keyword in expressions and patterns.
I'm not aware of any other proposals that would take advantage of this.

This would also be the first use of type inference for const initializers. Type
inference for named constants was proposed in [RFC 1349]. I don't believe the
blockers for this were technical, so I think this is possible.

[RFC 1349]: https://github.com/rust-lang/rfcs/issues/1349

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The main alternative is the status quo. Maintaining it will likely result in
promotion being used for more contexts. The lang-team decided to [explore this
approach](https://github.com/rust-lang/rust/pull/70042#issuecomment-612221597)
instead.

It would also possible to separate out the parts of this RFC relating to patterns
so that they can be decided upon separately.

# Prior art
[prior-art]: #prior-art

Zig has the `comptime` keyword that [works similarly][zig] when it appears
before a block.

I'm not aware of equivalents in other languages.

AFAIK, this was [first proposed] by **@scottmcm**.

[zig]: https://kristoff.it/blog/what-is-zig-comptime/#compile-time-function-calls
[first proposed]: https://internals.rust-lang.org/t/quick-thought-const-blocks/7803/9

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Naming

I prefer the name inline `const`, since it signals that there is no difference
between a named `const` and an inline one.

**@scottmcm** prefers "`const` block", which is closer to the syntax and parallels
the current terminology of `async` block and `unsafe` block. It also avoids any
accidental conflation with the `#[inline]` attribute, which is unrelated.
Additionally, it doesn't extend nicely to the single-expression variant
discussed in [future possibilities].

**@RalfJung** prefers "anonymous `const`". **@scottmcm** mentioned in Zulip
that this could be confused with the `const _: () = ...;` syntax introduced in
[RFC 2526]. The reference refers to these as "unnamed" constants.

[RFC 2526]: https://github.com/rust-lang/rfcs/pull/2526

## Lints about placement of inline `const`

An inline `const` is eligible for promotion in an implicit context (just like a
named `const`), so the following are all guaranteed to work:

```rust
let x: &'static i32 = &const { 4i32.pow(4) };
let x: &'static i32 = const { &4i32.pow(4) };

// If RFC 2203 is stabilized
let v = [const { Vec::new() }; 3];
let v = const { [Vec::new(); 3] };
```

I don't have strong feelings about which version should be preferred.
**@RalfJung** points out that `&const { 4 + 2 }` is more readable than `const {
&(4 + 2) }`.

Note that it may be possible for RFC 2203 to use the explicit rules for
promotability when `T: !Copy`. In this case, the last part of the example above
could simply be written as `[Vec::new(); 3]`.

Inline `const`s are allowed within `const` and `static` initializers, just as we
currently allow nested `const` declarations. Whether to lint against inline
`const` expressions inside a `const` or `static` is also an open question.

# Future possibilities
[future possibilities]: #future-possibilities

It would be possible to allow the syntax `const expr` for an inline `const` that
consists of a single expression. This is analogous to the single expression
variant of closures: `|| 42`. This is backwards compatible with the current proposal.

At some point (an edition boundary?), we may want to narrow the scope of
expressions that are eligible for implicit promotion. Inline `const`
expressions would be the recommended replacement for expressions that were no
longer eligible.
