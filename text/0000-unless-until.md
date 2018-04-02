- Feature Name: unless_until
- Start Date: 2018-04-02
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add `unless` and `until` as reserved keywords to the Rust language, with the
option to fully implement them.

These keywords are complementary to the `if` and `while` keywords, and can be
used in any context where they are permitted (standalone and as `_ let`).

# Motivation
[motivation]: #motivation

Complementary logic tests are a common operation in the decision-making control
constructs we use in programming. There are numerous occasions where a specific
condition is tested, but may not itself be desirable to handle. Consider a
condition test used in an `if` tree with an empty or small body, while the
`else` body is significantly larger or more interesting to the application
logic.

The common solution to this case is to invert the condition: instead of testing
equality, test inequality; instead of testing set inclusion, test set exclusion;
etcetera.

This is often possible (De Morgan's Law states that all Boolean arithmetic can
be expressed inversely, for example) but not always ergonomic, and for some
occasions in Rust, fully impossible.

The `if let` and `while let` control structures, for example, are not able to
be inverted. The `if let` structure can have an `else` branch, but this still
requires an empty `if let` body. The `while let` structure has no inversion at
all.

For control cases where a specific case should trigger the *exit* of the flow,
rather than the *entry*, it is advantageous to have a means of inverting the
condition. For example: executing a loop body forever until a specific status is
met, while permitting any number of non-terminal statuses to be accepted:

```rust
enum FsmState {
    Start,
    Continue,
    Stop,
}
loop {
    if let FsmState::Stop = execute_machine(); {
        break;
    }
    //  continue looping for FsmState::Start or FsmState::Continue
}
```

This cannot be refactored into a `while let` structure, because there is no way
to indicate "continue the loop while the condition is either `Start` or
`Continue`," and the test `while let FsmState::Stop = execute_machine() {}` is
fully incorrect.

Similarly, single-run control branches of `if let` are unpleasant to trigger
negatively:

```rust
if let FsmState::Stop = execute_machine() {
    //  do nothing
}
else {
    //  do significant work on the positive execution
}
```

There are two solutions to these problems: permit use of the `!=` operator in
the test (a case the author considers to be a non-starter, as it further blurs
the line between assignment and equivalence), or to use negative keywords in the
syntax: `if true` receives `unless false`, and `while true` receives
`until false`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust provides the programmer with facilities to inspect the state of the world
and make decisions based upon the result. The simplest and most common of these
are the `if` and `while` structures, which execute a section of code once or
repeatedly, according to some test.

The `if COND { body }` structure examines the condition `COND`, and if the
examination reports that the condition is true, executes `body`. If the
condition is not true, then `body` does not execute, and the program
fast-forwards to the end of the `body` code.

The `while COND { body }` structure does the same thing as the `if` structure,
except that it repeats itself until the condition is not true.

Suppose you want to have code that executes if the condition you're testing is
*not* true. This is easy to accomplish with `if`: append an `else { body }`
structure to it, and if the condition is false when tested, the `else` body
executes. If the condition was true, it does not.

This is harder to accomplish with `while`: you must construct a condition that
is the logical opposite of what you were examining, and test it instead. For
example, if you used to have `VALUE == variable`, you must now have
`VALUE != variable`; `(VAL_A == var_a && VAL_B == var_b)` becomes
`(VAL_A != var_a || VAL_B != var_b)`. Successfully inverting a Boolean test is
tricky when the condition is not simple, and provides opportunities for
mistakes.

The problem is compounded when you use pattern matching rather than simple
equivalence checks! Suppose you are using `if let` or `while let` structures,
which make decisions based not on Boolean arithmetic, but on whether Rust
patterns are appropriate.

In an `if let PAT = expr { body }` structure, the `body` is executed if the test
expression matches the `PAT` pattern. `if let Some(_) = iter.next()` is true
when `iter.next()` returns `Some(thing)`, and false when it returns `None`. This
can be made highly specific by increasing the specificity of the pattern, but
there is no way to negate the condition like there is with Boolean arithmetic!

```rust
if let PAT = expr {
    true_case();
}
else {
    false_case();
}
```

is possible, but this is not possible on `while let` loops.

As such, in any instance where you want to make your program act on the
*opposite* of a test pattern, you can use the `unless` or `until` keywords.

```rust
unless COND {
    cond_is_false();
}
else {
    cond_is_true();
}
```

evaluates the conditional `COND`, running the first body if the test failed and
the second (optional) body if the test succeeded. It is exactly equivalent to

```rust
if !COND {
    cond_is_false();
}
else {
    cond_is_true();
}
```

For loops,

```rust
until COND {
    cond_is_false();
}
```

evalueates the conditional `COND`, executing the loop if the test failed and
moving forward if the test succeeded. It is exactly equivalent to

```rust
while !COND {
    cond_is_false();
}
```

For pattern-matching cases, there is no `!` operator. When you want to test a
condition and have the positive case be anything other than what you examine,
you can write

```rust
unless let PAT = expr {
    pat_does_not_match();
}
else {
    pat_does_match();
}
```

to be equivalent to

```rust
if let PAT = expr {
    pat_does_match();
}
else {
    pat_does_not_match();
}
```

You can also write

```rust
until let PAT = expr {
    pat_does_not_match();
}
```

to execute the loop body until the expression matches the pattern. This does not
have any simple equivalent in Rust: the closest you can get is with

```rust
loop {
    if let PAT = expr {
        break;
    }
    pat_does_not_match();
}
```

This is not nearly as nice to write!

Generally, Rust programmers consider it good style to have the interesting part
of the logic come first in an `if` structure, and to have the condition being
tested be as specific and clear as possible.

This means that if we are testing something that is not an even 50/50 split, the
narrow case (for instance, rolling a d20 and getting a 20) should be the
interesting path, and the wide case (rolling a d20 and getting anything else)
should be the boring path.

```rust
if 20 == d20() {
    interesting();
}
else {
    boring();
}
```

But in cases where the specific case is boring, and the general case is
interesting (for example: rolling a d20 and getting a 1), then one of the two
ideals breaks down. Either the condition becomes wide, or the interesting part
goes last.

```rust
if 1 != d20() { // 19 of 20 matches happen here! That's very wide :(
    interesting();
}
else {
    boring();
}

if 1 == d20() { // 1 of 20 matches happen here! That's what we want :)
    boring();   // but now the boring case comes first, and that's not :(
}
else {
    interesting();
}
```

The `unless` and `until` keywords let us keep all our good ideas: the
interesting logic is given higher placement than the boring logic, the condition
being tested stays narrow and specific, *and* there are no convoluted
incantations needed to make the test you actually want.

```rust
// if let Roll::CritMiss = roll() {
// }
// else {
//     do_a_move();
// }
unless let Roll::CritMiss = roll() {
    do_a_move();
}

until let Roll::CritMiss = roll() {
    play_the_game();
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The technical implementation of these two keywords should be fairly
straightforward. Syntactically, they are paired with `if` and `while`, and have
identical rules for placement in the syntax. These keywords are essentially
sugar for altering the control flow layout, and do not need to alter the
condition under test, as demonstrated below:

- `unless` branch without `else` branch:

    ```rust
    unless COND { BODY }
    ```

    is equivalent to

    ```rust
    if COND {} else { BODY }
    ```

- `unless` branch with `else` branch:

    ```rust
    unless COND { ONE } else { TWO }
    ```

    is equivalent to

    ```rust
    if COND { TWO } else { ONE }
    ```

- `until` loop:

    ```rust
    until COND { BODY }
    ```

    is equivalent to

    ```rust
    loop { if COND { break; } BODY }
    ```

- `unless let` branch without `else` branch:

    ```rust
    unless let PAT = EXPR { BODY }
    ```

    is equivalent to

    ```rust
    if let PAT = EXPR {} else { BODY }
    ```

- `unless let` branch with `else` branch:

    ```rust
    unless let PAT = EXPR { ONE } else { TWO }
    ```

    is equivalent to

    ```rust
    if let PAT = EXPR { TWO } else { ONE }
    ```

- `until let` loop:

    ```rust
    until let PAT = EXPR { BODY }
    ```

    is equivalent to

    ```rust
    loop { if let PAT = EXPR { break; } BODY }
    ```

- `match` arm guard clauses:

    ```rust
    match EXPR {
        PATTERN unless CONDITION => BODY,
    }
    ```

    is *semantically*, but not necessarily *mechanically*, equivalent to

    ```rust
    match EXPR {
        PATTERN if CONDITION => {},
        PATTERN => BODY,
    }
    ```

Note that this, unlike the previous cases, is likely not representable as a
simple source-to-source transform due to move semantics in `match` arm
evaluations. It is possible to construct patterns which induce partial moves
which, if the arm then fails to satisfy the guard clause, may render the
subsequent unconditional pattern unusable.

This RFC makes no changes to Rust's control flow structure. It is *solely* a
source-code-level expansion that simplifies representation of specific branch
cases. This is a strict expansion of the set of possible branches representable
in source code, and does not affect existing code in any way beyond the
reservation of `until` and `unless` as keywords.

Rust flow-control constructs are value-producing expressions from the interior
block. These keywords would not change this behavior. `unless` branches produce
the value of the path that was executed, and `until` loops produce `()` just as
`while` loops do.

Rust may eventually choose to have `while` (and thus, if accepted, `until`)
loops evaluate to be the most recent value of the loop body; this is outside the
scope of this RFC.

# Drawbacks
[drawbacks]: #drawbacks

- These keywords were not previously reserved, and so reserving them may break
    existing code. This RFC would have to be implemented as weak keywords or in
    the next epoch.

- This expands the surface area of control flow syntax; even more ways to make
    branches and loops is not always ideal.

- Ambiguity or confusion in choosing between `if`/`unless` or `while`/`until`.

    A 50/50 branch (such as `if n % 2 == 0`) should favor using the positive
    keywords `if` or `while` rather than the negative keywords `unless` or
    `until`.

    The negative words may lead casual readers to form improper assumptions
    about control flow, inducing confusion or stutter when reading in more depth

- Patterns *cannot* have interior bindings.

    When an `unless let` or `until let` pattern matches, the branch governed by
    it is **not** taken. As such, any bindings in the pattern would only be
    accessible in blocks where they values to which they refer are **not**
    alive.

    As such, the following is invalid:

    ```rust
    unless let Err(e) = fallible() {
        //  e is not in scope, because fallible() is not Err
        //  the interior fields must be _
    }
    else {
        //  e is accessible and in scope here, but it *should not be* in
        //  scope, and NLL may later enforce this
    }
    ```

    Patterns with interior data can be formed and inspected, but they cannot
    bind:

    ```rust
    unless let Counter(x @ 1 ... 5) = expr() {
        //  expr() might be a Counter(x > 5), OR any other variant!
        //  Thus, the Counter interior data cannot be in scope
    }
    else {
        //  Control jumps here when Counter(x @ 1 ... 5) matches, but if
        //  you need access to the x binding, you should be using
        //  `if let` because this is now the more interesting branch
    }
    ```

    The guard clause can still be used, but without the `binding @` prefix:

    ```rust
    //  unnamed fields
    unless let Counter(1 ... 5) = expr {}

    //  named fields
    struct Foo { x: i32 }
    unless let Foo { x: 1 ... 5 } = expr {}
    ```

    This is compatible with existing Rust, where destructuring does not bind
    unless an explicit `@` operator is used.

    ```rust
    let expr = Foo { x: 3 }
    if let Foo { x: 1 ... 5 } = expr {
        //  this branch enters, because expr.x is 3, but there is no
        //  binding to x in scope
    }
    ```

    If interior bindings are desired, this is a strong indication that your code
    should be using `if let` or `while let` instead.

# Rationale and alternatives
[alternatives]: #alternatives

- Why is this design the best in the space of possible designs?

    Changing `let` bindings to have a negative operator such as `!=` is probably
    way worse, since it seems Rust is explicitly trying to differentiate between
    "these two concepts are logically equivalent" (`Eq` trait, `==` and `!=`
    operators) and "this value is shaped like that pattern" (`let`, `match`).

    Another concept is to introduce a *negative binding*, `!let`, which does not
    appear to be as good a solution as discrete keywords, but discussion is
    certainly worth having. The author personally favors keywords over sigils
    for readability purposes.

    Full pattern arithmetic (OR, AND, NOT) is discussed next.

- What other designs have been considered and what is the rationale for not
    choosing them?

    We could expand arithmetic on pattern sets like we do on trait sets. This is
    something that is occasionally brought up as an idea, and does not often get
    significant traction.

    Patterns already support expressing combination with `|` in `match` arms.
    Ideas get raised periodically to add `&&` combinators to `let` bindings,
    such as `let PAT_A = expr_a && PAT_B = expr_b`, which if implemented would
    give patterns two of the three logical arithmetic operations; the last
    remaining operation is negation, `!`.

    Adding full logical arithmetic to patterns would likely be worth pursuing in
    the long run, but is also likely to require significantly more complex work
    in the compiler to support, and may be more complex to teach.

    [RFC #2175][rfc_2175] adds `|` to `if let` and `while let` constructs (which
    desugar to match anyway, just as `unless let` and `until let` would). That
    RFC is logically equivalent to this RFC, courtesy of set arithmetic — for
    any closed set $$F = { A, B, C }$$, the expression $$\lnot A$$ is equivalent
    to $$B \lor C$$. As such, the implementation of #2175 may well be grounds
    for rejecting this RFC. The author belives that the prevalence of
    pre-existing sugar, including additional keywords, in the Rust language
    indicates a preference for semantically clear keywords and structures in
    addition to, if not in favor over, the equivalent structures with less
    semantic or syntactic clarity.

    The `until` and `unless` keywords can, with one exception, be implemented as
    a desugaring pass similar to the mechanism that desugars `for` loops into
    `while` loops. The exception (`PAT unless GUARD`) is likely able to be
    expressed in current Rust compiler logic, but the author does not know how
    at this time.

- What is the impact of not doing this?

    Paper cuts on a few instances that cannot be represented in current flow
    constructs.

# Prior art
[prior-art]: #prior-art

## `unless`

- [Ruby][ruby_unless]

    It is often used in raising exceptions for specific circumstances
    (`raise alarm unless ok?` is a common pattern), for better or for worse.

- [Perl][perl_unless]

## `until`

- [IBM HLASM][ibm]
- [Autoit][autoit]
- [Bash][bash]
- [Perl][perl_until]
- [Ruby][ruby_until]

[Ruby style guides](https://github.com/bbatsov/ruby-style-guide/issues/329) have
encountered the matter before, and this issue nicely summarizes what the author
believes to be an acceptable guideline for `unless` versus `if not`.

## Pattern Arithmetic

[RFC #2175][rfc_2175], discussed above.

# Unresolved questions
[unresolved]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process
    before this gets merged?

    Do we want two more keywords? Would it be better to have negatable patterns?

- What related issues do you consider out of scope for this RFC that could be
    addressed in the future independently of the solution that comes out of this
    RFC?

    Increasingly expressive pattern syntax.

[autoit]: https://www.autoitscript.com/autoit3/docs/keywords/Do.htm
[bash]: http://tldp.org/HOWTO/Bash-Prog-Intro-HOWTO-7.html#ss7.4
[ibm]: https://www.ibm.com/support/knowledgecenter/en/SSLTBW_2.1.0/com.ibm.zos.v2r1.asmk200/asmtug2128.htm
[perl_unless]: https://www.tutorialspoint.com/perl/perl_unless_statement.htm
[perl_until]: https://www.tutorialspoint.com/perl/perl_until_loop.htm
[rfc_2175]: https://github.com/rust-lang/rfcs/blob/master/text/2175-if-while-or-patterns.md
[ruby_unless]: https://en.wikibooks.org/wiki/Ruby_Programming/Syntax/Control_Structures#unless_expression
[ruby_until]: https://en.wikibooks.org/wiki/Ruby_Programming/Syntax/Control_Structures#until
