- Feature Name: `enum_variant_cast_patterns`
- Start Date: 2026-07-15
- RFC PR: rust-lang/rfcs#0000
- Rust Issue: rust-lang/rust#0000

# Summary
[summary]: #summary

Permit a unit variant of a unit-only enum with an explicit primitive integer
representation to be cast to that representation type directly in a pattern.

```rust
#[repr(u8)]
enum Operation {
    Banana = 1,
    Apple = 2,
}

fn handle(value: u8) {
    match value {
        Operation::Banana as u8 => {}
        Operation::Apple as u8 => {}
        _ => {}
    }
}
```

This is syntactic sugar for introducing a named constant of the representation
type and using that constant as a pattern. It does not introduce an implicit
conversion, permit general expressions in patterns, or change enum layout,
validity, or exhaustiveness.

# Motivation
[motivation]: #motivation

Rust permits a fieldless enum value to be cast to an integer in expression
position. Explicitly represented enums are consequently useful for describing
integer protocols and formats while retaining meaningful names in Rust code:

```rust
#[repr(u8)]
enum Opcode {
    Ping = 0x01,
    Data = 0x02,
    Close = 0xff,
}

let wire_value: u8 = Opcode::Ping as u8;
```

The same spelling cannot currently be used when decoding an integer:

```rust,compile_fail
fn decode(byte: u8) {
    match byte {
        Opcode::Ping as u8 => {}
        Opcode::Data as u8 => {}
        Opcode::Close as u8 => {}
        _ => {}
    }
}
```

Although `Opcode::Ping as u8` is a valid constant expression with the same type
as the scrutinee, it is not part of Rust's pattern grammar. Users must repeat the
type and introduce a second name for every value they wish to match:

```rust
const PING: u8 = Opcode::Ping as u8;
const DATA: u8 = Opcode::Data as u8;
const CLOSE: u8 = Opcode::Close as u8;

fn decode(byte: u8) {
    match byte {
        PING => {}
        DATA => {}
        CLOSE => {}
        _ => {}
    }
}
```

This indirection is particularly awkward in parsers for binary protocols,
hardware registers, file formats, and foreign interfaces. The enum already
provides names for the integer values, but the decoding side must create and
maintain a parallel set of names solely because the cast is not accepted in
pattern position.

Allowing the cast directly makes the relationship visible at the use site. The
pattern states both where the value comes from and which representation is being
matched, without adding a second declaration that can drift away from the enum.

The proposal is intentionally narrower than general constant expressions in
patterns. The accepted expression has a single, statically known meaning: read a
unit variant's discriminant in the enum's declared integer representation. Both
the source and target are constrained by the enum declaration.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An explicitly represented, unit-only enum may use a variant cast as a pattern
when matching a value of its representation type:

```rust
#[repr(i16)]
enum Status {
    Failed = -2,
    Ready = 4,
}

fn describe(raw: i16) -> &'static str {
    match raw {
        Status::Failed as i16 => "failed",
        Status::Ready as i16 => "ready",
        _ => "unknown",
    }
}
```

The form is accepted in every position in which a constant pattern may occur,
including nested and or-patterns:

```rust
fn is_terminal(raw: i16) -> bool {
    matches!(raw, Status::Failed as i16 | Status::Ready as i16)
}
```

For a cast to be accepted as a pattern, all of the following must hold:

1. The left operand resolves to a unit variant.
2. The variant's enum is unit-only: every variant in the enum has no fields.
3. The enum has an explicit primitive integer representation, such as
   `#[repr(u8)]` or `#[repr(i16)]`.
4. The cast target is exactly that primitive integer representation type.
5. The ordinary visibility and name-resolution rules permit referring to the
   variant.

The cast target is not inferred:

```rust,compile_fail
#[repr(u8)]
enum Command {
    Start = 1,
}

fn f(value: u16) {
    match value {
        // Not accepted: the enum's representation is u8, not u16.
        Command::Start as u16 => {}
        _ => {}
    }
}
```

An enum without an explicit representation is not eligible, even if the same
cast is valid in expression position:

```rust,compile_fail
enum Command {
    Start = 1,
}

fn f(value: u8) {
    match value {
        Command::Start as u8 => {}
        _ => {}
    }
}
```

Enums with data-carrying variants are also outside the scope of this feature:

```rust,compile_fail
#[repr(u8)]
enum Message {
    Ping = 1,
    Data(u8) = 2,
}

fn f(value: u8) {
    match value {
        Message::Ping as u8 => {}
        _ => {}
    }
}
```

No conversion from an integer into the enum is performed. The scrutinee remains
an integer, and values that are not enum discriminants remain valid integer
values. Consequently, exhaustiveness is checked using the integer type:

```rust,compile_fail
#[repr(u8)]
enum Bit {
    Zero = 0,
    One = 1,
}

fn f(value: u8) {
    // Still non-exhaustive: a u8 has values other than 0 and 1.
    match value {
        Bit::Zero as u8 => {}
        Bit::One as u8 => {}
    }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Syntax

Add the following pattern form, where `EnumVariantPath` is a value namespace
path and `PrimitiveIntegerType` is one of `u8`, `u16`, `u32`, `u64`, `u128`,
`usize`, `i8`, `i16`, `i32`, `i64`, `i128`, or `isize`:

```text
EnumVariantCastPattern :
    EnumVariantPath `as` PrimitiveIntegerType
```

`EnumVariantCastPattern` is a non-binding, refutable pattern and may appear
wherever a constant pattern may appear. It has the precedence of an atomic
pattern. In particular, in an or-pattern such as
`E::A as u8 | E::B as u8`, each cast is a complete pattern.

This RFC does not add arbitrary cast expressions to patterns. The left operand
is syntactically a path and must semantically resolve to an eligible enum
variant; the right operand is a primitive integer type.

## Eligibility

Let the resolved variant belong to enum `E`, and let `R` be the primitive
integer type named by the cast target. The pattern is well-formed if:

- the resolved variant is a unit variant;
- every variant of `E` is a unit variant;
- `E` has an explicit primitive integer representation whose integer type is
  `R` (including when combined with `C`, as in `#[repr(C, u8)]`);
- the corresponding enum cast is otherwise legal, including the existing
  restriction on enums that implement `Drop`; and
- the pattern's expected type is `R` under the ordinary pattern type-checking
  rules.

Type aliases do not satisfy the syntactic cast-target requirement. For example,
given `type Byte = u8`, `E::A as Byte` remains an expression-only form. Keeping
the representation spelling explicit makes the construct locally recognizable
and avoids requiring type normalization during parsing or early pattern
classification.

## Semantics

After name resolution and type checking, an enum variant cast pattern is
equivalent to a fresh, hygienic constant of the representation type followed by
a use of that constant as a pattern. Informally:

```rust,ignore
// Source
E::V as R

// Semantic equivalent
const __VALUE: R = E::V as R;
__VALUE
```

The generated constant is an explanatory desugaring only; no name is introduced
into the user's scope.

The value of the pattern is the discriminant of `E::V`, represented as `R`.
Because `R` must exactly equal the explicit representation type, this pattern
form never performs a subsequent widening, truncating, or signedness-changing
numeric cast.

The pattern participates in usefulness, reachability, and exhaustiveness
checking exactly like the corresponding integer constant pattern. The enum does
not constrain the valid values of the matched integer type. Existing diagnostics
for duplicate or unreachable integer patterns therefore apply.

The feature does not affect:

- the layout or valid values of any enum;
- the rules for assigning explicit or implicit discriminants;
- casts in expression position;
- conversion from integers to enums;
- `core::mem::Discriminant`;
- matching an enum value against one of its own variants; or
- the exhaustiveness domain of the scrutinee type.

## Macros

The new form is accepted by `pat` and `pat_param` macro fragment specifiers in
the same circumstances as any other pattern. It is not accepted by a `path`
fragment because the complete form contains `as R`. Existing macro invocations
remain unaffected because token sequences containing `as` are not currently
valid instances of this pattern form.

## Diagnostics

When a cast-shaped construct is encountered in pattern position but fails an
eligibility rule, diagnostics should identify the failed rule rather than report
only that arbitrary expressions are forbidden. Where applicable, the compiler
should suggest a named constant pattern as the general-purpose alternative.

# Drawbacks
[drawbacks]: #drawbacks

This feature adds a specialized expression-like form to the pattern grammar.
Every special case has a language-complexity and implementation cost, and users
must learn why this particular constant expression is permitted while similar
ones are not.

The feature also provides only one direction of a common protocol conversion.
It makes matching a representation value against enum discriminants concise,
but does not turn an arbitrary integer into an enum. Applications still need to
choose how unknown integer values are handled.

Requiring the exact representation type means code matching a wider storage
type must continue to use a named constant, a literal, or first narrow the
scrutinee. This is intentionally restrictive, but may feel inconsistent with the
more permissive enum casts available in expression position.

Finally, unit-only enums with explicit integer representations are already a
compact way to give integers names, but they do not declare that every value of
the integer type is a valid enum value. This RFC must not encourage unsafe
integer-to-enum transmutation; no such conversion is provided here.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Status quo: named constants

Users can define one constant per discriminant and use those constants in
patterns. This requires no language change and works for arbitrary constant
expressions. However, it creates parallel names and declarations for values the
enum already names, obscures the connection at the match site, and creates an
additional maintenance point.

## Use integer literals

The example can be written with `1`, `2`, and so on. Literals are concise but
discard the semantic names supplied by the enum and can silently become stale if
a discriminant changes.

## Match the enum instead

Code can validate or convert the integer first and then match an enum value. Rust
does not provide a general safe integer-to-enum cast because not every integer is
necessarily a valid discriminant. Hand-written conversion is useful when the
caller needs an enum value, but it is unnecessary ceremony when the desired
operation is simply to dispatch on a few integer protocol values while retaining
an unknown-value arm.

## Match guards

```rust
match byte {
    value if value == Opcode::Ping as u8 => {}
    _ => {}
}
```

Guards work today, but are more verbose and are not considered by exhaustiveness
or usefulness checking in the same way as constant patterns.

## Stabilize inline const patterns

RFC 2920 proposed the more general spelling:

```rust,ignore
match byte {
    const { Opcode::Ping as u8 } => {}
    _ => {}
}
```

Inline const expressions were stabilized, but inline const patterns were later
removed after remaining unstable. Reviving inline const patterns would solve
this use case and many others with one general mechanism. It would also inherit
the broader semantic, syntactic, and implementation questions of arbitrary
inline constants in patterns. This RFC instead covers a small form whose value,
type, and structural equality are all immediately determined.

Even if inline const patterns are added in the future, the direct spelling may
remain preferable for symmetry with expression-position enum casts.

## Permit all constant expressions directly in patterns

Allowing any const-evaluable expression before `=>` would be more general, but
would blur the syntactic distinction between expressions and patterns, create
parsing ambiguities, complicate macro fragment behavior, and reopen the semantic
issues that led Rust to restrict constants in patterns. This proposal does not
require or imply that generalization.

## Permit any integer cast target

Expression-position enum casts may target integer types other than the enum's
representation type. Extending that rule to this pattern form would allow, for
example, `E::V as u8` for `#[repr(i16)] enum E`. Such casts can truncate or change
signedness. Requiring the exact representation type keeps the pattern a direct
name for the declared representation value rather than a general numeric
conversion.

## Omit the explicit representation requirement

Unit-only enums without `#[repr(...)]` also support integer casts in expression
position. Their discriminants have logical integer values, but no explicit
primitive representation contract was selected by the author. Requiring an
explicit integer representation makes the intended integer interface visible in
the enum declaration and gives the pattern a unique, non-inferred target type.

## Use an implicit pattern conversion

Another possible spelling is `Opcode::Ping` in a pattern whose expected type is
`u8`. That would make an enum variant pattern mean either enum construction or
an integer discriminant depending on the expected type. The implicit conversion
would be less visible, would interact with inference, and could make diagnostics
and code review harder. The explicit `as u8` spelling preserves Rust's existing
conversion syntax and makes the operation unambiguous.

# Prior art
[prior-art]: #prior-art

C and C++ allow enumerator constants in integral constant-expression contexts,
including `case` labels. Rust keeps enum values nominally typed and uses explicit
casts instead. This RFC preserves Rust's explicitness while allowing the cast in
the corresponding pattern context.

Rust already allows named constants as patterns and enum-to-integer casts as
constant expressions. This proposal combines those existing semantics in one
narrow syntactic form.

Relevant Rust design history includes:

- [RFC 1445, restricting constants in patterns](https://rust-lang.github.io/rfcs/1445-restrict-constants-in-patterns.html);
- [RFC 2363, arbitrary enum discriminants](https://rust-lang.github.io/rfcs/2363-arbitrary-enum-discriminant.html);
- [RFC 2920, inline const](https://rust-lang.github.io/rfcs/2920-inline-const.html); and
- [RFC 3535, constants in patterns](https://rust-lang.github.io/rfcs/3535-constants-in-patterns.html).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should `#[repr(C, Int)]` enums be accepted, as proposed, or should the feature
  initially be limited to an attribute containing only the primitive integer
  representation?
- Should type aliases for the exact representation type be accepted? This draft
  deliberately requires the primitive type to be written directly.
- Should the feature use the Reference's broader "field-less enum" category, or
  remain limited to unit-only enums? This draft chooses unit-only enums because
  the intended spelling names a unit variant and because it is the simplest,
  most uniform subset.

# Future possibilities
[future-possibilities]: #future-possibilities

Experience with this feature could inform whether other narrowly recognizable
constant forms belong in patterns. Any such extension should be proposed on its
own merits; acceptance of this RFC does not imply acceptance of arbitrary
expressions or arbitrary casts in patterns.

Rust may separately gain APIs or syntax for obtaining discriminants from enums
with data, or derive-based integer-to-enum conversion. Those features address
different directions of conversion and are not blocked or specified by this
RFC.

# Acknowledgements
[acknowledgements]: #acknowledgements

This RFC was developed from the public Zulip proposal
[“Allow explicitly represented fieldless enum variants”](https://rust-lang.zulipchat.com/#narrow/channel/122651-general/topic/Allow.20explicitly.20represented.20fieldless.20enum.20variants/with/610897517).
