- Feature Name: `destructuring_assignment`
- Start Date: 2020-04-17
- RFC PR: [rust-lang/rfcs#2909](https://github.com/rust-lang/rfcs/pull/2909)
- Rust Issue: [rust-lang/rust#71126](https://github.com/rust-lang/rust/issues/71126)
- Proof-of-concept: [rust-lang/rust#71156](https://github.com/rust-lang/rust/pull/71156)

# Summary
[summary]: #summary

We allow destructuring on assignment, as in `let` declarations. For instance, the following are now
accepted:

```rust
(a, (b.x.y, c)) = (0, (1, 2));
(x, y, .., z) = (1.0, 2.0, 3.0, 4.0, 5.0);
[_, f, *baz(), a[i]] = foo();
[g, _, h, ..] = ['a', 'w', 'e', 's', 'o', 'm', 'e', '!'];
Struct { x: a, y: b } = bar();
Struct { x, y } = Struct { x: 5, y: 6 };
```

This brings assignment in line with `let` declaration, in which destructuring is permitted. This
will simplify and improve idiomatic code involving mutability.

# Motivation
[motivation]: #motivation

Destructuring assignment increases the consistency of the language, in which assignment is typically
expected to behave similarly to variable declarations. The aim is that this feature will increase
the clarity and concision of idiomatic Rust, primarily in code that makes use of mutability. This
feature is [highly desired among Rust developers](https://github.com/rust-lang/rfcs/issues/372).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You may destructure a value when making an assignment, just as when you declare variables. See the
[Summary](#Summary) for examples. The following structures may be destructured:

- Tuples.
- Slices.
- Structs (including unit and tuple structs).
- Unique variants of enums.

You may use `_` and `..` as in a normal declaration pattern to ignore certain values.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The feature as described here has been implemented as a proof-of-concept
(https://github.com/rust-lang/rust/pull/71156). It follows essentially the [suggestions of
@Kimundi](https://github.com/rust-lang/rfcs/issues/372#issuecomment-214022963) and [of
@drunwald](https://github.com/rust-lang/rfcs/issues/372#issuecomment-262519146).

The Rust compiler already parses complex expressions on the left-hand side of an assignment, but
does not handle them other than emitting an error later in compilation. We propose to add
special-casing for several classes of expressions on the left-hand side of an assignment, which act
in accordance with destructuring assignment: i.e. as if the left-hand side were actually a pattern.
Actually supporting patterns directly on the left-hand side of an assignment significantly
complicates Rust's grammar and it is not clear that it is even technically feasible. Conversely,
handling some classes of expressions is much simpler, and is indistinguishable to users, who will
receive pattern-oriented diagnostics due to the desugaring of expressions into patterns.

To describe the context of destructuring assignments more precisely, we add a new class of
expressions, which we call "assignee expressions".
Assignee expressions are analogous to [place
expressions](https://doc.rust-lang.org/reference/expressions.html#place-expressions-and-value-expressions)
(also called "lvalues") in that they refer to expressions representing a memory location, but may
only appear on the left-hand side of an assignment (unlike place expressions). Every place
expression is also an assignee expression.

The class of assignee expressions is defined inductively:

- Place: `place`.
- Underscore: `_`.
- Tuples: `(assignee, assignee, assignee)`, `(assignee, .., assignee)`, `(.., assignee, assignee)`, `(assignee, assignee, ..)`.
- Slices: `[assignee, assignee, assignee]`, `[assignee, .., assignee]`, `[.., assignee, assignee]`, `[assignee, assignee, ..]`.
- Tuple structs: `path(assignee, assignee, assignee)`, `path(assignee, .., assignee)`, `path(.., assignee, assignee)`,
  `path(assignee, assignee, ..)`.
- Structs: `path { field: assignee, field: assignee }`, `path { field: assignee, field: assignee, .. }`.
- Unit structs: `path`.

The place expression "The left operand of an assignment or compound assignment expression." ibid.
is changed to "The left operand of a compound assignment expression.", while
"The left operand of an assignment expression." is now an assignee expression.

The general idea is that we will desugar the following complex assignments as demonstrated.

```rust
(a, b) = (3, 4);

[a, b] = [3, 4];

Struct { x: a, y: b } = Struct { x: 3, y: 4};

// desugars to:

{
    let (_a, _b) = (3, 4);
    a = _a;
    b = _b;
}

{
    let [_a, _b] = [3, 4];
    a = _a;
    b = _b;
}

{
    let Struct { x: _a, y: _b } = Struct { x: 3, y: 4};
    a = _a;
    b = _b;
}
```

Note that the desugaring ensures that destructuring assignment, like normal assignment, is an
expression.

We support the following classes of expressions:

- Tuples.
- Slices.
- Structs (including unit and tuple structs).
- Unique variants of enums.

In the desugaring, we convert the expression `(a, b)` into an analogous pattern `(_a, _b)` (whose
identifiers are fresh and thus do not conflict with existing variables). A nice side-effect is that
we inherit the diagnostics for normal pattern-matching, so users benefit from existing diagnostics
for destructuring declarations.

Nested structures may be destructured, for instance:

```rust
let (a, b, c);
((a, b), c) = ((1, 2), 3);

// desugars to:

let (a, b, c);
{
    let ((_a, _b), _c) = ((1, 2), 3);
    a = _a;
    b = _b;
    c = _c;
};
```

We also allow arbitrary parenthesisation, as with patterns, although unnecessary parentheses will
trigger the `unused_parens` lint.

Note that `#[non_exhaustive]` must be taken into account properly: enums marked `#[non_exhaustive]`
may not have their variants destructured, and structs marked `#[non_exhaustive]` may only be
destructured using `..`.

Patterns must be irrefutable. In particular, only slice patterns whose length is known at compile-
time, and the trivial slice `[..]` may be used for destructuring assignment.

Unlike in usual `let` bindings, default binding modes do *not* apply for the desugared destructuring
assignments, as this leads to counterintuitive behaviour since the desugaring is an implementation
detail.

## Diagnostics

It is worth being explicit that, in the implementation, the diagnostics that are reported are
pattern diagnostics: that is, because the desugaring occurs regardless, the messages will imply that
the left-hand side of an assignment is a true pattern (the one the expression has been converted
to). For example:

```rust
[*a] = [1, 2]; // error: pattern requires 1 element but array has 2
```

Whilst `[*a]` is not strictly speaking a pattern, it behaves similarly to one in this context. We
think that this results in a better user experience, as intuitively the left-hand side of a
destructuring assignment acts like a pattern "in spirit", but this is technically false: we should
be careful that this does not result in misleading diagnostics.

## Underscores and ellipses

In patterns, we may use `_` and `..` to ignore certain values, without binding them. While range
patterns already have analogues in terms of range expressions, the underscore wildcard pattern
currently has no analogous expression. We thus add one, which is only permitted in the left-hand side
of an assignment: any other use results in the same "reserved identifier" error that currently
occurs for invalid uses of `_` as an expression. A consequence is that the following becomes valid:

```rust
_ = 5;
```

Functional record update syntax (i.e. `..x`) is forbidden in destructuring assignment, as we believe
there is no sensible and clear semantics for it in this setting. This restriction could be relaxed
in the future if a use-case is found.

The desugaring treats the `_` expression as an `_` pattern and the fully empty range `..` as a `..`
pattern. No corresponding assignments are generated. For example:

```rust
let mut a;
(a, _) = (3, 4);
(.., a) = (1, 2, 3, 4);

// desugars to:

{
    let (_a, _) = (3, 4);
    a = _a;
}

{
    let (.., _a) = (1, 2, 3, 4);
    a = _a;
}
```

and similarly for slices and structs.

## Unsupported patterns

We do not support the following "patterns" in destructuring assignment:

- `&x = foo();`.
- `&mut x = foo();`.
- `ref x = foo();`.
- `x @ y = foo()`.
- (`box` patterns, which are deprecated.)

This is primarily for learnability: the behaviour of `&` can already be slightly confusing to
newcomers, as it has different meanings depending on whether it is used in an expression or pattern.
In destructuring assignment, the left-hand side of an assignment consists of sub*expressions*, but
which act intuitively like patterns, so it is not clear what `&` and friends should mean. We feel it
is more confusing than helpful to allow these cases. Similarly, although coming up with a sensible
meaning for `@`-bindings in destructuring assignment is not inconceivable, we believe they would be
confusing at best in this context. Conversely, destructuring tuples, slices or structs is very
natural and we do not foresee confusion with allowing these.

Our implementation is forwards-compatible with allowing these patterns in destructuring assignment,
in any case, so we lose nothing by not allowing them from the start.

Additionally, we do not give analogues for any of the following, which make little sense in this
context:

- Literal patterns.
- Range patterns.
- Or patterns.

Therefore, literals, bitwise OR, and range expressions (`..`, `..=`) are not permitted on the
left-hand side of a destructuring assignment.

## Compound destructuring assignment

We forbid destructuring compound assignment, i.e. destructuring for operators like `+=`, `*=` and so
on. This is both for the sake of simplicity and since there are relevant design questions that do
not have obvious answers, e.g. how this could interact with custom implementations of the operators.

## Order-of-assignment

The right-hand side of the assignment is always evaluated first. Then, assignments are performed
left-to-right. Note that component expressions in the left-hand side may be complex, and not simply
identifiers.

In a declaration, each identifier may be bound at most once. That is, the following is invalid:

```rust
let (a, a) = (1, 2);
```

For destructuring assignments, we currently permit assignments containing identical identifiers. However, these trigger an "unused assignment"
warning.

```rust
(a, a) = (1, 2); // warning: value assigned to `a` is never read
assert_eq!(a, 2);
```

We could try to explicitly forbid this. However, the chosen behaviour is justified in two ways:
- A destructuring
assignment can always be written as a series of assignments, so this behaviour matches its
expansion.
- In general, we are not able to tell when overlapping
assignments are made, so the error would be fallible. This is illustrated by the following example:

```rust
fn foo<'a>(x: &'a mut u32) -> &'a mut u32 {
    x
}

fn main() {
    let mut x: u32 = 10;
    // We cannot tell that the same variable is being assigned to
    // in this instance.
    (*foo(&mut x), *foo(&mut x)) = (5, 6);
    assert_eq!(x, 6);
}
```

We thus feel that a lint is more appropriate.

# Drawbacks
[drawbacks]: #drawbacks

- It could be argued that this feature increases the surface area of the language and thus
  complexity. However, we feel that by decreasing surprise, it actually makes the language less
  complex for users.
- It is possible that these changes could result in some confusing diagnostics. However, we have not
  found any during testing, and these could in any case be ironed out before stabilisation.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As we argue above, we believe this change increases the perceived consistency of Rust and improves
idiomatic code in the presence of mutability, and that the
implementation is simple and intuitive.

One potential alternative that has been put forth in the past is to allow arbitrary patterns on the
left-hand side of an assignment, but as discussed above and [extensively in this
thread](https://github.com/rust-lang/rfcs/issues/372), it is difficult to see how this could work in
practice (especially with complex left-hand sides that do not simply involve identifiers) and it is
not clear that this would have any advantages.

Another suggested alternative is to introduce a new keyword for indicating an assignment to an
existing expression during a `let` variable declaration. For example, something like the following:

```rust
let (a, reassign b) = expr;
```

This has the advantage that we can reuse the existing infrastructure for patterns. However, it has
the following disadvantages, which we believe make it less suitable than our proposal:

- It requires a new keyword or overloading an existing one, both of which have syntactic and
  semantic overhead.
- It is something that needs to be learnt by users: conversely, we maintain that it is natural to
  attempt destructuring assignment with the syntax we propose already, so does not need to be
  learnt.
- It changes the meaning of `let` (which has previously been associated only with binding new
  variables).
- To be consistent, we ought to allow `let reassign x = value;`, which introduces another way
  to simply write `x = value;`.
- It is longer and no more readable than the proposed syntax.

# Prior art
[prior-art]: #prior-art

The most persuasive prior art is Rust itself, which already permits destructuring declarations.
Intuitively, a declaration is an assignment that also introduces a new binding. Therefore, it seems
clear that assignments should act similarly to declarations where possible. However, it is also the
case that destructuring assignments are present in many languages that permit destructuring
declarations.

- JavaScript
[supports destructuring assignment](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Destructuring_assignment).
- Python [supports destructuring assignment](https://blog.tecladocode.com/destructuring-in-python/).
- Perl
[supports destructuring assignment](https://perl6advent.wordpress.com/2017/12/05/day-5-destructure-your-arguments-with-perl-6-signatures/).
- And so on...

It is a general pattern that languages support destructuring assignment when they support
destructuring declarations.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

- The implementation already supports destructuring of every class of expressions that currently
  make sense in Rust. This feature naturally should be extended to any new class of expressions for
  which it makes sense.
- It could make sense to permit [destructuring compound
  assignments](#Compound-destructuring-assignment) in the future, though we defer this question for
  later discussions.
- It could make sense to permit [`ref` and `&`](#Unsupported-patterns) in the future.
- It [has been suggested](https://github.com/rust-lang/rfcs/issues/372#issuecomment-365606878) that
  mixed declarations and assignments could be permitted, as in the following:

```rust
let a;
(a, let b) = (1, 2);
assert_eq!((a, b), (1, 2));
```

We do not pursue this here, but note that it would be compatible with our desugaring.
