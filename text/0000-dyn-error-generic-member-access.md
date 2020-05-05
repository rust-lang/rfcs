- Feature Name: Add functions for generic member access to dyn Error and the `Error` trait
- Start Date: 2020-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/2895)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes additions to the `Error` trait to support accessing generic
forms of context from `dyn Error` trait objects. This generalizes the pattern
used in `backtrace` and `source`. This proposal adds the method
`Error::provide_context` to the `Error` trait, which offers `TypeId`-based member
lookup and a new inherent function `<dyn Error>::context` which makes use of an
implementor's `provide_context` to return a typed reference directly. These
additions would primarily be useful for error reporting, where we typically no
longer have type information and may be composing errors from many sources.

_note_: This RFC focuses on the more complicated of it's two proposed
solutions. The proposed solution provides support accessing dynamically sized
types. The [alternative proposal] is easier to understand and may be more
palatable.

## TLDR

Add this method to the `Error` trait

```rust
pub trait Error {
    // ...

    /// Provides an object of type `T` in response to this request.
    fn provide_context<'a>(&'a self, request: Pin<&mut Request<'a>>) {}
}
```

Example implementation:

```rust
fn provide_context<'a>(&'a self, mut request: Pin<&mut Request<'a>>) {
    request
        .provide::<Backtrace>(&self.backtrace)
        .provide::<SpanTrace>(&self.span_trace)
        .provide::<dyn Error>(&self.source)
        .provide::<Vec<&'static Location<'static>>>(&self.locations)
        .provide::<[&'static Location<'static>]>(&self.locations);
}
```

Example usage:

```rust
let e: &dyn Error = &concrete_error;

if let Some(bt) = e.context::<Backtrace>() {
    println!("{}", bt);
}
```

# Motivation
[motivation]: #motivation

In Rust, errors typically gather two forms of context when they are created:
context for the *current error message* and context for the *final* *error
report*. The `Error` trait exists to provide an interface to context intended
for error reports. This context includes the error message, the source error,
and, more recently, backtraces.

However, the current approach of promoting each form of context to a method on
the `Error` trait doesn't leave room for forms of context that are not commonly
used, or forms of context that are defined outside of the standard library.

## Extracting non-std types from `dyn Errors`

By adding a generic form of these member access functions we are no longer
restricted to types defined in the standard library. This opens the door to
many new forms of error reporting.

### Example use cases this enables

* using alternatives to `std::backtrace::Backtrace` such as
  `backtrace::Backtrace` or [`SpanTrace`]
* zig-like Error Return Traces by extracting `Location` types from errors
  gathered via `#[track_caller]` or similar.
* error source trees instead of chains by accessing the source of an error as a
  slice of errors rather than as a single error, such as a set of errors caused
  when parsing a file TODO reword
* Help text such as suggestions or warnings attached to an error report

## Moving `Error` into `libcore`

Adding a generic member access function to the `Error` trait and removing the
`backtrace` function would make it possible to move the `Error` trait to libcore
without losing support for backtraces on std. The only difference being that
in places where you can currently write `error.backtrace()` on nightly you
would instead need to write `error.context::<Backtrace>()`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Error handling in Rust consists of three steps: creation/propagation, handling,
and reporting. The `std::error::Error` trait exists to bridge the gap between
creation and reporting. It does so by acting as a interface that all error
types can implement that defines how to access context intended for error
reports, such as the error message, source, or location it was created. This
allows error reporting types to handle errors in a consistent manner when
constructing reports for end users while still retaining control over the
format of the full report.

The `Error` trait accomplishes this by providing a set of methods for accessing
members of `dyn Error` trait objects. It requires that types implement the
display trait, which acts as the interface to the main member, the error
message itself.  It provides the `source` function for accessing `dyn Error`
members, which typically represent the current error's cause. It provides the
`backtrace` function, for accessing a `Backtrace` of the state of the stack
when an error was created. For all other forms of context relevant to an error
report, the `Error` trait provides the `context` and `provide_context` functions.

As an example of how to use this interface to construct an error report, let’s
explore how one could implement an error reporting type. In this example, our
error reporting type will retrieve the source code location where each error in
the chain was created (if it captured a location) and render it as part of the
chain of errors. Our end goal is to get an error report that looks something
like this:

```
Error:
    0: Failed to read instrs from ./path/to/instrs.json
        at instrs.rs:42
    1: No such file or directory (os error 2)
```

The first step is to define or use a type to represent a source location. In
this example, we will define our own:

```rust
struct Location {
    file: &'static str,
    line: usize,
}
```

Next, we need to gather the location when creating our error types.

```rust
struct ExampleError {
    source: std::io::Error,
    location: Location,
    path: PathBuf,
}

impl fmt::Display for ExampleError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Failed to read instrs from {}", path.display())
    }
}

fn read_instrs(path: &Path) -> Result<String, ExampleError> {
    std::fs::read_to_string(path).map_err(|source| {
        ExampleError {
            source,
            path: path.to_owned(),
            location: Location {
                file: file!(),
                line: line!(),
            },
        }
    })
}
```

Then, we need to implement the `Error` trait to expose these members to the error reporter.

```rust
impl std::error::Error for ExampleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }

    fn provide_context<'a>(&'a self, mut request: Pin<&mut Request<'a>>) {
        request.provide::<Location>(&self.location);
    }
}
```

And, finally, we create an error reporter that prints the error and its source
recursively, along with any location data that was gathered

```rust
struct ErrorReporter(Box<dyn Error + Send + Sync + 'static>);

impl fmt::Debug for ErrorReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current_error = Some(self.0.as_ref());
        let mut ind = 0;

        while let Some(error) = current_error {
            writeln!(fmt, "    {}: {}", ind, error)?;

            if let Some(location) = error.context::<Location>() {
                writeln!(fmt, "        at {}:{}", location.file, location.line)?;
            }

            ind += 1;
            current_error = error.source();
        }

        Ok(())
    }
}
```

As you can see the `Error` trait provides the facilities needed to create error
reports enriched by information that may be present in source errors.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The following changes need to be made to implement this proposal:

### Add a type like [`Request`] to core

This type fills the same role as `&dyn Any` except that it supports other trait
objects as the requested type.

Here is the implementation for the proof of concept, taken from Nika Layzell's
[object-provider crate]:

```rust
use core::any::TypeId;
use core::cell::Cell;
use core::fmt;
use core::marker::PhantomData;
use core::pin::Pin;

/// A dynamic request for an object based on its type.
///
/// `'out` is the lifetime of the requested reference.
#[repr(transparent)]
pub struct Request<'out>(RequestBuf<PhantomData<&'out Cell<()>>>);
// FIXME: The argument of the RequestBuf should be a thin unsized type,
// but `extern type` is impossible to use correctly right now
// (it cannot be placed at offset > 0, and it cannot be placed inside a union).
// Since miri doesn't complain we'll let it slide.

impl<'out> Request<'out> {
    /// Provides an object of type `T` in response to this request.
    ///
    /// Returns `Err(FulfilledRequest)` if the value was successfully provided,
    /// and `Ok(self)` if `T` was not the type being requested.
    ///
    /// This method can be chained within `provide` implementations using the
    /// `?` operator to concisely provide multiple objects.
    pub fn provide<T: ?Sized + 'static>(&mut self, value: &'out T) -> &mut Self {
        self.provide_with(|| value)
    }

    /// Lazily provides an object of type `T` in response to this request.
    ///
    /// Returns `Err(FulfilledRequest)` if the value was successfully provided,
    /// and `Ok(self)` if `T` was not the type being requested.
    ///
    /// The passed closure is only called if the value will be successfully
    /// provided.
    ///
    /// This method can be chained within `provide` implementations using the
    /// `?` operator to concisely provide multiple objects.
    pub fn provide_with<T: ?Sized + 'static, F>(&mut self, cb: F) -> &mut Self
    where
        F: FnOnce() -> &'out T,
    {
        if self.is::<T>() {
            let this = unsafe { &mut *(self as *mut _ as *mut RequestBuf<Option<&'out T>>) };
            debug_assert!(
                this.value.is_none(),
                "Multiple requests to a `RequestBuf` were acquired?"
            );
            this.value = Some(cb());
        }
        self
    }

    /// Get the `TypeId` of the requested type.
    pub fn type_id(&self) -> TypeId {
        self.0.type_id
    }

    /// Returns `true` if the requested type is the same as `T`
    pub fn is<T: ?Sized + 'static>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
    }

    /// Calls the provided closure with a request for the the type `T`, returning
    /// `Some(&T)` if the request was fulfilled, and `None` otherwise.
    ///
    /// The `ObjectProviderExt` trait provides helper methods specifically for
    /// types implementing `ObjectProvider`.
    pub fn with<T: ?Sized + 'static, F>(f: F) -> Option<&'out T>
    where
        F: FnOnce(Pin<&mut Self>),
    {
        let mut buf = RequestBuf {
            type_id: TypeId::of::<T>(),
            value: None,
        };
        unsafe {
            let request = &mut *(&mut buf as *mut _ as *mut Request);
            f(Pin::new(request));
        }
        buf.value
    }
}

impl<'out> fmt::Debug for Request<'out> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("type_id", &self.type_id())
            .finish()
    }
}

// Needs to have a known layout so we can do unsafe pointer shenanigans.
#[repr(C)]
struct RequestBuf<T: ?Sized> {
    type_id: TypeId,
    value: T,
}
```

### Define a generic accessor on the `Error` trait

```rust
pub trait Error {
    // ...

    fn provide_context<'a>(&'a self, _request: Pin<&mut Request<'a>>) {}
}
```

### Use this `Request` type to handle passing generic types out of the trait object

```rust
impl dyn Error {
    pub fn context<T: ?Sized + 'static>(&self) -> Option<&T> {
        Request::with::<T, _>(|req| self.provide_context(req))
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* The `Request` api is being added purely to help with this function. This will
  likely need some design work to make it more generally applicable, hopefully
  as a struct in `core::any`.
* The `context` function name is currently widely used throughout the rust
  error handling ecosystem in libraries like `anyhow` and `snafu` as an
  ergonomic version of `map_err`. If we settle on `context` as the final name
  it will possibly break existing libraries.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Do Nothing

We could not do this, and continue to add accessor functions to the `Error`
trait whenever a new type reaches critical levels of popularity in error
reporting.

If we choose to do nothing we will continue to see hacks around the current
limitations on the `Error` trait such as the `Fail` trait, which added the
missing function access methods that didn't previously exist on the `Error`
trait and type erasure / unnecessary boxing of errors to enable downcasting to
extract members.
[[1]](https://docs.rs/tracing-error/0.1.2/src/tracing_error/error.rs.html#269-274).


## Use an alternative proposal that relies on the `Any` trait for downcasting

```rust
pub trait Error {
    /// Provide an untyped reference to a member whose type matches the provided `TypeId`.
    ///
    /// Returns `None` by default, implementors are encouraged to override.
    fn provide_context(&self, ty: TypeId) -> Option<&dyn Any> {
        None
    }
}

impl dyn Error {
    /// Retrieve a reference to `T`-typed context from the error if it is available.
    pub fn context<T: Any>(&self) -> Option<&T> {
        self.provide_context(TypeId::of::<T>())?.downcast_ref::<T>()
    }
}
```

### Why isn't this the primary proposal?

There are two big issues with using the `Any` trait that I believe justify the
more complicated solution.

- You cannot return dynamically sized types as `&dyn Any`
- It's easy to introduce runtime errors with `&dyn Any` by either comparing to
  or returning the wrong type

By making all the type id comparison internal to the `Request` type it is
impossible to compare the wrong type ids. By encouraging explicit type
parameters when calling `provide` the compiler is able to catch errors where
the type passed in doesn't match the type that was expected. So while the API
for the main proposal is more complicated it should be less error prone.

# Prior art
[prior-art]: #prior-art

I do not know of any other languages whose error handling has similar
facilities for accessing members when reporting errors. For the most part,
prior art for this proposal comes from within rust itself in the form of
previous additions to the `Error` trait.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* What should the names of these functions be?
    * `context`/`context_ref`/`provide_context`/`provide_context`
    * `member`/`member_ref`
    * `provide`/`request`
* Should there be a by-value version for accessing temporaries?
    * We bring this up specifically for the case where you want to use this
      function to get an `Option<&[&dyn Error]>` out of an error, in this case,
      it is unlikely that the error behind the trait object is actually storing
      the errors as `dyn Error`s, and theres no easy way to allocate storage to
      store the trait objects.
* How should context handle failed downcasts?
    * suggestion: panic, as providing a type that doesn't match the typeid
      requested is a program error

# Future possibilities
[future-possibilities]: #future-possibilities

This opens the door to supporting `Error Return Traces`, similar to zigs, where
if each return location is stored in a `Vec<&'static Location<'static>>` a full
return trace could be built up with:

```rust
let mut locations = e
    .chain()
    .filter_map(|e| e.context::<[&'static Location<'static>]>())
    .flat_map(|locs| locs.iter());
```

[`SpanTrace`]: https://docs.rs/tracing-error/0.1.2/tracing_error/struct.SpanTrace.html
[`Request`]: https://github.com/yaahc/nostd-error-poc/blob/master/fakecore/src/any.rs
[alternative proposal]: #use-an-alternative-proposal-that-relies-on-the-any-trait-for-downcasting
[object-provider crate]: https://github.com/mystor/object-provider
