- Feature Name: looping_expr_forms
- Start Date: 2015-03-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Restrict grammar of Rust language for 1.0 so that all looping
syntactic forms (`for`, `loop`, `while`, and `while let`) are
statements (instead of expressions). Forms like `let d = loop { };` and
`(while foo() { })` all produce errors at parse time.

The errors also provide a note saying that one can write `{
<looping-stmt> }` in the expression's original context to get the
program building again.

The feature name above (`looping_expr_forms`) is used to restore
support for parsing them as expressions, with the understanding that
the semantics of looping forms in expression context is unstable.


# Motivation

[RFC PR 352] ("Allow loops to return values other than ()") proposes
extending the syntax of the looping forms so that they can be used to
return non-unit values.

 * (In a nutshell, the syntax changes of [RFC PR 352] use `break
   <expr>` to indicate the value to return. Since generally
   `for`, `while`, and `while let` loops do not terminate via `break`,
   it also adds the suffix form `<for-or-while-header> { <block> }
   else { <block> }` to such loops that return non-unit values.
   However, these details are *not* the subject of this RFC.)

The changes that [RFC PR 352] proposes are backwards compatible.
However, a number of commenters did not see the proposed extension as
a natural fit for the Rust language, and some commenters suggested
alternative approaches that might work better.

[RFC PR 352]: https://github.com/rust-lang/rfcs/pull/352

This document is *not* assigning any judgement about whether non-unit
looping expressions should or should not be adopted. It is merely
proposing that we can increase the range of variants of non-unit
looping expressions that can be added backwards-compatibly if we start
by first classifying all of today's looping forms as statements
rather than expressions.

In other words, this document is proposing that by imposing a (*very
small*) restriction today, we avoid painting ourselves into a corner
with respect to how we handle [RFC PR 352] tomorrow.

# Detailed design

Change the parser so that the existing looping forms (`for`, `loop`,
`while`, `while let`) can be parsed in statement context, and remove
the ability to parse those forms in expression context (unless the
feature `looping_expr_forms` is turned on).

In a context that can accept either a statement or an expression
(e.g. the final form in a block expression), disambiguate by always
choosing the statement form of a loop rather than the expression form.
(For discussion of this point, see [Does this change accomplish nothing].)

## The grammar

In the current grammar, I believe the effect of the above paragraph
can be roughly summarized as:

In the grammatical production for `<stmt>`, add the cases
```
<stmt> ::= ...
        |  <stmt_while>
        |  <stmt_while_let>
        |  <stmt_loop>
        |  <stmt_for>

<stmt_while>     ::= <expr_while>
<stmt_while_let> ::= <expr_while_let>
<stmt_loop>      ::= <expr_loop>
<stmt_for>       ::= <expr_for>
```

In the grammatical production for `<block_expr>`,
```
<block_expr> ::= ...
              |  <expr_while>
              |  <expr_while_let>
              |  <expr_loop>
              |  <expr_for>
```
conditionalize the latter four productions so that they are
only present when the feature `looping_expr_forms` is turned on.
(I am not sufficiently familiar with bison grammar specifications to
know how to encode the statement/expression context disambiguation
rule.)

(The main reason I want to continue supporting the expression forms is
to encourage the compiler developers to continue thinking of the
looping forms as expressions, even if the stable language only exposes
them as statements. If we remove support for parsing them as
expressions, then it will be harder to readd it later.)

## Examples

Most important: Much Rust code will continue to compile without
needing any modification under the change proposed here. In particular,
in the prototype developed by pnkfelix, only 17 files needed to be
changed after the parser itself had been updated. *Seventeen* -- that's
*including* the test suite.

You can see the [prototype fallout], as well as the [prototype parser changes] pnkfelix
used to emulate this restriction to the language.

[prototype fallout]: https://github.com/pnkfelix/rust/compare/c1e2151e2b5fd88c4d335a1671c373845b95a675...32c5726452ff875694c4aed81b87206faadcb640

[prototype parser changes]: https://github.com/pnkfelix/rust/commit/c1e2151e2b5fd88c4d335a1671c373845b95a675

Here are some examples of expressions that will start to
fail to compile:

```rust
// cannot use stmt as RHS of a let
let a = while foo() { };

// cannot use stmt directly as body of arm
match b { 1 => (), _ => loop { }, }

// cannot use stmt as content of paren expr
let c = { hi(); (for i in 0.. { }); bye() };

// cannot use stmt as argument to function call
hello(while let Some(x) = i.next() { });
```

However, *any* code that does start to fail to compile can be fixed by
wrapping the former expression within a trivial block expression:

```rust
let a = { while foo() { } };

match b { 1 => (), _ => { loop { } } }

// (this is silly, of course;
//  but wasn't it silly already? -- well, see next section.)
let c = { hi(); {(for i in 0.. { })}; bye() };

hello({while let Some(x) = i.next() { }});
```

The reason that much code continues to compile without requiring any
modification is that all of the looping forms currently have type
`()`, and thus the change to parsing them as statements in the context
of a block has no effect in the end.


## Does this change accomplish nothing?
[Does this change accomplish nothing]: #does-this-change-accomplish-nothing

The above text introduces the block expression workaround
(which I will call just the "block workaround") of replacing
e.g. `let d: () = loop { };` with `let d: () = { loop { } };`.
One might reasonably wonder:
Is it not true that the block workaround proves that this change is
accomplishing nothing, at least if the goal is to maximize flexibility
for the future?

As a concrete example: let us assume, for the sake of argument, that
some future core team decides that the looping expression forms should
return `Option<T>` instead of `()`. But then consider code using the
block workaround: once support for the new looping expression forms is
turned on, won't the `{ loop { } }` block treat the inner `loop { }`
no longer as a statement, but instead as a tail expression in the
block? And therefore the code `let d: () = { loop { } };` would start
to fail to compile (since the return type of `loop { }` is some form
of `Option<_>`, which will not unify with the unit type `()` that has
been explicitly ascribed to the let-binding for `d`.

The mistake in the above reasoning is that it overlooks the detail in
this proposal that in the final form in a block expression, this RFC
states that we disambiguate by always choosing the statement form of a
loop rather than the expression form. Thus, `{ loop { } }` would
continue to have the type `()`. In the context of the final form in a
block, the `loop { }` is a statement unless it is e.g. wrapped in
parentheses to force it to be an expression, concretely:

```rust
let d: Option<_> = { ...; (loop { }) };
```

If the use of parentheses here seems overly strange to you,
take heart in the fact that there is precedent, sort of;
see the appendix [Precedent for proposed disambiguation].

# Drawbacks

It is a restriction designed to accommodate a hypothetized variant of
the non-unit looping expressions feature, but the language may never
adopt the feature; furthermore, it is possible to adopt the feature
without requiring this restriction, as described in [RFC PR 352] as
written.

# Alternatives

  * Obvious 1: We could jump straight to adopting one of the non-unit
    looping expressions variants.

  * Obvious 2: We could decide that non-unit looping expressions are
    only worth adopting if they can be added without requiring a change
    like this.

    Okay, now to less obvious.

  * We could restrict *just* `for`, `while`, and `while let` to
    statement forms, but leave `loop { ... }` as an expression.

    A justification for this is that there are more issues to resolve
    for the former three forms (namely in terms of how to deal with
    control-flow that does not hit a `break <value>`).

    But `loop { ... }` is really simple. Its control flow never exits
    to its immediate expression context, except via `break`, and thus
    one can justify leaving `loop` as an expression form, and just
    dictate that any furture additions for looping expression will
    have to accommdate it (presumably in a manner similar to that used
    by [RFC PR 352], i.e. `break` with no attached `<expr>` is sugar
    for `break ()`).

    Adopting this variant would tie our hands slightly, but not
    too much, and would actually remove much of the (already
    tiny) [prototype fallout], since many of the cases that
    had to be updated were uses of `let d = loop { };`
    within test code exercising the type system
    (i.e. essentially as synonymous for `let d = unimplemented!();`).

# Unresolved questions

None.

# Appendices

## Precedent for proposed disambiguation
[Precedent for proposed disambiguation]: #precedent-for-proposed-disambiguation

The statement/expression ambiguity at the end of a block expression
discussed above, and this same manner of disambiguation, has precedent
in Rust. Sort of.

In particular, an `if`-expression followed by minus sign and an
expression could be interpreted as either a statement followed by a
unary negation, or as a single subtraction with the `if`-expression as
its [minuend]. Just as in this RFC, the situation with
`if`-expressions is disambiguated by treating it as a statement
followed by a unary negation.

[minuend]: http://en.wikipedia.org/wiki/Subtraction#Notation_and_terminology

Here is a concrete example of this phenomenon.

```rust
// (not using Default so that we can differentiate negation from minus
//  expression)
trait Def { fn def() -> Self; }
impl Def for () { fn def() -> () { () } }
impl Def for i32 { fn def() -> i32 { -1 } }

fn h<D:Def+std::fmt::Debug>(context: &str) -> D {
    let d = Def::def();
    println!("h from {} returns {:?}", context, d);
    d
}

fn g(t: bool) -> i32 {
    (if t { h("g true") } else { h("g false") } - 3)
}
//   ^~~~~~~~~~~~~~~ if-expression returning -1
//
//  -1 - 3 ==> -4

fn f(t: bool) -> i32 {
    if t { h("f true") } else { h("f true") } - 3
}
//  ^~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ statement
//   (nested if is forced to have type `()`)
//
//  (); -3 ==> -3

fn main() {
    let x = f(true);
    let y = g(false); println!("x: {} y: {}", x, y);
}
```

The above code prints

```
h from f true returns ()
h from g false returns -1
x: -3 y: -4
```

However, I am drawing somewhat of a false analogy here. Namely, an
if-expression as the last form in a block will today be parsed as an
expression; this RFC is written under the expectation that if we do
adopt a form of non-unit looping expressions that require non-unit
return type, then we probably will require the use of parentheses
around such forms when they appear as the tail expression of a block.
