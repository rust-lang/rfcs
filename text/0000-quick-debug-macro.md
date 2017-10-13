- Feature Name: quick_debug_macro
- Start Date: 2017-10-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds a macro `dbg!(expr)` for quick and dirty `Debug`:ing of an expression to the terminal. The macro evaluates the expression, prints it to `STDERR`, and finally yields `expr`. On release builds, the macro is the identity function and has no side effects. The macro is added to the prelude of the standard library.

# Motivation
[motivation]: #motivation

The motivation to add this new `dbg!` macro is two-fold.

## For aspiring rustaceans

One of the often asked questions is how to print out variables to the terminal.
Delaying the need to explain formatting arguments in the statement `println!("{:?}", expr);` can help aspiring rustaceans to quickly learn the language. With `dbg!(expr);` there is no longer such need, which can be delayed until the developer actually cares about the format of the output and not just what value the expression evaluates to.

## For experienced developers

By using `dbg!(expr);`, the burden of a common papercut: writing `println!("{:?}", expr);` every time you want to see the evaluted-to value of an expression, can be significantly reduced. The developer no longer has to remember the formatting args and has to type significantly less (12 characters to be exact).

The shortness is also beneficial when asking `rustbot` on #rust@irc.mozilla.org to evaluate and print an expression.

To increase the utility of the macro, it acts as a pass-through function on the expression by simply printing it and then yielding it. On release builds, the macro is the identity function - thus, the macro can be used in release builds while hurting performance while also helping to debug the program.

## Why not use the `log` crate?

While the `log` crate offers a lot of utility, it first has to be used with `extern crate log;`. A logger then has to be set up before expressions can be logged. It is therefore not suitable for introducing newcommers to the language.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## On debug builds:

First, some preliminaries:

```rust
#[derive(Debug)] // The type of expr in dbg!(expr) must be Debug.
struct Point {
    x: usize,
    y: usize,
}
```

With the following example (which most newcomers will benefit from):

```rust
fn main() {
    dbg!(Point {
        x: 1,
        y: 2,
    });

    let p = Point {
        x: 4,
        y: 5,
    };
    dbg!(p);
}
```

The program will print the points to `STDERR` as:

```
[DEBUGGING, src/main.rs:1:4]:
=> Point{x: 1, y: 2,} = Point {
    x: 1,
    y: 2
}
[DEBUGGING, src/main.rs:7:4]:
=> p = Point {
    x: 4,
    y: 5
}
```

Here, `7:4` is the line and the column.

You may also save the debugged value to a variable or use it in an expression
since the debugging is pass-through:

```rust
fn main() {
    let x = dbg!(1 + 2);
    let y = dbg!(x + 1) + dbg!(3);
    dbg!(y);
}
```

This prints the following to `STDERR`:

```
[DEBUGGING, src/main.rs:1:12]:
=> 1 + 2 = 3
[DEBUGGING, src/main.rs:2:12]:
=> x + 1 = 4
[DEBUGGING, src/main.rs:2:26]:
=> 3 = 3
[DEBUGGING, src/main.rs:3:4]:
=> y = 7
```

This way of using the macro will mostly benefit existing Rust programmers.

## On release builds:

The same examples above will print nothing to `STDERR` and will instead simply
evaluate the expressions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `dbg!` macro will be implemented as:

```rust
macro_rules! dbg {
    ($val: expr) => {
        {
            let tmp = $val;
            if cfg!(debug_assertions) {
                eprintln!("[DEBUGGING, {}:{}:{}]:\n=> {} = {:#?}",
                    file!(), line!(), column!(), stringify!($val), tmp );
            }
            tmp
        }
    }
}
```

Branching on `cfg!(debug_assertions)` means that if the program is built as a
release build, nothing will be printed, and the result of using the macro on
an expression is simply the expression itself. In effect the result is applying
the identity function on the expression, but the call will be inlined away such
that the overhead is zero.

The file name, line number and column is included for increased utility when included in production quality code. The expression is also stringified, so that
the developer can easily see the syntactic structure of the expression that
evaluted to the RHS of the equality.

**NOTE:** The exact output format is not meant to be stabilized even when/if the
macro is stabilized.

# Drawbacks
[drawbacks]: #drawbacks

It could be considered bloat, and `println!("{:#?}", expr)` might be
sufficiently ergonomic for both experienced rustaceans and newcomers.

# Rationale and alternatives
[alternatives]: #alternatives

The formatting is informative, but could be formatted in other ways depending
on what is valued. A more terse format could be used if `stringify!` or `file!()` line and column numbers is not deemed beneficial, which this RFC argues it should.

The impact of not merging the RFC is that the papercut, if considered as such,
remains.

# Unresolved questions
[unresolved]: #unresolved-questions

The format used by the macro should be resolved prior to merging.
Some questions regarding the format are:

1. Should the `file!()` be included?
2. Should the line number be included?
3. Should the column number be included?
4. Should the `stringify!($val)` be included?

Other questions, which should also be resolved prior to merging, are:
5. Should the macro be pass-through with respect to the expression?
   In other words: should the value of applying the macro to the expression be
   the value of the expression?
6. Should the macro act as the identity function on release modes?
   If the answer to this is yes, 5. must also be yes, i.e: 6. => 5.