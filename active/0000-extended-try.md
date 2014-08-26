- Start Date: 2014-06-25
- RFC PR:
- Rust Issue:

# Summary

Extend the `try!` macro to include an optional second parameter that is a constructor to wrap
around the original error in case of failure.

# Motivation

`try!` is a useful macro when dealing with many functions that return `Result`s, but they become
useless when the `Result` type that the programmer wants to return has a different failure type.
For example, in a function that uses Io and Regex, two different error types could be generated
(IoError, and Regex::Error).  The author could not choose either of these errors to return because
neither is extendable with the other.  Instead it is common for library and application authors
to create their own error types that wrap the errors that could possibly be produced.  Unfortunately,
this means that the `try!` macro is no longer useful.

# Detailed design

This RFC proposes adding another argument to the `try!` macro that would be used as a constructor
to wrap around existing error types.  For example:

```rust
enum MyError {
  IoFailed(IoError),
  RegexFailed(regex::Error)
}

fn read_then_regex(filename: &str, regex: &str) -> MyError {
   let file = try!(File::open(filename), IoFailed);
   let regex = try!(Regex::new(regex), RegexFailed);
   // do things
}

```

The desugared version of this example (which is required to implement this pattern today)
would look like:

```rust
fn read_then_regex(filename: &str, regex: &str) -> MyError {
   let file = match File::open(filename) {
     Ok(a) => a,
     Err(x) => IoFailed(x)
   };
   let regex = match Regex::new(regex) {
     Ok(a) => a,
     Err(x) => RegexFailed(x)
   };
   // do things
}
```

The motivation for this improvement is the exact same as the motivation for the original `try!`
macro.

The original form of the `try!` macro would still be valid and would continue to work without
any changes.

# Drawbacks

Adds confusion.  It is not immediately obvious as to what the 2nd argument is for if
the reader is not already familiar with it.

# Alternatives

* Create another macro that is very similar
  * (named `try_or!` ?).
* Create another macro that is very similar and place it in an external library
