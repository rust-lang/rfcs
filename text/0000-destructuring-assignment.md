- Feature Name: `destructuring_assignment`
- Start Date: 2020-04-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#71126](https://github.com/rust-lang/rust/issues/71126)

# Summary
[summary]: #summary

We allow destructuring on assignment, as in `let` declarations. For instance, the following are now
accepted:

```rust
(a, b) = (0, 1);
(x, y, .., z) = (1.0, 2.0, 3.0, 4.0, 5.0);
[_, f] = foo();
Struct { x: a, y: b } = bar();
```

This brings assignment in line with `let` declaration, in which destructuring is permitted. This
will simplify and improve idiomatic code involving mutability.

# Motivation
[motivation]: #motivation

Destructuring assignment increases the consistency of the language, in which assignment is typically
expected to behave similarly to variable declations. The aim is that this feature will increase the
clarity and concision of idiomatic Rust, primarily in code that makes use of mutability. This
feature is
[highly desired among Rust developers](https://github.com/rust-lang/rfcs/issues/372).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You may destructure a value when making an assignment, just as when you declare variables. See the
[Summary](#Summary) for examples.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The feature as described here has been implemented as a proof-of-concept. It follows essentially the
[suggestions of @Kimundi](https://github.com/rust-lang/rfcs/issues/372#issuecomment-214022963) and
[of @drunwald](https://github.com/rust-lang/rfcs/issues/372#issuecomment-262519146).

The Rust compiler already parses complex expressions on the left-hand side of an assignment, but
does not handle them other than emitting an error later in compilation. We propose to add
special-casing for several classes of expressions on the left-hand side of an assignment, which act
in accordance with destructuring assignment: i.e. as if the left-hand side were actually a pattern.
Actually supporting patterns directly on the left-hand side of an assignment significantly
complicates Rust's grammar and it is not clear that it is even technically feasible. Conversely,
handling some classes of expressions is much simpler, and is indistinguishable to users, who will
receive pattern-oriented diagnostics due to the desugaring of expressions into patterns.

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

We support the following classes of expressions:

- Tuples.
- Slices.
- (Tuple) structs.

In the desugaring, we convert the expression `(a, b)` into an analogous pattern `(_a, _b)` (whose
identifiers are fresh and thus do not conflict with existing variables). A nice side-effect is that
we inherit the diagnostics for normal pattern-matching, so users benefit from existing diagnostics for destructuring declarations.

## Diagnostics

It is worth being explicit that in with implementation, the diagnostics that are reported are
pattern diagnostics: that is, because the desugaring occurs regardless, the messages will imply that
the left-hand side of an assignment is a true pattern (the one the expression has been converted
to). We think that this results in a better user experience, as intuitively the left-hand side of a
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

The desugaring treats the `_` expression as an `_` pattern and the fully empty range `..` as a `..` pattern. No corresponding assignments are generated. For example:

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

## Compound destructuring assignment

We forbid destructuring compound assignment, i.e. destructuring for operators like `+=`, `*=` and so
on. This is both for the sake of simplicity and since there are relevant design questions that do not have obvious answers,
e.g. how this could interact with custom implementations of the operators.

## Order-of-assignment

In a declaration, each identifier may be bound at most once. That is, the following is invalid:

```rust
let (a, a) = (1, 2);
```

For destructuring assignments, we currently permit assignments containing identical identifiers, with assignments performed left-to-right. However, these trigger an "unused assignment"
warning. Assignments are performed left-to-right.

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

We find no convincing reason not to allow this. Though technically this increases the complexity of
the compiler, it does so minimally: the desugaring is noninvasive and simple. On the other hand, for
users, this change makes the language feel more consistent and decreases surprise, as evidenced by the discussion in [the open issue](https://github.com/rust-lang/rfcs/issues/372) for this feature.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As we argue above, we believe this change increases the perceived consistency of Rust and improves
idiomatic code in the presence of mutability, and that the
implementation is simple and intuitive.

One potential alternative that has been put forth in the past is to allow arbitrary patterns on the left-hand side of an assignment,
but as discussed above and [extensively in this
thread](https://github.com/rust-lang/rfcs/issues/372), it is difficult to see how this could work in
practice (especially with complex left-hand sides that do not simply involve identifiers) and it is not clear that this would have any advantages.

# Prior art
[prior-art]: #prior-art

The most persuasive prior art is Rust itself, which already permits destructuring
declarations. Intuitively, a declaration is an assignment that also introduces a new binding.
Therefore, it seems clear that assignments should act similarly to declarations where possible.
However, it is also the case that destructuring assignments are present in many languages that permit destructuring
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

- The implementation already supports destructuring of every class of expressions that currently make
sense in Rust. This feature naturally should be extended to any new class of expressions for which
it makes sense.
- It could make sense to permit
[destructuring compound assignments](#Compound-destructuring-assignment) in the future, though we
defer this question for later discussions.
- It [has been suggested](https://github.com/rust-lang/rfcs/issues/372#issuecomment-365606878) that
mixed declarations and assignments could be permitted, as in the following:

```rust
let a;
(a, let b) = (1, 2);
assert_eq!((a, b), (1, 2));
```

We do not pursue this here, but note that it would be compatible with our desugaring.
