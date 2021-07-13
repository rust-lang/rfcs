- Feature Name: `box_error_alias`
- Start Date: 2019-11-20
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a type alias to std::error of the form

```rust
pub type BoxError = Box<dyn Error + Send + Sync>;
```

which given the absence of a lifetime is also `'static`.

# Motivation
[motivation]: #motivation

Rust error handling has a long and complicated history, and it is still evolving right now. This
reflects the fact that errors perform a multitude of functions, and these different functions
require different implementations: for example, if errors are regularly encountered in a real-time
thread, then one would not want their creation to involve an allocation or the indirection of a
trait object, whereas for a CLI app, most errors may not occur on the happy path, and so
their performance is inconsequential. Likewise, a low-level library may want fine-grained control
of error handling and recovery, whereas a CLI app may only want to handle a small subset of
errors, simply printing a message to screen for the rest.

The error trait in Rust's standard library provides a helpful way to perform two tasks: firstly it
requires a `Display` implementation - so all errors can be logged/written/etc, and secondly it
provides the error with the option of exposing another inner error, thereby allowing chains of
errors to be created. [Research into the best way to represent errors](https://internals.rust-lang.org/t/thoughts-on-error-context-in-error-handling-libraries/10349/4)
is ongoing in Rust, and this RFC does not attempt to resolve the discussion around how error
handling in Rust should evolve. Instead, it proposes a simple type alias to serve the following
motivations

 1. A unified name for boxed errors, to make them easier to recognise,
 2. A short descriptive name for error trait objects, to reduce code noise and cognitive load,
 3. A place to show the value of making errors `Send + Sync + 'static`, and
 3. A place in the standard library to document the pattern of using a trait object to return any
    error, aiding discoverability.

Currently what happens in practice is Rust programmers either create an alias for
`Box<dyn Error + Send + Sync>` in some utility module, where each user will use a
sligtly different name, or just write out the full type that `BoxFuture` aliases.

This idea evolved on a [thread in irlo](https://internals.rust-lang.org/t/proposal-add-std-boxerror/10953)
before this RFC was written, and there names such as `AnyError` were suggested, as well as using an
opaque object with a raw pointer as is the case in the [`failure` crate][`failure`].
However, this RFC only satisfies the narrow goal of providing a trait alias for `Box<Error + ..>`,
allowing more experimentation to take place regarding the specific design of an opaque and more
full-featured error type in the crate ecosystem.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Once you start doing anything more advanced than arithmetic with a computer, you will encounter
routines of the computer that may fail. The source of these failures may be hardware-related (e.g.
a resistor has shorted in your temperature sensor peripheral), a bug in some code you (or someone
else) wrote, the misuse of an interface, or a network failure, amongst many others. Ideally we
design programming languages and libraries so that programming mistakes are caught when the program
is compiled rather than at run-time, but obviously this is not always possible. It is therefore
necessary to have run-time error handling.

Not all errors are the same: sometimes an error can be corrected on the fly, sometimes an operation
must be re-tried (e.g. a tcp packet failed to arrive), and sometimes the error represents something
catastrophic and the best response is to terminate the program with a useful error message. These
different errors require different types of data: an error that will be instantly corrected should
be able to choose whether it is stored on the heap or the stack, whereas for an error that will
terminate the program the performance impact of a trait object/allocation will be negligable. The
functionality of errors may be further constrained by the requirements of the programming
environment: an embedded chip with low memory performing tasks in realtime may not have run-time
memory allocation as an option.

In general, Rust programmers will create different types to handle these different situations. The type
for a recoverable error may just be some fixed-size block of data, whereas a more serious error may
contain references to descriptions of the error or other errors that cause it. Rust provides the
`Error` trait, which includes converting the error into a string and referencing any underlying
error, but Rust programmers can still implement this trait even though they don't want to be allocating
strings or inner errors, since it is the caller of the methods who causes the allocations/etc. In
this way, Rust allows all errors to be able to describe themselves, which is very useful during
development and prototyping, as well as helping to support different use cases for library code.

Whist the `Error` trait helps error authors to provide self-describing errors, it does not help
code using these errors to easily combine them with other errors of different types, and displaying
the contents of an error that might be one of multiple types. The standard way to solve this
problem is to create a trait object behind a thick pointer, hence the signature `Box<dyn Error..>`.
The trait object obscures the actual type, instead presenting the `Error` interface, and is easily
created from any error using the `impl From<'a, E: Error + Send + Sync + 'a> for Box<dyn Error +
Send + Sync + 'a>`.

`BoxError` is simply a synonym for `Box<dyn Error + Send + Sync>`. Here are some examples
showing its usefulness.

## Example: Simple database function.

Here we are getting a database version from a file and returning it. We are in the prototyping
stage and don't want to devote too much time to complex error handling.

```rust
fn db_version() -> Result<usize, BoxError> {
    let version = fs::read_to_string("/path/to/version/file")?
        .parse()?;
    Ok(version)
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation is very straight forward, simply the code in the [summary].

# Drawbacks
[drawbacks]: #drawbacks

 1. Increases std api surface area, although semantically there is no change since it only a type
    alias is added.
 2. Could potentially encourage people to use it instead of a specialized type like
    [`anyhow::Error`](https://docs.rs/anyhow/latest/anyhow/struct.Error.html).

   - I would challenge this drawback with the argument that even after the addition of a
     hypothetical `AnyError`, there will still be times when a `BoxError` is appropriate for its
     simplicity. It may turn out that this satisfies 99% of use cases for type-erased errors, and
     any more specialized solutions can live in crates.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This RFC proposes a very small addition that provides a type alias that many people define
manually. The only real alternative is to do nothing and maybe land a more specialized type-erased
error type in the future.

The rationale for not including a specialized error type is that it is not clear what the best
design for it would be, especially since the story around backtraces is not yet finalised. It's
also not clear if a specialized type is necessary. The disadvantages of `BoxError` over a
hypothetical `AnyError` are

  1. An increased memory footprint (increasing the size of `Result` in the case that the `Ok`
     variant is small), but with fewer memory indirections. If this is an issue, then a concrete
     error type is more appropriate.
  2. The `Debug` implementation defers to the inner error type. The hypothetical `AnyError` could
     use the inner error's `Display` type, leading to better messages in the
     `fn main() -> Result<(), AnyError>` case.


# Prior art
[prior-art]: #prior-art

There are many error models in the Rust ecosystem, including many type-erased error types. Crates
include [`anyhow`] and [`failure`] among others. Prior art for a type synonym is the many crates
that define it internally.

An example of prior art of name choice is the [`BoxFuture`] type in the [`futures`] crate.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - Should the error be `Send + Sync`. It seems like a good default, and nothing is stopping Rust
   programmers simply not using the alias if they want a `Box<dyn Error>`.

# Future possibilities
[future-possibilities]: #future-possibilities

There are many ways that the Error trait could be enhanced, for example by providing a dedicated
`AnyError` type, or providing a method like `Error::wrap` that would take an error, and make it the
source of an `AnyError`. This RFC attempts to be very focused on a small change, and so these ideas
are better discussed elsewhere.

[`anyhow`]: https://github.com/dtolnay/anyhow
[`failure`]: https://github.com/rust-lang-nursery/failure
[`BoxFuture`]: https://docs.rs/futures/0.3.1/futures/future/type.BoxFuture.html
[`futures`]: https://github.com/rust-lang-nursery/futures-rs
