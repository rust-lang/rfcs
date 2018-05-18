- Feature Name: `#![forbid(assign_in_call)]`
- Start Date: 2018-05-19
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Introduce a forward-compatibility lint for using an assignment expression in a function call, and
make this an error in the Rust 2018 edition.

```rust
let mut a = 1;
f(a = 2); //~ ERROR: Assignment expression cannot be used as function argument
```

# Motivation
[motivation]: #motivation

[Named arguments][irlo/3831] in function call is an often requested (yet controversial) feature. One
possible invocation syntax would be `run(from=1, to=2)`, but this conflicts with the existing
syntax, since `from=1` and `to=2` are assignment expressions. Using assignment this way is very
confusing though. The function call could be better written as three separate statements.

```rust
from = 1;
to = 2;
run((), ());
```

It would be very sad to exclude this choice when considering the invocation syntax for named
arguments, since assignment expression in a function call isn't something we normally use.

With Rust 2018, we could introduce syntactical breaking change, and it is high time we reserve the
`f(a=b, c=d)` syntax the coming edition.

> Note: This RFC does not propose named arguments. It simply reserves a competitive syntax for
> consideration in case we eventually need to tackle with named arguments.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Assignment expressions can no longer be used inside a function call. The following examples will
produce forward-compatibility lints in Rust 2015, and will emit errors in Rust 2018 until they could
be repurposed for other uses.

```rust
foo(a = 1);
//  ^~~~~ error
let x = Some(b = 2);
//           ^~~~~ error
let y = (|c| c)(c = 3);
//              ^~~~~ error
```

Since assignment expressions always return `()`, you may extract the assignment into its own
statement and replace the argument with `()`

```rust
a = 1;
foo(());

b = 2;
let x = Some(());

c = 3;
let y = (|c| c)(());
```

You may also put the expression inside parenthesis so the assignment doesn't appear directly inside
a function call, but this is not a good style we'd recommend.

```rust
foo((a = 1));

let x = Some((b = 2));

let y = (|c| c)((c = 3));
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Basic rules

1. When an assignment is directly used as an argument in a function or method call, forbid.

    ```rust
    foo(a = 1);
    //  ^~~~~ error
    Some(a = 1);
    //   ^~~~~ error
    (|c| c)(a = 1);
    //      ^~~~~ error
    bar.unwrap_or(a = 1);
    //            ^~~~~ error
    cmp::max(left = 1, right = 2);
    //       ^~~~~~~~  ^~~~~~~~~ errors
    ```

2. For simplicity, all kinds of assignments are forbidden.

    ```rust
    foo(a = 1);
    //  ^~~~~ error
    foo(*b = 1);
    //  ^~~~~~ error
    foo(c.field = 1);
    //  ^~~~~~~~~~~ error
    foo(d.0 = 1);
    //  ^~~~~~~ error
    foo(e[0] = 1);
    //  ^~~~~~~~ error
    foo(***(f + g).stuff() = 1);
    //  ^~~~~~~~~~~~~~~~~~~~~~ error
    foo(concat_idents!(h, i) = 1);
    //  ^~~~~~~~~~~~~~~~~~~~~~~~ error
    foo(global::J = 1);
    //  ^~~~~~~~~~~~~ error
    ```

3. If the assignment is not directly used, allow.

    ```rust
    foo((a = 1));
    // allowed, because the assignment is wrapped inside parenthesis
    foo({ a = 1 });
    // allowed, because the assignment is wrapped inside a block
    foo(unsafe { a = 1 });
    // allowed, because the assignment is wrapped inside an unsafe block
    foo(return a = 1);
    // allowed, the call contains a return statement directly, not an assignment
    foo(if let Some(x) = y { 1 } else { 2 })
    // allowed, the `=` is part of the if-let expression, not an assignment
    ```

4. All other operators including an `=` (e.g. compound assignment) are still allowed.

    ```rust
    foo(a %= 1);
    foo(a <= 1);
    foo(a ..= 1);
    foo(a >>= 1);
    ```

5. Assignment expressions appearing elsewhere are allowed.

    ```rust
    let _ = [a = 1];
    let _ = [a = 1, a = 2];
    let _ = [a = 1; 5];
    //^ arrays
    let _ = (a = 1,);
    let _ = (a = 1, a = 2);
    //^ tuples
    let _ = || a = 1;
    //^ closures
    let _ = a = 1;
    b = a = 1;
    c += a = 1;
    //^ assignments
    let _ = d[a = 1];
    d[a = 1] = 0;
    //^ indexing
    break a = 1;
    break 'label a = 1;
    return a = 1;
    let _ = yield a = 1;
    //^ return-like expressions
    match x { _ => a = 1 };
    //^ match arm
    let _ = S { a: a = 1 };
    //^ struct literal
    ```

## Rules involving macros

6. A macro variable which evaluates to an assignment expression is allowed.

    ```rust
    macro_rules! call { ($e:expr) => { foo($e) } }
    call!(a = 1);
    // allowed, expands to `foo((a = 1))`.
    ```

7. A macro expression which evaluates to an assignment expression is allowed.

    ```rust
    macro_rules! assign { ($a:ident) => { $a = 1 } }
    foo(assign!(a));
    // allowed, expands to `foo((a = 1))`.
    ```

8. Other non-expression expansions will be forbidden.

    ```rust
    macro_rules! call_tt { ($($a:tt)*) => { foo($($a)*) } }
    call_tt!(a = 1);
    // forbidden, token streams are pasted literally

    macro_rules! subst_op { ($i:ident, $op:tt) => { foo($i $op 3) }
    subst_op!(a, =);
    // forbidden
    ```

## Implementation

A new forward-compatibility "early" lint `assign_in_call` would be introduced. This lint would be
part of the lint group `rust-2018-breakage`. The lint must be in "forbid" level when the edition is
2018.

It would check for all `Call` and `MethodCall` expressions. The lint would be emitted if any of its
arguments is an `Assign` expression.

When emitting the lint, at minimum, it should suggest adding parenthesis around it so `rustfix`
could automatically migrate the code.

If possible, it should suggest moving the assignment into its own statement, and replace the
function argument by `()`, as the fixed result would be more idiomatic.

## Termination of contract

In case any of the following happens, the lints and errors introduced by this RFC should be removed
from all editions (including 2015 and 2018), i.e. the syntax `f(a = b)` should be unreserved:

1. We decided to permanently reject named arguments, or
2. We accepted named arguments but have chosen a different syntax (e.g. `f(a: b)`).

# Drawbacks
[drawbacks]: #drawbacks

This RFC assumes named arguments will use this as the invocation syntax. If eventually we rejected
named arguments or used a different syntax, this RFC would become pointless (and could be
unreserved).

Although syntax breakage is allowed through the edition mechanism, this would still bother the end
users when they want to migrate to newer edition.

# Rationale and alternatives
[alternatives]: #alternatives

## Rationale

This RFC reserves the `f(a = b)` syntax based these observations:

1. We already used `=` for named parameter in a [format macro][std::fmt]: `println!("{a}", a = 3);`.
    Extending such syntax to named argument function is pretty natural.

2. Similarly, we already used `=` for configuration in the attribute syntax
    (e.g. `#[stable(version = "1.29.0", feature = "etc")]`).

2. As stated in the motivation, `f(a = b)` is unclear and useless which is better rewritten as
    `a = b; f(())`.

3. In clippy, the expression `f(a = b)` is already warned under the more general lint [`unit_arg`].

4. Because of these, we expect the breakage would be very small and nobody will miss it when we take
    this construct away.

## Alternative: Reserve only assignment to an identifier

If the name of named arguments can only be identifiers, it may be too aggressive to reserve for
every kinds of place expressions (lvalues). We may still allow syntax like
`foo(*c::D[e].borrow().field = 4)`, and only forbid when the LHS of the assignment is exactly an
identifier.

This RFC chooses not to further narrow the restriction because we feel that there isn't much gain
from it.

## Alternative: Make assignment a statement

Another direction would be to reserve more. For instance, we could make assignment a statement, so
that it cannot be used like `[a = 1, b = 2]` or `x = y = z`. Nevertheless, assignment expressions
are pretty common in a [`match` arm][a] and [closure body][b], so special cases are likely needed
for them.

This RFC chooses to still allow assignment expressions, as we can't think of a valid reason to
forbid them, when the cost is to introduce more special cases.

## Alternative: Reserve `f(a: b)`

There are dozens of proposed syntaxes for named arguments. Another strong contender is using `:`
i.e. `run(from: 1, to: 2)`. Some reasons are

1. Struct literals already used `:`
2. It looks nicer
3. Symmetric with the current function declaration syntax `fn f(a: u32, b: u32)`
4. No other *stable* expressions conflicts with `f(a: b)`

A serious drawback is that `:` does conflict with [type ascription][rfc803] (an unstable feature).
Unlike assignment expressions, the result of type ascription is very useful, and together with
universal `impl Trait` it will increase the chance of having type ascription in a function call:

```rust
fn print(v: impl Debug) { ... }

print(x.into(): String);
```

The only chance that named arguments can use `:` is if we decide to change the type ascription
syntax (e.g. `x.into() is String`, `x.into() -> String` etc), or we think forcing user to write
`print((x.into(): String))` is an acceptable cost.

Because of these, and also type ascription is still unstable, this RFC is not going to give any
special treatment about `f(a: b)`. Still, this reservation should be considered when we decide to
stabilize type ascription.

## Alternative: Do nothing

Besides `f(a = b)` and `f(a: b)`, there are several named argument syntaxes proposed before which
does not conflict with existing syntax.

* `run(from => 1, to => 2)` (original proposal)
* `run(from := 1, to := 2)`
* `run(from <- 1, to <- 2)` (conflicts with placement-in, though it has been unapproved)
* `run({from: 1, to: 2})`
* `run(use from: 1, to: 2)`
* `run(from ~ 1, to ~ 2)`
* `run(:from 1, :to 2)`

However we feel that having one more choice is not bad, and thus would still like to reserve
`f(a = b)` for the next edition.

# Prior art
[prior-art]: #prior-art

## Assignment expression

Rust is among the few languages which returned unit/null from an assignment expression (many of them
are ML-inspired). Most C-derived languages return the assigned value to allow chained assignment
`a = b = c`, and would accept `f(a = b)` without any warnings.

| Assignment is… | Languages |
|----------------|-----------|
| Expression, returning assigned value | C, C++, C#, CoffeeScript, D, Dart, Groovy, Java, JavaScript, Julia, Objective-C, Perl, PHP, R, Ruby, Tcl, TypeScript
| Expression, returning unit | F#, OCaml, Rust, Scala, Swift
| Statement | Ada, Fortran, Go, Kotlin, Lua, Nim, Python, Visual Basic*
| Immutable binding only | Elixir, Erlang, Haskell

(\*: In VB, `a = b` when used as statement means assignment, and when used as an expression means
equality comparison.)

## Named arguments

The primary reason of reserving `f(a = b)` is due to named argument syntax of `println!()`, which
itself is inspired by Python. Here we list [choices of the delimiter from other languages][c].

| Syntax | Languages |
|--------|-----------|
| Not supported | C, C++, D, Haskell, Java, Rust |
| `a: b` | C#, CoffeeScript, Dart, Elixir, Go*, Groovy, JavaScript*, Objective-C, Ruby, Swift, TypeScript* |
| `a = b` | F#, Fortran, Julia, Kotlin, Lua*, Nim, Python, R, Scala |
| `a => b` | Ada, Perl 6, PHP* |
| `a := b` | Visual Basic |
| `-a b` | PowerShell, Tcl |
| `:a b` | Clojure |
| `{a, b}` | Erlang* |
| `~a: b` | OCaml |

(\*: simulated via anonymous record / map / associated array / struct etc)

# Unresolved questions
[unresolved]: #unresolved-questions

* None yet.

[irlo/3831]: https://internals.rust-lang.org/t/pre-rfc-named-arguments/3831
[rfc803]: https://github.com/rust-lang/rfcs/pull/803
[std::fmt]: https://doc.rust-lang.org/std/fmt/index.html#named-parameters
[`unit_arg`]: https://rust-lang-nursery.github.io/rust-clippy/current/index.html#unit_arg
[a]: https://sourcegraph.com/search?q=repogroup:crates+%3D%3E%5Cs*%5Cw%2B%5Cs*%3D%5B%5E%3D%5D
[b]: https://sourcegraph.com/search?q=repogroup:crates+file:%5C.rs%24+%5C%7C%5C%7C%5Cs*%5Cw%2B%5Cs*%3D%5B%5E%3D%3E%5D
[c]: https://rosettacode.org/wiki/Named_parameters
