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

The following changes need to be made to implement this proposal:

### Add a `Request` type to `libcore` for type-indexed context

`Request` is a type to emulate nested dynamic typing. This type fills the same
role as `&dyn Any` except that it supports other trait objects as the requested
type.

Here is the implementation for the proof of concept, based on Nika Layzell's
[dyno crate]:

A usable version of this is available in the [proof of concept] repo under
`fakecore/src/any.rs`.

```rust
use core::any::TypeId;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

pub mod provider {
    //! Tag-based value lookup API for trait objects.
    //!
    //! This provides a similar API to my `object_provider` crate, built on top of
    //! `dyno`

    use super::{Tag, Tagged};

    /// An untyped request for a value of a specific type.
    ///
    /// This type is generally used as an `&mut Request<'a>` outparameter.
    #[repr(transparent)]
    pub struct Request<'a> {
        tagged: dyn Tagged<'a> + 'a,
    }

    impl<'a> Request<'a> {
        /// Helper for performing transmutes as `Request<'a>` has the same layout as
        /// `dyn Tagged<'a> + 'a`, just with a different type!
        ///
        /// This is just to have our own methods on it, and less of the interface
        /// exposed on the `provide` implementation.
        fn wrap_tagged<'b>(t: &'b mut (dyn Tagged<'a> + 'a)) -> &'b mut Self {
            unsafe { &mut *(t as *mut (dyn Tagged<'a> + 'a) as *mut Request<'a>) }
        }

        pub fn is<I>(&self) -> bool
        where
            I: Tag<'a>,
        {
            self.tagged.is::<ReqTag<I>>()
        }

        pub fn provide<I>(&mut self, value: I::Type) -> &mut Self
        where
            I: Tag<'a>,
        {
            if let Some(res @ None) = self.tagged.downcast_mut::<ReqTag<I>>() {
                *res = Some(value);
            }
            self
        }

        pub fn provide_ref<I: ?Sized + 'static>(&mut self, value: &'a I) -> &mut Self
        {
            use crate::any::tag::Ref;
            if let Some(res @ None) = self.tagged.downcast_mut::<ReqTag<Ref<I>>>() {
                *res = Some(value);
            }
            self
        }

        pub fn provide_with<I, F>(&mut self, f: F) -> &mut Self
        where
            I: Tag<'a>,
            F: FnOnce() -> I::Type,
        {
            if let Some(res @ None) = self.tagged.downcast_mut::<ReqTag<I>>() {
                *res = Some(f());
            }
            self
        }
    }

    pub trait Provider {
        fn provide<'a>(&'a self, request: &mut Request<'a>);
    }

    impl dyn Provider {
        pub fn request<'a, I>(&'a self) -> Option<I::Type>
        where
            I: Tag<'a>,
        {
            request::<I, _>(|request| self.provide(request))
        }
    }

    pub fn request<'a, I, F>(f: F) -> Option<<I as Tag<'a>>::Type>
    where
        I: Tag<'a>,
        F: FnOnce(&mut Request<'a>),
    {
        let mut result: Option<<I as Tag<'a>>::Type> = None;
        f(Request::<'a>::wrap_tagged(<dyn Tagged>::tag_mut::<ReqTag<I>>(
            &mut result,
        )));
        result
    }

    /// Implementation detail: Specific `Tag` tag used by the `Request` code under
    /// the hood.
    ///
    /// Composition of `Tag` types!
    struct ReqTag<I>(I);
    impl<'a, I: Tag<'a>> Tag<'a> for ReqTag<I> {
        type Type = Option<I::Type>;
    }
}

pub mod tag {
    //! Simple type-based tag values for use in generic code.
    use super::Tag;
    use core::marker::PhantomData;

    /// Type-based `Tag` for `&'a T` types.
    pub struct Ref<T: ?Sized + 'static>(PhantomData<T>);

    impl<'a, T: ?Sized + 'static> Tag<'a> for Ref<T> {
        type Type = &'a T;
    }

    /// Type-based `Tag` for `&'a mut T` types.
    pub struct RefMut<T: ?Sized + 'static>(PhantomData<T>);

    impl<'a, T: ?Sized + 'static> Tag<'a> for RefMut<T> {
        type Type = &'a mut T;
    }

    /// Type-based `Tag` for concrete types.
    pub struct Value<T: 'static>(PhantomData<T>);

    impl<T: 'static> Tag<'_> for Value<T> {
        type Type = T;
    }
}

/// An identifier which may be used to tag a specific
pub trait Tag<'a>: Sized + 'static {
    /// The type of values which may be tagged by this `Tag`.
    type Type: 'a;
}

mod private {
    pub trait Sealed {}
}

/// Sealed trait representing a type-erased tagged object.
pub unsafe trait Tagged<'a>: private::Sealed + 'a {
    /// The `TypeId` of the `Tag` this value was tagged with.
    fn tag_id(&self) -> TypeId;
}

/// Internal wrapper type with the same representation as a known external type.
#[repr(transparent)]
struct TaggedImpl<'a, I>
where
    I: Tag<'a>,
{
    _value: I::Type,
}

impl<'a, I> private::Sealed for TaggedImpl<'a, I> where I: Tag<'a> {}

unsafe impl<'a, I> Tagged<'a> for TaggedImpl<'a, I>
where
    I: Tag<'a>,
{
    fn tag_id(&self) -> TypeId {
        TypeId::of::<I>()
    }
}

// FIXME: This should also handle the cases for `dyn Tagged<'a> + Send`,
// `dyn Tagged<'a> + Send + Sync` and `dyn Tagged<'a> + Sync`...
//
// Should be easy enough to do it with a macro...
impl<'a> dyn Tagged<'a> {
    /// Tag a reference to a concrete type with a given `Tag`.
    ///
    /// This is like an unsizing coercion, but must be performed explicitly to
    /// specify the specific tag.
    pub fn tag_ref<I>(value: &I::Type) -> &dyn Tagged<'a>
    where
        I: Tag<'a>,
    {
        // SAFETY: `TaggedImpl<'a, I>` has the same representation as `I::Type`
        // due to `#[repr(transparent)]`.
        unsafe { &*(value as *const I::Type as *const TaggedImpl<'a, I>) }
    }

    /// Tag a reference to a concrete type with a given `Tag`.
    ///
    /// This is like an unsizing coercion, but must be performed explicitly to
    /// specify the specific tag.
    pub fn tag_mut<I>(value: &mut I::Type) -> &mut dyn Tagged<'a>
    where
        I: Tag<'a>,
    {
        // SAFETY: `TaggedImpl<'a, I>` has the same representation as `I::Type`
        // due to `#[repr(transparent)]`.
        unsafe { &mut *(value as *mut I::Type as *mut TaggedImpl<'a, I>) }
    }

    /// Tag a Box of a concrete type with a given `Tag`.
    ///
    /// This is like an unsizing coercion, but must be performed explicitly to
    /// specify the specific tag.
    #[cfg(feature = "alloc")]
    pub fn tag_box<I>(value: Box<I::Type>) -> Box<dyn Tagged<'a>>
    where
        I: Tag<'a>,
    {
        // SAFETY: `TaggedImpl<'a, I>` has the same representation as `I::Type`
        // due to `#[repr(transparent)]`.
        unsafe { Box::from_raw(Box::into_raw(value) as *mut TaggedImpl<'a, I>) }
    }

    /// Returns `true` if the dynamic type is tagged with `I`.
    #[inline]
    pub fn is<I>(&self) -> bool
    where
        I: Tag<'a>,
    {
        self.tag_id() == TypeId::of::<I>()
    }

    /// Returns some reference to the dynamic value if it is tagged with `I`,
    /// or `None` if it isn't.
    #[inline]
    pub fn downcast_ref<I>(&self) -> Option<&I::Type>
    where
        I: Tag<'a>,
    {
        if self.is::<I>() {
            // SAFETY: Just checked whether we're pointing to a
            // `TaggedImpl<'a, I>`, which was cast to from an `I::Type`.
            unsafe { Some(&*(self as *const dyn Tagged<'a> as *const I::Type)) }
        } else {
            None
        }
    }

    /// Returns some reference to the dynamic value if it is tagged with `I`,
    /// or `None` if it isn't.
    #[inline]
    pub fn downcast_mut<I>(&mut self) -> Option<&mut I::Type>
    where
        I: Tag<'a>,
    {
        if self.is::<I>() {
            // SAFETY: Just checked whether we're pointing to a
            // `TaggedImpl<'a, I>`, which was cast to from an `I::Type`.
            unsafe { Some(&mut *(self as *mut dyn Tagged<'a> as *mut I::Type)) }
        } else {
            None
        }
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub fn downcast_box<I>(self: Box<Self>) -> Result<Box<I::Type>, Box<Self>>
    where
        I: Tag<'a>,
    {
        if self.is::<I>() {
            unsafe {
                // SAFETY: Just checked whether we're pointing to a
                // `TaggedImpl<'a, I>`, which was cast to from an `I::Type`.
                let raw: *mut dyn Tagged<'a> = Box::into_raw(self);
                Ok(Box::from_raw(raw as *mut I::Type))
            }
        } else {
            Err(self)
        }
    }
}
```

### Define a generic accessor on the `Error` trait

```rust
pub trait Error {
    // ...

    fn provide_context<'a>(&'a self, _request: &mut Request<'a>) {}
}
```

### Use this `Request` type to handle passing generic types out of the trait object

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

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* What should the names of these functions be?
    * `context`/`context_ref`/`provide_context`/`provide_context`/`request_context`
    * `member`/`member_ref`
    * `provide`/`request`
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

This opens the door to supporting [`Error Return Traces`](https://ziglang.org/documentation/master/#toc-Error-Return-Traces), similar to zigs, where
if each return location is stored in a `Vec<&'static Location<'static>>` a full
return trace could be built up with:

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
