- Feature Name: let_expression
- Start Date: 2021-08-08
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Convert `let <pat> = <expr>` from a statement to an expression of type `bool`. Make
the language more consistent and remove corners by generalizing, plus adding
a bunch of magic constructs that are useful. After this RFC, you'll be able to write, among many other things:

```rust
// generalized matches! macro:
assert!(let Some(x) = y && x > 2);

// || counterpart of if-let-chain 

if let Some(x) = foo || let Some(x) = bar {
    println!("{}", x);
}

// generalized let-else construct
let Some(a) = b
&& let Some(c) = f(&a)
|| return Err("failed");
println!("{}, {}", a, c);

// generalized assignment with default
let Some(x) = y
|| let Foo(x) = bar
|| let x = default;
println!("{}", x);
```

# Motivation
[motivation]: #motivation

The main motivation for this RFC is improving consistency and ergonomics of language.

## Consistency

Currently we have `if`, `if let` and `if let && let` and we teach them as three different
constructs (plus `let else` in future). But if we make `let` a `bool` expression, all will become the same and it would be
easier for new learners to get it. After this RFC you get only an unused parenthesis warning
for `if (let Some(x) = y) { }`, not a hard error. And it will have the same behavior with
classic `if let`. This is actually [a mistake by a new learner](https://github.com/rust-lang/rust/issues/82827)
that show us new learners expect it.

This situation is worse with `if let chain` that mix let expressions with `&&` and
other bools. In fact the compiler will understand it via interpreting let as an
expression, so why we force humans to understand it another way?

Also, this RFC support use case of a new feature, approved but not stablized, called let-else. let-else is not
consistent with if-let-chains and it has taken a completely different path. This RFC can replace let-else
and so remove this inconsistency, without loss in expressive power.

This proposal is also in-line with "everything is an expression" that we have
in rust.

## Ergonomics

*This RFC extensively use short circuit operators `&&`, `||`. This operators are called
`andalso`, `orelse` in standard ML and some other languages, reading them in this way
instead of traditional `and`, `or` remind you the short circuit nature of them and help
understanding them better.*

It also available many super powers for us that can
help decreasing rightward drift without adding to implementation and understanding complexity, and
actually decreasing it by removing `let-else` and preventing from future similar constructs.

### New constructs

```rust
// reuse if let body
if let Some(x) = a || let Some(x) = b || Ok(x) = c {
    // body with many lines of code
} else {
    // else with many lines of code
}

// today alternative without code duplication:
if let ((Some(x), _, _) | (_, Some(x), _) | (_, _, Ok(x)) = (a, b, c) {

// assignment with default
(let Foo(x) = a)
|| (let Bar(x) = b)
|| (let x = default);

// today alternative:
let x = if let Foo(x) = a {
    x
} else if let Bar(x) = b {
    x
} else {
    default
};

// simple let expression
assert!((let Some(x) = a) && (let Some(y) = b(x)) && x == y);

// today alternative with if let chains:
assert!(matches!(a, Some(x) if let Some(y) = b(x) && x == y));
assert!(if let Some(x) = a && let Some(y) = b(x) && x == y { true } else { false });
```

### Practical usage of this features

People find let expressions theoretical at first look, and think only traditional cases (if-let-chain and let-else)
can become useful. In this section there are some usages for new features of this RFC in real codes.

This is an example from rust-clippy repository:
```rust
for w in block.stmts.windows(2) {
    if_chain! {
        if let StmtKind::Semi(first) = w[0].kind;
        if let StmtKind::Semi(second) = w[1].kind;
        if !differing_macro_contexts(first.span, second.span);
        if let ExprKind::Assign(lhs0, rhs0, _) = first.kind;
        if let ExprKind::Assign(lhs1, rhs1, _) = second.kind;
        if eq_expr_value(cx, lhs0, rhs1);
        if eq_expr_value(cx, lhs1, rhs0);
        then {
            // 30 lines of code with massive rightward drift
        }
    }
}
```

Which by a generalized let-else can become:
```rust
for w in block.stmts.windows(2) {
    let StmtKind::Semi(first) = w[0].kind
    && let StmtKind::Semi(second) = w[1].kind
    && !differing_macro_contexts(first.span, second.span)
    && let ExprKind::Assign(lhs0, rhs0, _) = first.kind
    && let ExprKind::Assign(lhs1, rhs1, _) = second.kind
    && eq_expr_value(cx, lhs0, rhs1)
    && eq_expr_value(cx, lhs1, rhs0)
    || continue;
    // 30 lines of code with two less tab
}
```
Every `if let` or `if_chain!` that fill body of a loop or function can refactored in this way. You can easily find
dozens of them just in rust-clippy. Note that if-let-chain alone can't solve this problem, because you have access to
variables of if-let inside of block, but you have access to variables of a binding statement under it, thus you can
save one indentation.

This pattern which we can call let-chain-else, can also be obtained with the simple let-else:
```rust
let StmtKind::Semi(first) = w[0].kind else { continue; }
let StmtKind::Semi(second) = w[1].kind else { continue; }
if differing_macro_contexts(first.span, second.span) { continue; }
let ExprKind::Assign(lhs0, rhs0, _) = first.kind else { continue; }
let ExprKind::Assign(lhs1, rhs1, _) = second.kind else { continue; }
if !eq_expr_value(cx, lhs0, rhs1) { continue; }
if !eq_expr_value(cx, lhs1, rhs0) { continue; }
```
And let-else with flavor of this RFC:
```rust
let StmtKind::Semi(first) = w[0].kind || continue;
let StmtKind::Semi(second) = w[1].kind || continue;
!differing_macro_contexts(first.span, second.span) || continue;
let ExprKind::Assign(lhs0, rhs0, _) = first.kind || continue;
let ExprKind::Assign(lhs1, rhs1, _) = second.kind || continue;
eq_expr_value(cx, lhs0, rhs1) || continue;
eq_expr_value(cx, lhs1, rhs0) || continue;
```
*Do you see boolean algebra here?* This RFC enables reusing code in let-else with equal else block, similar
to merging ifs with equal body or else body via logic operators. This can be specially more useful when
there is something more complex than `continue` like:
```rust
{
    do_something1();
    do_something2();
    continue;
}
```
You should copy paste it or make it a function (without continue) in let-else example, but let-chain-else has no problem.

`||` is not only useful for let-else style things. This is a real example from [sentry-cli](https://github.com/getsentry/sentry-cli/):

```rust
if let Ok(val) = env::var("SENTRY_DSN") {
    Ok(val.parse()?)
} else if let Some(val) = self.ini.get_from(Some("auth"), "dsn") {
    Ok(val.parse()?)
} else {
    bail!("No DSN provided");
}
```
Which contains duplicate code `Ok(val.parse()?)`. With this RFC we can write:
```rust
if let Ok(val) = env::var("SENTRY_DSN") || let Some(val) = self.ini.get_from(Some("auth"), "dsn") {
    Ok(val.parse()?)
} else {
    bail!("No DSN provided");
}
```

Originaly, this code from deno was the example for if-let-or-chain:

```rust
let nread = if let Some(s) = resource.downcast_rc::<ChildStdoutResource>() {
    s.read(buf).await?
} else if let Some(s) = resource.downcast_rc::<ChildStderrResource>() {
    s.read(buf).await?
} else if let Some(s) = resource.downcast_rc::<TcpStreamResource>() {
    s.read(buf).await?
} else if let Some(s) = resource.downcast_rc::<TlsStreamResource>() {
    s.read(buf).await?
} else if let Some(s) = resource.downcast_rc::<UnixStreamResource>() {
    s.read(buf).await?
} else if let Some(s) = resource.downcast_rc::<StdFileResource>() {
    s.read(buf).await?
} else {
    return Err(not_supported());
};
```
Which by `||` counterpart of if-let-chain can become:
```rust
let nread = if let Some(s) = resource.downcast_rc::<ChildStdoutResource>()
    || let Some(s) = resource.downcast_rc::<ChildStderrResource>()
    || let Some(s) = resource.downcast_rc::<TcpStreamResource>()
    || let Some(s) = resource.downcast_rc::<TlsStreamResource>()
    || let Some(s) = resource.downcast_rc::<UnixStreamResource>()
    || let Some(s) = resource.downcast_rc::<StdFileResource>() {
    s.read(buf).await?
} else {
    return Err(not_supported());
};
```
Unfortunately, it doesn't compile because types of `s` are not equal. Downcasting in this way is a
popular pattern, and in some cases anonating some `dyn` type can solve the problem. Anyway, if-let-or-chain
with even equal types has many usecases.

`||` is also useful for assignment with default, specially when `unwrap_or_else` isn't available:
```rust
let size = if let Some(Size(size)) = $v.size { size } else { expand_size };
```
Can become:
```rust
let Some(Size(size)) = $v.size || let size = expand_size;
```
Which is smaller and can better show the intent of operation.

A different class of practical usages of this RFC is let expression usage as a bool. People
wrap their let expressions with `if expr { true } else { false }` manually. This need is almost met
with `matches!` macro, but `if let true else false` is still a thing in rust code bases. Again from rust-clippy:
```rust
fn is_repeat_zero(&self, expr: &Expr<'_>) -> bool {
    if_chain! {
        if let ExprKind::Call(fn_expr, [repeat_arg]) = expr.kind;
        if is_expr_path_def_path(self.cx, fn_expr, &paths::ITER_REPEAT);
        if let ExprKind::Lit(ref lit) = repeat_arg.kind;
        if let LitKind::Int(0, _) = lit.node;

        then {
            true
        } else {
            false
        }
    }
}
```
After this RFC, we can write this:
```rust
fn is_repeat_zero(&self, expr: &Expr<'_>) -> bool {
    let ExprKind::Call(fn_expr, [repeat_arg]) = expr.kind
    && is_expr_path_def_path(self.cx, fn_expr, &paths::ITER_REPEAT)
    && let ExprKind::Lit(ref lit) = repeat_arg.kind
    && matches!(lit.node, LitKind::Int(0, _)) // you can use let expression here as well
}
```
Some people may argue that current state is more readable, but [this lint](https://rust-lang.github.io/rust-clippy/master/index.html#needless_bool)
is not agree with them. A more complex example in this category from sentry-cli:
```rust
if let Ok(var) = env::var("SENTRY_DISABLE_UPDATE_CHECK") {
    &var == "1" || &var == "true"
} else if let Some(val) = self.ini.get_from(Some("update"), "disable_check") {
    val == "true"
} else {
    false
}
```
Which can become:
```rust
{ let Ok(var) = env::var("SENTRY_DISABLE_UPDATE_CHECK") && (&var == "1" || &var == "true") }
|| { let Some(val) = self.ini.get_from(Some("update"), "disable_check") && val == "true" }
```
this doesn't redefine logic operators. `if x { y } else { false }` is definition of `x && y`.

## Why now?
This RFC exists because of the `if let` syntax we know today.
That syntax wasn't the only choice and there was other options like `iflet`, `if match`, `let if`, `if is` or another `keyword`. 
If one of those had been picked this RFC would not have been necessary or would have been very different.
Similary, If the `,` symbol is used instead of `&&` for the if-let-chain RFC, this RFC would not be necessary since it would not be compatible.

But luck is not always with us. We can't expect each new RFC to add another piece of the let expression puzzle to
the language. For example, `matches!` and `let-else` are potentially not compatible with
this RFC, `matches!` can [coexist with let expression][future-of-matches] but let-else is not compatible with
let expression and therefore can't coexist. Some people have felt that `let-else` is not compatible with `if let chain`, and this is one of the unresolved questions in that RFC. The answer to this question is: they are not compatible! This RFC
has more compatiblity with if-let-chain and less additions to the language grammar.

The goal of proposing this now is to prevent `let-else` and future similar RFCs to be stabilized. Originally, authors
of the if-let-chain RFC had an incremental plan toward let expression.
The implementation of the first part took a long time and is still not done. Since the if-let-chain RFC is
still not completed we didn't see next steps toward let expressions. So RFCs like `let-else` came to solve problems in their own way, without
compatibility with let expressions and therefore, if-let-chains.

Even if it doesn't fit in this year road-map, we should decide if we want it or not today. Even today
is too late!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

*Note: Examples in this section are here for showing the corner cases of let expression, not code encouraged or intended to use in real codebases.*


This section examines the features proposed by this RFC.

## `let` as a bool expression

The `let <pattern> = <expr>` returns a bool expression that returns `true` when `<expr>` matches the `<pattern>`
and otherwise will be evaluated to `false`. For example we have this:

```rust
let bar = Some(4);
assert!(let Some(x) = bar);

let foo = 'f';
assert!(let 'A'..='Z' | 'a'..='z' = foo);
```

## Binding statements

Every `let` expression have some (maybe zero) free variable in it's pattern that
we call them bindings of a let expression. If a bool expression
comes with a `;` (as a statement) and compiler can prove it is always `true` (for simple
let expressions it means pattern is irrefutable) it will bind all bindings to the local scope
after `;` and init them with result of pattern matching. So we have this:
```rust
let a = 2;
let Point { x, y, z } = p;
// we have a, x, y, z here
```

## Combining with `||`
If we combine two let expressions via `||`, their bindings should be equal, otherwise
we will get a compile error. Bindings of result expression is equal to bindings of it's operands. So
from previous part we have:

```rust
let Some(x) = foo || let x = default;
// we have x here
```
How it will be run? We will reach first line, then:
* If foo matches Some(x), we fill `x` based of foo, `let Some(x) = foo` will be evaluated to true, and short circuit the `||` so go to next line.
* Otherwise we will go to next operand, assign default to x, evaluate `let x = default` to true and go to next line.

Why their bindings should be equal? Because from knowing that the expression is true, we
know one side of the `||` is true, but we don't know which side is true. If their
bindings is equal (name-vise and type-vise) we can sure that they can be filled in
run-time, either from first operand or second operand. So they must be equal.

This limit isn't new. We already have it in `|` pattern bindings. Today, `let (Some(y) | None) = x;` doesn't compile
with error `variable y is not bound in all patterns`. And in let expression equivalent form `(let Some(y) = x) || (let None = x);` we
will get a similar error `variable y is not bound in all cases`.

In addition to `true`, binding statements are allowed to diverge (have type of `!`) so
we can write this:

```rust
let Some(x) = foo || panic!("foo is none");
println("{}", x);
```

But what about rule of equal bindings? What is binding set of `panic!("foo is none");`? As `!` can cast
to all types, their bindings can cast to any set of bindings and wouldn't make an error. This make
sense because we don't care about after a return or a panic.

## Combining with `&&`
If we combine two let expressions via `&&`, bindings of whole expression would be the
merged set of both bindings. So we will have:
```
let Point { x, y, z } = p && let a = 2;
// we have a, x, y, z here
```
These are useless alone (equal to separating with `;`) but can become useful inside if scrutinee (which we don't know yet) or with `||`:
```
let Some(x) = foo && let Some(y) = bar(x)
|| let (x, y) = (default_x, default_y);
```
Also, in `EXP1 && EXP2` you can use and shadow bindings of `EXP1` inside `EXP2`. This
is because if we are in `EXP2` we can be sure that `EXP1` was true because
otherwise `&&` would be short circuited and `EXP2` won't run. Example:

```rust
let foo = Some(2);
let shadow = 5;
(let Some(x) = foo || panic!("paniiiiiiic"))
&& let x = shadow;
println!("{}, {}", x); // 5
let a = Some(y);
println!("{}", (let Some(b) = a) && b > 3); // true
```

And you can mix this with normal bool expressions. They have no binding but act like
any other bool expression. So we can have this:
```rust
is_good(x)
&& (let y = Some(x))
|| (let y = None);
// we have y: Option<type of x> here
```

## Consuming bool expressions outside of bool operators
If we consume a bool expression in anything other than bool operators (such as
function calls or match expressions) it would lose its bindings.
```rust
let bar = Some(Foo(4));
assert!(let Some(x) = bar && let Foo(y) = x && y > 2);
// no x and y here
```

Specially, `{}` expressions will consume bools and lose its bindings. This behavior is
consistent with our expectation from `{}` that have bindings only local to itself. So for example:
```rust
assert!(let Some(x) = foo && x.is_bar() || baz == 2);
```
Doesn't compile because of different bindings in `||` (`baz == 2` has no binding but `let Some(x) = foo && x.is_bar()` has `x`) but
```rust
assert!({ let Some(x) = foo && x.is_bar() } || baz == 2);
```
will compile, because `{}` would discard all of bindings. With `()` instead of `{}` we will get same error
of first example.

## Bool operators `!`, `^`, `&`, `|`, `==`, `=` and others

This RFC reserve usage of bool operators for binding expressions. Originaly, binding rules for `!` operator
was in this RFC. This reservation means expressions like `!x` or `x^y` when `x`, `y` have some binding variables, like
`!(let Some(x) = foo || let Some(x) = bar)` will be rejected by compiler. If you just need bool value of these
expressions and don't expect some bound variable, you can discard binding of expressions with `{}` and use
them like normal bool expressions: `!{ let Some(x) = foo || let Some(x) = bar }`.

Specially, assignment is an operator so in something like this:

```rust
let is_foo = { let Some(x) = opt && foo(x) };
```
The `{}` are mandatory. Unlike `!` and `^` motivation for `=` isn't reserving for future possiblities, but
for making it consistent with other operators, and more importantly make the fact that assignment
will discard bindings visually clear and reduce confusion.

## `if` and `while`

From definition of `if` we know:
```rust
if b {
    // if we are here b is true
} else {
    // if we are here b is false
}
```

So compiler can (and will) allow us to access bindings inside of then block of if, or body block of while.

## Anything else?

No. Hurrah, you just learned all of `if let` and `while let` and `if let chain` and (an alternative syntax for) `let-else` without
one line of sugar and `match` and inconsistency and special case.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Many parts of this proposal (such as grammar changes) are already implemented for
the `if-let-chains` RFC.

Previously, `if let` and `if let chains` implementations was via desugaring to match expression. This
is useful because it doesn't create new rules for borrow checker and scoping. We can do the same
with this proposal and do just some desugaring, as explained below. In addition to desugaring, we
need to implement rules of bindings in the compiler as explained in guide-level explanation. Also
it has some problems that we explain later.

## Desugar rules
`&&` in top level of if scrutinee:
```rust
if a && b {
    EXPR_IF
} else {
    EXPR_ELSE
}
```
would become:
```rust
if a {
    if b {
        EXPR_IF
    } else {
        EXPR_ELSE
    }
} else {
    EXPR_ELSE
}
```
and `||` become:
```rust
if a {
    EXPR_IF
} else {
    if b {
        EXPR_IF
    } else {
        EXPR_ELSE
    }
}
```
and `!`:
```rust
if !a {
    EXPR_IF
} else {
    EXPR_ELSE
}
```
become:
```rust
if a {
    EXPR_ELSE
} else {
    EXPR_IF
}
```
We will follow this desugar rules until we reach atomic `if let` and `if bool` and
desugar them to `match` expressions as we do it today.

While has a syntax sugar from `if let chain` proposal which desugars:
```
while condition {
    EXPR_WHILE
}
```
into:
```
loop {
    if condition {
        { EXPR_WHILE }
        continue;
    }
    break;
}
```


Consumed let statements in function calls or other places will change from `b` into:
```rust
if b { true } else { false }
```

Binding statements that contains just a simple `let` work today.
For desugaring complex binding statements we need to compute bindings of the statement, then
we can convert it to:
```rust
let (B1, B2, B3, ...) = if BINDING_STMT {
    (B1, B2, B3, ...)
} else {
    unreachable!()
    //compiler should prove this or return a compile error.
}
```

## Prove binding statement is always `true`
We can outsource this to the smartness of compiler. If a human use a complex binding statement
that believes it is ok (and it isn't a wrong assumption) there is no point in rejecting that. This
is contrary to binding rules in which we use strong static rules. If we rely on smartness of compiler,
it can allow us:
```rust
if let Some(x) = foo { } else { return; }
// compiler can figure out that accessing x here is ok
// but we don't allow this because it is unclear for humans
// and can create problem in combination with shadowing
// so changing it can be breaking
println!("{}", x); // compile error!
```
This is harmful and we don't allow this.

But for checking binding statements, even surprising ones like:
```rust
if let None = foo {
    return;
}
let Some(x) = foo;
// use x here
```
has no harm. People can try if compiler is smart enough to understand their code, and if it isn't
and they are sure that their binding statement is always true, they can add a `|| unreachable!()` at the end
manually.

For start, we can allow trivial cases, e.g. `let <irrefutable> = expr`, `divergent`, `true && true`,
`x || true`, ... . In next steps things like `(let Foo(x) = y) || (let Bar(x) = y)` can be allowed. And
allowing something like above example seems infeasible in near future.

## Rules of bindings
What would be happen if we don't check those rules? For example look at desugar of `||`:
```rust
if a {
    EXPR_IF
} else {
    if b {
        EXPR_IF
    } else {
        EXPR_ELSE
    }
}
```
When this will compiles? It will compiles when bindings of `EXPR_if` is subset of bindings of both `a` and `b` so
a generalized and natural rule for bindings of `||` would be intersection of bindings in both sides. This
doesn't need any more check for bindings. But
this can confuse humans, especially in combination with shadowing:
```rust
let x = 2;
if (let Some(x) = foo) || is_happy(bar) {
    // x is 2, even if foo is Some(_)
}
```
If people really need this behavior and doesn't made it by mistake, they can do:
```rust
let x = 2;
if { let Some(x) = foo } || is_happy(bar) {
    // x is 2, even if foo is Some(_)
}
```
which explicitly shows that `x` is local to that block. 

This extra limit is also consistent with other parts of the language. We could take a similar approach
in `|` pattern and silently don't bind `y` in a pattern like `Some(y) | None` so there wouldn't be
an error until `y` used. But people decided against this (with good reason) and this RFC follow them
in this decision.

## Precedence of `||` and `&&` operator

Currently, precedence of assignment operator `=` is lower than `||` and `&&`, so `let pat = x || y` parse
as `let pat = (x || y)` so parenthesis around `(let pat = x)` is mandatory. Changing this precedence break existing codes
so this RFC doesn't change it in all places.

`let pat = (x || y)` is only useful when type of `x` and `y` and `pat` are `bool`, because `||`, `&&` can not
be overloaded. detecting type of `x` is not easy in parse level, but few patterns can possibly get bool variables:
* Idents and wildcards
* Bool literals
* `pat1 | pat2`, `ident @ pat`, `& pat`, `(pat)` when inner patterns are bool-possible
* Constants, which are equivalent to idents in parser level.

If pattern of a let expression isn't bool-possible, parser looks for an expression with precedence
higher than `&&`. It will solve the need for `()` in majority of cases, but not always:
* Constants and enum variants like `let None = opt || x` will need a parenthesis, but they can written via `==` in most of cases.
* Idents, they can be in terminals, but in the middle of expression they need `()`. It shouldn't be popular because `let i = x` is always true.

It isn't the only corner case of `||`, `&&` and let expressions. Another corner case which is added in
if-let-chain RFC, is that in context of if scrutinee, precedence of this operators is reversed.

## Divergent case

It should be noted that divergent expressions are specially handled. If they happen in top-level
of if scrutinee, body of if is unreachable and we discard it. For example this:
```rust
(let Some(x) = foo) || panic!("foo is none");
println!("{}", x);
```
would become normally to:
```rust
let x = if let Some(x) = foo {
    x
} else {
    if panic!("foo is none") {
        x
    } else {
        provably_unreachable!();
    }
};
println!("{}", x);
```
which doesn't compile (because second x isn't declared). But desugar procedure can remove second if safely:
```rust
let x = if let Some(x) = foo {
    x
} else {
    panic!("foo is none")
};
println!("{}", x);
```
If we don't do this, binding set of divergent expressions would become empty set like other bools. But
it limit use-cases of let expression and we need them to be able to cast to every possible
set. So we handle divergent case in this way.

## Code duplication
As you see, code is duplicated in desugaring, and this can be exponential. This is unacceptable
in compiler. `if let chain` RFC prevent this problem with desugaring this
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

into:

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

We can't use this as-is. Because it lose else part so it can't apply recursively. But we maybe
able to do something like this (for example for `||`):

```rust
if a {
    'here: EXPR_IF
} else {
    if b {
        goto 'here;        
    } else {
        EXPR_ELSE
    }
}
```

This is not valid rust syntax so we can't call it desugaring. but if we check that context
in those positions are equal (rules of bindings) we can do that jump safely.

## Implementing without sugar
Implementors are free to implement it another way, for example implement let expressions directly.
They should take desugaring behavior (the one with code duplicating) as a reference and
implement the same behavior in a desired way.

# Drawbacks
[drawbacks]: #drawbacks

## Big change in language

This RFC is big and the language specification
is possibly made more complex by it. While this complexity will be used by some
and therefore, the RFC argues, motivates the added complexity, it will not be
used all users of the language. However,
by unifying constructs in the language conceptually,
we may also say that complexity is *reduced*. Specially when we think about
macros and RFCs that this RFC will prevent. Macros and special constructs are
simple patterns with this RFC.

## Let expression type isn't bool in other languages

`let` as a bool expression can become surprising for people coming from other languages. People
see `let` equal to their variable declaring statements, and let expression with `bool` type are
very different from them.

In C family and Java, `int x = y` is equivalent of `let` in rust which isn't
expression, but `x = y` is a expression equal to `{ x = y; x }` in rust. So, people from those
language may expect `let x = y` returns `y` instead of `true`. Using these expressions is considered
an anti-pattern and therefore the rust does not have them. Due to its similarity to let expressions, it
may be thought that let expressions are also a anti-pattern, but they are different conceptually. In
python `x = y` also does declaration, and in JS `let`, `const` and `var` are in place of `int` in declaration, and
this assignment behaviour is wellknown and exists in many of imperative languages.

In functional languages like Haskell and OCaml, there is `let x = y in f(x)` expression which returns `f(x)`. This is closer
to what we have in rust, it does irrefutable pattern matching, and `let a || let b in c` can be considered as a valid
extension to their syntax. But this isn't a thing in current state of those languages and may look strange a bit at first.

There are many things that rust is unique in them, specially `if let`, which is the root of let expressions with `bool` type.
Let expression replaces if-let and if-let-chain in list of things that rust is unique in them.

## Hard to read let expressions

Aggressive use of let expressions can lead to complex and hard to read results:
```rust
(
    (
        let Some(x) = a
        && let Some(y) = x.transform()
        || panic!("failed to get y")
    )
    && (
        let Some(a) = y.transform1()
        || let Ok(a) = y.transform2()
        || let Some(a) = if let either = y.transform3() && let Either::Left(left) = either {
            Some(transform_left(left))
        } else {
            None
        }
    )
)
|| panic("fun just ended!");
```
it can be written on one line, but hopefully rustfmt will prevent that. Also rules of bindings will prevent
people to write arbitary let expressions. For example:
```rust
let Some(a) = y.transform1()
|| ((let result = y.transform2()) && (let Ok(a) = result || return result));
println!("{}", a);
```
won't compile because binding set of `let Some(a) = y.transform1()` doesn't contain `result`. This rule also
make it possible to find which variables will bound with a quick look, that is, every binding variable that
appear in a top-level let (not let expressions inside blocks or function calls) will be in the binding set of
final expression. so first example will bound `x`, `y` and `a` and we will get it with a quick look.

This problem is not limited to let expressions and all powerful structures have it. In
particular, let expressions correspond to patterns: `let a = b && let c = d` is roughly
equivalent to `let (a, c) = (b, d)` and `let a = b || let c = d` is roughly equivalent to `let ((a, _) | (_, c)) = (b, d)` so
every complex let expression has a dual complex expression with patterns (with different behaviour and capabilities), example of a complex
pattern matching:
```rust
let ((Foo(x), Some(y), (Some(z), _, _) | (_, Ok(z), _) | (_, _, Some(z)))
    | (Bar(z), x @ None, (Some(y), _, _) | (_, Err(y), _) | (_, _, Some(y)))) =
        (a, b, (c.transform1(), c.transform2(), c.transform3()));
```
This shows that same complexity is possible in patterns, and in both cases the complexity can be scaled to infinity.
Since this complexity in the patterns did not cause a serious problem, we can hope that
it does not cause a problem in let expressions either.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## The `is` operator
Some people argue that `let` doesn't read well as an expression. So we introduce
an operator `<expr> is <pat>` equal to `let <pat> = <expr>` expression that explained
in this RFC. This may read better in context of `if` but it has some problems:
* It is a new construct
* It is an infix operator, those can become hard in parsing
* It will duplicate some part of languages. After implementing it, we will
  deprecate `if let`? It make a huge gap between old and new rust code.
* If we put it in this RFC as-is, we should accept `2 is x;` as a declaration
  for `x` which isn't familiar for programmers and also duplicates let and has problem above.

## Macros
We can detect popular patterns that this RFC makes possible and create special macros
for them. `matches!`, `if_chain!` and `guard!` macro are today examples of this and
we can add more later.

But macros are complex and everyone should learn every of them separately. A consistent
language feature that make couple of macros unnecessary is better. Also, `let-else` and
similar proposals shows that macros aren't enough.

### Future of matches
[future-of-matches]: #future-of-matches

Note that this RFC is not intended to deprecate `matches!`. `matches!` and let expressions can co-exist
together like `match` and `if let` because each has its own application. Specially for patterns that
doesn't have bindings, matches macro is superior and even a linter can suggest changing things like
`let 'a'..'z' = foo` to `matches!(foo, 'a'..'z')`. But when there are bindings, let expressions are
better, for example `let Some(x) = foo && let Some(y) = parse(x) && is_good(y)` is more clear than
`matches!(foo, Some(x) if let Some(y) = parse(x) && is_good(y))`.

## let-else RFC
A large part of the this RFC interferes with the let-else RFC, and in fact one of the purposes of this is
to replace the let-else. Although let-else do its job well, the expressive power of let expressions is much greater
and they are more consistent with the rest of the language (especially if-let-chain). If we need language
changes for this feature, why make a change just for this particular application? With let expression, this
RFC and similar RFCs in the future won't be happen and their task will be taken with this consistent syntax.


### Compare expressive power of this RFC with let-else

```rust
// simple let else
let Some(x) = y else {
    return Err("fail");
};

// with let expression
let Some(x) = y || {
    return Err("fail");
};

// or even better
let Some(x) = y || return Err("fail");

// let else else future possiblity
let Some(x) = a else b else c else { return; };

// with let expression
let Some(x) = a
|| let Some(x) = b
|| let Foo(x) = bar
|| return;

// duplicate else block of consecutive let-else
let Some(Foo(x)) = bar else {
    panic!("a very long message which needs to change every day");
};
let Some((y, z)) = baz(x) else {
    panic!("a very long message which needs to change every day");
};

// with let expression
let Some(Foo(x)) = bar
&& let Some((y, z)) = baz(x)
|| panic!("a very long message which needs to change every day");
```

### `else` vs `||`

Some people argue that `else` is a better choice and `||` doesn't read very well. But in fact using
short circuit operators in this way is a wellknown pattern in general, and it is popular in bash scripting. In
standard ML, short circuit operators are called `OrElse` and `AndAlso` which shows that this similarity is known
and `else` in let-else is more like `OrElse` rather than `else` in if-else, so `||` is a good choice.

As a benefit for `||` over `else`, `||` is already in the language and working for
normal bools ([playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=13b6c01bca742a083b0f48fee07b21d3)).
but `is_prime(x) else { continue };` isn't and won't be valid syntax in rust. So this RFC need much less changes
in the language.

For consistency with bool expressions, we should allow something like `let Some(x) = y || panic!("fail")`. For
normal bools it is considered a bad practice, (though [not everyone is agree](https://github.com/rust-lang/rust/issues/69466#issuecomment-591097014)
with it) and explicit `if` is preferred. In let expressions, explicit `if` is not an option due to difference in bindings,
and demand for such construct is real (The presence of let-else is a sign of this), so why we should consider a
pattern that exists in current rust and many other languages a bad practice, and then invent a new construct
that do exactly the same work and read almost the same (let-else vs let-orelse) and consider it a good practice?

## A subset of this RFC
Not all things introduced here are useful and some of them are because consistency and completeness. We can make
some subsets of this RFC hard errors and make them future possiblity. Subsets that are candidate of removing
are:
* ~~The `!` operator:~~ It has been removed and it is now just a future possiblity.
* Consuming let expressions outside of if and binding statements:
Some people argue that let expressions in arbitary places can be confusing because the scope of bindings in not clear,
and they are useless for common cases (simple matching) in presence of `matches!` macro.
"Consuming expressions outside of if scrutinee and toplevel of block will discard bindings" is a easy to
remember rule, but somehow it isn't visually clear. By rejecting those with a hard error, this concern will be
solved but we will lose the mental model of "every let expression is a simple bool expression". We can mandate
a `{}` block around consumed let expressions to make scope visually clear but this is a surprising restriction that
exists currently nowhere in the language. As another claim against this alternative, situations that compiler
doesn't catch with a undefined variable error or a unused variable warning are extremely rare, so compiler will
teach developers binding rules, which are easy to learn.

# Prior art
[prior-art]: #prior-art

There is a great discussion around this topic in this RFCs and their comments:
* [RFC 2497 (if let chain)](https://github.com/rust-lang/rfcs/blob/master/text/2497-if-let-chains.md) and [comments](https://github.com/rust-lang/rfcs/blob/master/text/2497-if-let-chains.md)
* [Comments of let else 2015 RFC](https://github.com/rust-lang/rfcs/pull/1303) (it was at first `if !let`) [and related issue](https://github.com/rust-lang/rfcs/issues/2616)
* [RFC 160 (if let) and comments](https://github.com/rust-lang/rfcs/pull/160)

In other languages, there are `is operator` somehow similar to let expression proposed here:
* [Kotlin](https://kotlinlang.org/docs/typecasts.html)
* [C#](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/operators/is)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

To be determined.

# Future possibilities
[future-possibilities]: #future-possibilities

## Binding rules for `!` and other operators
`!` operator was originaly part of this RFC, but because its binding rules was confusing and it wasn't useful in
practical codes, so doesn't pay for its costs, has been removed.

One possible idea for binding rules of `!` operator is this:
If we negate a bool expression, all of its normal bindings (which we now call positive binding or PB) become NB (Negative binding) and
its NBs become PB. NBs behavior in `&&` and `||` is reversed. In `&&` they should
be equal and in `||` they will be merged. But it is not the only possible idea.

Similar ideas exist for `^` and other boolean operators. And for operators like `=` need for
unneccesary `{}` can be lifted in a future RFC.

## Convert assignment to a bool expression
In [RFC 2909](https://github.com/rust-lang/rfcs/blob/master/text/2909-destructuring-assignment.md) we
allow destructing on assignments. A future RFC can make them a bool expression which returns true if
pattern matched.

