- Feature Name: `dbg_macro`
- Start Date: 2018-03-13
- RFC PR: [rust-lang/rfcs#2361](https://github.com/rust-lang/rfcs/pull/2361)
- Rust Issue: [rust-lang/rust#54306](https://github.com/rust-lang/rust/issues/54306)

# Summary
[summary]: #summary

Add a `dbg!($expr)` macro to the prelude (so that it doesn’t need to be imported)
that prints its argument with some metadata (source code location and stringification)
before returning it.

This is a simpler and more opinionated counter-proposal
to [RFC 2173](https://github.com/rust-lang/rfcs/pull/2173).


# Motivation
[motivation]: #motivation

Sometimes a debugger may not have enough Rust-specific support to introspect some data
(such as calling a Rust method), or it may not be convenient to use or available at all.
“`printf` debugging” is possible in today’s Rust with:

```rust
println!("{:?}", expr);
```

This RFC improves some aspects:

* The `"{:?}",` part of this line is boilerplate that’s not trivial to remember
  or even type correctly.
* If the expression to be inspected is part of a larger expression,
  it either needs to be duplicated (which may add side-effects or computation cost)
  or pulled into a `let` binding which adds to the boilerplate.
* When more than one expression is printed at different places of the same program,
  and the formatting itself (for example a plain integer)
  doesn’t indicate what value is being printed,
  some distinguishing information may need to be added.
  For example: `println!("foo = {:?}", x.foo());`

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To inspect the value of a given expression at run-time,
it can be wrapped in the `dbg!` macro to print the value to `STDERR`,
along with its source location and source code:

```rust
fn foo(n: usize) {
    if let Some(_) = dbg!(n.checked_sub(4)) {
        /*…*/
    }
}

foo(3)
```

This prints the following to `STDERR`:

```
[example.rs:2] n.checked_sub(4) = None
```

Another example is `factorial` which we can debug like so:

```rust
fn factorial(n: u32) -> u32 {
    if dbg!(n <= 1) {
        dbg!(1)
    } else {
        dbg!(n * factorial(n - 1))
    }
}

fn main() {
    dbg!(factorial(4));
}
```

Running this program, in the playground, will print the following to `STDERR`:

```
[src/main.rs:1] n <= 1 = false
[src/main.rs:1] n <= 1 = false
[src/main.rs:1] n <= 1 = false
[src/main.rs:1] n <= 1 = true
[src/main.rs:2] 1 = 1
[src/main.rs:4] n * factorial(n - 1) = 2
[src/main.rs:4] n * factorial(n - 1) = 6
[src/main.rs:4] n * factorial(n - 1) = 24
[src/main.rs:9] factorial(4) = 24
```

Using `dbg!` requires type of the expression to implement the `std::fmt::Debug`
trait.

## Move semantics

The `dbg!(x)` macro moves the value `x` and takes ownership of it,
unless the type of `x` implements `Copy`, and returns `x` unchanged.
If you want to retain ownership of the value,
you can instead borrow `x` with `dbg!(&x)`.

## Unstable output format

The exact output printed by this macro should not be relied upon and is subject to future changes.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The macro below is added to `src/libstd/macros.rs`,
with a doc-comment based on the [Guide-level explanation][guide-level-explanation] of this RFC.

```rust
#[macro_export]
macro_rules! dbg {
    ($expr:expr) => {
        match $expr {
            expr => {
                // The exact formatting here is not stable and may change in the future.
                eprintln!("[{}:{}] {} = {:#?}", file!(), line!(), stringify!($expr), &expr);
                expr
            }
        }
    }
}
```

The use of `match` over `let` is similar to the implementation of `assert_eq!`.
It [affects the lifetimes of temporaries](
https://stackoverflow.com/questions/48732263/why-is-rusts-assert-eq-implemented-using-a-match#comment84465322_48732525).

# Drawbacks
[drawbacks]: #drawbacks

Adding to the prelude should be done carefully.
However a library can always define another macro with the same name and shadow this one.

# Rationale and alternatives
[alternatives]: #alternatives

[RFC 2173] and provides a more complex alternative that offers more control but is also more complex.
This RFC was designed with the goal of being a simpler and thus better fit for the standard library.

## Alternative: tweaking formatting

Any detail of the formatting can be tweaked. For example, `{:#?}` or `{:?}`?

## A simple macro without any control over output

This RFC does not offer users control over the exact output being printed.
This is because a use of this macro is intended to be run a small number of times before being removed.
If more control is desired, for example logging in an app shipped to end users,
other options such as `println!` or the `log` crate remain available.

## Accepting a single expression instead of many

If the macro accepts more than one expression (returning a tuple),
there is a question of what to do with a single expression.
Returning a one-value tuple `($expr,)` is probably unexpected,
but *not* doing so creates a discontinuty in the macro's behavior as things are added.
With only one expression accepted,
users can still pass a tuple expression or call the macro multiple times.

## Including `file!()` in the output

In a large project with multiple files,
it becomes quite difficult to tell what the origin of the output is.
Including `file!()` is therefore quite helpful in debugging.
However, it is not very useful on the [playground](https://play.rust-lang.org),
but that exception is acceptable.

## Including the line number

The argument is analogous to that for `file!()`. For a large file,
it would also be difficult to locate the source of the output without `line!()`.

## Excluding the column number

Most likely, only one `dbg!(expr)` call will occur per line.
The remaining cases will likely occur when dealing with binary operators such as with:
`dbg!(x) + dbg!(y) + dbg!(z)`, or with several arguments to a function / method call.
However, since the macro prints out `stringify!(expr)`,
the user can clearly see which expression on the line that generated the value.
The only exception to this is if the same expression is used multiple times and
crucically has side effects altering the value between calls.
This scenario is probably uncommon.
Furthermore, even in this case, one can visually distinguish between the calls
since one is first and the second comes next.

Another reason to exclude `column!()` is that we want to keep the macro simple, and thus,
we only want to keep the essential parts that help debugging most.

However, the `column!()` isn't very visually disturbing
since it uses horizontal screen real-estate but not vertical real-estate,
which may still be a good reason to keep it.
Nonetheless, this argument is not sufficient to keep `column!()`,
wherefore **this RFC will not include it**.

## Including `stringify!(expr)`

As discussed in the rationale regarding `column!()`,
`stringify!(expr)` improves the legibility of similar looking expressions.

Another major motivation is that with many outputs,
or without all of the source code in short term memory,
it can become hard to associate the printed output with the logic as you wrote it.
With `stringify!`, you can easily see how the left-hand side reduces to the right-hand side.
This makes it easier to reason about the trace of your program and why things happened as they did.
The ability to trace effectively can greatly improve the ability to debug with ease and speed.

## Returning the value that was given

One goal of the macro is to intrude and disturb as little as possible in the workflow of the user.
The macro should fit the user, not the other way around.
Returning the value that was given, i.e: that `dbg!(expr) == expr`
and `typeof(expr) == typeof(dbg!(expr))` allows just that.

To see how writing flow is preserved, consider starting off with:

```rust
let c = fun(a) + fun(b);
let y = self.first().second();
```

Now, you want to inspect what `fun(a)` and `fun(b)` evaluates to.
But you would like to avoid going through the hassle of:

1. saving `fun(a)` and `fun(b)` to a variable
2. printing out the variable
3. using it in the expression as `let c = fa + fb;`.

The same logic applies to inspecting the temporary state of `self.first()`.
Instead of the hassle, you can simply do:

```rust
let c = dbg!(fun(a)) + dbg!(fun(b));
let y = dbg!(self.first()).second();
```

This modification is considerably smaller and disturbs flow while debugging code to a lesser degree.

## Keeping output when `cfg!(debug_assertions)` is disabled

When `cfg!(debug_assertions)` is false,
printing could be disabled to reduce runtime cost in release builds.
However this cost is not relevant if uses of `dbg!` are removed before shipping to production,
where crates such as `log` may be better suited,
and deemed less important than the ability to easily investigate bugs that only occur with optimizations.
These kinds of bugs [do happen](https://github.com/servo/servo/issues/19519) and can be a pain to debug.

## `STDERR` should be used over `STDOUT` as the output stream

The messages printed using `dbg!` are not usually errors,
which is one reason to use `STDOUT` instead.
However, `STDERR` is often used as a second channel for extra messages.
This use of `STDERR` often occurs when `STDOUT` carries some data which you can't mix with random messages.

If we consider a program such as `ripgrep`,
where should hypothetical uses of `dbg!` print to in the case of `rg some_word < input_file > matching_lines`?
Should they end up on the terminal or in the file `matching_lines`?
Clearly the former is correct in this case.

## Outputting `lit = lit` for `dbg!(lit);` instead of `lit`

The left hand side of the equality adds no new information wherefore it might be a redundant annoyance.
On the other hand, it may give a sense of symmetry with the non-literal forms such as `a = 42`.
Keeping `5 = 5` is also more consistent.
In either case, since the macro is intentionally simple,
there is little room for tweaks such as removing `lit = `.
For these reasons, and especially the last one, the output format `lit = lit` is used.

# Prior art
[prior-art]: #prior-art

Many languages have a construct that can be as terse as `print foo`.

Some examples are:
+ [Haskell](http://hackage.haskell.org/package/base-4.10.1.0/docs/Prelude.html#v:print)
+ [python](https://docs.python.org/2/library/pprint.html)
+ [PHP](http://php.net/manual/en/function.print-r.php)

[`traceShowId`]: http://hackage.haskell.org/package/base-4.10.1.0/docs/Debug-Trace.html#v:traceShowId

The specific idea to return back the input `expr` in `dbg!(expr)` was inspired by [`traceShowId`] in Haskell.

# Unresolved questions
[unresolved]: #unresolved-questions

Unbounded bikeshedding.
