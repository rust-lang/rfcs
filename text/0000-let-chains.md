- Feature Name: let_chains
- Start Date: 2017-12-23
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC extends `if let`- and `while let`-expressions with chaining, allowing
you to combining multiple `let`s and conditions together naturally. With this
RFC implemented, you will, among other things, now be able to write:

```rust
let name = meta.name();
if set.is_some() {
    error::set_again(name);
} else if let &NameValue(_, Lit::Int(val, ty)) = meta
        , val <= u32::MAX as u64
        , is_unsigned(ty) {
    *set = Some(val as u32); 
} else {
    error::weight_malformed(name);
}
```

and with side effects:

```rust
if let Ok(user) = read_user(::std::io::stdin())
 , user.name == "Alan Turing"
 , let Ok("Hacking Enigma") = read_hobby_of(&user) {
    println!("Yep, It's you.");
} else {
    panic!("You are not Alan...!");
}
```

# Motivation
[motivation]: #motivation

The main motivation for this RFC is improvements in ergonomics and reducing
paper cuts. As a by product of the changes proposed here, the stress put on
LLVM can be reduced in some circumstances.

## Right-ward drift

Each `if-let` needs a brace, which means that you usually, to keep the code
readable, indent once to the right. So matching multiple things quickly leads
to way too much indent that overflows the text editor or IDE horizontally.
This is in particular bad for readers that can only fit around 80 characters
per line in their editor.

The usual solution is matching a tuple, but that goes poorly when there are
side effects or expensive computations involved, and doesn't necessarily work
as *DSTs* and *lvalues* can't go in tuples.

Another solution to avoid right-ward drift is to create a new function for
part of the indentation. When the inner scopes depend on a lot of variables
and state from outer scopes, all of these variables have to be passed on to
the newly created function, which may not even be a natural unit to abstract
into a function. Creating a new function, especially one that feels artificial,
can also inhibit local reasoning. A new level of function (or [IIFE]) also
changes the behavior of `return`, `break`, `?`, and friends.

[IIFE]: https://en.wikipedia.org/wiki/Immediately-invoked_function_expression

## Mixing conditions and pattern matching

A `match` expression can have `if` guards, but `if let` currently requires
another level of conditionals.  This is particularly troublesome for cases
that can't be matched, like `x.fract() == 0`, or error enums that disallow
matching, like `std::io::ErrorKind`.

## Duplicating code in `else` clauses

In some cases, you may have written something like:

```rust
if let A(x) = foo() {
    if let B(y) = bar(x) {
        do_stuff_with(x, y)
    } else {
        some_long_expression
    }
} else {
    some_long_expression
}
```

In this example `foo()` and `bar(x)` has side effects, but more crucially,
there is a dependency between matching on the result of `foo()` to execute
`bar(x)`. Therefore, matching on `(foo(), bar(x))` is not possible in this
case. So you have no choice but to write it in this way.

However, now `some_long_expression` is repeated, and if more let bindings
are added, more repetition ensues. To avoid repeating the long expression,
you might encapsulate this in a new function, but that new function may feel
like an artificial abstraction as discussed above.

This is problematic even with a macro to simplify, as it results in more code
emitted that LLVM commonly cannot simplify.

## Bringing the language closer to the mental model

The readability of programs is often about the degree to which the code
corresponds to the mental model the reader has of said program. Therefore,
we should aim to bring the language closer to the mental model of the reader.
With respect to `if let`-expressions, rather than saying (out loud):

> if A matches, and
> > if x holds and
> > > if B matches
> > > > do X, Y, and Z

..it is more common to say:

> If A matches, x holds, and B matches, do X, Y, and Z

This RFC is more in line with the latter formulation and thus brings the
language closer to the readers mental model.

## Instead of macros

A macro like [`if_chain!`] as a solution also has the problem of not being
part of the language specification. Thus, it is not part of the common syntax
that experienced Rust programmers are familiar with and is instead local to
the project itself. The non-universality of syntax therefore hurts readability.

[`if_chain!`]: https://crates.io/crates/if_chain

## Real-world use cases

[reverse dependencies]: https://crates.io/crates/if_chain/reverse_dependencies

[defines a function]: https://github.com/rust-lang-nursery/rust-clippy/blob/ed589761e62735ebb803510e01bfd8b278527fb9/clippy_lints/src/print.rs#L207-L219

By taking a look at the [reverse dependencies] of [`if_chain!`] we find many
real-world use cases that this RFC facilitates.

As an example, `clippy` [defines a function]:
```rust
/// Returns the slice of format string parts in an `Arguments::new_v1` call.
fn get_argument_fmtstr_parts(expr: &Expr) -> Option<(InternedString, usize)> {
    if_chain! {
        if let ExprAddrOf(_, ref expr) = expr.node; // &["…", "…", …]
        if let ExprArray(ref exprs) = expr.node;
        if let Some(expr) = exprs.last();
        if let ExprLit(ref lit) = expr.node;
        if let LitKind::Str(ref lit, _) = lit.node;
        then {
            return Some((lit.as_str(), exprs.len()));
        }
    }
    None
}
```

with this RFC, this would be written, without any external dependencies, as:

```rust
/// Returns the slice of format string parts in an `Arguments::new_v1` call.
fn get_argument_fmtstr_parts(expr: &Expr) -> Option<(InternedString, usize)> {
    if let ExprAddrOf(_, ref expr) = expr.node // &["…", "…", …]
     , let ExprArray(ref exprs) = expr.node
     , let Some(expr) = exprs.last()
     , let ExprLit(ref lit) = expr.node
     , let LitKind::Str(ref lit, _) = lit.node {
        Some((lit.as_str(), exprs.len()))
    } else {
        None
    }
}
```

In the above example, some right-ward drift and noise has been reduced.

This kind of deep pattern matching is common for parsers and when dealing
with ASTs. One place which deals with ASTs is the compiler itself. Thus,
with this RFC, some compiler internals may be simplified.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section examines the features proposed by this RFC.

## if-let-chains

An *if-let-chain* refers to a chain of multiple let bindings,
which may mixed with conditionals in an if-expression.

One example of such a chain is:

```rust
if let A(x) = foo(),
   let B(y) = bar() {
    computation_with(x, y)
}
```

It is important to note here that this is not equivalent to the
following expression:

```rust
if let (A(x), B(y)) = (foo(), bar()) {
    computation_with(x, y)
}
```

Unlike the first example, there is no short circuiting logic in the in this
example with tuples. Assuming that there are no panics, both functions are
always executed in the latter example.

If we desugar the first example, we can clearly see the difference:

```rust
if let A(x) = foo() {
    if let B(y) = bar() {
        computation_with(x, y)
    }
}
```

What is the practical difference, and why is short circuiting behavior an
important distinction? The call to `bar()` may be an expensive one.
Avoiding useless work is beneficial to performance. There is however a more
fundamental reason. Assuming that `bar()` has side effects, the meaning of
the tuple example is different from the nested if-let expressions because in
the case of the former, the side effect of `bar()` always happens while it
will not if `let A(x) = foo()` does not match.

The difference between the tuple-example and if-let-chains becomes even
greater if we also consider a dependence between `foo()` and `bar(..)`
as in the following example:

```rust
if let A(x) = foo() {
    if let B(y) = bar(x) {
        computation_with(x, y)
    }
}
```

Calling `bar(x)` is now dependent on having an `x` that is only available
to us by first pattern matching on `foo()`. Therefore, there is no tuple-based
equivalent to the above example. With this RFC implemented, you can more
ergonomically write the same expression as:

```rust
if let A(x) = foo(),
   let B(y) = bar(x) {
    computation_with(x, y)
}
```

The new expression form introduced by this RFC is also not limited to simple
`if-let` expressions, you may of course also add `else` branches as seen in
the example below.

```rust
if let A(x) = foo(),
   let B(y) = bar() {
    computation_with(x, y)
} else {
    alternative_computation()
}
```

While the below snippet is not what the compiler would desugar the above one
to, you can think of the former as semantically equivalent to it. The compiler
is free to not actually emit two calls to `alternative_computation()` in your
compiled binary. For details, please see the [reference-level-explanation].

```rust
if let A(x) = foo() {
    if let B(y) = bar(x) {
        computation_with(x, y)
    } else {
        alternative_computation()
    }
} else {
    alternative_computation()
}
```

As briefly explained above if-let-chain expression form is also not limited to
pattern matching. You can also mix in any number of conditionals in any place
you like, as done in the example below:

```rust
if independent_condition,
   let A(x) = foo(),
   let B(y) = bar(),
   y.has_really_cool_property() {
    computation_with(x, y)
}
```

The above example example can be thought of as equivalent to:

```rust
if independent_condition {
   if let A(x) = foo() {
       if let B(y) = bar() {
           if y.has_really_cool_property() {
                computation_with(x, y)
            }
        }
    }
}
```

## while-let-chains

A **while-let-chain** is similar to an if-let-chain but instead applies to
while-let expressions.

Since we've already introduced the basic idea previously with *if-let-chains*,
we will jump straight into a more complex example.

The popular [`itertools`] crate has an `izip` macro that allows you to
*"Create an iterator running multiple iterators in lockstep"*. An example
of this, taken from the documentation of `izip` is:

[`itertools`]: https://docs.rs/itertools/0.7.4/itertools/macro.izip.html

```rust
#[macro_use] extern crate itertools;

// iterate over three sequences side-by-side
let mut results = [0, 0, 0, 0];
let inputs = [3, 7, 9, 6];

for (r, index, input) in izip!(&mut results, 0..10, &inputs) {
    *r = index * 10 + input;
}

assert_eq!(results, [0 + 3, 10 + 7, 29, 36]);
```

With this RFC, we can write this, admittedly not as succinctly, as:

```rust
let mut results = [0, 0, 0, 0];
let inputs = [3, 7, 9, 6];

let r_iter = results.iter_mut();
let c_iter = 0..10;
let i_iter = inputs.iter();

while let Some(r) = r_iter.next(),
      let Some(index) = c_iter.next(),
      let Some(input) = i_iter.next(),
{
    *r = index * 10 + input;
}

assert_eq!(results, [0 + 3, 10 + 7, 29, 36]);
```

The loop in the above snippet is equivalent to:

```rust
loop {
    if let Some(r) = r_iter.next() {
       let Some(index) = c_iter.next() {
       let Some(input) = i_iter.next() {
        *r = index * 10 + input;
        continue;
    }
    break;
}
```

Notice in particular here that just as we could rewrite `while-let` in
terms of `loop` + `if-let`, so too can we rewrite `while-let-chains` with
`loop` + `if-let-chains`.

While these two first snippets are equivalent in this example, this does not
generally hold. If `i_iter.next()` has side effects, then those will not
happen when `Some(index)` does not match. This is important to keep in mind.
Short-circuiting still applies to `while-let-chains` as with `if-let-chains`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## if-let-chains

Using the same internal syntax that exists for `catch`, this code:

```rust
if let A(x) = foo(),
   let B(y) = bar() {
    computation_with(x, y)
} else {
    alternative_computation()
}
```

desugars into:

```rust
'a: {
    if let A(x) = foo() {
        if let B(y) = bar() {
            break 'a computation_with(x, y);
        }
    }
    alternative_computation()
}

```

This avoids any code duplication and requires no new semantics.  The rules for
borrowing and scoping are just those that result directly from the desugar.

An `else if` branches can be added as in:

```rust
if let A(x) = foo(),
   let B(y) = bar() {
    computation_with(x, y)
} else if let C(z) = baz(), condition(x) {
    other_with(z)
} else {
    alternative_computation()
}
```

which is desugared as:

```rust
'a: {
    if let A(x) = foo() {
        if let B(y) = bar() {
            break 'a computation_with(x, y);
        }
    }
    if let C(z) = baz() {
        if condition(x) {
            break 'a other_with(z)
        }
    }
    alternative_computation()
}
```

Having an `else` branch is optional.
The following example without an `else` branch:

```rust
if let A(x) = foo(), let B(y) = bar() {
    computation_with(x, y)
}
```

is simply desugared into:

```rust
if let A(x) = foo() {
    if let B(y) = bar() {
        computation_with(x, y);
    }
}
```

### Additions to the grammar

The grammar in [§ 7.2.24] is changed from:

[§ 7.2.24]: https://doc.rust-lang.org/grammar.html#if-let-expressions

```ebnf
if_let_expr : "if" "let" pat '=' expr '{' block '}'
               else_tail ? ;

```

to:

```ebnf
let_pattern: "let" pat '=' expr
           | expr
           ;

let_pattern_list: let_pattern [ ',' let_pattern ] * ','? ;

if_let_expr : "if" let_pattern_list '{' block '}'
               else_tail ? ;
```

## while-let-chains

Similar to normal `while let`-expressions and `if-let-chain`-expressions,
there are `while-let-chains`. One example of such a chain is:

```rust
while let Some(x) = first_iter.next()
    , let Ok(y) = foo(x, second_iter.next())
    , a_really_important_precondition(x, y) {
    computation_with(x, y);
}
```

This expression desugars into:

```rust
loop {
    if let Some(x) = first_iter.next(),
       let Ok(y) = foo(x, second_iter.next()),
       a_really_important_precondition(x, y) {
        computation_with(x, y);
        continue;
    }
    break;
}
```

This desugaring relies on desugaring for `if-let-chains`.

### Additions to the grammar

The grammar in [§ 7.2.25] is changed from:

[§ 7.2.25]: https://doc.rust-lang.org/grammar.html#while-let-loops

```ebnf
while_let_expr : [ lifetime ':' ] ? "while" "let" pat '=' expr '{' block '}' ;
```

to:

```ebnf
let_pattern: "let" pat '=' expr
           | expr
           ;

let_pattern_list: let_pattern [ ',' let_pattern ] * ','? ;

while_let_expr : [ lifetime ':' ] ? "while" let_pattern_list '{' block '}' ;
```

Note that trailing commas are allowed for both if-let-chains
and while-let-chains.

# How do we teach this?

The proper place to discuss these new features in the documentation and book
is to first discuss `if-let` and then `if-let-chains` as if it were the same
concept but just "more". The same applies to `while-let` and `while-let-chains`.

# Drawbacks
[drawbacks]: #drawbacks

This RFC mandates additions to the grammar as well as adding syntax
lowering passes. These are not large additions, but nonetheless the
language specification is made more complex by it.

While this complexity will be used by some and therefore, the RFC argues,
motivates the added complexity, it will not be used all users of the language.

When it comes to `if-let-chains`, the feature is already supported by the
macro `if_chain!`. Some may feel that this is enough.

# Rationale and alternatives
[alternatives]: #alternatives

We will now discuss how and why this RFC came about in its current form
and why it matters.

## The impact of not doing this

There are at least two sides to power in language expressivity:
1. The ability to express something in a language.
2. The ability to express something effortlessly.

Nothing proposed in this RFC adds to point 1. While this is the case,
it is not sufficient. The second point is important to make the language
pleasurable to use and this is what this RFC is about. Not including the
changes proposed here would keep some paper cuts around.

## Design considerations and constraints

There are a some constraints and/or design considerations on this feature,
namely:

+ That the variables bound in the pattern have a clear and consistent
  scope so that a let-chain is not confused with an expression that can
  be used in other places.
+ That the syntax mixes well with normal boolean conditionals.
+ That the additions be simple conceptually and build on what language  
  users already know.
+ That instead of a heap of special cases, the grammar should be simple.

With these considerations in mind, the RFC was developed.
There are however some alternatives to consider.

## Keeping the door open for `if-let-or`-expressions

Should a user be able to write something like the following snippet?

```rust
if let A(x) = e1 | let A(x) = e2 {
    do_stuff_with(x)
} else {
    do_other_stuff()
}
```

What does this expression even mean? It means that if one of the patterns
match, then the first one of those will bind a value to `x` and the expression
evaluates to `do_stuff_with(x)`. If no patterns match, the expression instead
evaluates to `do_other_stuff()`.

This RFC does not propose such a facility, but crucially does not shut the
door on it, making the feature future proof and allowing discussion on such
a facility in the future to continue.

## Alternative feature: [RFC 2046], label break value

[RFC 2046]: https://github.com/rust-lang/rfcs/pull/2046
[the macros to be improved]: https://github.com/rust-lang/rfcs/pull/2046#issuecomment-320483246
[CFG]: https://en.wikipedia.org/wiki/Control_flow_graph

[RFC 2046] is a more general *control flow graph* ([CFG]) control feature.
While it doesn't solve the rightward drift or ergonomic issues that this
RFC does, it allows [the macros to be improved] by removing duplication of
`else` blocks.  The closest syntax today for that is `loop-break`, but that
doesn't work as `continue` is intentionally non-hygenic.

RFC 2046 is also a bit orthogonal in the sense that it's fully compatible
with this RFC. The general label break is useful and powerful, as seen in
the [reference-level-explanation] of this RFC and of `catch`'s, but is
verbose and unfamiliar. Having a substantially more ergonomic feature for
this particularly common case is valuable regardless.

## Alternative: `if e1 && let p2 = e2 && .. {..} else {..}`

Using `&&` in the syntax leads the the obvious "well, what about `||`?"
followup, which is unnecessary given the existence of `|` already in patterns.

It also causes confusion with `&&` being part of the expression.
While that's technically solvable with backtracking, that's generally
undesirable and still a speed bump for the reader.

The syntax gives the impression that `let` is now an expression, which it
isn't. This impression is harmful, since a reader may now see this and start
using `e1 && let p2 = e2` in places not preceded by `if` or `while`,
only to get a syntax error as a result.

## Alternative: `if let p1 = e1, p2 = e2, .. {..} else {..}`

`if` constraints in `match` have demonstrated that mixing conditionals with
destructuring is valuable, which would be unavailable in an ergonomic fashion
with this syntax. It's also nice to consistently have `let` in front of the
pattern, to emphasize what's coming, provide easy syntax highlighting,
and improved diagnostics.

It is still possible to mix in conditionals with destructuring by
realizing that pattern matching on a `bool` typed expression is possible
with `let true = expr`. However, such pattern matching is somewhat awkward.

# Unresolved questions
[unresolved]: #unresolved-questions

## 1. Could this be part of general patterns?

Is there a more generalized version of this that can integrate into existing
patterns without adding custom syntax to `if let` and `while let`?

Unfortunately, having a `let`-like feature inside a pattern introduces
expressions into the middle, whereas today patterns cannot cause side effects.
So this may be better left outside of pattern syntax, like how `if` guards
in `match` are part of `match`'s grammar, not part of pattern syntax.

## 2. Irrefutable let bindings after the first refutable binding

Should temporary and irrefutable `let`s without patterns be allowed as
in the following example?

```rust
if let &List(_, ref list) = meta
 , let mut iter = list.iter().filter_map(extract_word) // <-- Irrefutable
 , let (Some(ident), None) = (iter.next(), iter.next()) {
    *set = Some(syn::Ty::Path(None, ident.clone().into()));
} else {
    error::param_malformed();
}
```
  
With normal `if-let` expressions, this is an error as seen with the 
following example:

```rust
fn main() {
    if let x = 1 { 2 } else { 3 };
}
```

Compiling the above ill-formed program results in:

```
error[E0162]: irrefutable if-let pattern
```

[RFC 2086]: https://github.com/rust-lang/rfcs/pull/2086

However, with the implementation of [RFC 2086], this error will instead
become a warning.  This is understandable - while the program could have
perfectly well  defined semantics, where the value of the expression is
always 2,  allowing the form would invite some developers to write in a
non-obvious way. A warning is however a good middle ground.

However, when the non-first let binding is irrefutable, there is some value in
not warning against the construct. In the case of the initial example in this
subsection, it would be written as follows without irrefutable let bindings:

```rust
if let &List(_, ref list) = meta {
   let mut iter = list.iter().filter_map(extract_word);
    if let (Some(ident), None) = (iter.next(), iter.next()) {
        *set = Some(syn::Ty::Path(None, ident.clone().into()));
    } else {
        error::param_malformed();
    }
} else {
    error::param_malformed();
}
```

However, now we have introduced rightward drift again, which we wanted to avoid.

On the other hand, allowing irrefutable patterns without a warning after
one refutable pattern may give the impression that the irrefutable pattern
is refutable, or cast doubt on it making semantics harder to grasp quickly.

## 3. How should the proposed syntax be formatted?

How should this be formatted? This is not a make or break question
but rather a style question for `rustfmt` that we can kick off now.
Unfortunately, the usual block indent feels like it's off-by-one here,
with the `let`s not lining up nicely.
  
Here are a few variants on indentation to consider for `rustfmt`:

### 3.1. Commas after bindings

```rust
if independent_condition,
   let Alan(x) = turing(),
   let Alonzo(y) = church(x),
   y.has_really_cool_property() {
    computation_with(x, y)
}
```

It is clear that `computation_with(x, y)` is visually distinct
from the generalized conditions. One could say that it is both a
benefit and downside of the one-off indent.

### 3.2. Commas at the start of a line

```rust
if independent_condition
 , let Alan(x) = turing()
 , let Alonzo(y) = church(x)
 , y.has_really_cool_property() {
    computation_with(x, y)
}
```

The call `computation_with(x, y)` is still visually distinct.
If you prefer the formatting:

```
[ foo
, bar
, baz
]
```

You will likely also like the formatting in 3.2.

### 3.3. Aligning the equals sign together

```rust
if independent_condition,
   let Alan(x)   = turing(),
   let Alonzo(y) = church(x),
   y.has_really_cool_property() {
    computation_with(x, y)
}
```

[rustfmt guidelines]: https://github.com/rust-lang-nursery/fmt-rfcs/blob/master/guide/principles.md#overarching-guidelines

While this might look visually pleasing, visual indent like this is
against the [rustfmt guidelines].

### 3.4. Newline after `else if`

```rust
if independent_condition,
   let Alan(x) = turing(),
   let Alonzo(y) = church(x),
   y.has_really_cool_property() {
    computation_with(x, y)
} else if // <-- Notice newline.
   let Haskell(x) = curry(),
   let Edsger(y) = dijkstra(x) {
    computation_with(x, y)
}
```

In this version we look at whether or not a newline should be
inserted after an `else if` branch. The benefit of inserting a
newline is that it aligns well with the `let` bindings in the
`if` branch.

### 3.5. Open-brace after newline

```rust
if independent_condition,
   let Alan(x) = turing(),
   let Alonzo(y) = church(x),
   y.has_really_cool_property(),
{
    computation_with(x, y)
}
```

Moving the open brace down a line may help emphasize the split between
a lengthy condition and the block body.

There are of course more versions one can contemplate and the various
combination of them, but in the interest of brevity, we keep to this list here.

There are no more unresolved questions.
The exact syntax-lowering-transformations can be deferred to stabilization.