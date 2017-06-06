- Feature Name: unwrap-macro
- Start Date: 2016-07-07
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add an `unwrap!` macro to the standard library. This behaves the same as
calling the `unwrap`/`expect` methods except it allows for context-aware
reporting of panics with a format string and arguments.

# Motivation
[motivation]: #motivation

Currently, if a user calls `unwrap` or `expect` on an `Option` or `Result` the
generated error message reports the error as coming from inside the
implementation of `Option`/`Result`. It would be much more useful if the error
message instead pointed to the site where `unwrap`/`expect` was invoked. It's
possible to get this information by running a program with `RUST_BACKTRACE=1`
and watching it crash, but this requires forethought and is useless when all
you have is a crash report and you don't know how to reproduce the bug.

By instead invoking unwrap through a macro it's possible to pass in contextual
information such as file and line number to the underlying method.

# Detailed design
[design]: #detailed-design

The design is already published in a crate here:
https://crates.io/crates/unwrap. It's being used heavily at
[MaidSafe](https://github.com/maidsafe) where it has become the standard way of
unwrapping. I'm raising an RFC for it because I think it's just plain better
than what's currently offered by the standard library - once you have the
`unwrap!` macro available there's (almost) no reason to ever use the
`unwrap`/`expect` methods and forego having the additional useful information.

The full design consists of a trait:

    pub trait VerboseUnwrap {
        type Wrapped;
        fn verbose_unwrap(self, message: Option<Arguments>,
                                module_path: &str,
                                file: &str,
                                line_number: u32,
                                column: u32) -> Self::Wrapped;
    }

This trait is implemented for `Option<T>` and `Result<T, E> where E: Debug`.
Then there's a macro, `unwrap!`, which can be invoked with an optional format
string and arguments.

    macro_rules! unwrap(
        ($e:expr) => (
            $crate::VerboseUnwrap::verbose_unwrap($e, None,
                                                      module_path!(),
                                                      file!(),
                                                      line!(),
                                                      column!())
        );
        ($e:expr, $($arg:tt)*) => (
            $crate::VerboseUnwrap::verbose_unwrap($e, Some(format_args!($($arg)*)),
                                                      module_path!(),
                                                      file!(),
                                                      line!(),
                                                      column!())
        );
    );

This macro can then be used in place of `unwrap`/`expect` as such:

    x.unwrap();
    // becomes
    unwrap!(x);

    x.expect("There was an error!");
    // becomes
    unwrap!(x, "There was an error");
    // or possibly
    unwrap!(x, "There was an error: {:?}", some_useful_info);

# Drawbacks
[drawbacks]: #drawbacks

* Using these methods may add slightly to the size of binaries. The binaries
  will need to contain the extra information such as `module_path!()` and
  `file!()` strings even though, ideally, this informtion will never get used.
* Adds yet more stuff to the standard library.

# Alternatives
[alternatives]: #alternatives

Not do this.

# Unresolved questions
[unresolved]: #unresolved-questions

Should the `VerboseUnwrap` trait be called something else (such as just
`Unwrap`)? Should the `unwrap`/`expect` methods on `Option`/`Result` be moved
into the trait?

