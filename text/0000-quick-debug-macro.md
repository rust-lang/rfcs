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
builds. To see why this is useful, consider starting off with:

```rust
let c = fun(a) + fun(b);
let y = self.first().second();
```

Now you want to inspect what `fun(a)` and `fun(b)` is, but not go through the
hassle of 1) saving `fun(a)` and `fun(b)` to a variable, 2) printing out the
variable, 3) then finally use it in the expression as `let c = fa + fb;`.
The same applies to inspecting the temporary state of `self.first()`. Instead of
this hassle, you can simply do:

```rust
let c = dbg!(fun(a)) + dbg!(fun(b));
let y = dbg!(self.first()).second();
```

This modification is considerably smaller and disturbs flow while
developing code to a lesser degree.

Additionally, by allowing the user to pass in multiple expressions and label
them, the utility is further augmented.

## Why not use the `log` crate?

While the `log` crate offers a lot of utility, it first has to be used with
`extern crate log;`. A logger then has to be set up before expressions can be
logged. It is therefore not suitable for introducing newcommers to the language.

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
[DEBUGGING, src/main.rs:1]
=> Point{x: 1, y: 2,} = Point {
    x: 1,
    y: 2
}

[DEBUGGING, src/main.rs:7]
=> p = Point {
    x: 4,
    y: 5
}
```

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
[DEBUGGING, src/main.rs]
=> 1 + 2 = 3

[DEBUGGING, src/main.rs]
=> x + 1 = 4

[DEBUGGING, src/main.rs]
=> 3 = 3

[DEBUGGING, src/main.rs]
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
[DEBUGGING, src/main.rs:3]
=> a = 1

[DEBUGGING, src/main.rs:4]
=> a = 1, b = 2

[DEBUGGING, src/main.rs:5]
=> a = 1, b = 2, a + b = 3

[DEBUGGING, src/main.rs:9]
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
    dbg!("first point" => &p, "same point" => &p);
}
```

This allows the user to provide more descriptive names if necessary. With this
example, the following is printed to `STDERR`:
```
[DEBUGGING, src/main.rs:2]
=> "width" = 1, "height" = 2, "area" = 2

[DEBUGGING, src/main.rs:7]:
=> "first point" = Point {
    x: 4,
    y: 5
}, "same point" = Point {
    x: 4,
    y: 5
}
```

It is important to note here that since the type `Point` is not `Copy`, it has
move semantics. Since `dbg!(p)` would involve moving `p`, using `dbg!(p, p);`
would involve moving the value twice, which Rust will not allow. Therefore,
a borrow to `p` is used in `dbg!("first point" => &p, "second point" => &p);`.

The ways of using the macro used in later (not the first) examples will mostly
benefit existing Rust programmers.

### Compact mode:

Those developers who feel the source location header is overly verbose may
choose to opt-out by setting the environment variable `RUST_DBG_COMPACT` to
`"0"`. This is a one-time setup cost the developer has to make for all current
and future Rust projects.

The effect of flipping this switch off is to print out the following instead
for the two last examples:

```
[src/main.rs:3] a = 1
[src/main.rs:4] a = 1, b = 2
[src/main.rs:5] a = 1, b = 2, a + b = 3
[src/main.rs:9] &p = Point { x: 4, y: 5}, &q = Point { x: 2, y: 1 }
```

and:

```
[src/main.rs:2] "width" = 1, "height" = 2, "area" = 2
[src/main.rs:7] "first point" = Point { x: 4, y: 5 }, "same point" = Point { x: 4, y: 5 }
```

## On release builds

The same examples above will print nothing to `STDERR` and will instead simply
evaluate the expressions.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

**NOTE:** The exact output format is not meant to be stabilized even when/if the
macro is stabilized.

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

## Schematic/step-wise explanation (for debug builds)

1. The standard error is locked and wrapped in a buffered writer called `err`.

2. Assume `let p = option_env!("RUST_DBG_COMPACT").map_or(true, |s| s == "0");`.

If `p` holds, the file name (given by `file!()`) and line number (`line!()`) is
included in the print out for increased utility when the macro is used in
non-trivial code. This is wrapped by `[DEBUGGING, <location>]\n=> ` as in:

```rust
write!(&mut err, "[DEBUGGING, {}:{}]\n=> ", file!(), line!())
```

If `p` does not hold, this instead prints:

```rust
write!(&mut err, "[{}:{}] ", file!(), line!())
```

3. + For `($($val: expr),+)`

For each `$val` (the expression), the following is printed, comma separated:
The value of the expression is presented on the right hand side (RHS) of an
equality sign `=` while the result of `stringify!(expr)` is presented on the
left hand side (LHS). This is done so that the developer easily can see the
syntactic structure of the expression that evaluted to RHS.

In other words, the following:
`write!(&mut err, "{} = {:#?}", stringify!($lab), tmp);` is done.
If `p` holds, `{:?}` is used as the format instead of `{:#?}`.

3. + For `($($lab: expr => $val: expr),+)`:

For each `$lab => $val` (the label and expression), the following is printed,
comma separated: The value of the expression is presented on RHS of an equality
sign `=` while the label is presented on LHS.

In other words, the following:
`write!(&mut err, "{} = {:#?}", stringify!($lab), tmp);` is done.
If `p` holds, `{:?}` is used as the format instead of `{:#?}`.
The label is also verified to be a string slice literal.

4. Finally, a newline is printed, or two newlines in the case `p` holds.

## Example implementation

The `dbg!` macro is semantically (with the notable detail that the helper macros
and any non-`pub` `fn`s must be inlined in the actual implementation):

```rust
// For #[allow(unused_parens)]:
#![feature(stmt_expr_attributes)]

pub fn in_detailed_mode() -> bool {
    option_env!("RUST_DBG_COMPACT").map_or(false, |s| s == "0")
}

macro_rules! verify_str_lit {
    ($($expr: expr),*) => {
        $({
            let _ = concat!($expr, "");
            let _ : &'static str = $expr;
        })*
    };
}

macro_rules! w {
    ($err: ident, $($t: tt)+) => { write!(&mut $err, $($t)+).unwrap(); }
}

macro_rules! dbg_header {
    ($err: ident) => {
        #[cfg(debug_assertions)] {
            if in_detailed_mode() {
                w!($err, "[DEBUGGING, {}:{}]\n=> ", file!(), line!())
            } else {
                w!($err, "[{}:{}] ", file!(), line!())
            }
        }
    };
}

macro_rules! dbg_footer {
    ($err: ident) => {
        #[cfg(debug_assertions)] {
            if in_detailed_mode() { w!($err, "\n\n") } else { w!($err, "\n") }
        }
    };
}

macro_rules! dbg_expr {
    ($err: ident, $lab: expr => $val: expr) => {{
        let tmp = $val;
        #[cfg(debug_assertions)] {
            let l = stringify!($lab);
            if in_detailed_mode() { w!($err, "{} = {:#?}", l, tmp) }
            else { w!($err, "{} = {:?}",  l, tmp) }
        }
        tmp
    }}
}

macro_rules! dbg_core {
    ($labf: expr => $valf: expr $(, $lab: expr => $val: expr)*) => {
        //#[allow(unused_parens)] // requires: #![feature(stmt_expr_attributes)]
        {
            #[cfg(debug_assertions)]
            use ::std::io::Write;
            #[cfg(debug_assertions)]
            let stderr = ::std::io::stderr();
            #[cfg(debug_assertions)]
            let mut err = ::std::io::BufWriter::new(stderr.lock());

            dbg_header!(err);
            let ret = (
                dbg_expr!(err, $labf => $valf)
                $(, {
                    #[cfg(debug_assertions)] w!(err, ", ");
                    dbg_expr!(err, $lab => $val)
                } )*
            );
            dbg_footer!(err);
            ret
        }
    };
}

#[macro_export]
macro_rules! dbg {
    // Handle trailing comma:
    ($($val: expr),+,) => {
        dbg!( $($val),+ )
    };
    ($($lab: expr => $val: expr),+,) => {
        dbg!( $($lab => $val),+ )
    };
    // Without label, use source of $val:
    ($valf: expr $(, $val: expr)*) => {
        dbg_core!($valf => $valf $(, $val => $val)*)
    };
    // With label:
    ($labf: expr => $valf: expr $(, $lab: expr => $val: expr)*) => {{
        verify_str_lit!($labf, $($lab),*);
        dbg_core!($labf => $valf $(, $lab => $val)*)
    }};
}
```

## Exact implementation

The exact implementation, which is authoritative on the semantics of this RFC,
is given by:

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
    // Without label, use source of $val:
    ($valf: expr $(, $val: expr)*) => {
        #[allow(unused_parens)] // requires: #![feature(stmt_expr_attributes)]
        {
            // DEBUG: Lock STDERR in a buffered writer.
            // Motivation:
            // 1. to avoid needless re-locking of STDERR at every write(ln)!.
            // 2. to ensure that the printed message is not interleaved, which
            // would disturb the readability of the output, by other messages to
            // STDERR.
            #[cfg(debug_assertions)]
            use ::std::io::Write;
            #[cfg(debug_assertions)]
            let stderr = ::std::io::stderr();
            #[cfg(debug_assertions)]
            let mut err = ::std::io::BufWriter::new(stderr.lock());

            // Are we in not in compact mode (detailed)?
            // If so:
            // + {:?} is used instead of {:#?},
            // + Header is: [<location>]
            #[cfg(debug_assertions)]
            let detailed = option_env!("RUST_DBG_COMPACT")
                            .map_or(true, |s| s == "0");

            #[cfg(debug_assertions)] {
                (if detailed {
                    write!(&mut err, "[DEBUGGING, {}:{}]\n=> ", file!(), line!())
                } else {
                    write!(&mut err, "[{}:{}] ", file!(), line!())
                }).unwrap()
            }

            // Foreach label and expression:
            //     1. Evaluate each expression,
            //     2. Print out $lab = value of expression
            let ret = (
                {
                    // Evaluate, tmp is value:
                    let tmp = $valf;
                    // Print out $lab = tmp:
                    #[cfg(debug_assertions)] {
                        let l = stringify!($valf);
                        (if detailed {
                            write!(&mut err, "{} = {:#?}", l, tmp)
                        } else {
                            write!(&mut err, "{} = {:?}",  l, tmp)
                        }).unwrap()
                    }
                    // Yield tmp:
                    tmp
                }
                $(, {
                    // Comma separator:
                    #[cfg(debug_assertions)] {
                        write!(&mut err, ", ").unwrap();
                    }
                    {
                        // Evaluate, tmp is value:
                        let tmp = $val;
                        // Print out $lab = tmp:
                        #[cfg(debug_assertions)] {
                            let l = stringify!($val);
                            (if detailed {
                                write!(&mut err, "{} = {:#?}", l, tmp)
                            } else {
                                write!(&mut err, "{} = {:?}",  l, tmp)
                            }).unwrap()
                        }
                        // Yield tmp:
                        tmp
                    }
                } )*
            );

            // Newline:
            #[cfg(debug_assertions)] {
                (if detailed {
                    writeln!(&mut err, "\n")
                } else {
                    writeln!(&mut err, "")
                }).unwrap()
            }

            // Return the expression:
            ret
        }
    };
    // With label:
    ($labf: expr => $valf: expr $(, $lab: expr => $val: expr)*) => {
        #[allow(unused_parens)] // requires: #![feature(stmt_expr_attributes)]
        {
            // DEBUG: Lock STDERR in a buffered writer.
            // Motivation:
            // 1. to avoid needless re-locking of STDERR at every write(ln)!.
            // 2. to ensure that the printed message is not interleaved, which
            // would disturb the readability of the output, by other messages to
            // STDERR.
            #[cfg(debug_assertions)]
            use ::std::io::Write;
            #[cfg(debug_assertions)]
            let stderr = ::std::io::stderr();
            #[cfg(debug_assertions)]
            let mut err = ::std::io::BufWriter::new(stderr.lock());

            // Are we in not in compact mode (detailed)?
            // If so:
            // + {:?} is used instead of {:#?},
            // + Header is: [<location>]
            #[cfg(debug_assertions)]
            let detailed = option_env!("RUST_DBG_COMPACT")
                            .map_or(true, |s| s == "0");

            #[cfg(debug_assertions)] {
                (if detailed {
                    write!(&mut err, "[DEBUGGING, {}:{}]\n=> ", file!(), line!())
                } else {
                    write!(&mut err, "[{}:{}] ", file!(), line!())
                }).unwrap()
            }

            // Foreach label and expression:
            //     1. Evaluate each expression,
            //     2. Print out $lab = value of expression
            let ret = (
                {
                    // Evaluate, tmp is value:
                    let tmp = $valf;
                    // Print out $lab = tmp:
                    #[cfg(debug_assertions)] {
                        // Enforce is_literal_string($labf):
                        let _ = concat!($labf, "");
                        let _ : &'static str = $labf;

                        // Print:
                        let l = stringify!($labf);
                        (if detailed {
                            write!(&mut err, "{} = {:#?}", l, tmp)
                        } else {
                            write!(&mut err, "{} = {:?}",  l, tmp)
                        }).unwrap()
                    }
                    // Yield tmp:
                    tmp
                }
                $(, {
                    // Comma separator:
                    #[cfg(debug_assertions)] {
                        write!(&mut err, ", ").unwrap();
                    }
                    {
                        // Evaluate, tmp is value:
                        let tmp = $val;
                        // Print out $lab = tmp:
                        #[cfg(debug_assertions)] {
                            // Enforce is_literal_string($lab):
                            let _ = concat!($lab, "");
                            let _ : &'static str = $lab;

                            // Print:
                            let l = stringify!($lab);
                            (if detailed {
                                write!(&mut err, "{} = {:#?}", l, tmp)
                            } else {
                                write!(&mut err, "{} = {:?}",  l, tmp)
                            }).unwrap()
                        }
                        // Yield tmp:
                        tmp
                    }
                } )*
            );

            // Newline:
            #[cfg(debug_assertions)] {
                (if detailed {
                    writeln!(&mut err, "\n")
                } else {
                    writeln!(&mut err, "")
                }).unwrap()
            }

            // Return the expression:
            ret
        }
    };
}
```

On release builds, this macro reduces to:

```rust
// For #[allow(unused_parens)]:
#![feature(stmt_expr_attributes)]

#[macro_export]
macro_rules! dbg {
    // ...

    // Without label, use source of $val:
    ($valf: expr $(, $val: expr)*) => {
        #[allow(unused_parens)] // requires: #![feature(stmt_expr_attributes)]
        {
            let ret = (
                {
                    let tmp = $valf;
                    tmp
                }
                $(, {
                    {
                        let tmp = $val;
                        tmp
                    }
                } )*
            );
            ret
        }
    };
    // With label:
    ($labf: expr => $valf: expr $(, $lab: expr => $val: expr)*) => {
        #[allow(unused_parens)] // requires: #![feature(stmt_expr_attributes)]
        {
            let ret = (
                {
                    let tmp = $valf;
                    tmp
                }
                $(, {
                    {
                        let tmp = $val;
                        tmp
                    }
                } )*
            );
            ret
        }
    };
}
```

which further reduces to the following, which clearly shows that the invocation
is nothing more than the identity on the tuple passed:

```rust
#[macro_export]
macro_rules! dbg {
    // ...
    ($(              $val: expr),+) => {{ ( $($val),* ) }};
    ($($lab: expr => $val: expr),+) => {{ ( $($val),* ) }};
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

## Bikeshed: The name of the macro

Several names has been proposed for the macro. Some of the candidates were:

+ `debug!`, which was the original name. This was however already used by the
`log` crate.
+ `d!`, which was deemded to be too short to be informative and convey intent.
+ `dump!`, which was confused with stack traces.
+ `show!`, inspired by Haskell. `show` was deemed less obvious than `dbg!`.
+ `peek!`, which was also deemed less obvious.
+ `DEBUG!`, which was deemed too screamy.
+ `qdbg!`, which was deemed to hurt searchability and learnability since it
isn't prefixed with `d`(ebug).

While it is unfortunate that `debug!` was unavailable, `dbg!` was deemed the
next best thing, which is why it was picked as the name of the macro.

## How do we teach this?

Part of the [motivation][for-aspiring-rustaceans] for this macro was to delay
the point at which aspiring rustaceans have to learn how formatting arguments
work in the language. For this to be effective, the macro should be taught prior
to teaching formatting arguments, but after teaching the user to write their
first "hello world" and other uses of `println!("<string literal>")` which does
not involve formatting arguments, which should first be taught when formatting
is actually interesting, and not as a part of printing out the value of an
expression.

## Formerly unresolved questions
[formerly unresolved]: #formerly-unresolved-questions

Some questions regarding the format were:

1. Should the `file!()` be included?

**Yes**, since it would be otherwise difficult to tell where the output is coming
from in a larger project with multiple files. It is not very useful on
the [playground](https://play.rust-lang.org), but that exception is acceptable.

2. Should the line number be included?

**Yes**, for a large file, it would also be difficult to locate the source of the
output otherwise.

3. Should the column number be included?

It is more likely than not that no more than one `dbg!(...)` will occur. If it
does, it will most likely be when dealing with binary operators such as with:
`dbg!(x) + dbg!(y) + dbg!(z)`, or with several arguments to a function/method
call. However, since the macro prints out `stringify!(expr)`, which in the case
of the additions would result in: `x = <val>, y = <val>, z = <val>`, the user
can clearly see which expression on the line that generated the value. The only
exception to this is if the same expression is used multiple times and crucically
has side effects altering the value between calls. This scenario is probably
very uncommon. Furthermore, even in this case, one can distinguish between the
calls since one is first and the second comes next, visually.

However, the `column!()` isn't very visually disturbing since it uses horizontal
screen real-estate but not vertical real-estate, which may still be a good reason
to keep it. Nonetheless, this argument is not sufficient to keep `column!()`,
wherefore **this RFC will not include it**.

4. Should the `stringify!($val)` be included? **[answer: yes]**

Other, now resolved, questions, were:

5. Should the macro be pass-through with respect to the expression? 
   In other words: should the value of applying the macro to the expression be
   the value of the expression?

**Yes**, the pass-through mechanism allows the macro to be less intrusive as
discussed in the [motivation].

6. Should the macro act as the identity function on release modes?
   If the answer to this is yes, 5. must also be yes, i.e: 6. => 5.

**Yes**, since some users who develop programs, and not libraries, can leave
such `dbg!(..)` invocations in and push it to source control since it won't
affect debug builds of the program.

8. Should a trailing newline be added after each `dbg!(exprs...)`?

**Short answer: Yes.**

The result of answer in the negative would use the following format:

```
[DEBUGGING, src/main.rs:85]
=> a = 1
[DEBUGGING, src/main.rs:86]
=> a = 1, b = 2
[DEBUGGING, src/main.rs:87]
=> a = 1, b = 2, a + b = 3
```

instead of:

```
[DEBUGGING, src/main.rs:85]
=> a = 1

[DEBUGGING, src/main.rs:86]
=> a = 1, b = 2

[DEBUGGING, src/main.rs:87]
=> a = 1, b = 2, a + b = 3
```

The latter format, to many readers, look considerably more readable thanks to
visual association of a particular set of values with the `DEBUGGING` header and
make the users own `println!(..)` and `eprintln!(..)` calls stand out more due
to the absence of the header.

A counter argument to this is that users with IDEs or vertically short terminals
may have as little as `25%` of vertical screen space allocated for the program's
output with the rest belonging to the actual code editor. To these users, lines
are too precious to waste in this manner since scrolling may require the use of
the mouse or switching of keyboard input focus.

However, it is more unlikely that a user will see the information they are
looking for in a small window without scrolling. Here, searchability is aided by
grouping which is visually pleasing to process.

This was resolved by having the env var `RUST_DBG_COMPACT = 0` format the above
example as:

```
[src/main.rs:85] a = 1
[src/main.rs:86] a = 1, b = 2
[src/main.rs:87] a = 1, b = 2, a + b = 3
```

9. Should debugging literal expressions as in `dbg!(42);` print out `42 = 42` or
should it print out `5`?

**No**. The left hand side of the equality adds no new information wherefore it
might be a redundant annoyance. On the other hand, it may give a sense of
symmetry with the non-literal forms such as `a = 42`. Keeping `5 = 5` is also
more consistent, wherefore that format will be used. 

# Unresolved questions
[unresolved]: #unresolved-questions

The format used by the macro should be resolved prior to merging.

[`specialization`]: https://github.com/rust-lang/rfcs/pull/1210

To be revisited once [`specialization`] has been stabilized:

[`debugit`]: https://docs.rs/debugit/0.1.2/debugit/

7. Should expressions and values of non-`Debug` types be usable with this macro
by using `std::intrinsics::type_name` for such types and the `Debug` impl for
`T : Debug` types as done in version 0.1.2 of [`debugit`]? This depends on
specialization.