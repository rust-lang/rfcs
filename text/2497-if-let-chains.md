- Feature Name: `let_chains_2`
- Start Date: 2018-07-13
- RFC PR: [rust-lang/rfcs#2497](https://github.com/rust-lang/rfcs/pull/2497)
- Rust Issue: [rust-lang/rust#53667](https://github.com/rust-lang/rust/issues/53667)
- Rust Issue: [rust-lang/rust#53668](https://github.com/rust-lang/rust/issues/53668)

# Summary
[summary]: #summary

Extends `if let` and `while let`-expressions with chaining, allowing you
to combine multiple `let`s and `bool`-typed conditions together naturally.
After implementing this RFC, you'll be able to write, among other things:

```rust
fn param_env<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) -> ParamEnv<'tcx> {
    if let Some(Def::Existential(_)) = tcx.describe_def(def_id)
        && let Some(node_id) = tcx.hir.as_local_node_id(def_id)
        && let hir::map::NodeItem(item) = tcx.hir.get(node_id)
        && let hir::ItemExistential(ref exist_ty) = item.node
        && let Some(parent) = exist_ty.impl_trait_fn
    {
        return param_env(tcx, parent);
    }

    ...
}
```

and with side effects:

```rust
while let Ok(user) = read_user(::std::io::stdin())
    && user.name == "Alan Turing"
    && let Ok(hobby) = read_hobby_of(&user)
{
    if hobby == "Hacking Enigma" {
        println!("Yep, It's you.");
        return Some(read_encrypted_stuff());
    } else {
        println!("You can't be Alan! ");
    }
}

return None;
```

The main aim of this RFC is to decide that this is a problem worth solving
as well as discussing a few available options. **Most importantly, we want to
make `if let PAT = EXPR && ..` a possible option for Rust 2018.**

# Motivation
[motivation]: #motivation

The main motivation for this RFC is improving readability, ergonomics,
and reducing paper cuts.

## Right-ward drift

Today, each `if let` needs a brace, which means that you usually, to keep
the code readable, indent once to the right each time. Thus, matching multiple
things quickly leads to way too much indent that overflows the typical
text editor or IDE horizontally. This is in particular bad for readers that
can only fit around 80-100 characters per line in their editor. Keeping in
mind that code is read more than written, it is important to improve readability
where possible.

### Other solution: Tuples

One solution is matching a tuple, but that is a poor solution when there are
side effects or expensive computations involved, and doesn't necessarily work
as *DSTs* and *lvalues* can't go in tuples.

### Other solution: `break ...`

Another solution to avoid right-ward drift is to create a new function for
part of the indentation. When the inner scopes depend on a lot of variables
and state from outer scopes, all of these variables have to be passed on to
the newly created function, which may not even be a natural unit to abstract
into a function. Creating a new function, especially one that feels artificial,
can also inhibit local reasoning. A new level of function (or [IIFE]) also
changes the behaviour of `return`, `break`, `?`, and friends.

[IIFE]: https://en.wikipedia.org/wiki/Immediately-invoked_function_expression

A third solution involves using the expression form `break '<label>`.
You may then rewrite the snippet from the [summary] as:

```rust
fn param_env<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) -> ParamEnv<'tcx> {
    'stop: {
        if let Some(Def::Existential(_)) = tcx.describe_def(def_id) {
        } else {
            break 'stop;
        };

        let node_id = if let Some(node_id) = tcx.hir.as_local_node_id(def_id) {
            node_id
        } else {
            break 'stop;
        }

        let item = if let hir::map::NodeItem(item) = tcx.hir.get(node_id) {
            item
        } else {
            break 'stop;
        };

        let exists_ty = if let hir::ItemExistential(ref exist_ty) = item.node {
            exists_ty
        } else {
            break 'stop;
        }

        if let Some(parent) = exist_ty.impl_trait_fn {
            return param_env(tcx, parent);
        }
    }

    ...
}
```

while right-ward drift has been reduced, a significant amount of line noise
has been introduced. The user is also forced to track the label `'stop`.
All in all, this alternative significantly reduces readability wherefore we
discourage from this way of writing.

#### Boiler-plate reduction using macros

One way to reduce the noise from the above alternative solution is to refactor
some commonalities into a macro. However, refactoring into a macro means that
you need to understand the macro. In comparison, chained `if let`s constitute
something simpler that all Rust programmers will understand, as opposed to a
specialized macro.

## Mixing conditions and pattern matching

A `match` expression can have `if` guards, but `if let` currently requires
another level of conditionals.  This is particularly troublesome for cases
that can't be matched, like `x.fract() == 0`, or error `enum`s that disallow
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

In this example `foo()` and `bar(x)` have side effects, but more crucially,
there is a dependency between matching on the result of `foo()` to execute
`bar(x)`. Therefore, matching on `(foo(), bar(x))` is not possible in this
case because there's no `x` in scope. So you have no choice but to write it
in this way (or use `break 'label..`).

However, now `some_long_expression` is repeated, and if more `let` bindings
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

As we've previously touched upon, we may define and use a macro to reduce
boilerplate. A macro like [`if_chain!`] as a solution however has the problem
of not being part of the language specification. Thus, it is not part of the
common syntax that experienced Rust programmers are familiar with and is instead
local to the project itself. The non-universality of syntax therefore hurts
readability.

[`if_chain!`]: https://crates.io/crates/if_chain

## Plenty of Real-world use cases

[reverse dependencies]: https://crates.io/crates/if_chain/reverse_dependencies

[defines a function]: https://github.com/rust-lang-nursery/rust-clippy/blob/ed589761e62735ebb803510e01bfd8b278527fb9/clippy_lints/src/print.rs#L207-L219

We have already seen a real world example from the compiler in the [summary].
By taking a look at the [reverse dependencies] of [`if_chain!`] we can find
more real-world use cases that this RFC facilitates.

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
        && let ExprArray(ref exprs) = expr.node
        && let Some(expr) = exprs.last()
        && let ExprLit(ref lit) = expr.node
        && let LitKind::Str(ref lit, _) = lit.node
    {
        Some((lit.as_str(), exprs.len()))
    } else {
        None
    }
}
```

This kind of deep pattern matching is common for parsers and when dealing
with ASTs. One place which deals with ASTs is the compiler itself as seen above.
Thus, with this RFC, some compiler internals may be simplified.
Another common place is when authoring with custom derive macros using the 
`syn` crate.

## An expected feature
[an_expected_feature]: #an-expected-feature

As demonstrated in [Appendix B], the syntax proposed in this RFC is already
expected to be allowed in Rust by users today. Indeed, the author of this RFC
made this assumption at some point.

## *Unification*

In today's Rust, there is both a grammatical and conceptual distinction between
`if` and `if let` as well as `while` and `while let`. This RFC aims to erase
the divide and unify concepts. Henceforth, there is just `if` and `while`.
Thus `if let` is no longer the unit.

## "Why now?"

A legitimate question to ask is:
> Why implement this now?

In this case, the answer is simple: We can't wait.

Because Rust takes stability seriously, we would like to avoid any breakage
in-between editions even if the breakage is exceedingly (as in the case of this
RFC) unlikely. Instead, we want to deal with the vanishingly tiny degree of
breakage, as explained in the [reference-level-explanation], introduced by this
RFC with the edition mechanism.

As it happens, a new edition "Rust 2018" is in the works at the moment
(as of 2018-07-12). This is an excellent opportunity to take advantage of,
and that is precisely what we aim to do here.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section examines the features proposed by this RFC.

## `if let`-chains

An *`if let` chain*, refers to a chain of multiple `let` bindings,
which may mixed with conditionals in an `if` expression.

An example of such a chain is:

```rust
if let A(x) = foo()
    && let B(y) = bar()
{
    computation_with(x, y)
}
```

It is important to note that this is *not* generally equivalent to the
following expression:

```rust
if let (A(x), B(y)) = (foo(), bar()) {
    computation_with(x, y)
}
```

Unlike the first example, there is no short circuiting logic in the example
using tuples. Assuming that there are no panics, which is usually the case,
both functions are always executed in the latter example.

If we desugar the first example, we can clearly see the difference:

```rust
if let A(x) = foo() {
    if let B(y) = bar() {
        computation_with(x, y)
    }
}
```

What is the practical difference, and why is short circuiting behaviour
an important distinction? The call to `bar()` may be an expensive one.
Avoiding useless work is beneficial to performance. There is however a more
fundamental reason. Assuming that `bar()` has side effects, the meaning of
the tuple example is different from the nested `if let` expressions because in
the case of the former, the side effect of `bar()` always happens while it
will not if `let A(x) = foo()` does not match.

The difference between the tuple example and `if let`-chains become even
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
if let A(x) = foo()
    && let B(y) = bar(x)
{
    computation_with(x, y)
}
```

The new expression form introduced by this RFC is also not limited to simple
`if let` expressions, you may of course also add `else` branches as seen in
the example below.

```rust
if let A(x) = foo()
   && let B(y) = bar()
{
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

As briefly explained above, the `if let`-chain expression form is also not
limited to pattern matching. You can also mix in any number of conditionals
in any place you like, as done in the example below:

```rust
if independent_condition
   && let A(x) = foo()
   && let B(y) = bar()
   && y.has_really_cool_property()
{
    computation_with(x, y)
}
```

The above example example is semantically equivalent to:

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

Naturally, inside an `if-let`-chain expression, a `let` binding must come
before it is referred to. As such, the following snippet would be ill-formed
since we haven't implemented time-travel (yet):

```rust
if y.has_really_cool_property() // <-- y used before bound.
   && let B(y) = bar(x) // <-- x used before bound.
   && let A(x) = foo()
{
    computation_with(x, y)
}
```

## `while let`-chains

A **`while let`-chain** is similar to an `if let`-chain but instead applies to
`while let` expressions.

Since we've already introduced the basic idea previously with *`if let`-chains*,
we will jump straight into a more complex example.

The popular [`itertools`] crate has an `izip` macro that allows you to
*"Create an iterator running multiple iterators in lockstep"*. An example
of this, taken from the documentation of `izip` is:

[`itertools`]: https://docs.rs/itertools/0.7.8/itertools/macro.izip.html

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

while let Some(r) = r_iter.next()
    && let Some(index) = c_iter.next()
    && let Some(input) = i_iter.next()
{
    *r = index * 10 + input;
}

assert_eq!(results, [0 + 3, 10 + 7, 29, 36]);
```

The loop in the above snippet is equivalent to:

```rust
loop {
    if let Some(r) = r_iter.next()
        && let Some(index) = c_iter.next()
        && let Some(input) = i_iter.next()
    {
        *r = index * 10 + input;
        continue;
    }
    break;
}
```

Notice in particular here that just as we could rewrite `while let` in
terms of `loop` + `if let`, so too can we rewrite `while let`-chains with
`loop` + `if let`-chains.

While these two first snippets are equivalent in this example, this does not
generally hold. If `i_iter.next()` has side effects, then those will not
happen when `Some(index)` does not match. This is important to keep in mind.
Short-circuiting still applies to `while let`-chains as with `if let`-chains.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

[RFC 2046]: https://github.com/rust-lang/rfcs/pull/2046

This RFC introduces `if let`-chains and `while let`-chains in *Rust 2018*
and makes some enabling preparation for such chains in Rust 2015.

## Grammar

We replace the following productions:

```bnf
block_expr
: expr_match
| expr_if
| expr_if_let
| expr_while
| expr_while_let
| expr_loop
| expr_for
| UNSAFE block
| path_expr "!" maybe_ident braces_delimited_token_trees
;

expr_if
: IF expr_nostruct block
| IF expr_nostruct block ELSE block_or_if
;

expr_if_let
: IF LET pat "=" expr_nostruct block
| IF LET pat "=" expr_nostruct block ELSE block_or_if
;

block_or_if : block | expr_if | expr_if_let ;

expr_while : maybe_label WHILE expr_nostruct block ;
expr_while_let : maybe_label WHILE LET pat "=" expr_nostruct block ;
```

with:

```bnf
block_expr
: expr_match
| expr_if
| expr_while
| expr_loop
| expr_for
| UNSAFE block
| path_expr "!" maybe_ident braces_delimited_token_trees
;

expr_if
: IF in_if_list block
| IF in_if_list block ELSE block_or_if
;

block_or_if : block | expr_if ;

expr_while : maybe_label WHILE in_if_list block ;

in_if
: "let" pat "=" expr_nostruct
| expr_nostruct
| "(" in_if ")"
;

in_if_list : in_if [ ANDAND in_if ]*
```

### Dealing with ambiguity

There exists an ambiguity in this new grammar in how to parse:

```rust
if let PAT = EXPR && EXPR { .. }
```

It can either be parsed as (1):

```rust
if let PAT = (EXPR && EXPR) { .. }
```

or instead as (2):

```rust
if (let PAT = EXPR) && EXPR { .. }
```

In the interest of succinctness, we do not encode a grammar here that
resolves this ambiguity. Nonetheless, interpretation (2) is *always*
chosen.

[expression operator precedence]: https://github.com/rust-lang-nursery/reference/blob/master/src/expressions.md#expression-precedence

As specified in the reference in the section on [expression operator precedence],
the following operators all have a lower precedence than `&&`:

+ `||`
+ `..` and `..=`
+ `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`
+ `return`, `break`

To be precise, the changes in this RFC entail that `||` has the lowest
precedence at the top level of `if STUFF { .. }`. The operator `&&`
has then the 2nd lowest precedence and binds more tightly than `||`.
If the user wants to disambiguate, they can write `(EXPR && EXPR)` or
`{ EXPR && EXPR }` explicitly. The same applies to `while` expressions.

#### A few more examples

Given:

```rust
if let Range { start: _, end: _ } = true..true && false { ... }

if let PAT = break true && false { ... }

if let PAT = F..|| false { ... }

if let PAT = t..&&false { ... }
```

it is currently interpreted as:

```rust
if let Range { start: _, end: _ } = true..(true && false) { ... }

if let PAT = break (true && false) { ... }

if let PAT = F..(|| false) { ... }

if let PAT = t..(&&false) { ... }
```

but will be interpreted as:

```rust
if (let Range { start: _, end: _ } = true..true) && false { ... }

if (let PAT = break true) && false { ... }

if (let PAT = F..) || false { ... }

if (let PAT = t..) && false { ... }
```

### Rollout Plan and Transitioning to Rust 2018

Everything in this section also applies to `while let` expressions.

To enable the second interpretation in the previous section a warning must
be emitted in Rust 2015 informing the user that:

```rust
if let PAT = EXPR && EXPR ...? { .. }

if let PAT = EXPR || EXPR ...? { .. }
```

will both become *hard errors*, in the first version of Rust where the 2018
edition is stable, without the `let_chains` features having been stabilized.

Note that this applies when there's at least one `&&` or `||` operator at the
top level of the RHS. This means that it does *not* apply in, among others,
the following cases:

```rust
if let PAT = ( EXPR && EXPR ) { .. }

if let PAT = { EXPR && EXPR } { .. }

if let PAT = ( EXPR || EXPR ) { .. }

if let PAT = { EXPR || EXPR } { .. }
```

since the user has disambiguated the intent explicitly.

Pending the stabilization of the features in this RFC, to opt into the new
semantics, users will need to use a nightly compiler and add the usual feature
gate opt-in.

### Facilitating for macro authors

To facilitate for macro authors, we permit the following:

```rust
if (let PAT = EXPR) && ... { ... }
```

### `let PAT = EXPR` is *not* an expression

Note that `let PAT = EXPR` does *not* become an expression (typed at `bool`)
with this RFC. Thus, you may not write:

```rust
let foo: bool = let Some(_) = None;
let bar: bool = let Some(_) = Some(1);
assert_eq!(foo, false);
assert_eq!(bar, true);
```

## Semantics of `if let`-chains

The semantics of `if let`-chains can be understood by an in-surface-language
desugaring using only [RFC 2046] and `if let`.

The following:

```rust
if let PAT_1 = EXPR_1
    && let PAT_2 = EXPR_2
    && EXPR_3
    ...
    && let PAT_N = EXPR_N
{
    EXPR_IF
} else {
    EXPR_ELSE
}
```

desugars into:

```rust
'FRESH_LABEL: {
    if let PAT_1 = EXPR_1 {
        if let PAT_2 = EXPR_2 {
            if EXPR_3 {
                ...
                if let PAT_N = EXPR_N {
                    break 'FRESH_LABEL { EXPR_IF }
                }
            }
        }
    }
    { EXPR_ELSE }
}
```

This avoids any code duplication and requires no new semantics. The rules for
borrowing and scoping are just those that result directly from the desugar.

The `else if` branches:

```rust
if let PAT_1 = EXPR_1
    && let PAT_2 = EXPR_2
{
    EXPR_IF
} else if let PAT_3 = EXPR_3
    && EXPR_4
{
    EXPR_ELSE_IF
} else {
    EXPR_ELSE
}
```

are defined by their desugaring to:

```rust
'FRESH_LABEL: {
    if let PAT_1 = EXPR_1 {
        if let PAT_2 = EXPR_2 {
            break 'FRESH_LABEL { EXPR_IF }
        }
    }

    if let PAT_3 = EXPR_3 {
        if EXPR_4 {
            break 'FRESH_LABEL { EXPR_ELSE_IF }
        }
    }

    { EXPR_ELSE }
}
```

Having an `else` branch is optional.
The following example without an `else` branch:

```rust
if let PAT_1 = EXPR_1
    && let PAT_2 = EXPR_2
{
    EXPR_IF
}
```

is simply desugared into:

```rust
if let PAT_1 = EXPR_1 {
    if let PAT_2 = EXPR_2 {
        EXPR_IF
    }
}
```

If we have an `else if` branch but no `else` branch, such as in this example:

```rust
if let PAT_1 = EXPR_1
    && let PAT_2 = EXPR_2
{
    EXPR_IF
} else if let PAT_3 = EXPR_3
    && EXPR_4
{
    EXPR_ELSE_IF
}
```

the semantics are defined by the following desugaring:

```rust
'FRESH_LABEL: {
    if let PAT_1 = EXPR_1 {
        if let PAT_2 = EXPR_2 {
            break 'FRESH_LABEL { EXPR_IF }
        }
    }

    if let PAT_3 = EXPR_3 {
        if EXPR_4 {
            break 'FRESH_LABEL { EXPR_ELSE_IF }
        }
    }
}
```

## Semantics of `while let`-chains

The semantics of `while let`-chains can be understood by an in-surface-language
desugaring using only [RFC 2046], `loop` and `if let`.

For example:

```rust
while EXPR_1
    && let PAT_2 = EXPR_2
    && let PAT_3 = EXPR_3
    && EXPR_4
{
    EXPR_WHILE
}
```

is defined by desugaring into:

```rust
loop {
    if EXPR_1
        && let PAT_2 = EXPR_2
        && let PAT_3 = EXPR_3
        && EXPR_4
    {
        { EXPR_WHILE }
        continue;
    }
    break;
}
```

This desugaring relies on the previously discussed desugaring for `if let`-chains.

More generally, we may desugar:

```rust
while in_if_list {
    EXPR_WHILE
}
```

into:

```rust
loop {
    if in_if_list {
        { EXPR_WHILE }
        continue;
    }
    break;
}
```

# Drawbacks
[drawbacks]: #drawbacks

This RFC mandates additions to the grammar as well as adding syntax lowering
passes. These are small additions, but nonetheless the language specification
is possibly made more complex by it. While this complexity will be used by some
and therefore, the RFC argues, motivates the added complexity, it will not be
used all users of the language. However, as discussed in the [motivation],
by unifying constructs in the language conceptually and grammatically,
we may also say that complexity is *reduced*.

When it comes to `if let`-chains, the feature is already supported by the
macro `if_chain!`. Some may feel that this is enough.

It should also be taken into account that some breakage will occur as a result
of this RFC. Sergio Benitez has however done some review of the crates.io
ecosystem and found zero cases of actual breakage. At any rate, writing
`let PAT = EXPR && ..` as a user is a bad thing to do.

[petrochenkov_cite_1]: https://github.com/rust-lang/rfcs/pull/2260#issuecomment-353780537

Finally, some may argue, [as done by @petrochenkov][petrochenkov_cite_1],
that this is *"a lot of ad-hoc syntax to deprecate when the proper solution
solving all the listed problems is implemented"*.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

We will now discuss how and why this RFC came about in its current form.

## The impact of not doing this

There are at least two sides to power in language expressivity:

1. The ability to express something in a language at all.

2. The ability to express something with ease.

Nothing proposed in this RFC adds to point 1. While this is the case,
it is not sufficient. The second point is important to make the language
pleasurable to use and this is what this RFC is about. Not including the
changes proposed here would keep some paper cuts around.

## Design considerations
[design considerations]: #design-considerations

There are some design considerations on this feature. These are:

1. the syntax mixes well with normal `bool`ean conditionals.

2. the additions be simple conceptually and build on what language  
  users already know.

3. as little of the complexity budget as possible is used.

4. the bindings bound in the pattern have a clear and consistent
  scope.

5. the short-circuiting nature is clear.

6. instead of a heap of special cases, the grammar should be simple.

With these considerations in mind, the RFC was developed.

Note that these are considerations and have different levels of importance.
Note also that it is likely impossible to meet all of them, but we'd like
to tick as many boxers as possible.

## Keeping the door open for `if-let-or`-expressions

Should a user be able to write something like the following snippet?

```rust
if let A(x) = e1
    || let B(x) = e2 {
    do_stuff_with(x)
} else {
    do_other_stuff()
}
```

What does this expression even mean? It means that if one of the patterns
match, then the first one of those will bind a value to `x` and the expression
evaluates to `do_stuff_with(x)`. If no patterns match, the expression instead
evaluates to `do_other_stuff()`.

This RFC does not propose such a facility, but does not foreclose such
a possibility, making the feature future proof and allowing discussion on such
a facility in the future to continue. Alternatives should similarly try to
retain this ability.

## Alternative: [RFC 2046], label break value

[RFC 2046]: https://github.com/rust-lang/rfcs/pull/2046
[the macros to be improved]: https://github.com/rust-lang/rfcs/pull/2046#issuecomment-320483246
[CFG]: https://en.wikipedia.org/wiki/Control_flow_graph

[RFC 2046], which has been merged but not stabilized,
is a more general *control flow graph* ([CFG]) control feature.
While it doesn't as straightforwardly solve the rightward drift or ergonomic
issues as this RFC does, it allows [the macros to be improved] by removing
duplication of `else` blocks. The closest syntax today for that is `loop-break`,
but that doesn't work as `continue` is intentionally non-hygienic.

RFC 2046 is also a bit orthogonal in the sense that it's fully compatible
with this RFC. The general label break is useful and powerful, as seen in
the [reference-level-explanation] of this RFC and of `catch`'s, but is
verbose and unfamiliar. Having a substantially more ergonomic feature for
this particularly common case is valuable regardless. As such, we argue that
this RFC is mostly complementary wrt. RFC 2046.

Furthermore, as we've noted in the [motivation], a macro based approach is
not a construct that is universal among Rust programmers, which is an important
property for control flow in particular to improve the legibility of programs.

## The main alternatives

There are some alternatives to consider. Let's go through some of the
main ones.

First, there's the choice of a separator to use in-between `let`s and `bool`
typed condition expressions. We consider 3 different separators:

1. logical and (`&&`)
2. comma (`,`)
3. `if`

We also consider two different ways to bind inside `if`:

1. `let PAT = EXPR`
2. `EXPR is PAT`

Additionally, instead of the keyword `is`, we consider `match`.
In total, we have 6 (or 9 if we count `match`) variants to pick from.
These 6 alternatives are:

In this RFC, we propose the combination of `&&` and `let PAT = EXPR`.

### A survey - Method

To gain some data on what users of Rust think about the 6 different variants,
a multi-answer survey was done using Google Forms. The survey ran from
2017-12-31 06:25 to ~2018-01-06 ~14:00 and received 373 answers.
Participants were also able to provide free-form motivation ("comments")
to their answers if they so wished.

To decrease the risk of bias in favour of a particular alternative,
the order of the answers as presented to survey participants were randomized.
Furthermore, to make the survey more fair, all alternatives were syntax
highlighted as a normal IDE would do.

The survey answers had the following distribution in origin:

+ Reddit, 68.4%
+ internals.rust-lang.org, 16.6%
+ users.rust-lang.org, 7.5%
+ IRC, 5.1%
+ The RFC, 2.4%

### A survey - Data

For those interested in reading the survey answers you can do so by reading:
+ [A summary of the survey](https://docs.google.com/forms/d/e/1FAIpQLScwG0Y3ynA9aJZ-iprOey_GyCNeFMO9MSDJR1kiskpjsjL1Mw/viewanalytics)
+ [A CVS file of the survey](https://drive.google.com/file/d/1awyvryblSHFH9J77TPutW5BrRlKr0EKZ/view?usp=sharing)
+ [A PDF for the survey](https://drive.google.com/file/d/14ofF5on_Z_XLvhPr1I4dVgCfcybQO2GY/view?usp=sharing)

The breakdown of preferences were:

1. Using `&&` and `let PAT = EXPR` - liked: 66.2%, disliked: 16.9%

   ```rust
   if let PAT = EXPR
       && let PAT = EXPR
       && EXPR
   {
       ..
   }
   ```

2. Using `&&` and `EXPR is PAT` - liked: 24.9%, disliked: 48.5%

   ```rust
   if EXPR is PAT
       && EXPR is PAT
       && EXPR {
       ..
   }
   ```

3. Using `,` and `let PAT = EXPR` - liked: 16.9%, disliked: 56.3%

   ```rust
   if let PAT = EXPR,
      let PAT = EXPR,
      EXPR {
       ..
   }
   ```

4. Using `if` and `let PAT = EXPR` - liked: 12.3%, disliked: 66%

   ```rust
   if let PAT = EXPR
   if let PAT = EXPR
   if EXPR {
       ..
   }
   ```

5. Using `,` and `EXPR is PAT` - liked: 4.3%, disliked: 74.5%

   ```rust
   if EXPR is PAT,
      EXPR is PAT,
      EXPR {
        ..
   }
   ```

6. Using `if` and `EXPR is PAT` - liked: 2.4%, disliked: 80.4%

   ```rust
   if EXPR is PAT
   if EXPR is PAT
   if EXPR {
       ..
   }
   ```

Finally, 9.7% liked none of the options and 1.9% liked all of them.

### A survey - Analysis of Comments

There are too many answers to include here, instead, we select some of
the most interesting ones and highlight them.

#### Tried before

One participant, among 6 (see [Appendix B.1]) others who all positively inclined,
explicitly commented that they had tried the syntax proposed in this RFC before.

> The "`if let .. && let .. && ..`" feels like the intuitive way to do it if
> you don't think about the language syntax too much. It's definitely the way
> I tried doing it when I thought it was possible at the start of my Rust path.

This substantiates the claim made in the [motivation][an_expected_feature].

#### Consistency

An even greater number of people (48, see [Appendix B.2]) commented that they
thought that the proposed syntax was the *consistent* alternative. This was
by far the most frequent comment made in the survey.

> So I like that using `&&` is how we currently use it in the language,
> and everyone is already used to using `let A(x) = foo()`.
> Honestly, the one I chose feels the most consistent with the language.

#### Intuitiveness

A lesser number (8, see [Appendix B.3]) of participants said did not explicitly
say that the proposed syntax was *consistent*, but that they found it
*intuitive* nonetheless.

> `&&` makes the logic relationship clearer, and using `let` for binding is the same. 
> Conjunction is more readable with `&&`

This, and in particular the consistency, goes a long way to satisfy points
2-3 in the [design considerations].

#### Expectation that `(let PAT = EXPR) : bool`

A few participants (3, see [Appendix B.4]) hinted at that using `&&` together
with `let PAT = EXPR` set up the expectation that the latter is a `bool` typed
expression.

> Using `&&` for conjunction with `let PATTERN = EXPR` feels consistent with
> the existing `if let` syntax, however it causes potentially some confusion
> about data types and its existing function as a `boolean` operator, so that
> leads me to considering `,` as the conjunction instead.
> However, if "`let PATTERN = EXPR`" is an expression returning a boolean as
> well as setting up the pattern bindings then there's no issue with `&&` at
> all, and it's then preferable to me provided it's available where you'd
> expect expressions to be available and not treated particularly specially.

If that were the case you'd be able to write:
```rust
let is_some: bool = let Some(_) = the_option;
```
However, this is not the case in this proposal.

We expect that this will be one of the most frequent misconceptions in relation
to the proposed syntax. However, such misconceptions can be put to bed simply
when the user tries to write a snippet like the one above. They will then get
an error message that clears up that misconception. It should also be noted that
`if let`, which exists in the language today, also suffers from this problem.
That is, given `if let PAT = EXPR { .. }`, a user may get the impression that
it is the composition of `if EXPR { .. }` and `let PAT = EXPR` while it is *not*.
While the syntax changes in this RFC does enhance the risk of misconception
somewhat, ultimately we do not feel that it poses a critical problem.

#### Commas and `if` as separators - conjunction?

There were many people (19, see [Appendix B.5]) who felt that using `,` or `if`
as the separator did not clearly enough signal conjunction and thought that
the symbols may be mistaken for disjunction.

> Commas just aren't clear enough: on their own, to many people,
> they could easily be interpreted as logical ORs or logical ANDs.

In most cases, these comments were directed towards `,`, but there were also
some who thought this about `if`:

> `if` after `if` with no logical operator? is this AND? is this OR?

On the other hand, it could be argued that Rust already uses `if` for
conjunction since you can use `PAT if EXPR => ..` inside `match` expressions.
Indeed, a few people hinted at this:

1. > Clear and unambiguous, and similar to existing guards in match statements,
   > so it does not introduce completely new syntax.

2. > This is already basically how match arms work.

Our conclusion is that this at least presents a serious enough of a problem
for `,` as the separator for conjunction to rule it out while also being
problematic for `if`.

#### Commas and short-circuiting

A number of participants (5, see [Appendix B.6]) noted that using `,` as the
separator was not clearly enough indicating short-circuiting behaviour.

> On the other hand the comma'd version felt the least clear in meaning
> and execution order. I'm more used to things-separated-by-commas being
> roughly equivalent instead of being something that ends up short
> circuiting the evaluation.

This is a further blow to `,` in terms of our [design considerations].

#### `if` as separator is noisy

Some people argued that `if` as a separator felt noisy or that it felt like
there were missing braces. One also noted that multiple `if`s on one line
wouldn't work well on a single line. However, one respondent said that
the "eliding of braces"-interpretation was a *good* thing.

As an aside, we would like to note here that `if` as a separator would need
to be matched with `while` as a separator as well. This makes the separator
too context dependent in our view.

#### Patterns unexpectedly on the RHS

Some people (10, see [Appendix B.8]) thought that bindings introduced on the
RHS as in `EXPR is PAT` as opposed to `let PAT = EXPR` was backwards and weird.

> `expr is pat` reverses the directionality for pattern bindings seen
> everywhere else in Rust;

One could argue that bindings introduced in the arms of `match` expressions are
to the right if one formats such expressions as:

```rust
match EXPR { PAT => ... }
   // LHS // RHS
```

However, this is not the typical formatting of `match` expressions as they
tend to include more than one arm. When using the normal formatting of
such expressions, the match arms, and therefore the bindings, are introduced
on the LHS.

This inconsistency does not have to be an insurmountable problem as we believe
that `EXPR is PAT` generally reads well. However, having the pattern
consistently on LHS everywhere makes introductions of bindings more readily
scannable, which is a valuable property when reading code quickly.

#### The `is` operator introduces bindings

However, a more serious problem that some survey participants
(15, see [Appendix B.9]) identified was that `EXPR is PAT`, according
to the respondents, confusingly introduces a binding and that
it could be misconstrued as an equality test of some sort.

> `is` doesn't make any sense since we already have `if let PATTERN` and
> `is` in other languages is typically a reference equality check
> (e.g. Dart and Python).

> I dislike the `EXPR is PATTERN` syntax because while the word `let`
> indicates that there is some binding going on, I read the word `is` as
> passively checking whether the expression fits a pattern without binding.
> I also dislike `is` because it is new syntax that does the same thing as
> existing syntax.

We believe this problem to be more serious. As an alternative to `EXPR is PAT`,
some have proposed using the existing keyword `match` instead. You would then
instead write the example in the [motivation] as:

```rust
fn param_env<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>, def_id: DefId) -> ParamEnv<'tcx> {
    if tcx.describe_def(def_id) match Some(Def::Existential(_))
        && tcx.hir.as_local_node_id(def_id) match Some(node_id)
        && tcx.hir.get(node_id) match hir::map::NodeItem(item)
        && item.node match hir::ItemExistential(ref exist_ty)
        && exist_ty.impl_trait_fn match Some(parent)
    {
        return param_env(tcx, parent);
    }

    ...
}
```

As previously noted, using `is` is less scannable. This also applies to `match`.

As an aside, one survey participant confused `is` for `as`; This does seem like
a mistake that is likely to happen due to the similarity of these two words.

## Conclusion

We believe that the case for `&&` and `let PAT = EXPR` is strong.
As demonstrated by the survey, which we believe is statistically significant,
it is both consistent and intuitive for most users. The syntax also satisfies
most of the points in the [design considerations].

The only main drawbacks to this proposal is some tiny bit of breakage as
well as an increase in implementation complexity.
The breakage is considered OK, because writing `let true = p && q` is
at any rate a terrible style and because it is so infrequent.
As for the increased grammar complexity, we believe this is less important
in this case than making control flow more ergonomic and readable for users.

[RFC 2260]: https://github.com/rust-lang/rfcs/pull/2260

Some may view the fact that `let PAT = EXPR` is not an expression typed at
`bool` as an ad-hoc solution. However, we believe that we should live within
our means wrt. the complexity budget and spend it on more important things.
Furthermore, as evidenced in [RFC 2260], making `EXPR is PAT`,
which has other problems we've previously noted, an expression is also tricky
due to the non-obvious scoping rules for bindings it entails.
Mainly because of this, support for `EXPR is PAT` has been slow to develop.

For the use case of having some pattern matching construct that is typed at
`bool`, we could later introduce the form `EXPR is PAT` but prohibit  `PAT`
from introducing bindings.

# Prior art
[prior-art]: #prior-art

## Swift

[RFC 160]: https://github.com/rust-lang/rfcs/pull/160

The expression form `if let PAT = EXPR { .. }` was introduced to Rust by
accepting [RFC 160]. That RFC noted that:

> The if let construct is based on the precedent set by Swift,
> which introduced its own if let statement.
> In Swift, `if let var = expr { ... }` is directly tied to the notion of
> optional values, and unwraps the optional value that `expr` evaluates to.
> In this proposal, the equivalent is `if let Some(var) = expr { ... }`.

As the construct `if let` was inspired by Swift, it therefore makes sense
to consult Swift to see how the language deals with multiple `let`s in
`if`.

It turns out that you can by writing:

```swift
if let g = greetings, let s = salutations {
    print(g)
    print(s)
}
```

which with the syntax proposed in this RFC would be equivalent to:

```rust
if let Some(g) = greetings
    && let Some(s) = salutations
{
    print(g)
    print(s)
}
```

[`case let`]: http://alisoftware.github.io/swift/pattern-matching/2016/05/16/pattern-matching-4/

You can also use `case let` for more general pattern matching:

```swift
if case let Media.movie(_, _, year) = m, year < 1888 {
    ...
}
```

Previously in Swift, you would instead write:

```swift
if case let Media.movie(_, _, year) = m, where year < 1888 {
    ...
}
```

[SE-0099]: https://github.com/apple/swift-evolution/blob/master/proposals/0099-conditionclauses.md

but this was changed in favour of omitting `where` in [SE-0099].

Interestingly, the separator token that Swift uses for conjunctive chaining
in `if` is `,` (comma). [RFC 2260] proposed this, but this turned out not
to be as intuitive for many users as `&&` is (see [alternatives] for a discussion).

## Kotlin

In [RFC 2260] [@matklad](https://github.com/matklad) said that:

> It's interesting to compare it with Kotlin, which also uses is operator for the similar purpose: https://kotlinlang.org/docs/reference/typecasts.html#smart-casts.
>
> The differences is that instead of destructing, Kotlin's is supplies a flow-sensitive type information. The compiler indeed uses pretty smart control-flow analysis to check if every use of a variable is dominated by the is check.
>
> However, as long as the compiler does all the inference work for you, actually using this feature is easy: you don't have to replay the analysis in your head when reading or writing code, because the compiler catches all errors.

## [RFC 160]

Interestingly, the `EXPR is PAT` idea was floated in the original RFC 160 that
introduced `if let` expressions in the first place. There, the notion that an
operator named `is`, which introduces bindings, is confusing was brought up.

[lilyball_1]: https://github.com/rust-lang/rfcs/pull/160#issuecomment-48515260
[lilyball_2]: https://github.com/rust-lang/rfcs/pull/160#issuecomment-48551196
[liigo_1]: https://github.com/rust-lang/rfcs/pull/160#issuecomment-49234092
[lilyball_3]: https://github.com/rust-lang/rfcs/pull/160#issuecomment-49242255

It was also mentioned by [@lilyball][lilyball_1] that it would be appropriate
if, and only if, it was limited to pattern matching, but not introducing any
bindings. We make the same argument in this RFC. The issue of unintuitive
scopes was also mentioned [by @lilyball][lilyball_2] there.

Even the idea of `if EXPR match PAT` was floated by [@liigo][liigo_1] at the
time but that idea was ultimately also rejected. [@lilyball][lilyball_3] opined
that using `match` as a binary operator would be *"very confusing"* but did not
elaborate further at the time.

# Unresolved questions
[unresolved]: #unresolved-questions

## The final syntax

The main goal of this RFC is threefold:

1. Decide that this is a problem that needs to be solved *somehow*.

2. Make the proposed syntax in the RFC an option that is available in Rust 2018.

3. Adopt the proposed syntax in the RFC.

Of these points, the 1st and the 2nd are the most important for the time being.
The 3rd point is not unimportant, but it is not as time sensitive.
Thus, one path ahead of least resistance is to adopt the syntax in the RFC and
make it available in Rust 2018 while leaving the final syntax unresolved.
We can then debate alternatives, in particular using `EXPR match PAT`,
more rigorously post shipping Rust 2018. Finalizing the syntax and can
then be decided in a tracking issue or another RFC.

## Irrefutable let bindings after the first refutable binding

Should temporary and irrefutable `let`s without patterns be allowed as
in the following example?

```rust
if let &List(_, ref list) = meta
    && let mut iter = list.iter().filter_map(extract_word) // <-- Irrefutable
    && let Some(ident) = iter.next()
    && let None = iter.next()
{
    *set = Some(syn::Ty::Path(None, ident.clone().into()));
} else {
    error::param_malformed();
}
```
  
With normal `if let` expressions, this is an error as seen with the 
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
perfectly well defined semantics, where the value of the expression is
always 2, allowing the form would invite some developers to write in a
non-obvious way. A warning is however a good middle ground.

However, when let bindings in the middle are irrefutable, there is some value
in not warning against the construct. In the case of the initial example in this
subsection, it would be written as follows without irrefutable let bindings:

```rust
if let &List(_, ref list) = meta {
   let mut iter = list.iter().filter_map(extract_word);
    if let Some(ident) = iter.next()
        && let None = iter.next()
    {
        *set = Some(syn::Ty::Path(None, ident.clone().into()));
    } else {
        error::param_malformed();
    }
} else {
    error::param_malformed();
}
```

However, now we have introduced rightward drift and duplication again,
which we wanted to avoid.

On the other hand, allowing irrefutable patterns in the middle without a
warning may give the impression that the irrefutable pattern is refutable,
or cast doubt on it making semantics possibly harder to grasp quickly.

This is a tricky question, which we leave open for consideration during
the stabilization period or even after stabilization.

## Chained `if let`s inside `match` arms

Would the following be accepted by a Rust compiler?

```rust
match EXPR {
    PAT if let PAT = EXPR && EXPR && ... => { .. }
    _ => { .. }
}
```

[RFC 2294]: https://github.com/rust-lang/rfcs/pull/2294

The combination of the accepted, but yet to be stabilized, [RFC 2294], and this
RFC would entail that it would be accepted. However, at this point, and in the
interest of time, we leave this for a future RFC or for pre-stabilization.

# Appendix A - Style considerations

How should the features introduced in this RFC be formatted?
This is not a make or break question but rather a style question for `rustfmt`.
What you read here should not be taken as prescriptive but rather as discussion
material and to generate ideas. Any eventual decision on style will be made by
a separate style RFC.
  
Here are a few variants on indentation to consider for `rustfmt` while
may or may not be mutually compatible:

### 1. `&&` on a new line and indented + Open-brace after newline

```rust
if independent_condition
    && let Alan(x) = turing()
    && let Alonzo(y) = church(x)
    && y.has_really_cool_property()
{
    computation_with(x, y)
}
```

This style is maximally consistent with how conditions in `if` expressions are
currently formatted.

Moving the open brace down a line may help emphasize the split between
a lengthy condition and the block body.

### 2. `&&` after bindings

```rust
if independent_condition &&
   let Haskell(x) = curry() &&
   let Alonzo(y) = church(x) &&
   y.has_really_cool_property() {
    computation_with(x, y)
}
```

This style is consistent with how separators, such as `,`, are currently
formatted in Rust.

### 3. `&&` at the start of lines

```rust
if independent_condition
&& let Alan(x) = turing()
&& let Alonzo(y) = church(x)
&& y.has_really_cool_property() {
    computation_with(x, y)
}
```

This style of leading separators is inconsistent with current formatting.

### 4. Aligning the equals sign together

```rust
if independent_condition &&
   let Alan(x)   = turing() &&
   let Alonzo(y) = church(x) &&
   y.has_really_cool_property() {
    computation_with(x, y)
}
```

[rustfmt guidelines]: https://github.com/rust-lang-nursery/fmt-rfcs/blob/master/guide/principles.md#overarching-guidelines

While this might look visually pleasing, visual indent like this is
against the [rustfmt guidelines].

### 5. Newline after `else if`

```rust
if independent_condition &&
   let Conor(x) = mcbride() &&
   let Euginia(y) = cheng(x) &&
   y.has_really_cool_property() {
    computation_with(x, y)
} else if // <-- Notice newline.
    let Stephanie(x) = weirich() &&
    let Thierry(y) = coquand() {
    computation_with(x, y)
}
```

In this version we look at whether or not a newline should be
inserted after an `else if` branch. The benefit of inserting a
newline is that it aligns well with the `let` bindings in the
`if` branch.

### 6. No indent at all, just a list of conditions

```rust
if independent_condition &&
let Alan(x)   = turing() &&
let Alonzo(y) = church(x) &&
y.has_really_cool_property() {
    computation_with(x, y)
}
```

In this version, we do not indent the `let`s and the boolean side-conditions.
But we do place the `&&` on the end of lines. One benefit here is that the
body of the `if` expression more clearly stands out. However, a drawback
is that the `if` token stands less out.

There are of course more versions one can contemplate and the various
combination of them, but in the interest of brevity, we keep to this list here.

# Appendix B
[Appendix B]: #appendix-b

This appendix groups some survey answers together for the purposes of analysis.
Please note that this appendix is by no means complete and is only offered on
a best-effort basis. The comments cited below have also been cleaned up to
fix obvious spelling mistakes, etc.

## Appendix B.1
[Appendix B.1]: #appendix-b1

Here are a number of participants in the survey commenting that they expected
the proposed syntax in this RFC to work.

1. > The "`if let .. && let .. && ..`" feels like the intuitive way to do it if you don't think about the language syntax too much. It's definitely the way I tried doing it when I thought it was possible at the start of my Rust path.

2. > I tried to write this one and then realized it's not supported.

3. > I've already tried to do this before and find it didn't work.

4. > I was surprised to find that this syntax wasn't already supported.  Principle of least surprise for the win.

5. > I would expect "Using `&&` for conjunction and `let PATTERN = EXPR`" to work already today

6. > I tried to used this specific syntax and I expected it to work already.

7. > `let …` matches the current `if let`, and the `&&` matches the way I would write it. I've tried to write `if let Some(x) = foo && x.bar() { … }` before.

## Appendix B.2
[Appendix B.2]: #appendix-b2

Many participants in the survey opined that the proposed syntax was consistent
with current Rust. They thought that this was positive.

1. > It is the *only* option consistent with what we have today and expect once we learn about `let PATTERN = EXPR`.

2. > Close to current Rust syntax

3. > Most similar to existing syntaxes, which increases orthogonality.

4. > This seems most consistent with existing Rust syntax.

5. > consistency with current Rust

6. > consistency with current syntax

7. > Consistency with current syntax

8. > It's the least surprising syntax. It's obvious.

9. > Should be consistent and similar to how match patterns/existing
   > `let A(x) = b` works.

10. > Seems to most closely match existing syntax and style

11. > Seems consistent with existing syntax

12. > Consistent with current Rust syntax.

13. > Consistency with the syntax we already have.

14. > compatibility with current syntax

15. > It's feels consistent with the rest of the language.

16. > Consistency with existing Rust constructs and familiarity with C and Swift
    > syntax.

17. > consistent with already existing `if` and `let` patterns. intuitive

18. > close to current rust syntax (same assign syntax as in `match` and `if let`)

19. > I chose "Using `&&` for conjunction and `let PATTERN = EXPR`" because it
    > seems like the only choice that is consistent with Rust syntax as it is today.
    > The rest are... strange.

20. > we already have while `let A(x) = foo()` and `&&` in `if` statements,
    > I don't see how any other syntax makes sense

21. > Using `&&` unambiguously means conjunction and is, IMO, easier to read.
    > `let PATTERN = EXPR` does not introduce a new form of pattern matching to
    > the language.

22. > The "`let(x) = expr`" is consistent with the current syntax, the "`&&`"
    > makes it clear it's an AND (and in most languages it's short-circuited).

23. > So I like that using `&&` is how we currently use it in the language,
    > and everyone is already used to using `let A(x) = foo()`.
    > Honestly, the one I chose feels the most consistent with the language.

24. > double `&` is standard for logical AND, "`if let foo(x) = bar`" is just
    > as good as the other syntax but is already standard in rust,
    > so might as well keep it

25. > using `&&` and `let PATTERN = EXPR` is more intuitive because you're
    > checking a condition *and* whether a pattern matches.

26. > follows standard "`&&`" pattern and "`if-let`" pattern as well

27. > uses existing syntax

28. > It's what I already know in Rust

29. > this is the most similar to rust's current `if let` syntax

30. > Uses already established keywords and operators in a semantically similar way.

31. > It just looks like normal rust we're all used to (`if let` destructuring
    > syntactic sugar)

32. > The most natural extension of `let` expressions and `boolean` conditions

33. > The `&&` operator and "`if let`" are already in the language.
    > No reason to pick something totally different.
    > More on that on the next page.

34. > I don't really like any of them. I prefer `;` for conjunction because
    > that's more similar to how Go does it, though `&&` for conjunction and
    > `let PATTERN = ...` is okay because it's intuitive given other language
    > features in Rust.

35. > Using `&&` for conjunction is consistent with other languages I know,
    > using `let` for pattern matching is explicit about introducing new names.

36. > Seems the most natural. If I knew `if let x = y` could be combined with
    > other conditions, my first thought would be `&& z == z1`

37. > Smallest delta from current syntax. `&&` already exists,
    > `let PATTERN = EXPR` exists, just allowing the two in composition.

38. > The option I chose (`if expr && let pat = expr && let pat = expr && expr { body }`)
    > is the most consistent with existing Rust syntax. It's a fairly natural
    > extension of the `if let` syntax since it uses `let pat = expr` in a
    > place where you could otherwise use `expr`. Using `&&` as a conjunction
    > most clearly expresses the intention IMO, and it also clearly follows
    > short-circuit evaluation.

39. > `EXPR is PATTERN` is introducing new alternative syntax which is useless
    > when we already have the `let PATTERN = EXPR` syntax. `&&` is also
    > clearly the best choice for joining conditions because that is what
    > it is already used for!

40. > "`if let`" is already a well-known thing in Rust, so keep it.
    > Conjunction is already a well-known thing in Rust, so keep it.
    > In short, make minimal changes to the language that make the example work.

41. > Using `&&` for conjunction along with the existing syntax for `let`
    > bindings is the most intuitive and feels the least like it's special-casing.
    > I think this is less likely to confuse beginners, and makes it feel more cohesive.

42. > Follows the standards of current syntax relatively closely without
    > introducing new symbols, and builds on the existing understanding of
    > `let`-deconstruction while clearing showing (through the use of `let`)
    > that we have assigned `x` and `y`.

43. > This is the syntax that I would expect without reading the manual.

44. > I feel that including `let` is important to make it clear that the
    > pattern is exposing the variables `x` and `y` for use in the block body
    > and `&&` is by far the most intuitive way to AND together test conditions.
    > In fact, presenting alternatives to Rust's existing `&&` syntax for
    > ANDing together terms made even the use of `&&` confusing because the
    > claim that they were all equivalent meant that it "couldn't possibly be"
    > the existing meaning of `&&`. I didn't know what was going on until I
    > realized I'd glossed over tiny (ie. unimportant) text which actually
    > explained the meaning in plain English... at which point, I realized that
    > the syntaxes other than `&&` had set up a mistaken assumption that ruled
    > out the actual proper interpretation.

45. > I like Using `&&` for conjunction and `let PATTERN = EXPR`  because it,
    > for me, has the least surprises syntactically. `&&` indicates conjunction
    > of the predicate, and including `x` and `y` in subsequent scopes is
    > something I wish existed, but if we're not clear about it, it could get messy. 

46. > it does not introduce anything fancy new stuff

47. > I like the "`let`" syntax better than the `EXPR is PATTERN` syntax,
    > since it's used in other places already.

48. > 1. `&&` is already a familiar concept for working with boolean expressions
    > 2. `if let` is how we already achieve conditional binding
    >
    > The combination of "`is`" and `&&` is the only other choice I could
    > consider, albeit begrudgingly. I'm kind of uncomfortable with giving
    > up keyword real estate and having another way of doing `if let`.
    >
    > Other than that, I feel like the other choices alienate both new and old
    > Rust programmers alike. We should be focusing on keeping things as simple
    > and familiar as possible.

## Appendix B.3
[Appendix B.3]: #appendix-b3

Some survey participants did not explicitly say that the RFC's proposed syntax
was *consistent*, but they did say, in some way, that it was *intuitive*.

1. > `&&` is the logical conjunction operator and `let A(x) = foo` clearly
   > destructures for pattern matching

2. > The most intuitive

3. > It is not surprising

4. > Looks like straightforward boolean logic, the rest seem like arcane syntax.

5. > Reminiscent of boolean algebra

6. > It fits with my mental model of how patterns and Boolean logic work

7. > `&&` makes the logic relationship clearer, and using `let` for binding is the same. 
   > Conjunction is more readable with `&&`

8. > We already have “`if let`” elsewhere. Don’t introduce a new “`is`” syntax
   > here, it’s not any more intuitive.

## Appendix B.4
[Appendix B.4]: #appendix-b4

Some survey participants felt that the proposed syntax set up the expectation
of `let PAT = EXPR` being an expression typed at `bool` as opposed to a
statement which is currently the case.

1. > `&&` as separator would require boolean expressions.

2. > `&&` is for boolean expressions, and won't work right in generic usage.
   > `let` would have to return a boolean which is weird and probably a breaking change.

3. > Using `&&` for conjunction with `let PATTERN = EXPR` feels consistent with
   > the existing `if let` syntax, however it causes potentially some confusion
   > about data types and its existing function as a `boolean` operator, so that
   > leads me to considering `,` as the conjunction instead.
   > However, if "`let PATTERN = EXPR`" is an expression returning a boolean as
   > well as setting up the pattern bindings then there's no issue with `&&` at
   > all, and it's then preferable to me provided it's available where you'd
   > expect expressions to be available and not treated particularly specially.

## Appendix B.5
[Appendix B.5]: #appendix-b5

Another group of people opined that `,` and `if` did not clearly imply
conjunction and that it could be construed as disjunction instead.
The majority of these comments were directed towards `,` as opposed to `if`.

1. > Commas do not feel like natural `and` separators.

2. > `,` is bad because it already means "separate things",
   > and now it suddenly means "join things".

3. > `,` does not mean and to me

4. > Comma is not `&&`.

5. > "`,`" doesn't seem like a conjunction (usually means tuple)

6. > `,` as conjunction is ambiguous (could just as well be disjunction)

7. > using a comma to mean conjunction is *very* unclear.

8. > I find the comma ambiguous (is it AND or OR?).

9. > Commas just aren't clear enough: on their own, to many people,
   > they could easily be interpreted as logical ORs or logical ANDs.

10. > Although really the only reasonable interpretation of `,` is conjunction,
    > it's still not immediately obvious that that is the case.

11. > The tower of `if`s is quite ugly (although it seems less ambiguous than using commas, which to some people might be construed as disjunction).

12. > Ambiguous, are they 'or' or 'and'?
    
    note: this refers to `,` and not `if` as a separator.

13. > Commas don't imply conjunction to me and chained ifs just feel a bit
    > unnatural too

14. > Using `,` is a bad idea because, with Rust already having a perfectly
    > good `&&`, adding `,` is likely to evoke *"OK, I know `&&`, so `.` must be OR"*
    > or *"I know `&&` and `||`, so what the heck is `,`? I'm so confused."*
    > ...not to mention that it runs against the Rust design philosophy to
    > needlessly introduce alternative syntax and I can't see any practical reason
    > it would be necessary to distinguish between tests and pattern matches in
    > this context which can't be handled by putting `let` before the matches.

15. > these syntaxes don't make it clear that there is an ‘and’ relationship between the conditions

16. > `if` after `if` with no logical operator? is this AND? is this OR?

17. > They either imply 'or' or remind me of a switch fall through in other
    > languages (and thus also 'or') 

18. > Stacking repeated uses of "`if`" at the top level feels very confusing to
    > visually scan; it doesn't distinguish a conjunction very well.

19. > `if` for conjunction is confusing

## Appendix B.6
[Appendix B.6]: #appendix-b6

Does `,` entail short-circuiting behaviour or not? Some survey participants
did not think this was clear.

1. > I would also expect the comma options to not follow short-circuit evaluation.

2. > For users coming from other languages, comma is unclear about whether
   > short-circuiting will take place.

3. > Syntax does not fit in with other usages of '`,`' in rust (especially tuples).
   > It's non-obvious what the order of execution of sub-expressions are.

4. > The commas are out of left field: they bear no relation to anything
   > currently in Rust or any other language. The conditional looks like
   > some sort of tupling expression.

5. > On the other hand the comma'd version felt the least clear in meaning
   > and execution order. I'm more used to things-separated-by-commas being
   > roughly equivalent instead of being something that ends up short
   > circuiting the evaluation.

## Appendix B.7
[Appendix B.7]: #appendix-b7

A number of survey participants noted that separating with `if` is noisy
and looks as if braces are missing.

1. > Using multiple `if`s feels very weird (it looks like there are some
   > missing braces and the indentation is wrong).

2. > Chaining `if` statements is unclear since in most languages you can leaves
   > off the curly braces for an `if` with a single statement body. 

3. > chaining "if" keywords without braces or separators doesn't convey
   > the meaning of the statement well and seems out of place in rusts
   > present syntax, even more so if contracted to a single line.

4. > Using a bunch of `if` in a column within the same `if` statement should
   > stoke uncertainty about the intended meaning in anyone who remembers that
   > Rust is very forgiving about where you put your whitespace.

5. > Too many `if`s making it noisy.

## Appendix B.8
[Appendix B.8]: #appendix-b8

A number of survey participants noted that bindings introduced in `EXPR is PAT`
were unexpectedly on the RHS while they were used to it being on the LHS.

1. > Don't like '`is`' since it puts variable binding on the right.

2. > `is` seems backwards.

3. > The '`is`' operator creates new variables, but the pattern is on the right,
   > where variables are usually read from. 

4. > `foo() is A(x)` is backwards to binding in most other places. 

5. > `expr is pat` reverses the directionality for pattern bindings seen
    > everywhere else in Rust;

6. > Very unreadable, swapped order of unpacking confusing

7. > The "`is`" formulation is backwards from current `if let`.

8. > I really do no like how the `is` syntax has the left and right sides
   > reversed from the `if let ... = ...` syntax. It seems very odd to have
   > that sort of pattern matching written in opposite directions depending
   > on the syntax you choose.

9. >  The "`is`"-destructuring/pattern matching looks really weird because
   > normally names have to be located on the left side of a statement to
   > be bound to a value. The right side is there to retrieve the value.

10. > extracting with a pattern match is confusing when the pattern match is to
    > the right of the variable being matched. it looks like a statement of fact,
    > not the introduction of a new identifier.

## Appendix B.9
[Appendix B.9]: #appendix-b9

Some survey participants opined that they found it surprising that an operator
named `is` introduces bindings. Another group found that `is` could easily
be confused for some sort of equality test (as in the operator `==`) as in
Python.

1. > using `is` to introduce new bindings is very surprising.

2. > `is` is weird because it can bind variables.

3. > The "`is`" syntax is confusing, since it does an implicit pattern binding.
   > I think folks would get it wrong by trying to pass a bound variable there
   > and being surprised to find that it's a pattern instead.

4. > I dislike the `EXPR is PATTERN` syntax because while the word `let`
   > indicates that there is some binding going on, I read the word `is` as
   > passively checking whether the expression fits a pattern without binding.
   > I also dislike `is` because it is new syntax that does the same thing as
   > existing syntax.

5. > Without `let` it isn't clear that we are declaring a new variable via `is`.
   > Now we could introduce new keywords, but `is` still isn't clear about what
   > it's doing. It seems odd to introduce `is` when `if let` does the same thing.

6. > The 'is' keyword suggests a boolean operation but silently behaves like a '`let`'.

7. > If `expr is pattern` doesn't actually bind, and just pattern matches,
   > then I like it. This should have been a language feature imo.

8. > I don't like `is` because it doesn't look like a binding operator

9. > The "`is A(x)`" syntax looked nice on first sight, but it's backwards, as
   > in this case it's an assignment (to `x` and `y`) and not just a comparison.
   > Maybe it's ok as "`if foo() is A`" (like for "`if foo().is_some()`" but
   > more generic) but not in this case.

10. > I don't like using "`is`" for assignment. It sounds like equality (`==`),
    > but with an assignment as a side effect. "`if let`" is the established way
    > of doing equality and assignment together, and I think we should stick
    > with one way of doing it. I also think "`if let`" better highlights that
    > both equality and assignment happens, even when it is nested inside the
    > expression as here. 

11. > `is` doesn't make any sense since we already have `if let PATTERN` and
    > `is` in other languages is typically a reference equality check
    > (e.g. Dart and Python).

12. > `is` operator can be confusing (is the same as `==` or something else entirely?);

13. > `is` I don't like because it looks too much like subclass testing and/or
    > identity testing from other languages. `let` I like for uniformity with
    > `if let` and `while let`, but it needs *something* to make clear that the
    > `&&` isn't part of the thing being bound; maybe parens around the whole thing?
    > Require parens around the whole RHS if there's an `&&` anywhere in there?
    > I don't know how to resolve the ambiguity... use `match`, instead?

14. > The "`is`" keyword is not in Rust yet (afaik) but if we wanted to use it,
    > we should ponder that it means "reference equality" to Python people.
    > I would thus be hesitant about using it for pattern matching expressions,
    > especially given that we already have "`let`" for pattern matching.
    > If possible, I would prefer making `let`-bindings an expression.

15. > Rust already has meanings for `&&` and `let` which can be applied here.
    > Replacing `let ...` with `... is ...` is too different from existing
    > pattern syntax and too similar to Python's identity testing operator.
