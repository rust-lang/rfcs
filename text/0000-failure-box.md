- Start Date: 2014-11-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes changes to the `Error` trait and the introduction of a
`Failure<T>` wrapper type for errors and modifications to the `Error` trait for
better error interoperability and debugging support.

The changes as an overview:

* The `Error` trait will be modified to:
  * gain a mandatory `name()` method that returns the human readable name of
    the error.
  * the `description()` method will be changed to allow an optional description
    rather than a mandatory one.
  * it gains the `'static` trait bound to support `TypeId` to be used internally.
  * it subtypes `Clone` in additional to `Send` to allow cloning of errors.  This
    comes in useful for supporting debug information in tracebacks.
* It introduces the `Failure<T>` box type which works similar to `Box<T>` but
  only holds errors and has a different internal implementation in release builds
  to support building tracebacks.
* It introduces a `fail!` macro which abstracts over `FromError` to support the
  automatic creation of `Failure<T>` boxes.
* It changes the behavior of the `try!` macro to automatically use the `fail!`
  macro in the `Err` clause.

The traceback support can be kept optional in the beginning but ideally it lands
not long after the initial support.  The reason for this is that propagating
errors without location information is a lot more painful to debug than
to immediately panic.  Especially if the application ends up bubbling errors
up to a central point to render out an error message it becomes basically
impossible to tell the origin of the error if multiple places exist that produce
the same error.

# Intended Usage

After this RFC has been implemented libraries are supposed to only emit errors
that implement a confirming `Error` trait.  This RFC also changes the `try!`
macro to only work with errors in the future which will encourage libraries to
properly implement `Error`.

The usage of `Failure<T>` as provided by this RFC is entirely optional and
partially by libraries.  `Failure<T>` should be used by applications and
frameworks rather than libraries.  However if libraries have huge error types
(that might even wrap other errors) then `Failure<T>` is encouraged over
using results with huge error structs.

# Incompletenes

This RFC is incomplete and does not provide all implementation details for
`ConstructFailure`.  The reason for this is that an entire reference implementation
can be found in `rust-incidents` [2] and would be too long for the scope of
the RFC.  However all necessary implementation details are provided.

# Motivation

The introduction of the `FromError` and `Error` trait have transformed how error
handling in Rust works.  It made many great improvements over the most popular
error handling concept in Rust which is based on error conversion at library
boundaries.  Instead of libraries swallowing internal errors entirely or using
`Any` or similar concepts to propagate it onwards unchanged, libraries found a
clever middle ground where they can report their own error while still retaining
the original error information internally.

There are however some issues with the current error design that need addressing.
As the overall trend is to make APIs panic less, the use of results that carry
errors will become more widespread.  This means we need to be careful not to
advocate patterns that could have negative impact on the runtime behavior of
code using them.

At present the biggest problem with this pattern is that errors have grown to
very large structs which mean that results are now much bigger on the `Err` side
than the `Ok` side for a lot of code.  This is especially odd given that the
expectation is that the vast majority of code will not cause errors.  The
solution to this problem is to box the error up and move it to the heap.  While
heap allocation can sound controversial, the truth is that an `IoResult<u8>` is
currently 72 bytes large.  Given that the vast majority of IO calls will actually
succeed this is a huge overhead.

The problem manually boxing up introduces is that now there is a layer of
abstraction between the error and what it's represented at, which forces
developers to implement the error trait for the box as well.  It also makes
pattern matching over data contained on the error a lot more awkward as it
requires deref-ing the error first.

By using a error specific box (called `Failure<T>`) it opens up not only the
possibility to customize the error handling code to make the box more implicit,
but also to allow it to carry additional debug information when necessary.

In addition the new `try!` macro supports the interoperability between failures
and errors so it becomes entirely possible to write as much API as necessary
with just using errors instead of failures and to avoid heap allocation.  This
will allow APIs to just provide error enums as errors and then to use them in
failures in callers.

As this RFC makes generic error formatting more likely it also sets up some
guidelines for how errors should report information.  It also introduces a
mandatory name function on all errors which greatly aids the error messages
that are available to a user.  For generic error handling it also requires
the `'static` trait bound now which allows the internal use of `TypeId`
which supports more advanced generic error handling for debugging.

A good error handling API will also make it less common for Rust code to
panic which will greatly improve the experience for people that write libraries
exposed via a C ABI and OS code.  This is especially true if we can later put
syntax support such as the proposed `?` operator[1] around it.

# RFC Priority Defense

The error handling situation is currently not idea in Rust and it has some
big impact on the kind of APIs that are being created.  As we are approaching
a stable language revision it's important to get error handling right or we
will fundamentally suffer from this issue in the standard and core libs.

While some work can be deferred, the code support that already is in the
stdlib needs to be compatible enough that missing functionality can be
added at a later point.

# Detailed design

There are multiple parts to the RFC.  Most of this is based on the prior work
in the `rust-incidents` library [2] which demonstrates that the general
functionality can be provided.

## Changes To Error Trait

In order to facilitate good error reporting for both developers and end users
the error trait is changed to the following:

```rust
trait Error: 'static + Send + Clone {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str> { None }
    fn detail(&self) -> Option<String> { None }
    fn cause(&self) -> Option<&Error> { None }
}
```

`rust-incidents` internally also adds an undocumented additional function
called `get_error_type` which returns the `TypeId` of the error.  This is
done in order to support limited reflection for the traceback support.  More
about this later.  The implementation of this function looks like this:

```rust
fn get_error_type(&self) -> TypeId { TypeId::of::<Self>() }
```

The intended behavior of the functions added:

name():

> Return the capitalized name of the error.  This can be a variation of the
> type name, but might also be conditional to the data stored in the error.
> For instance for IO Errors the implementation can show "File Not Found"
> if the kind is `IoErrorKind::FileNotFound`.
>
> This is the only required method.

description():

> An optional static description of what this error means in detail.  This
> should only be provided if the name of the error is not clear enough by
> itself.  There is no point in repeating "the file does not exist" if
> the error is already called "File Not Found".  However it would make sense
> to provide information about why the file was not found.  For instance
> "was not able to find configuration file" is a good description.

detail():

> Detail is intended to be optionally calculated on the fly from error information
> stored on the error.  It's not intended that the detail is stored as such
> directly on the error as a formatted string.  The idea is that the detail
> information might be more expensive to create and is only interesting for
> error formatting.
>
> Under no circumstances should anyone ever be forced to parse the error
> detail.  All information contained within that might be relevant for
> error handling should be exposed as individual fields or through accessor
> functions.  For instance if a file does not exist, and the filename is
> relevant information it should be stored separately.
>
> Detail should never contain information that is already in description.
> It does not replace description as information, it just augments it.

cause():

> If an error was caused by another error and the conversion happened through
> `FromError` it can be a good idea to expose the original error here.  There
> is no guarantee and requirement that this happens.  This also is completely
> orthogonal to the optional traceback support.  See later for more information.

## Error Trait Bounds and TypeId

Errors are currently considered `Send` but it might make sense to drop this
requirement.  The particular trait bounds are up to discussion.  The `'static`
bound is not entirely necessary but it is required if `TypeId` should be
provided.  The implementation of the tracebacks currently heavily depends on
this.  It also is a requirement for working with `cause()` properly.

`rust-incidents` provides an error extension that provides a way to cast and
typecheck an error:

```rust
trait ErrorExt<'a> {
    fn is<E: Error>(self) -> bool;
    fn cast<E: Error>(self) -> Option<&'a E>;
}

impl<'a> ErrorExt<'a> for &'a Error {
    fn is<E: Error>(self) -> bool {
        self.get_error_type() == TypeId::of::<E>()
    }

    fn cast<E: Error>(self) -> Option<&'a E> {
        if self.is::<E>() {
            unsafe {
                let to: raw::TraitObject = mem::transmute_copy(&self);
                Some(mem::transmute(to.data))
            }
        } else {
            None
        }
    }
}
```

This has been a heavily discussed point in the past and it would be possible
to defer this point for later.  In any case the `'static` trait bound should
be added so that dynamic errors can be added back later.

## Failures

Failures are an intelligent wrapper for errors that move errors to the heap.  In
a way they work like `Box<T>` but they are specific to errors.  This allows greater
flexibilty by knowing the subset they operate on.  In `rust-incidents` this means
that they can handle tracebacks which greatly aids the debugging experience.

The following layout is proposed:

```rust
struct Failure<E: Error> {
    error: Box<E>,
}
```

To support tracebacks additional traceback information can be included in debug
builds.  In `rust-incidents` the failure is implemented by storing the error
in a box in release, an by moving it into the traceback in debug:

```rust
struct Failure<E: Error> {
    #[cfg(ndebug)]
    error: Box<E>,
    #[cfg(not(ndebug))]
    traceback: Traceback,
}
```

If the language supports in the future it might be an option to also remove the
box if the wrapped type is word-sized.

Currently there is a limitation in the optimizer that makes `Result<(), Failure<E>>`
larger than `Result<(), Box<E>>` but it should be possible to improve this in the
compiler.  This means that the very common case of returning unit or an error should
only be as large as a single pointer.

Failures implement deref:

```rust
impl<E: Error> Deref<E> for Failure<E> {
    #[cfg(ndebug)]
    fn deref(&self) -> &E {
        &*self.error
    }
}
```

And if tracebacks are supported the implementation changes slightly for the
debug case.

## Failure and Error Construction

At present the `try!` macro goes through the `FromError` trait directly.  This RFC
introduces a new level of indirection which allows debug information to be captured
if necessary.  In addition it also enables one way interoperability between errors
and failures.

```rust
trait ConstructFailure<A> {
    fn construct_failure(args: A, loc: Option<LocationInfo>) -> Self;
}
```

The purpose of the `ConstructFailure` trait is to support the interoperability between
errors and failures.  In `rust-incidents` the following conversions are implemented:

```rust
impl<E: Error> ConstructFailure<(E,)> for E
impl<E: Error, D: Error + FromError<E>> ConstructFailure<(Failure<E>,)> for D
impl<E: Error, T: Error + FromError<E>> ConstructFailure<(E,)> for Failure<T>
impl<E: Error, C: Error, T: Error + FromError<E>> ConstructFailure<(E, Failure<C>)> for Failure<T>
impl<E: Error, C: Error, T: Error + FromError<E>> ConstructFailure<(E, C)> for Failure<T>
impl<E: Error> ConstructFailure<(Failure<E>,)> for Failure<E>
```

This allows the following trivial conversions:

* `E: Error` to `Failure<E: Error>`
* `Failure<E: Error>` to `E: Error`
* `Failure<E> to Failure<E>`

And the following extended:

* `E: Error` to `Failure<X: Error>` where `X: FromError<E>`
* `Failure<E: Error>` to `X: Error` where `X: FromError<E>`
* `Failure<E: Error> to Failure<X: Error>` where `X: FromError<E>`

Additionally the argument to construct failure is a tuple which allows additional
parameters to be passed in as well.  For instance there is an implementation of
constructing a failure with an original failure or error as cause which supports
the traceback.

## Tracebacks

The benefit of the failure wrapper type over a regular box is not just that it is
error specific and can as such be customized for `ConstructFailure` without causing
confusion, but also that it can internally support tracebacks.  In `rust-incidents`
it has been demonstrated that this is a viable option and can greatly aid debugging.

Internally tracebacks are based on frames where each frame is of different
implementation defined types.  There are frames that hold original errors, frames
that just link back to the previous frame and frames that replaces an error with
another error (errors with causes).  In `rust-incidents` the failure actually only
holds a traceback in debug builds instead of the error, and the error is actually
held somewhere in the linked traceback.  While this makes the debug builds slower by
having to traverse the traceback to find the error, it has the benefit that it works
through safe code internally and does not have to fall back to `Rc` or similar
wrappers.

These tracebacks are created through `ConstructFailure`.  As an example here the
implementation of `ConstructFailure` that creates a brand new traceback with an
error frame:

```rust
impl<E: Error, T: Error+FromError<E>> ConstructFailure<(E,)> for Failure<T> {
    #[cfg(ndebug)]
    fn construct_failure((err,): (E,), _: Option<LocationInfo>) -> Failure<T> {
        Failure {
            error: box FromError::from_error(err),
        }
    }

    #[cfg(not(ndebug))]
    fn construct_failure((err,): (E,), loc: Option<LocationInfo>) -> Failure<T> {
        let err: T = FromError::from_error(err);
        Failure {
            traceback: Traceback {
                frame: box BasicErrorFrame {
                    error: err,
                    location: loc,
                } as Box<Frame + Send>
            }
        }
    }
}
```

As you can see, the implementation is different depending on the `ndebug` config.
This in itself just makes an empty traceback.  The bubbling of the error through
tracebacks is created by another `ConstructFailure`:

```rust
impl<E: Error> ConstructFailure<(Failure<E>,)> for Failure<E> {
    #[cfg(ndebug)]
    fn construct_failure((parent,): (Failure<E>,), _: Option<LocationInfo>) -> Failure<E> {
        parent
    }

    #[cfg(not(ndebug))]
    fn construct_failure((parent,): (Failure<E>,), loc: Option<LocationInfo>) -> Failure<E> {
        Failure {
            traceback: Traceback {
                frame: box PropagationFrame {
                    parent: parent.traceback.frame,
                    location: loc,
                } as Box<Frame + Send>
            }
        }
    }
}
```

## The Macros

To support all of this new machinery the `try!` macro needs to change and a new
`fail!` macro needs to be introduced.

The fail macro:

```rust
macro_rules! fail {
    ($($expr:expr),*) => ({
        #[cold]
        #[inline(never)]
        fn fail<A, X, T: ::incidents::ConstructFailure<A>>(args: A) -> Result<X, T> {
            Err(::incidents::ConstructFailure::construct_failure(
                args,
                if cfg!(ndebug) {
                    None
                } else {
                    Some(::incidents::LocationInfo::new(file!(), line!()))
                }
            ))
        }
        return fail(($($expr,)*));
    });
}
```

This is a bit more complicated than strictly necessary but this should help the
compiler to make the error path less interesting than the success path.  Depending
on if debug is enabled or not, debug information is included.  The idea of holding
a "burned in" filename and line is quite controversial but there are alternatives
possible.  For instance it would be possible to just record the instruction pointer
when then macro expands and then have `LocationInfo` use DWARF information to
find the associated location in the source.

In general the macro basically just takes a variable number of arguments and passes
it onwards to the `ConstructFailure` as tuple.

The try macro is similar, but it just takes either one result where the error
part is passed onwards to `fail!` and the `Ok` part is unwrapped; or alternatively
a two argument version of where the second argument is an optional cause that
is passed as second argument to `fail!`.

```rust
macro_rules! try {
    ($expr:expr) => (match $expr {
        Err(x) => fail!(x),
        Ok(x) => x,
    });
    ($expr:expr, $cause:expr) => (match $expr {
        Err(x) => fail!(x, $cause),
        Ok(x) => x,
    });
}
```

# Drawbacks

This RFC is very lengthy and potentially controversial.  The reason it's one package
instead of smaller independent ones is that error handling needs to be a core language
feature and it's nearly impossible to provide the necessary interoperability in
an external library.

# Questions

Q: Why is `name()` and `description()` necessary?  What about translations?

> A: Support for translations is out of the scope for this.  However it should be
> possible to map translations to errors through the use of `TypeId`.  For this it
> might be a good idea to expose the underlying type id.
>
> Name and description are necessary for both a developer having something to work
> off other than just a random (and unstable) hex number, and also for an end user
> to have something to punch into Google if everything else fails.

Q: Why are heap allocations necessary?

> A: They are not actually necessary in this proposal.  While the use of `Failure<T>`
> requires a heap allocation for boxing up the error, the error itself can be used
> without a failure.  However if developers want to use errors directly they should
> take great care of keeping the errors small.  Ideally errors like `IoError` would
> be always boxed or converted into much smaller types.

Q: An Internal box does not implement non-nullable pointers correctly!

> A: That's not a question, but it's correct.  That however only affects
> `Result<(), Failure<T>>` and is something that can be easily fixed by making
> this optimization work through a struct indirection.

Q: Why does the error need to be `Clone`?

> A: It does not need to but it is very useful for both making tracebacks work
> as well as other means of error reporting.  For instance an error might have
> to go two ways to be handled: one error is converted in the process into something
> that can be presented to the user while a clone of the error would go to an
> error reporting service.  By knowing that any error can be `Clone` it allows
> such usages.

Q: Why is `try!` now restricted to `Error`s?

> A: partially this is a limitation of Rust.  Because the failure construction
> needs to differentiate between failures and errors it is currently not
> possible to have a fallback implementation that works on arbitrary types that
> do not implement `Error` and are not failures.  If negative trait bounds will
> be introduced the same macro could be used to also work with arbitrary values.
>
> However the fact that `try!` now requires failures or errors is probably
> beneficial for the user as it keeps the mental overhead of what happens on
> re-throwing manageable.  It might be reasonable to implement a separate
> macro to propagate failed results that are not errors.

# References:

  [1]: https://github.com/glaebhoerl/rfcs/blob/trait-based-exception-handling/active/0000-trait-based-exception-handling.md "Trait Based Error Handling"
  [2]: https://github.com/mitsuhiko/rust-incidents
