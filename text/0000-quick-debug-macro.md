- Feature Name: quick_debug_macro
- Start Date: 2017-10-13
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Adds a macro `dbg!(expr1 [, expr2, .., exprN])` for quick and dirty `Debug`:ing
of expressions to the terminal. The macro evaluates expressions, prints it to
`STDERR`, and finally yields a flat tuple of `(expr1 [, expr2, .. exprN])`.
On release builds, the macro is the identity function and has no side effects.
The macro is added to the prelude of the standard library.

# Motivation
[motivation]: #motivation

The motivation to add this new `dbg!` macro is two-fold.

## For aspiring rustaceans
[for-aspiring-rustaceans]: #for-aspiring-rustaceans

One of the often asked questions is how to print out variables to the terminal.
Delaying the need to explain formatting arguments in the statement
`println!("{:?}", expr);` can help aspiring rustaceans to quickly learn the
language. With `dbg!(expr);` there is no longer such need, which can be delayed
until the developer actually cares about the format of the output and not just
what value the expression evaluates to.

## For experienced developers

By using `dbg!(expr);`, the burden of a common papercut: writing
`println!("{:?}", expr);` every time you want to see the evaluted-to value of an
expression, can be significantly reduced. The developer no longer has to remember
the formatting args and has to type significantly less (12 characters to be exact).

To increase the utility of the macro, it acts as a pass-through function on the
expression by simply printing it and then yielding it. On release builds, the
macro is the identity function - thus, the macro can be used in release builds
without hurting performance while allowing the debugging of the program in debug
builds.

Additionally, by allowing the user to pass in multiple expressions and label
them, the utility is further augmented.

## Why not use the `log` crate?

While the `log` crate offers a lot of utility, it first has to be used with
`extern crate log;`. A logger then has to be set up before expressions can be
logged. It is therefore not suitable for introducing newcommers to the language.

## Bikeshed: The name of the macro

Several names has been proposed for the macro. Some of the candidates were:

+ `debug!`, which was the original name. This was however already used by the
`log` crate.
+ `d!`, which was deemded to be too short to be informative and convey intent.
+ `dump!`, which was confused with stack traces.
+ `show!`, inspired by Haskell. `show` was deemed less obvious than `dbg!`.
+ `peek!`, which was also deemed less obvious.
+ `DEBUG!`, which was deemed too screamy.

While it is unfortunate that `debug!` was unavailable, `dbg!` was deemed the
next best thing, which is why it was picked as the name of the macro.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## On debug builds

First, some preliminaries:

```rust
#[derive(Debug)] // The type of expr in dbg!(expr) must be Debug.
struct Point {
    x: usize,
    y: usize,
}
```

With the following example, which most newcomers will benefit from:

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
since the debugging is pass-through. This is seen in the following example:

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

More expressions may be debugged in one invocation of the macro, as seen in the
following example:
```rust
fn main() {
    let a = 1;
    let b = 2;
    let _ : u32 = dbg!(a);
    let _ : (u32, u32) = dbg!(a, b);
    let _ : (u32, u32, u32) = dbg!(a, b, a + b);

    let p = Point { x: 4, y: 5 };
    let q = Point { x: 2, y: 1 };
    let qp : (&Point, &Point) = dbg!(&p, &q);
}
```

As seen in the example, the type of the expression `dbg!(expr)` is the type of
`expr`. For `dbg!(expr1, expr2 [, .., exprN])` the type is that of the tuple
`(expr1, expr2 [, .., exprN])`.

The example above prints the following to `STDERR`:
```
[DEBUGGING, src/main.rs:3:18]:
=> a = 1
[DEBUGGING, src/main.rs:4:25]:
=> a = 1, b = 2
[DEBUGGING, src/main.rs:5:30]:
=> a = 1, b = 2, a + b = 3
[DEBUGGING, src/main.rs:9:31]:
=> &p = Point {
    x: 4,
    y: 5
}, &q = Point {
    x: 2,
    y: 1
}
```

Furthermore, instead of using `stringify!` on the expressions, which is done by
default, the user may provide labels, as done in:

```rust
fn main() {
    let w = 1;
    let h = 2;
    dbg!("width" => w, "height" => h, "area" => w * h);

    let p = Point { x: 4, y: 5 };
    let q = Point { x: 2, y: 1 };
    dbg!("first point" => &p, "second point" => &p);
}
```

This allows the user to provide more descriptive names if necessary. With this
example, the following is printed to `STDERR`:
```
[DEBUGGING, src/main.rs:2:4]:
=> "width" = 1, "height" = 2, "area" = 2
[DEBUGGING, src/main.rs:7:4]:
=> "first point" = Point {
    x: 4,
    y: 5
}, "second point" = Point {
    x: 2,
    y: 1
}
```

The ways of using the macro used in later (not the first) examples will mostly
benefit existing Rust programmers.

### Omitting the source location

Those developers who feel the source location header is overly verbose may
choose to opt-out by setting the environment variable `RUST_DBG_NO_LOCATION` to
`"0"`. This is a one-time setup cost the developer has to make for all current
and future Rust projects.

## On release builds

The same examples above will print nothing to `STDERR` and will instead simply
evaluate the expressions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The macro is called `dbg` and accepts either a non-empty comma-separated or
comma-terminated list of `expr`, or a non-empty list of `label => expr` which
is also separated or terminated with commas.

The terminated versions are defined as:

1. `($($val: expr),+,) => { dbg!( $($val),+ ) };`
2. `($($lab: expr => $val: expr),+,) => { dbg!( $($lab => $val),+ ) };`

The separated versions accept the following:

1. `($($val: expr),+)`
2. `($($lab: expr => $val: expr),+)`

The macro only prints something if `cfg!(debug_assertions)` holds, meaning that
if the program is built as a release build, nothing will be printed, and the
result of using the macro on an expressions or expressions is simply the
expression itself or a flat tuple of the expressions themselves. In effect the
result is applying the identity function on the expression(s), but the call will
be inlined away such that the overhead is zero.

## The type of `dbg!(expressions)`

"Applying" `dbg` on a non-empty list of expressions
`expr1 [, expr2 [, .., exprN])` gives back an expression of the following type
and value:

+ List of size 1, `dbg!(expr)`: The type is the type of `expr` and the value is
the value of `expr`.

+ Otherwise, `dbg!(expr1, expr2 [, expr3, .., exprN])`: The type is the type
of the tuple `(expr1, expr2 [, expr3, .., exprN])` which is the value.

## Schematic/step-wise explanation

1. Assume
`let p = option_env!("RUST_DBG_NO_LOCATION").map_or(true, |s| s == "0");`.
If `p` holds, the file name (given by `file!()`), line number (`line!()`) and
column (`column!()`) is included in the print out for increased utility when the
macro is used in non-trivial code. This is wrapped by `[DEBUGGING, <location>]:`
as in:

```rust
eprintln!("[DEBUGGING, {}:{}:{}]:", file!(), line!(), column!());
```

If `p` does not hold, this step prints nothing.

2. An arrow is then printed on the next line: `eprint!("=> ");`.

3. + For `($($val: expr),+)`

For each `$val` (the expression), the following is printed, comma separated:
The value of the expression is presented on the right hand side (RHS) of an
equality sign `=` while the result of `stringify!(expr)` is presented on the
left hand side (LHS). This is done so that the developer easily can see the
syntactic structure of the expression that evaluted to RHS.

In other words, the following: `eprint!("{} = {:#?}", stringify!($lab), tmp);`.

3. + For `($($lab: expr => $val: expr),+)`:

For each `$lab => $val` (the label and expression), the following is printed,
comma separated: The value of the expression is presented on RHS of an equality
sign `=` while the label is presented on LHS.

In other words, the following: `eprint!("{} = {:#?}", stringify!($lab), tmp);`.

**NOTE:** The label is only guaranteed to work when it is a string slice literal.

**NOTE:** The exact output format is not meant to be stabilized even when/if the
macro is stabilized.

## Example implementation

The `dbg!` macro is semantically (with the notable detail that the helper macros
and any non-`pub` `fn`s must be inlined in the actual implementation):

```rust
macro_rules! cfg_dbg {
    ($($x:tt)+) => {
        if cfg!(debug_assertions) { $($x)* }
    };
}

fn dbg_with_location() -> bool {
    option_env!("RUST_DBG_NO_LOCATION").map_or_else(|| true, |s| s == "0")
}

macro_rules! dbg_header {
    ($expr: expr) => {{
        cfg_dbg! {
            if dbg_with_location() {
                eprintln!("[DEBUGGING, {}:{}:{}]:", file!(), line!(), column!());
            }
            eprint!("=> ");
        }
        let ret = $expr;
        cfg_dbg! { eprintln!(""); }
        ret
    }};
}

macro_rules! dbg_comma {
    ($valf: expr) => { $valf };
    ($valf: expr, $($val: expr),+) => {
        ( $valf, $({ cfg_dbg! { eprint!(", "); } $val}),+ )
    }
}

macro_rules! dbg_term {
    ($val: expr) => { dbg_term!($val => $val) };
    ($lab: expr => $val: expr) => {{
        let tmp = $val;
        cfg_dbg! { eprint!("{} = {:#?}", stringify!($lab), tmp); }
        tmp
    }};
}

#[macro_export]
macro_rules! dbg {
    ($($val: expr),+,) => {
        dbg!( $($val),+ )
    };
    ($($lab: expr => $val: expr),+,) => {
        dbg!( $($lab => $val),+ )
    };
    ($($val: expr),+) => {
        dbg_header!(dbg_comma!($(dbg_term!($val)),+))
    };
    ($($lab: expr => $val: expr),+) => {
        dbg_header!(dbg_comma!($(dbg_term!($lab => $val)),+))
    };
}
```

## Exact implementation

The exact implementation is given by:

```rust
// For #[allow(unused_parens)]:
#![feature(stmt_expr_attributes)]

#[macro_export]
macro_rules! dbg {
    // Handle trailing comma:
    ($($val: expr),+,) => {
        dbg!( $($val),+ )
    };
    ($($lab: expr => $val: expr),+,) => {
        dbg!( $($lab => $val),+ )
    };
    // Without label, use source of $val as label:
    ($($val: expr),+) => {
        dbg!($($val => $val),+)
    };
    // With label:
    ($labf: expr => $valf: expr $(, $lab: expr => $val: expr)*) => {
        #[allow(unused_parens)] // requires: #![feature(stmt_expr_attributes)]
        {
            if cfg!(debug_assertions) {
                // Print out source location unless silenced by setting
                // the env var RUST_DBG_NO_LOCATION != 0.
                let p = option_env!("RUST_DBG_NO_LOCATION")
                            .map_or_else(|| true, |s| s == "0");
                if p {
                    eprintln!("[DEBUGGING, {}:{}:{}]:",
                        file!(), line!(), column!());
                }
                // Print out arrow (on a new line):
                eprint!("=> ");
            }

            // Foreach label and expression:
            // 1. Evaluate each expression to value,
            // 2. Print out $lab = value
            // Separate with comma.
            let ret = (
                {
                    // Evaluate, tmp is value:
                    let tmp = $valf;
                    // Print out $lab = tmp:
                    if cfg!(debug_assertions) {
                        eprint!("{} = {:#?}", stringify!($labf), tmp);
                    }
                    // Yield tmp:
                    tmp
                }
                $(, {
                    // Comma separator:
                    if cfg!(debug_assertions) { eprint!(", "); }
                    {
                        // Evaluate, tmp is value:
                        let tmp = $val;
                        // Print out $lab = tmp:
                        if cfg!(debug_assertions) {
                            eprint!("{} = {:#?}", stringify!($lab), tmp);
                        }
                        // Yield tmp:
                        tmp
                    }
                } )*
            );

            // Newline:
            if cfg!(debug_assertions) { eprintln!(""); }

            // Return the expression:
            ret
        }
    };
}
```

# Drawbacks
[drawbacks]: #drawbacks

It could be considered bloat, and `println!("{:#?}", expr)` might be
sufficiently ergonomic for both experienced rustaceans and newcomers.

# Rationale and alternatives
[alternatives]: #alternatives

The formatting is informative, but could be formatted in other ways depending
on what is valued. A more terse format could be used if `stringify!` or
`file!()`, line and column numbers is not deemed beneficial, which this RFC
argues it should. The RFC argues that the possibility of opting out to this
header via an env var strikes a good balance.

The impact of not merging the RFC is that the papercut, if considered as such,
remains.

# Unresolved questions
[unresolved]: #unresolved-questions

The format used by the macro should be resolved prior to merging.

## Formerly unresolved

Some questions regarding the format were:

1. Should the `file!()` be included?
2. Should the line number be included?
3. Should the column number be included?
4. Should the `stringify!($val)` be included?

Other questions, which should also be resolved prior to merging, were:

5. Should the macro be pass-through with respect to the expression?
   In other words: should the value of applying the macro to the expression be
   the value of the expression?
6. Should the macro act as the identity function on release modes?
   If the answer to this is yes, 5. must also be yes, i.e: 6. => 5.

They have all been answered in the affirmative.

## Currently unresolved

[`specialization`]: https://github.com/rust-lang/rfcs/pull/1210

To be revisited once [`specialization`]
has been stabilized:

[`debugit`]: https://docs.rs/debugit/0.1.2/debugit/

7. Should expressions and values of non-`Debug` types be usable with this macro
by using `std::intrinsics::type_name` for such types and the `Debug` impl for
`T : Debug` types as done in version 0.1.2 of [`debugit`]? This depends on
specialization.