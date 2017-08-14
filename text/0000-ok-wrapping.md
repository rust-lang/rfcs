- Feature Name: ok_wrapping
- Start Date: 2017-06-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

[RFC 243]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md
[RFC 1859]: https://github.com/rust-lang/rfcs/blob/master/text/1859-try-trait.md
[RFC 1937]: https://github.com/rust-lang/rfcs/blob/master/text/1937-ques-in-main.md

# Summary
[summary]: #summary

Add additional help for writing functions in an error handling mindset.
Support a `?` on function definitions and `catch` blocks that automatically
wraps the result in an `Ok`, `Some`, `Poll::Ready`, etc.  In particular,
allow function authors to avoid the need for `Ok(())` at the end of
functions returning `Result<(), E>`.

(Note that this _ok wrapping_ only ever applies to the success case.  At no
point would this allow a function to implicitly return `Err(e)`, `None`,
`Poll::NotReady`, etc.)

# Motivation
[motivation]: #motivation

The question mark operator ([RFC 243]) has made great strides in the
ergonomics of error handling in Rust.  Having an explicit-but-short marker
for fallible locations that propagates the error on failure allows function
authors to operate in an error handling mindset, concentrating on the
success path while acknowledging the failure path.

There's still one place where this model breaks down, though: when
generating the return value.

Take this example from [RFC 1937]:

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

And compare the `unwrap`-equivalent one might see without that RFC:

```rust
fn main() {
    let argv = env::args();
    let _ = argv.next();
    let data_path = argv.next().unwrap();
    let city = argv.next().unwrap();

    let file = File::open(data_path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);

    for row in rdr.decode::<Row>() {
        let row = row.unwrap();

        if row.city == city {
            println!("{}, {}: {:?}",
                row.city, row.country, row.population.unwrap());
        }
    }
}
```

Two of the differences are fundamental to error handling:

1. The signature is different.
2. The `.unwrap()`s have turned into `?`s.

But then there's the ergonomically-unfortunate piece:

3. An `Ok(())` showed up.

This is especially true since it's just noise.  It was fine for "getting to
the end of `main`" to mean success before, so it can be fine now.  Having
this mean success is by no means unusual either.  `#[test]`s are also
successful if they reach the end.  And the `?` operator only further
emphasizes this, as many functions using it only error via propagation.

In practice, `Ok(())` is so unergonomic that people go out of their way to
avoid it.  The same RFC also contains this example:

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

* Not having the `?` obscures the property from [RFC 243] that the question
  mark "lets the reader determine at a glance where an exception may or may
  not be thrown".
* For the writer, this blocks the "always put a `?` after every fallible
  method call" muscle memory.
* If a non-`io` error type is encountered (say a `ParseIntError`) and the
  return type needs to be changed, the direct return doesn't have the error
  conversion that `?` does, so will stop working.  (Note that, unlike the
  earlier example, this doesn't use `Result<(), Box<Error>>`.)
* If additional logic is added after it, the line will need to be touched
  anyway, and the diff will be worse than it could be.
* If flushing is needed in other places, it cannot just be copy-pasted.

These same advantages to ok-wrapping occur with non-`()` values as well.
For example, this RFC would allow the following:

```rust
fn checked_mul_add(x: i32, y: i32, z: i32)? -> Option<i32> {
    x.checked_mul(y)?.checked_add(z)?
}
```

That way the `checked_*` methods consistently have a `?`, and there's no
need to put the whole thing in a distracting `Some()`.

This consistency particularly helps when a previously-infallible method
needs to start calling something fallible or when a fallible form of
something that currently just panics is needed.

As a simple example, the previous snippit is the checked equivalent of

```rust
fn mul_add(x: i32, y: i32, z: i32) -> i32 {
    x.mul(y).add(z)
}
```

The translation between the two is simple, and only involves touching
things where failure points are introduced:

1. Change the definition from `-> i32` to `? -> Option<i32>`
2. Introduce fallible calls by changing `.foo(w)` to `.checked_foo(w)?`

Ok-wrapping also avoids the explicit ["'unwrap' only to 'wrap' again"] that
can happen when using custom error types.  Today the code ends up being

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

This [was not implemented](https://github.com/rust-lang/rust/issues/41414)
out of a desire to keep function bodies and `catch` bodies in sync.

With this RFC, the example would become

```rust
catch? { foo()?.bar()?.baz()? }
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(Continuing on from [A Shortcut for Propagating Errors: `?`])

[A Shortcut for Propagating Errors: `?`]: https://doc.rust-lang.org/book/second-edition/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-

Listing 9-6 can also be simplified further by using _ok wrapping_:

```rust
use std::io;
use std::io::Read;
use std::fs::File;

fn read_username_from_file()? -> Result<String, io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    s
}
```

By adding the `?` to the definition, the function body opts into ok-wrapping.
This allows the function to be written from an error handling mindset,
concentrating on the success path and marking possible failure points with
`?`.  Ok-wrapping will automatically wrap the value of the function body in
`Ok`.  In this case, it means the function must return `String`, not a
`Result`.  As such, `Err`s can only come through `?`.

This is particularly helpful for functions that don't need to return anything
in the success case:

```rust
use std::io;
use std::io::Read;
use std::fs::File;

fn print_username_from_file()? -> Result<(), io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    println!("{}", s);
    // not needed, thanks to ok wrapping: Ok(())
}
```

This is analogous to the infallible case, where `-> ()` methods don't need to
explicitly return the `()` value.

Ok-wrapping can be used with any return type that implements `Try`:

```rust
fn get_two<T>(slice: &[T], i: usize, j: usize)? -> Option<(&T,&T)> {
    (slice.get(i)?, slice.get(j)?)
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar updates

[Closure Types](https://doc.rust-lang.org/grammar.html#closure-types)

```diff
 closure_type := [ 'unsafe' ] [ '<' lifetime-list '>' ] '|' arg-list '|'
+                [ '?' ]
                 [ ':' bound-list ] [ '->' type ]
```

Similarly for functions, an optional `?` before the `->`.

And for `catch`, an optional `?` before the block.

## Desugar

Functions and closure that opt into ok-wrapping are rewritten as follows
(using the same ["block break"] as in [RFC 243]):

["block break"]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#early-exit-from-any-block

* The existing function `BODY` is wrapped into `Try::from_ok('a: { BODY })`
* All syntactic `return EXP` expressions in the body are changed to `break 'a EXP`
* `return`s from `?` desugaring are **not** changed

`catch` blocks are simpler, as they don't capture `return`s, so they only
need wrapping in `Try::from_ok`.

(`Try::from_ok` was introduced in anticipation of this purpose in [RFC 1859],
though that RFC does not use it itself.)

Take this example:

```rust
fn read_username_or_anonymous_from_file()? -> Result<String, io::Error> {
    let mut f = File::open("hello.txt")?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    if s.is_empty() { return String::from("anonymous"); }
    s
}
```

It shallowly desugars to

```rust
fn read_username_or_anonymous_from_file() -> Result<String, io::Error> {
    Try::from_ok('a: {
        let mut f = File::open("hello.txt")?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        if s.is_empty() { break 'a String::from("anonymous"); }
        s
    })
}
```

Desugaring the `?`s as well gives

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

## Error messages

With just the desugar above, this example

```rust
fn foo()? -> i32 { 4 }
```

Will give the error "the trait bound `i32: std::ops::Try` is not satisfied",
but will likely point to the result of the function, not the return type.

Instead, it should give an error pointing at the question mark with an
error like "cannot use ok-wrapping in a function that returns `i32`",
with a suggestion such as "consider changing the return type to
`Result<i32, Box<Error>>`".

Because ok-wrapping is explicitly requested, type mismatch errors will
happen naturally in both directions:

```rust
fn always_five() -> Result<i32, !> { 5 } // Error: expected enum `std::result::Result`, found integral variable
fn always_five()? -> Result<i32, !> { Ok(5) } // Error: expected i32, found enum `std::result::Result`
```

And the function author chooses explicitly how the type of its body
should be inferred:

```rust
fn def1() -> Option<i32> { Default::default() } // gives None
fn def2()? -> Option<i32> { Default::default() } // gives Some(0)
```

# Drawbacks
[drawbacks]: #drawbacks

The `?` appears to be part of the signature, but it's not.  A function like
`fn foo()? -> Option<i32>` coerces to `fn()->Option<i32>`; there's no such
thing as a `fn()?->Option<i32>`.  This is not the only thing that appears in
function definitions that's not part of their type, however.  Patterns for
parameters, particularly `mut`, are also part of the definition without
affecting the signature.  And having the `?` there means it mirrors the
position of a `?` used when calling the function: `let x: i32 = foo()?;`.

Needing an explicit `?` on `main` to avoid `Ok(())` means one more thing
to teach in order to use `?` in main.  The first non-`main` function a
newcomer will write will likely be infallible, and thus will not have the
`?`, adding another difference.  `main` is already special in a number of
ways, however, and a definition like `fn main() -> Result<(), Box<Error>>`
is complex enough even without ok-wrapping that the book may avoid it
anyway for a first program.

# Rationale and Alternatives
[alternatives]: #alternatives

The desugaring is chosen to enforce consistency inside a method.

* Only one level of ok-wrapping will be applied

```rust
fn foo()? -> Option<Option<i32>> {
    5 // Error: expected enum `std::option::Option`, found integral variable
}
```

* All returns, implicit and explicit, must have the same type

```rust
fn checked_neg(x: i32)? -> Option<i32> {
    if x == i32::min_value() { return None }
    -x // Error: expected enum `std::option::Option`, found i32
}
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
  It would also allow heterogeneous-appearing array literals
  like `[4, Ok(4), 7, Err(())]`.

["into trick"]: http://www.suspectsemantics.com/blog/2016/11/29/the-into-trick/

## Omit the explicit marker on the functions/blocks

It may be possible to allow the compiler to apply ok-wrapping
automatically in situations where the code doesn't compile today.

That would be simpler to use, but can have surprising behaviours.

A function such as the following compiles, while being almost certainly not
what was desired:

```rust
fn some_default<T: Default>() -> Option<T> { Default::default() } // Produces None, never using the bound
```

But could be accidentally written after a number of other functions in which
the author relied upon the `T` -> `Option<T>` ok wrapping.

There are also ordering complications since in certain examples it's
unclear which of multiple possible blocks should have ok-wrapping
applied.  Take this example:

```rust
let _: Option<i32> =
    catch {
       let x = catch { 4 };
       println!("{:?}", x);
    };
```

Both `4` and `Some(4)` are logically-consistent output, depending which
catch block is chosen to do the wrapping. (Yes,
`let _: i32 = do catch { 4 };` compiles today (2017-08-07) in nightly
play. If that changes, the example still applies by adding another
level of `Option<>`.)

An equivalent example can also be produced using closures.

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

## `-> T throws E` syntax

This was [mentioned] as a future possibility in [RFC 243].  It proposes
`-> T throws E` where this RFC has `? -> Result<T, E>`.

[mentioned]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#throw-and-throws

Restricting this to `Result` would be a real shame when [RFC 1859] was
just accepted to extend `?` to more types, `Option` particularly.  The
two general options are 1) a new type variable with a `Try` bound, or
2) `impl Try<Ok=T, Error=E>`.

Unfortunately, neither of those are backwards compatible to use this
with an existing `Result`-returning method.  A new variable cannot be
inferred by use with `?`, nor with inherent methods.  `impl Try` could
allow some combinators (`and` could be written on the existing trait)
but not all (`map` needs ATC, which don't exist yet and isn't on the
trait).  And even on new methods, these both lose the valuable
distinction between `Result` and `Option`, in terms of `must_use` as
well as things like `ok` vs `ok_or` inherent methods.

## Do nothing

Always an option.  This doesn't increase the power of the language, and
some feel that `Ok(())` just isn't that bad, or even that it helps
understanding the expression-orientated nature of Rust.

# Unresolved questions
[unresolved]: #unresolved-questions

* Bikeshedding on syntax

# Future Possibilities

## `throw` sugar

Because this disallows having both implicit-`Ok` and explicit `return Err`
in the same method, it may encourage a pattern like this:

```rust
struct IntMinError;
fn checked_neg(x: i32)? -> Result<i32, IntMinError> {
    if x == i32::min_value() { Err(IntMinError)? }
    -x
}
```

It may be nice to add a [`throw x`] sugar to avoid `Err(x)?`:

```rust
struct IntMinError;
fn checked_neg(x: i32)? -> Result<i32, IntMinError> {
    if x == i32::min_value() { throw IntMinError }
    -x
}
```

But as in [RFC 243], it can be left for the future.

[`throw x`]: https://github.com/rust-lang/rfcs/blob/master/text/0243-trait-based-exception-handling.md#throw-and-throws
