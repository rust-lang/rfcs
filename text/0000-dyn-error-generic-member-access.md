- Feature Name: Add functions for generic member access to dyn Error and the `Error` trait
- Start Date: 2020-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/2895)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes additions to the `Error` trait to support accessing generic
forms of context from `dyn Error` trait objects. This generalizes the pattern
used in `backtrace` and `source`. This proposal adds the method
`Error::provide_context` to the `Error` trait, which offers `TypeId`-based
member lookup and a new inherent function `<dyn Error>::context` and `<dyn
Error>::context_ref` which makes use of an implementor's `provide_context` to
return a typed reference directly. These additions would primarily be useful
for error reporting, where we typically no longer have type information and
may be composing errors from many sources.

## TLDR

Add this method to the `Error` trait

```rust
pub trait Error {
    // ...

    /// Provides type based access to context intended for error reports
    ///
    /// Used in conjunction with [`context`] and [`context_ref`] to extract
    /// references to member variables from `dyn Error` trait objects.
    ///
    /// # Example
    ///
    /// ```rust
    /// use backtrace::Backtrace;
    /// use core::fmt;
    /// use fakecore::any::Request;
    ///
    /// #[derive(Debug)]
    /// struct Error {
    ///     backtrace: Backtrace,
    /// }
    ///
    /// impl fmt::Display for Error {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "Example Error")
    ///     }
    /// }
    ///
    /// impl fakecore::error::Error for Error {
    ///     fn provide_context<'a>(&'a self, mut request: &mut Request<'a>) {
    ///         request.provide_ref::<Backtrace>(&self.backtrace);
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let backtrace = Backtrace::new();
    ///     let error = Error { backtrace };
    ///     let dyn_error = &error as &dyn fakecore::error::Error;
    ///     let backtrace_ref = dyn_error.context_ref::<Backtrace>().unwrap();
    ///
    ///     assert!(core::ptr::eq(&error.backtrace, backtrace_ref));
    /// }
    /// ```
    fn provide_context<'a>(&'a self, request: &mut Request<'a>) {}
}
```

And these inherent methods on `dyn Error` trait objects:

```rust
impl dyn Error {
    pub fn context_ref<T: ?Sized + 'static>(&self) -> Option<&T> {
        Request::request_ref(|req| self.provide_context(req))
    }

    pub fn context<T: 'static>(&self) -> Option<T> {
        Request::request_value(|req| self.provide_context(req))
    }
}
```

Example implementation:

```rust
fn provide_context<'a>(&'a self, mut request: &mut Request<'a>) {
    request
        .provide_ref::<Backtrace>(&self.backtrace)
        .provide_ref::<SpanTrace>(&self.span_trace)
        // supports dynamically sized types
        .provide_ref::<dyn Error>(&self.source)
        .provide_ref::<Vec<&'static Location<'static>>>(&self.locations)
        .provide_ref::<[&'static Location<'static>]>(&self.locations)
        // can be used to upcast self to other trait objects
        .provide_ref::<dyn Serialize>(&self)
        // or to pass owned values
        .provide_value::<ExitCode>(self.exit_code);
}
```

Example usage:

```rust
let e: &dyn Error = &concrete_error;

if let Some(bt) = e.context_ref::<Backtrace>() {
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

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Error handling in Rust consists of three main steps: creation/propagation,
handling, and reporting. The `std::error::Error` trait exists to bridge the
gap between creation and reporting. It does so by acting as an interface that
all error types can implement that defines how to access context intended for
error reports, such as the error message, source, or location it was created.
This allows error reporting types to handle errors in a consistent manner
when constructing reports for end users while still retaining control over
the format of the full report.

The `Error` trait accomplishes this by providing a set of methods for accessing
members of `dyn Error` trait objects. It requires that types implement the
`Display` trait, which acts as the interface to the main member, the error
message itself.  It provides the `source` function for accessing `dyn Error`
members, which typically represent the current error's cause.

For all other forms of context relevant to an error report, the `Error` trait
offers the `provide_context` method. The report renderer indirectly calls
`provide_context` for any `Error` type that implements it using standard
library methods on `dyn Error` itself: `<dyn Error>.request_ref` and `<dyn
Error>.request_value`.

As an example of how to use this interface to construct an error report, let’s
explore how one could implement an error reporting type. In this example, our
error reporting type will retrieve the source code location where each error in
the chain was created (if it captured a location) and render it as part of the
chain of errors. Our end goal is to get an error report that looks something
like this:

```
Error:
    0: ERROR MESSAGE
        at LOCATION
    1: SOURCE'S ERROR MESSAGE
        at SOURCE'S LOCATION
    2: SOURCE'S SOURCE'S ERROR MESSAGE
    ...
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
        write!(fmt, "Failed to read instrs from {}", self.path.display())
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

    fn provide_context<'a>(&'a self, mut request: &mut Request<'a>) {
        request.provide_ref::<Location>(&self.location);
    }
}
```

And, finally, we create an error reporter that prints the error and its source
recursively, along with any location data that was gathered

```rust
struct ErrorReporter(Box<dyn Error + Send + Sync + 'static>);

impl fmt::Debug for ErrorReporter {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error: &(dyn Error + 'static) = self.0.as_ref();
        let errors = std::iter::successors(Some(error), |e| e.source());

        for (ind, error) in errors.enumerate() {
            writeln!(fmt, "    {}: {}", ind, error)?;
            if let Some(location) = error.request_ref::<Location>() {
                writeln!(fmt, "        at {}:{}", location.file, location.line)?;
            }
        }

        Ok(())
    }
}
```

Now we have an error reporter that is ready for use, a simple program using it
would look like this.

```rust
fn main() -> Result<(), ErrorReporter> {
    let path = "./path/to/instrs.json";
    let _instrs = read_instrs(path.into())?;
}
```

Which, if run without creating the `instrs.json` file prints this error report:

```
Error:
    0: Failed to read instrs from ./path/to/instrs.json
        at instrs.rs:42
    1: No such file or directory (os error 2)
```

Mission accomplished! The error trait gave us everything we needed to build
error reports enriched by context relevant to our application. This same
pattern can implement many error reporting patterns, such as including help
text, spans, http status codes, or backtraces in errors which are still
accessible after the error has been converted to a `dyn Error`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A proof of concept implementation, can be seen in  Nika Layzell's [dyno crate].

A usable version of this proposal is available in master branch of the [rust
repo](rust-repo). 

## User-facing elements

The user-facing elements of this proposal include the following (snippets
copied from master branch of standard library):

### Add a `Request` type to `libcore` for type-indexed context

`Request` is a type intended to emulate nested dynamic typing. This type fills
the same role as `&dyn Any` except that it supports other trait objects as the
requested type. The only users of this type outside of the standard library
will be implementors of `Error.provide_context`, in order to respond to
requests for specific types of data. Requests for data types not supported by
the implementor of `Error.provide_context` will result in `None` responses to
`<dyn Error>.request_ref` and `<dyn Error>.request_value`.

```rust
pub struct Request<'a>(dyn Erased<'a> + 'a);
impl<'a> Request<'a> {
    /// Provide a value or other type with only static lifetimes.
    ///
    #[unstable(feature = "provide_any", issue = "96024")]
    pub fn provide_value<T>(&mut self, value: T) -> &mut Self
    where
        T: 'static,
    {
        self.provide::<tags::Value<T>>(value)
    }

    /// Provide a value or other type with only static lifetimes computed using a closure.
    ///
    #[unstable(feature = "provide_any", issue = "96024")]
    pub fn provide_value_with<T>(&mut self, fulfil: impl FnOnce() -> T) -> &mut Self
    where
        T: 'static,
    {
        self.provide_with::<tags::Value<T>>(fulfil)
    }

    /// Provide a reference. The referee type must be bounded by `'static`,
    /// but may be unsized.
    ///
    #[unstable(feature = "provide_any", issue = "96024")]
    pub fn provide_ref<T: ?Sized + 'static>(&mut self, value: &'a T) -> &mut Self {
        self.provide::<tags::Ref<tags::MaybeSizedValue<T>>>(value)
    }

    /// Provide a reference computed using a closure. The referee type
    /// must be bounded by `'static`, but may be unsized.
    ///
    #[unstable(feature = "provide_any", issue = "96024")]
    pub fn provide_ref_with<T: ?Sized + 'static>(
        &mut self,
        fulfil: impl FnOnce() -> &'a T,
    ) -> &mut Self {
        self.provide_with::<tags::Ref<tags::MaybeSizedValue<T>>>(fulfil)
    }

    /// Check if the `Request` would be satisfied if provided with a
    /// value of the specified type. If the type does not match or has
    /// already been provided, returns false.
    ///
    #[unstable(feature = "provide_any", issue = "96024")]
    pub fn would_be_satisfied_by_value_of<T>(&self) -> bool
    where
        T: 'static,
    {
        self.would_be_satisfied_by::<tags::Value<T>>()
    }

    /// Check if the `Request` would be satisfied if provided with a
    /// reference to a value of the specified type. If the type does
    /// not match or has already been provided, returns false.
    ///
    #[unstable(feature = "provide_any", issue = "96024")]
    pub fn would_be_satisfied_by_ref_of<T>(&self) -> bool
    where
        T: ?Sized + 'static,
    {
        self.would_be_satisfied_by::<tags::Ref<tags::MaybeSizedValue<T>>>()
    }
}

```

### Define a generic accessor on the `Error` trait

A new method on trait `Error`. Implementors of this `provide_context` method
can respond to requests for specific types of data using the given `&mut
Request` instance. Requests for data types not supported by this method on a
particular type (via `<dyn Error>.request_ref` or `<dyn Error>.request_value`)
will result in an `Option::None` response.

```rust
    /// Provides type based access to context intended for error reports.
    ///
    /// Used in conjunction with [`Request::provide_value`] and [`Request::provide_ref`] to extract
    /// references to member variables from `dyn Error` trait objects.
    #[unstable(feature = "error_generic_member_access", issue = "99301")]
    #[allow(unused_variables)]
    fn provide_context<'a>(&'a self, request: &mut Request<'a>) {}
```

Note that `provide_context` is not a user-facing function, and is in the
current unstable form only used indirectly through methods on `dyn Error`. It
is provided as a means for implementors of `Error` to respond to requests
routed to them via this method.

### Use the `Request` type to handle passing generic types out of the trait object

New methods on `dyn Error` trait objects, intended for downstream receivers of
`dyn Error` instances to request data of specific types. These methods take
care of implementation details related to constructing `Request`s and
attempting to fulfill them via `Error.provide_context`.

```rust
impl<'a> dyn Error + 'a {
    /// Request a reference of type `T` as context about this error.
    #[unstable(feature = "error_generic_member_access", issue = "99301")]
    pub fn request_ref<T: ?Sized + 'static>(&'a self) -> Option<&'a T> {
    }

    /// Request a value of type `T` as context about this error.
    #[unstable(feature = "error_generic_member_access", issue = "99301")]
    pub fn request_value<T: 'static>(&'a self) -> Option<T> {
    }

    /// Request a ref of type `T` from the given `Self` instance.
    #[unstable(feature = "error_generic_member_access", issue = "99301")]
    pub fn request_ref_from<T: ?Sized + 'static>(this: &'a Self) -> Option<&'a T> {
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* The `Request` api is being added purely to help with this function. This will
  likely need some design work to make it more generally applicable,
  hopefully as a struct in `core::any`. **Update**: this API might also be
  useful for `std::task::Context` to help pass data to executors in a backend
  agnostic way.
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

There are two significant issues with using the `Any` trait that motivate the
more complicated solution.

- You cannot return dynamically sized types as `&dyn Any`
- It's easy to introduce runtime errors with `&dyn Any` by either comparing to
  or returning the wrong type

By making all the `TypeId` comparison internal to the `Request` type it is
impossible to compare the wrong `TypeId`s. By encouraging explicit type
parameters when calling `provide` the compiler is able to catch errors where
the type passed in doesn't match the type that was expected. So while the API
for the main proposal is more complicated it should be less error prone.

# Prior art
[prior-art]: #prior-art

I do not know of any other languages whose error handling has similar
facilities for accessing members when reporting errors. For the most part,
prior art for this proposal comes from within rust itself in the form of
previous additions to the `Error` trait.

## Rust RFC 3192

The now-defunct
[rust-lang/rfcs#3192](https://github.com/rust-lang/rfcs/pull/3192) proposed a
way for users outside the standard library to offer similar functionality as
with the `Error.provide_context` trait method being proposed here. The libs
meeting team ultimately decided to limit usage of the `Request` type to the
`Error` trait:

* May 2023 libs team meeting (summary of requested
  changes](https://github.com/rust-lang/rust/issues/96024#issuecomment-1554773172)
* July 9 2023 [confirmation that the libs team meeting only wants this
  functionality to be available for `Error`
  types](https://rust-lang.zulipchat.com/#narrow/stream/219381-t-libs/topic/error_generic_member_access/near/373733742)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* ~~What should the names of these functions be?~~
    * `context`/`context_ref`/`provide_context`/`provide_context`/`request_context`
    * `member`/`member_ref`
    * `provide`/`request`
    * **Update** as of https://github.com/rust-lang/rust/pull/113464 we are
      settling on `Request` and `Error`'s `fn provide<'a>(&'a self, demand: &mut Request<'a>)`
      and https://github.com/rust-lang/rust/pull/112531 renames `provide` to
      `provide_context`
* ~~Should there be a by-value version for accessing temporaries?~~ **Update**:
  The object provider API in this RFC has been updated to include a by-value
  variant for passing out owned data.
    * ~~We bring this up specifically for the case where you want to use this
      function to get an `Option<&[&dyn Error]>` out of an error, in this case,
      it is unlikely that the error behind the trait object is actually storing
      the errors as `dyn Error`s, and theres no easy way to allocate storage to
      store the trait objects.~~

# Future possibilities
[future-possibilities]: #future-possibilities

This opens the door to supporting [`Error Return
Traces`](https://ziglang.org/documentation/master/#toc-Error-Return-Traces),
similar to zig's, where if each return location is stored in a `Vec<&'static
Location<'static>>` a full return trace could be built up with:

```rust
let mut locations = e
    .chain()
    .filter_map(|e| e.context_ref::<[&'static Location<'static>]>())
    .flat_map(|locs| locs.iter());
```


[`SpanTrace`]: https://docs.rs/tracing-error/0.1.2/tracing_error/struct.SpanTrace.html
[`Request`]: https://github.com/yaahc/nostd-error-poc/blob/master/fakecore/src/any.rs
[alternative proposal]: #use-an-alternative-proposal-that-relies-on-the-any-trait-for-downcasting
[dyno crate]: https://github.com/mystor/dyno
[proof of concept]: https://github.com/yaahc/nostd-error-poc
[rust-repo] https://githbu.com/rust-lang/rust
