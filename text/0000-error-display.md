- Feature Name: separate_error_fmt
- Start Date: 2023-07-19
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

A new `error_fmt` method on `std::error::Error`,
so that we can distinguish:

 * Requests to just display an error for human consumption
   (`Display`)
 * The internal implementation of printing a particular error,
   excluding its `source`s (`error_fmt`).

Transitional and compatibility arrangements to make this workable.

# Motivation
[motivation]:

Correctly printing errors in Rust
(and defining errors that print correctly)
is too hard.

We want to be able to get from where we are now
to a situation with the following properties:

 * Just printing an error with `eprintln!("{error}")` will reliably
   do something useful.
 * Errors can be printed in a fancy report-like style with inspection
   of source errors, if desired.
 * Messages, and parts of them, are not duplicated.
 * Implementing an error type isn't significantly harder than today.
 * Warts (induced by backward compatibility requirements) are avoided
   as much as possible.

# Guide-level explanation (synchronic - where we want to end up)
[guide-level-explanation]: #guide-level-explanation

### Background (existing situation, will not be changed by this RFC)

Most errors should implement `std::error::Error`.

Errors can have a "source": an underlying error which caused this one.
That underlying error can in turn have a source,
forming a causal chain.

### Printing errors (new doctrine)

Errors can be printed in two main ways:
Every error implements `Display`
and provides an `error_fmt` method.

The `Display` implementation *does* print the source.
and should be used whenever an error (possibly and its causes)
needs to be printed for human consumption or logging.

The `error_fmt` method does *not* print the source of an error.
It is called to print the details of *this* error.

Normally, an implementor of an error will
provide an implementation of `error_fmt`.
There are macro packages in the crate ecosystem to help with this.

An implementor of an error type will usually
rely on a standard library default implementation
of `Display`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
pub trait Error: Debug + Display {
    /// Format *this* error (excluding its `source`, if there is one).
    ///
    /// The default implementation is provided for backward compatibility
    /// only; all new implementations of `Error` should provide an
    /// implementation of `error_fmt`.
    ///
    /// The default implementation uses `Self as Display`.
    fn error_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { ... }

    ...
}

/// Displays `E` and all its sources; output is similar to "error: source".
///
/// When used with `{:#}`, prints a multi-line "caused by" chain.
///
/// Does ad-hoc deduplication, as follows: Records the string of the
/// each error displayed, and suppresses printing of the source if the
/// source error text is textually contained within the previous error
/// text.
default impl<E> Display for E where E: Error { ... }
```

If neither `error_fmt`, nor an explicit `Display` impl,
is provided, a deny-by-default lint
(or perhaps a compilation failure) is triggered.

## Technical background

Rust is confused about how to print errors.
The key question is "should `Display` print the `source`"?
There is no good answer.

"Yes" implies that every error is responsible for its own formatting,
and can result in duplicated output.

"No" means that the `Display` implementation is a footgun:
if you just print an error in the most obvious way,
your program will print vacuous errors in the common case
where libraries wrap up errors from lower libraries.

This question has been considered by the
Error Handling Working Group.
Their [recommendation](https://blog.rust-lang.org/inside-rust/2021/07/01/What-the-error-handling-project-group-is-working-towards.html#guidelines-for-implementing-displayfmt-and-errorsource)
is that the answer should be "no".

This RFC proposes an alternative to that decision.
Principally,
because experience shows that the "vacuous error messages"
problem can be quite pervasive and severe.
By their nature, error paths are less well-tested,
so it is important that the obvious way of error handling is correct
(or that tooling will catch mistakes).

## Analysis

The problem stems from the fact that there are necessarily
two error printing concepts:

 1. Reporting a whole error including its sources,
 
 2. Printing only *this* error

Here, (2) forms part of the implementation of (1).
The operation (1) of printing a whole error chain
can be done in terms of the `source()` method and
(2) printing individual errors.

The question is:
what should these two APIs be called
and where should they live?

The EHWG recommendation answers this as:
(1) should be provided by a separate reporting function,
such as a (not yet existing) stdlib facility,
or crates like `anyhow` and `eyre`.
(2) should be provided through the `Display` impl.
But this approach is is wrong:
the "usual" way of printing an error should be (1),
and that is what the `Display` impl ought to mean
(since that is what `Display` is *for*).

In this RFC we answer these questions as follows:

 1. Reporting a whole error is done by `Display`ing it,
    or by using a special library if you want more control.

 2. The implementation API for "print just this error"
    is a new trait method `Error::error_fmt`.

The remainder of the RFC follows from this decision,
and from the need to maintain backwards compatibility.

## Transition plan

 1. Introduce the new `error_fmt` method
    and default `Display` impl
    (including necessary language/compiler features).

 2. Packages whose MSRV is new enough
    implement `error_fmt` instead of `Display`.

 3. For example, macro packages like `thiserror` release a major version:

      1. newer MSRV
      2. implement `error_fmt` (as per 2.)
      3. fail to compile if a provided format error string
         includes the error's source.

 4. In the 2024 edition,
    issue a warning for use of the provided `error_fmt`
    (ie, for non-implementation of `error_fmt`).

# Drawbacks
[drawbacks]: #drawbacks

 * This is reversing a recommendation by the Error Handling Working Group.
   (This recommendation is not, however, present in the stdlib documentation.)

 * Almost every implementor of `Error` will need to change eventually.
   (But this is often done with macro packages.)

 * The ad-hoc deduplication in the default `Display`
   impl is rather unprincipled,
   and involves rather too much boxing.
   (However, it is simple and effective.)

 * This exposes the use of specialisation in the stdlib API.

 * This introduces the use of `#[feature(specialization)]` to `core`
   rather than just `min_specialization`.
   Moreover, the proposed blanket impl does not compile with current Rust.
   Compiler work would be needed.

 * Additionally, compiler work may be needed to provide the lint
   for failure to manually implement either `Display` or `error_fmt`.

 * Codebases that wish to avoid using the default error formatting,
   and always want to use a custom reporter,
   will need to somehow find a way to lint for that.
   This is not a thing that clippy can currently do.

# Alternatives
[alternatives]: #alternatives

## Firm up EHWG recommendation to *not* include source in `Display`

If that recommendation were followed by all types implementing `Error`,
and all programs that wanted to print errors
didn't just use `Display`,
but some reporting facility that does print sources,
then programs would have correct behaviour overall.

Achieving this, and maintaining that state, is not trivial.
It would probably involve:

 * A new lint when an `Error`'s `Display` is used,
   but `Error::source` isn't called "nearby".
   This new lint is necessary to catch the easy mistake
   of printing an error without its source;
   experience shows that this mistake can be ubiquitous in codebases
   that adopt the EHWG recommendation.

 * A convenient new facility in the stdlib for printing errors.
   For example, a new provided method on `Error`
   that returns something that is `Display`
   and which prints the error and all its sources.

A downside of this approach is that the `Display`
impl for every error is forever "wrong":
normally, `Display` prints a thing in the most usual way,
but for errors, `Display` is part of the implementation,
and actual printing must be done with some kind of reporter.

## Marker trait or macro for implementing `Display`

Instead of `default impl Display for Error`,
we could have a library function for use in `Display` impls,
and a macro that implements `Display` in terms of it.

But macro calls have a much less obvious meaning
to the reader of the code.

Alternatively, there could be a marker trait:

```rust
pub trait ErrorDisplay { }

/// Displays `E` and all its sources; output is similar to "error: source".
///
/// When used with `{:#}`, prints a multi-line "caused by" chain.
/// Does ad-hoc deduplication.
impl<E> Display for E where E: Error + ErrorDisplay { ... }
```

But the blanket
`impl<E> Display for E where E: Error + ErrorDisplay`
is rejected by the current compiler,
because of a conflict with the blanket
`Display` impls for references, `Pin` etc,
if user crates `impl ErrorDisplay for &...`.
This would still need to be dealt with by specialisation.

## Declare a difference between `{:#}` and `{}`

We could say that whether to include sources should depend on
`fmt::alternate()`, which comes from the `#` in `{:#}`.

However:

 * Conceptually, this is wrong.
   The two kinds of output are not different styles of display
   of the same information;
   indeed, they aren't really sensibly used by the same callers.
   Sources should *always* be included in errors shown to the user.
   When omitting them is required, it is not because they are clutter,
   but because somewhere else in the reporting machinery is printing them.

 * Formatting without the source is needed only
   by error reporting/formatting machineries,
   of which there are going to be relatively few
   (and their authors will be error display experts).
   Conversely, most programmers must frequently write code to
   display of errors to the human user,
   and in that case the sources should be included.
   That suggests `{}` should include the source and
   `{:#}` should exclude it.
   But usually the output from `{:#}` is longer,
   whereas here it would be shorter.
   And `eyre::Report` has the opposite convention.

 * The two kinds of display want to be implemented in different places:
   we want to provide a default implementation of
   the user-visible display including sources;
   conversely, we want errors to define the display
   of their own content.
   But `fmt::alternate()` isn't sensible to use for dispatch.

 * `{:#}` vs `{}` has a better potential meaning for errors:
   do we display everything on a single line,
   or in multi-line "caused by" format.

## Replace the `Error` trait completely

This would be a very big job
and probably highly disruptive.

It might involve inventing a new mechanism for allowing
evolution of stdlib traits across editions,
or something.

## Do nothing

We could let the ecosystem blunder on,
perpetrating programs that produce
vacuous or duplicated error messages.

# Prior art
[prior-art]: #prior-art

The problems with the `Error` trait are specific to Rust.

The EHWG [recommends](https://blog.rust-lang.org/inside-rust/2021/07/01/What-the-error-handling-project-group-is-working-towards.html#guidelines-for-implementing-displayfmt-and-errorsource) not to print error sources as part of `Display`.

`anyhow::Error` etc. don't implement `std::error::Error`.
They *do* implement a useful `Display`
which includes all error sources.

`eyre::Report` provides a way to define the way errors are reported.
Like `anyhow::Error`, it doesn't implement `std::error::Error`.
`eyre::Report` includes error sources when printed with `{:#}`
and not when printed with `{}`.

[`snafu::CleanedErrorText`](https://docs.rs/snafu/latest/snafu/struct.CleanedErrorText.html)
implements textual error message deduplication
which is similar in spirit to that proposed in this RFC.

[Arti](https://gitlab.torproject.org/tpo/core/arti)'s
codebase follows the EHWG recommendation, and
has [tools](https://docs.rs/tor-error/0.5.2/tor_error/trait.ErrorReport.html)
for use in error display contexts (such as logging).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 * What should `error_fmt` be called.

 * What about `no_std`?
   The proposed ad-hoc duplication can't sensibly be done without all cation.

 * What about localisation and message translation?
   Are future efforts in that area going to render this all moot?

 * Should there be a way for someone who has an `Error`
   to tell if `error_fmt` was defaulted to "use `Display`" ?
   Without this, we might never be able to get rid of 
   the extra string formatting and allocations.

# Future possibilities
[future-possibilities]: #future-possibilities

Hopefully this will be the last churn in this area.

The default error reporter with string deduplication
could use some magic to discover whether the provided
`error_fmt`-in-terms-of-`Display` was being used by a particular error.
If it *isn't* it knows it won't need to deduplicate it;
it then doesn't need to format to a string.
So the old efficiency is regained.
