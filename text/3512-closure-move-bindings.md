- Feature `closure_move_bindings`
- Start Date: 2023-10-09
- RFC PR: [rust-lang/rfcs#3512](https://github.com/rust-lang/rfcs/pull/3512)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Adds the syntax `move(bindings) |...| ...`
to explicitly specify how to capture bindings into a closure.

# Motivation
[motivation]: #motivation

Currently there are two ways to capture local bindings into a closure,
namely by reference (`|| foo`) and by moving (`move || foo`).
This mechanism has several ergonomic problems:

- It is not possible to move some bindings and reference the others.
To do so, one must define another binding that borrows the value
and move it into the closure:

```rs
{
    let foo = &foo;
    move || run(foo, bar)
}
```

- It is a very frequent scenario to clone a value into a closure
(especially common with `Rc`/`Arc`-based values),
but even the simplest scenario requires three lines of boilerplate:

```rs
{
    let foo = foo.clone();
    move || foo.run()
}
```

This RFC proposes a more concise syntax to express these moving semantics.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

A closure may capture bindings in its defining scope.
By default, bindings are captured by usage,
i.e. by the first possible of shared reference, mutable reference or move.

```rs
let mut foo = 1;
let mut closure = || { foo = 2; };
closure();
dbg!(foo); // foo is now 2
```

You can add a `move` keyword in front of the closure
to indicate that all captured bindings are always moved into the closure,
useful for avoiding references to local variables:

```rs
let mut foo = 1;
let mut closure = move || { foo = 2; };
closure();
dbg!(foo); // foo is still 1, but the copy of `foo` in `closure` is 2
```

Note that `foo` is _copied_ during move in this example
as `i32` implements `Copy`.

If a closure captures multiple bindings,
the `move` keyword makes them all captured by moving.
To only indicate this for specific bindings,
list them in parentheses after `move`:

```rs
let foo = 1;
let mut bar = 2;
let mut closure = move(mut foo) || {
    foo += 10;
    bar += 10;
};
closure();
dbg!(foo, bar); // foo = 1, bar = 12
```

Note that the outer `foo` no longer requires `mut`;
it is relocated to the closure since it defines a new binding.
Meanwhile, `bar` continues to capture by usage (i.e. by reference).

Moved bindings may also be renamed:

```rs
let mut foo = 1;
let mut closure = move(mut bar = foo) || {
    foo = 2;
    bar = 3;
};
closure();
dbg!(foo); // the outer `foo` is 2 as it was captured by reference
```

Bindings may be transformed when moved:

```rs
let foo = vec![1];
let mut closure = move(mut foo = foo.clone()) || {
    foo.push(2);
};
closure();
dbg!(foo); // the outer `foo` is still [1] because only the cloned copy was mutated
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A closure expression has the following syntax:

> **<sup>Syntax</sup>**\
> _ClosureExpression_ :\
> &nbsp;&nbsp; ( `move` _MoveBindings_<sup>?</sup> )<sup>?</sup>\
> &nbsp;&nbsp; ( `||` | `|` _ClosureParameters_<sup>?</sup> `|`  )\
> &nbsp;&nbsp; (_Expression_ | `->` _TypeNoBounds_&nbsp;_BlockExpression_)>\
> _MoveBindings_ :\
> &nbsp;&nbsp; `(` ( _MoveBinding_ (`,` _MoveBinding_)<sup>\*</sup> `,`<sup>?</sup> )<sup>?</sup> `)`\
> _MoveBinding_ :\
> &nbsp;&nbsp; _NamedMoveBinding_ | _UnnamedMoveBinding_\
> _NamedMoveBinding_ :\
> &nbsp;&nbsp; _PatternNoTopAlt_ `=` _Expression_\
> _UnnamedMoveBinding_ :\
> &nbsp;&nbsp; `mut`<sup>?</sup> _IdentifierExpression_ \
> _ClosureParameters_ :\
> &nbsp;&nbsp; _ClosureParam_ (`,` _ClosureParam_)<sup>\*</sup> `,`<sup>?</sup>\
> _ClosureParam_ :\
> &nbsp;&nbsp; _OuterAttribute_<sup>\*</sup> _PatternNoTopAlt_&nbsp;( `:` _Type_  )<sup>?</sup>

Closure expressions are classified into two main types,
namely _ByUsage_ and _FullMove_.
A closure expression is _FullMove_ IF AND ONLY IF
it starts with a `move` token immediately followed by a `|` token,
without any parentheses in between.

## _ByUsage_ closures

When the parentheses for _MoveBindings_ is present,
or when the `move` keyword is absent,
the closure expression is of the _ByUsage_ type, where
all local variables in the closure construction scope not shadowed by any _MoveBinding_
are implicitly captured into the closure
by shared reference, mutable reference or move on demand,
preferring the first possible type.

Each _MoveBinding_ declares binding(s) in its left-side pattern,
assigned with the value of the right-side expression evaluated during closure construction,
thus referencing any relevant local variables if necessary.

If the left-side pattern is omitted (_UnnamedMoveBinding_),
the expression must be a single-segment (identifier) `PathExpression`.
The left-side pattern is then automatically inferred to be a _IdentifierPattern_
using the identifier as the new binding.

### Mutable bindings

If a captured binding mutated inside the closure is declared in a _NamedMoveBinding_,
the `IdentifierPattern` that declares the binding must have the `mut` keyword.

If it is declared in an _UnnamedMoveBinding_,
the `mut` keyword must be added in front of the expression;
since the declared binding is always the first token in the expression,
the `mut` token is always immediately followed by the mutable binding,
thus yielding consistent readability.

If it is implicitly captured from the parent scope
instead of declared in a _MoveBinding_,
the local variable declaration must be declared `mut` too.

## _FullMove_ closures

When the `move` keyword is present but _MoveBindings_ is absent (with its parentheses absent as well),
the closure expression is of the _FullMove_ type, where
all local variables in the closure construction scope
are implicitly moved or copied into the closure on demand.

Note that `move` with an empty pair of parentheses is allowed and follows the former rule;
in other words, `move() |...| {...}` and `|...| {...}` are semantically equivalent.
This allows macros to emit repeating groups of `_MoveBinding_ ","` inside a pair of parentheses
and achieve correct semantics when there are zero repeating groups.

If a moved binding is mutated inside the closure,
its declaration in the parent scope must be declared `mut` too.

# Drawbacks
[drawbacks]: #drawbacks

Due to backwards compatibility, this RFC proposes a new syntax
that is an extension of capture-by-move
but actually looks more similar to capture-by-reference,
thus confusing new users.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Capture-by-reference is the default behavior for implicit captures for two reasons:

1. It is more consistent to have `move(x)` imply `move(x=x)`,
which leaves us with implicit references for the unspecified.
2. Move bindings actually define a new shadowing binding
that is completely independent of the original binding,
so it is more correct to have the new binding explicitly named.
Consider how unintuitive it is to require that
a moved variable be declared `mut` in the outer scope
even though it is only mutated inside the closure (as the new binding).

The possible syntax for automatically-inferred _MoveBinding_ pattern
is strictly limited to allow maximum future compatibility.
Currently, many cases of captured bindings are in the form of
`foo = foo`, `foo = &foo` or `foo.clone()`.
This RFC intends to solve the ergonomic issues for these common scenarios first
and leave more room for future enhancement when other frequent patterns are identified.

Alternative approaches previously proposed
include explicitly adding support for the `clone` keyword.
This RFC does not favor such suggestions
as they make the fundamental closure expression syntax
unnecessarily dependent on the `clone` language item,
and does not offer possibilities for alternative transformers.

## `move` inside parameter list

As an alternative to having `move(binding)` ahead of a closure, we could put `move binding` in the parameter list: `|x: T, move y, move z = z.clone()| { ... }`.

This would have the advantage of keeping the list of declared names in one place, and giving a view of closures as having some bindings passed in as arguments and other bindings captured from the containing scope.

However, this would have the disadvantage of visually looking like the caller could pass the value in as a parameter, which it cannot.
# Prior art
[prior-art]: #prior-art

## Other languages

Closure expressions (with the ability to capture) are known to many languages,
varying between explicit and implicit capturing.
Nevertheless, most such languages do not support capturing by reference.
Examples of languages that support capture-by-reference include
C++ lambdas (`[x=f(y)]`) and PHP (`use(&$x)`).
Of these, C++ uses a leading `&`/`=` in the capture list
to indicate the default behavior as move or reference,
and allows an initializer behind a variable:

```cpp
int foo = 1;
auto closure = [foo = foo+1]() mutable {
    foo += 10; // does not mutate ::foo
    return foo;
}
closure(); // 12
closure(); // 22
```

This RFC additionally proposes the ability to omit the capture identifier,
because use cases of `foo.clone()` are much more common in Rust,
compared to C++ where most values may be implicitly cloned.

## Rust libraries

Attempts to improve ergonomics for cloning into closures were seen in proc macros:

- [enclose](https://crates.io/crates/enclose)
- [clown](https://crates.io/crates/clown)
- [closet](https://crates.io/crates/closet)
- [capture](https://crates.io/crates/capture)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- This RFC actually solves two not necessarily related problems together,
namely clone-into-closures and selective capture-by-move.
It might be more appropriate to split the former to a separate RFC,
but they are currently put together such that
consideration for the new syntax includes possibility for both enhancements.

# Future possibilities
[future-possibilities]: #future-possibilities

- Should we consider deprecating the _FullMove_ syntax
in favor of explicitly specifying what gets moved,
especially for mutable variables,
considering that moved variables actually create a new, shadowing binding?
- The set of allowed expressions may be extended in the future.
