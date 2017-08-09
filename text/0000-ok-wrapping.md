- Feature Name: ok_wrapping
- Start Date: 2017-06-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

[RFC 243]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md
[RFC 1859]: https://github.com/rust-lang/rfcs/blob/master/text/1859-try-trait.md
[RFC 1937]: https://github.com/rust-lang/rfcs/blob/master/text/1937-ques-in-main.md

# Summary
[summary]: #summary

Allow functions and catch blocks that produce a `T: Try` to internally return
`T::Ok`, eliminating the need to manually wrap that value in an `Ok`, `Some`,
`Poll::Ready`, etc.  In particular, avoid the need to end functions returning
`Result<(), E>` with an `Ok(())`.

# Motivation
[motivation]: #motivation

Thanks to [RFC 1937], we can now use `?` in `main`.  This is a big help in
using it earlier in the teaching process, instead of encouraging `.unwrap()`.

But there's still a touch of unfortunateness with it.  Take this example from
that RFC:

```rust
fn main() -> Result<(), Box<Error>> {
    let argv = env::args();
    let _ = argv.next();
    let data_path = argv.next()?;
    let city = argv.next()?;

    let file = File::open(data_path)?;
    let mut rdr = csv::Reader::from_reader(file);

    for row in rdr.decode::<Row>() {
        let row = row?;

        if row.city == city {
            println!("{}, {}: {:?}",
                row.city, row.country, row.population?);
        }
    }
    Ok(())
}
```

It's a shame that using `?`, and thus returning a `Result`, forces the
introduction of the `Ok(())`.  In comparison, the `unwrap` code could just use
the more-ergonomic `()` default from statements.

This is especially true since it's just noise.  It was fine for "getting to
the end of `main`" to mean success before, so it can be fine now.  Having
this mean success is by no means unusual, either.  `#[test]`s are also
successful if they reach the end.

Supporting the success wrapping also increases consistency.  The same RFC
also contains this example:

```rust
fn main() -> Result<(), io::Error> {
    let mut stdin = io::stdin();
    let mut raw_stdout = io::stdout();
    let mut stdout = raw_stdout.lock();
    for line in stdin.lock().lines() {
        stdout.write(line?.trim().as_bytes())?;
        stdout.write(b"\n")?;
    }
    stdout.flush()
}
```

That last line is different from how it would be anywhere else only to avoid
typing `Ok(())`; otherwise it'd be the more-consistent `stdout.flush()?;`.

This inconsistency has various suboptimal implications:

* If flushing is needed in other places, it cannot just be copy-pasted.
* If additional logic is added after it, the line will need to be touched
  anyway, and the diff will be worse than it could be.
* If a non-`io` error type is encountered (say a `ParseIntError`) and the
  return type needs to be changed, the direct return doesn't have the error
  conversion that `?` does, so will stop working.
* Not having the `?` obscures the property from [RFC 243] that the question
  mark "lets the reader determine at a glance where an exception may or may
  not be thrown".
* For the writer, this blocks the "always put a `?` after every fallible
  method call" muscle memory.

These same advantages to ok-wrapping occur with non-`()` values as well.
For example, this RFC would allow the following:

```rust
fn checked_mul_add(x: i32, y: i32, z: i32) -> Option<i32> {
    x.checked_mul(y)?.checked_add(z)?
}
```

That way the `checked_*` methods consistently have a `?`, and there's no
need to put the whole thing in a distracting `Some()`.

Ok-wrapping also avoids the explicit ["'unwrap' only to 'wrap' again"] that
can happen when using custom error types.  Today (as seen in that thread)
the code ends up being

["'unwrap' only to 'wrap' again"]: https://internals.rust-lang.org/t/unified-errors-a-non-proliferation-treaty-and-extensible-types/5361

```rust
fn something(bytes: &[u8]) -> Result<Foo> {
    Ok(Foo::parse(bytes)?)
}
```

Where the outfix `Ok()` is unhelpful, and interrupts the normal
left-to-right writing flow.

This all also applies to `catch` blocks -- so much so that [RFC 243] in fact
[requires] ok wrapping of the inner value, with an example not unlike
`checked_mul_add` above:

[requires]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#catch-expressions

```rust
catch { foo()?.bar()?.baz()? }
```

This [was not implemented](https://github.com/rust-lang/rust/issues/41414).
Adding ok-wrapping will allow use of `catch` as designed by that RFC while
keeping its behaviour equivalent to that of function bodies.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(Continuing on from [A Shortcut for Propagating Errors: `?`])

[A Shortcut for Propagating Errors: `?`]: https://doc.rust-lang.org/book/second-edition/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-

Listing 9-6 can also be simplified further by using _ok wrapping_:

```rust
use std::io;
use std::io::Read;
use std::fs::File;

fn read_username_from_file() -> Result<String, io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    s
}
```

A function that returns `Result<T, E>` can optionally be written as though it
just returned a `T`.  In that case, Rust will automatically wrap that value
into an `Ok`.  This lets the function look nearly identical to an infallible
body, other than the `?`s marking the locations of possible errors.

This is particularly helpful for functions that don't need to return anything
in the success case:

```rust
use std::io;
use std::io::Read;
use std::fs::File;

fn print_username_from_file() -> Result<(), io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    println!("{}", s);
    // not needed, thanks to ok wrapping: Ok(())
}
```

This is analagous to the infallible case, where `-> ()` methods don't need to
explicitly return the `()` value.

Note that, even with ok wrapping, a function body must still have a single
type.  For example, this is still an error:

```rust
fn read_username_or_anonymous_from_file() -> Result<String, io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    if s.is_empty() { return Ok(String::from("anonymous")); }
    s // Error: expected enum `std::result::Result`, found struct `std::string::String`
}
```

The syntactic return and block value here must both be `String`s or both be
`Result`s.  This can be made to compile by switching the syntactic return
to just `return String::from("anonymous");`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

All code that compiled before this RFC continues to compile and behave
exactly as it did before.

Otherwise, functions that return `T: Try` are rewritten as follows (using
the same ["block break"] as in [RFC 243]):

["block break"]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#early-exit-from-any-block

* The existing function `BODY` is wrapped into `Try::from_ok('a: { BODY })`
* All syntactic `return EXP` expressions in the body are changed to `break 'a EXP`
* `return`s from `?` desugaring are **not** changed

`catch` blocks are simpler, as they don't capture `return`s, so they only
need wrapping in `Try::from_ok`.

(`Try::from_ok` was introduced in anticipation of this purpose in [RFC 1859],
though that RFC does not use it itself.)

Returning to this example:

```rust
fn read_username_or_anonymous_from_file() -> Result<String, io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    if s.is_empty() { return Ok(String::from("anonymous")); }
    s // Error: expected enum `std::result::Result`, found struct `std::string::String`
}
```

It shallowly desugars to

```rust
fn read_username_or_anonymous_from_file() -> Result<String, io::Error> {
    Try::from_ok('a: {
        let mut f = File::open("hello.txt")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        if s.is_empty() { break 'a Ok(String::from("anonymous")); }
        s // Error: expected enum `std::result::Result`, found struct `std::string::String`
    })
}
```

Thus the type error happens from the mismatch between the break and the
final expression in the block.

Removing the incorrect `Ok` and desugaring the `?`s as well gives

```rust
fn read_username_or_anonymous_from_file() -> Result<String, io::Error> {
    Try::from_ok('a: {
        let mut f =
            match File::open("hello.txt").into_result() {
                Ok(x) => x,
                Err(e) => return Try::from_error(From::from(e)),
            };
        let mut s = String::new();
        match f.read_to_string(&mut s).into_result() {
            Ok(x) => x,
            Err(e) => return Try::from_error(From::from(e)),
        };
        if s.is_empty() { break 'a String::from("anonymous"); }
        s
    })
}
```

This shows the untouched `return`s in the `?` desugar.

# Drawbacks
[drawbacks]: #drawbacks

The main drawback to this design is that it's implicit, so may not trigger
when it was expected.

A function such as the following compiles, while being almost certainly not
what was desired:
```rust
fn some_default<T: Default>() -> Option<T> { Default::default() } // Produces None, never using the bound
```
But could be accidentally written after a number of other functions in which
the author relied upon the `T` -> `Option<T>` ok wrapping.


# Rationale and Alternatives
[alternatives]: #alternatives

The chosen design places restrictions to enforce consistency inside a method
and limit the locations where this is applicable:

* Only one level of ok-wrapping will be applied

```rust
fn foo() -> Option<Option<i32> {
    5 // Error: expected enum `std::option::Option`, found integral variable
}
```

* All returns, implicit and explicit, must have the same type

```rust
fn checked_neg(x: i32) -> Option<i32> {
    if x == i32::min_value() { return None }
    -x // Error: expected enum `std::option::Option`, found i32
}
```

* Ok-wrapping only applies at existing `?` boundaries

```rust
let x: Option<i32> = 4; // Error: expected enum `std::option::Option`, found integral variable
```

## Make this a general coercion

This could instead be done as a "try coercion" from `T::Ok` to `T` where
`T:Try`, adding it to the existing [coercion types].

[coercion types]: https://doc.rust-lang.org/reference/type-coercions.html#coercion-types

Coercions have properties that are undesirable from this feature however:

* Coercions are transitive, so this would enable things like `i32`
  to `Result<Option<i32>, WhateverError>`.
* Coercions apply in far more locations than are `?` boundaries.  Notably
  function arguments are a coercion site, so adding a "try coercion" would
  mean the ["into trick"] would happen on everything taking an `Option`.
  It would also allow hetrogeneous-appearing array literals
  like `[4, Ok(4), 7, Err(())]`.

["into trick"]: http://www.suspectsemantics.com/blog/2016/11/29/the-into-trick/

## Use an explicit marker on the functions/blocks

Having some sort of explicit request for this behaviour would eliminate
the ambiguous cases.

One possible design would be to allow `?` in function definitions,
mirroring its use in function calls:
```rust
fn always_five()? -> Result<i32, !> { 5 }
```
That allows errors in both directions
```rust
fn always_five() -> Result<i32, !> { 5 } // Error: expected enum `std::result::Result`, found integral variable
fn always_five()? -> Result<i32, !> { Ok(5) } // Error: expected i32, found enum `std::result::Result`
```
And lets one choose explicitly how the body's type should be inferred:
```rust
fn def1() -> Option<i32> { Default::default() } // gives None
fn def2()? -> Option<i32> { Default::default() } // gives Some(0)
```

This RFC asserts that introducing such a syntax is unnecessary in practice.
It would be particularly unfortunate to need such a sigil on `main` at the
very beginning, as the first non-main functions written by a beginner would
not have it.

(The particular demonstration syntax above has the problem that the `?`
looks like it's part of the type, but isn't.)

## Don't touch `return`s

The `catch` case only needs to wrap the value of the block; one could
choose to also only wrap the value of the block in functions, not
wrapping `return`ed values.

Doing so, however, adds friction to the process of turning a `-> T` function
into a `-> Result<T, E>` function.  With this RFC's proposed design, that
change will always compile (though may result in different inference).
Notably, [RFC 1937] would like to change the [generated main for doctests]
to `-> Result<(), ErrorT>`, which will only be compatible if `return;`
continues to work in such a method.

[generated main for doctests]: https://github.com/rust-lang/rfcs/blob/master/text/1937-ques-in-main.md#changes-to-doctests

## Handle only the `Ok(())` case

`()` is already special, being the result of `else`-less `if` blocks,
expression statements, and more.  It could be made more special to handle
the particularly-egregious `Ok(())` problem, and punt on ok wrapping for
other types.  This might be done by adding a coercion from `()` to
`Try::from_ok(())`, as the coercion downsides are less-bad with a unit type.

This would make the "usually `?` but sometimes not" inconsistency worse,
however, as it'd only be fixed for `()`.

## Do nothing

Always an option.  This doesn't increase the power of the language, and
some feel that `Ok(())` just isn't that bad, or even that it helps
understanding the expression-orientated nature of Rust.

# Unresolved questions
[unresolved]: #unresolved-questions

## Types in nested wraps

What should the following method do, for example?

```rust
let _: Option<i32> =
    catch {
       let x = catch { 4 };
       println!("{:?}", x);
    };
```

Both `4` and `Some(4)` are logically-consistent output, depending which
catch block is chosen to do the wrapping. (Yes, `let _: i32 = do catch { 4 };`
compiles today (2017-08-07) in nightly play.  If that changes, the example
still applies by adding another level of options.)

An equivalent example can also be produced using closures.

## Additional restrictions

Are there any further limitations that could be placed on it to restrict
surprises without losing the ergonomic advantages?  Perhaps it should only
be allowed in methods that use `?`?

# Future Possibilities

## `throw` sugar

Because this disallows having both implicit-`Ok` and explicit `return Err`
in the same method, it may encourage a pattern like this:

```rust
struct IntMinError;
fn checked_neg(x: i32) -> Result<i32, IntMinError> {
    if x == i32::min_value() { Err(IntMinError)? }
    -x
}
```

It may be nice to add a [`throw x`] sugar to avoid `Err(x)?`, but as in
[RFC 243], that can be left for the future.

[`throw x`]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#throw-and-throws
