- Feature Name: Add fns for generic member access to dyn Error and the Error trait
- Start Date: 2020-04-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a pair of additions to the `Error` trait to support accessing
generic forms of context from `dyn Error` trait objects, one method on the
`Error` trait itself for returning references to members based on a given
typeid, and another fn implemented for `dyn Error` that uses a generic return
type to get the type id to pass into the trait object's fn. These functions
will act as a generalized version of `backtrace`, `source`, and `cause`, and
would primarily be used during error reporting when rendering a chain of opaque
errors.

# Motivation
[motivation]: #motivation

Today, there are a number of forms of context that are traditionally gathered
when creating errors. These members are gathered so that a final error
reporting type or function can access them and render them independently of the
`Display` implementation for that specific error type to allow for consistently
formatted and flexible error reports. Today, there are 2 such forms of context
that are traditionally gathered, `backtrace` and `source`.

However, the current approach of promoting each form of context to a fn on the
`Error` trait doesn't leave room for forms of context that are not commonly
used, or forms of context that are defined outside of the standard library.

By adding a generic form of these functions that works around the issues of
monomorphization on trait objects we can support more forms of context and
forms of context that are experimented with outside of the standard library
such as:

* `SpanTrace` a backtrace like type from the `tracing-error` library
* zig-like Error Return Traces by extracting `Location` types from errors
  gathered via `#[track_caller]`
* error source trees instead of chains by accessing the source of an error as a
  slice of errors rather than as a single error, such as a set of errors caused
  when parsing a file

With a generic form of member access available on `Error` trait objects we
could support a greater diversity of error handling needs and make room for
experimentation on new forms of context in error reports.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When implementing error handling in rust there are two main aspects that should
be considered, the creation of errors and the reporting of errors.

Error handling in rust consists mainly of two steps, creation/propogation and
reporting. The `std::error::Error` trait exists to bridge this gap. It does so
by acting as a consistent interface that all Error types can implement to allow
Error Reporting types to handle them in a consistent manner when constructing
reports for end users.

The error trait accomplishes this by providing a set of methods for accessing
members of `dyn Error` trait objects. For accessing the message that should be
rendered to the end user the Error trait implements the `Display` trait. For
accessing `dyn Error` members it provides the `source` function, which
conventionally represents the lower level error that caused a subsequent error.
For accessing a `Backtrace` of the state of the stack when an error was created
it provides the `backtrace` function. For all other forms of context relevant
to an Error Report the error trait provides the `context`/`context_any`
functions.

As an example lets explore how one could implement an error reporting type that
retrieves the Location where each error in the chain was created, if it exists,
and renders it as part of the chain of errors.

The goal is to implement an Error Report that looks something like this:

```
Error:
    0: Failed to read instrs from ./path/to/instrs.json
        at instrs.rs:42
    1: No such file or directory (os error 2)
```

The first step is to define or use a Location type. In this example we will
define our own but we could use also use `std::panic::Location` for example.

```rust
struct Location {
    file: &'static str,
    line: usize,
}
```

Next we need to gather the location when creating our error types.

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

Next we need to implement the `Error` trait to expose these members to the
Error Reporter.

```rust
impl std::error::Error for ExampleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }

    fn context_any(&self, type_id: TypeID) -> Option<&dyn Any> {
        if id == TypeId::of::<Location>() {
            Some(&self.location)
        } else {
            None
        }
    }
}
```

And finally, we create an error reporter that prints the error and its source
recursively along with the location data if it was gathered.

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

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are two additions necessary to the standard library to implement this
proposal:


First we need to add a function for dyn Error trait objects that will be used
by error reporters to access members given a generic type. This function
circumvents restrictions on generics in trait functions by being implemented
for trait objects only, rather than as a member of the trait itself.

```rust
impl dyn Error {
    pub fn context<T: Any>(&self) -> Option<&T> {
        self.context_any(TypeId::of::<T>())?.downcast_ref::<T>()
    }
}
```

Second we need to add a member to the `Error` trait to provide the `&dyn Any`
trait objects to the `context` fn for each member based on the type_id.

```rust
trait Error {
    /// ...

    fn context_any(&self, id: TypeId) -> Option<&dyn Any> {
        None
    }
}
```

# Drawbacks
[drawbacks]: #drawbacks

* The API for defining how to return types is cumbersome and possibly not
  accessible for new rust users.
    * If the type is stored in an Option getting it converted to an `&Any` will
      probably challenge new devs, this can be made easier with documented
      examples covering common use cases and macros like `thiserror`.
```rust
} else if typeid == TypeId::of::<SpanTrace>() {
    self.span_trace.as_ref().map(|s| s as &dyn Any)
}
```
* When you return the wrong type and the downcast fails you get `None` rather
  than a compiler error guiding you to the right return type, which can make it
  challenging to debug mismatches between the type you return and the type you
  use to check against the type_id
    * The downcast could be changed to panic when it fails
    * There is an alternative implementation that mostly avoids this issue
* Introduces more overhead from the downcasts
* This approach cannot return slices or trait objects because of restrictions
  on `Any`
    * The alternative solution avoids this issue

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The two alternatives I can think of are:

## Do Nothing

We could not do this, and continue to add accessor functions to the `Error`
trait whenever a new type reaches critical levels of popularity in error
reporting.


## Use an alternative to Any for passing generic types across the trait boundary

Nika Layzell has proposed an alternative implementation using a `Provider` type
which avoids using `&dyn Any`. I do not necessarily think that the main
suggestion is necessarily better, but it is much simpler.
    * https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=0af9dbf0cd20fa0bea6cff16a419916b
    * https://github.com/mystor/object-provider

With this design an implementation of the `context_any` fn might instead look like:

```rust
fn provide<'r, 'a>(&'a self, request: Request<'r, 'a>) -> ProvideResult<'r, 'a> {
    request
        .provide::<PathBuf>(&self.path)?
        .provide::<Path>(&self.path)?
        .provide::<dyn Debug>(&self.path)
}
```

The advantages of this design are that:

1. It supports accessing trait objects and slices
2. If the user specifies the type they are trying to pass in explicitly they
   will get compiler errors when the type doesn't match.
3. Less verbose implementation

The disadvatages are:

1. More verbose function signature, very lifetime heavy
2. The Request type uses unsafe code which needs to be verified
3. could encourage implementations where they pass the provider to
   `source.provide` first which would prevent the error reporter from knowing
   which error in the chain gathered each piece of context and might cause
   context to show up multiple times in a report.

# Prior art
[prior-art]: #prior-art

I do not know of any other languages whose error handling has similar
facilities for accessing members when reporting errors. For the most part prior
art exists within rust itself in the form of previous additions to the `Error`
trait.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What should the names of these functions be?
    - `context`/`context_ref`/`context_any`
    - `member`/`member_ref`
    - `provide`/`request`
- Should we go with the implementation that uses `Any` or the one that supports
  accessing dynamically sized types like traits and slices?
- Should there be a by value version for accessing temporaries?
    - I bring this up specifically for the case where you want to use this
      function to get an `Option<&[&dyn Error]>` out of an error, in this case
      its unlikely that the error behind the trait object is actually storing
      the errors as `dyn Errors`, and theres no easy way to allocate storage to
      store the trait objects.

# Future possibilities
[future-possibilities]: #future-possibilities

I'd love to see the various error creating libraries like `thiserror` adding
support for making members exportable as context for reporters.

Also, I'm interested in adding support for `Error Return Traces`, similar to
zigs, and I think that this accessor function might act as a critical piece of
that implementation.
