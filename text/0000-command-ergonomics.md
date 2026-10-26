- Feature Name: (fill me in with a unique ident, `my_awesome_feature`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Improve the ergonomics of `std::process::Command`,
and better protect programmers from error handling mistakes.

This RFC is principally an attempt to
gain consensus on a fleshed out version of
the MR [#89004](https://github.com/rust-lang/rust/pull/89004)
which proposed to add some convenience APIs to `std::process::Command`.

# Motivation
[motivation]: #motivation

The current API for `std::process::Command`
makes it unnecessarily difficult to perform common tasks,
such as "simply running a program",
and running a program and collecting its output.

The APIs that are currently provided invite mistakes
(for example, failing check the exit status in `Output`,
and deadlock errors).
Writing code that is fully correct
and produces good error messages is cumbersome,
and sometimes subtle.

Running subprocesses is inherently complex, and provides many opportunites for errors to occur - so there are some questions about how to best represent this complexity in a convenient API.

Existing work in this area has been fragmented into a number of 
MRs and issues, making it hard to see the wood for the trees,
and has become bogged down due to lack of decision
on the overall approach.  Some references:

 * [#84908](https://github.com/rust-lang/rust/issues/84908)
   Stabilisation tracking issue for `ExitStatusError` 
 * [#81452](https://github.com/rust-lang/rust/pull/81452)
   MR, closed as blocked: Add `#[must_use]` to `process::Command`, `process::Child` and `process::ExitStatus` 
 * [#89004](https://github.com/rust-lang/rust/pull/89004)
   MR, closed: Add convenience API to `std::process::Command`
 * [#88306](https://github.com/rust-lang/rust/pull/88306)
   MR, closed: `ErrorKind::ProcessFailed` and `impl From<ExitStatusError>`
 * [#93565](https://github.com/rust-lang/rust/pull/93565)
   MR, draft: `impl Try for ExitStatus`
 * [#73126](https://github.com/rust-lang/rust/issues/73126)
   issue: `std::process::Command` `output()` method error handling hazards
 * [#73131](https://github.com/rust-lang/rust/issues/73131)
   Overall tracking Issue for `std::process` error handling

## Currently-accepted wrong programs

The following incorrect program fragments are all accepted today and run to completion without returning any error:

```rust
    Command::new("touch")
        .args(&["/nonexistent/touch-1"]);
    // ^ programmer surely wanted to actually run the command

    Command::new("touch")
        .args(&["/nonexistent/touch-2"])
        .spawn()?;
    // ^ accidentally failed to wait, if programmer wanted to make a daemon
    //   or zombie or something they should have to write let _ =.

    Command::new("touch")
        .args(&["/nonexistent/touch-3"])
        .spawn()?
        .wait()?;
    // ^ accidentally failed to check exit status
```

Corrected versions tend to have lots of boilerplate code,
especially if good error messages are wanted.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We will introduce new APIs on `Command`,
for running the command and collecting its output:

```rust
impl Command {
    fn run(&mut self) -> Result<(), ProcessError>;
    fn read_stdout(&mut self) -> Result<Vec<u8>, ProcessError>;
    fn read_stdout_string(&mut self) -> Result<String, ProcessError>;
    fn stdout_reader(&mut self) -> impl std::io::Read;
}
struct ProcessError { ... }
impl From<ProcessError> for io::Error { ... }
```

No significant changes are made to the `Command` construction APIs,
but it may become necessary to call `.stderr()` explicitly
to use the new methods (see Unresolved Questions).

## Use cases

We aim to serve well each of the following people:

 * Alice writes a CLI utility to orchestrate processes and wants top-notch error reporting from subprocesses.

 * Bob migrates ad-hoc automation scripts from bash to Rust.

 * Carol writes a generic application and just wants all errors to be maximally useful by default.

 * Dionysus wants to run `diff`, which exits `0` for "no difference",
   `1` for "difference found" and
   another value for failure.

(Partly cribbed from one of
@matklad's [comments](https://github.com/rust-lang/rust/pull/89004#issuecomment-923803209) in #89004)

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## New methods on `Command`:

 * `fn run(&mut self) -> Result<(), ProcessError>`:

   Runs the command.
   Equivalent to `.spawn()` followed by `.status()`,
   but with better error handling.

 * `fn read_stdout(&mut self) -> Result<Vec<u8>, ProcessError>`:

   Runs the command and collects its stdout.
   After the child indicates EOF on its stdout,
   we will wait for it to finish and check the exit status.

 * `fn read_stdout_string(&mut self) -> Result<String, ProcessError>`:

   Runs the command and collects its stdout, as for `read_stdout`.
   Decodes the stdout as UTF-8, and fails if that's not possible.
   Does not trim any trailing line ending.

 * `fn stdout_reader(&mut self) -> std::process::ChildOutputReader`
   (where `struct ChildOutputReader` implements `io::Read`
   and is `Send + Sync + 'static`).

   Starts the command, allowing the caller to
   read the stdout in a streaming way.
   Neither EOF nor an error will be reported by `ChildOutputReader`
   until the child has exited, *and* the stdout pipe reports EOF.
   (This includes errors due to nonempty stderr,
   if stderr was set to `piped`.)

Most callers should use these methods,
rather than `.spawn()` or `.status()`.
These new methods should be be
recommended by docs for other, lower-level functions
which people were previously required to use.

## Deprecations

 * Apply `#[must_use]` to `Command` and `Child`.

 * Apply `#[must_use]` to `ExitStatus`.
   (May require fixing quite a few of the examples.)

 * Add a warning to the docs for `Command.output()` about the lost error bugs,
   in particular the need to check `.status` and the lack of any compiler
   warning if one doesn't.  Suggest to the reader to consider
   `.run()` or `.read_stdout*` instead.
   Do not deprecate `.output()` though.

## stderr handling

If `stderr(Stdio::piped())`,
these new functions all collect the child's stderr.
Then,
if the stderr output is nonempty, this is considered an error,
and reported in the `ProcessError`.

These functions *do not* wait for EOF on stderr.
Rather, they wait for child process termination and expect that
any relevant error messages have been printed by that point.
Any further stderr output 
(for example, from forked but unawaited children of the command)
might not be reported via the Rust std API.
Such output might be ignored,
or might accumulate in a temporary disk file not deleted
until the last escaped handle onto the file has gone.
Ideally, any escaped writer(s) would experience broken pipe errors.

This non-waiting behaviour on stderr could be important
when Rust programs invoke tools like `ssh`,
which sometimes hang onto their stderr well after the
intended remote command has completed.

The implementation may involve a temporary file,
or an in-memory buffer,
or both.

## New `struct ProcessError`

This new struct is used as the error type for the new methods.

It can represents zero or more of the various
distinct problems that can occur while running a process.
A `ProcessError` returned by a `std` function will always
represent at least one problem (unless otherwise stated),
but it may represent several

For example a process which exited nonzero
probably printed to stderr;
with `piped` we capture that, and represent both the stderr
and the exit status as problems within the `ProcessError`.

```rust
/// Problem(s) which occurred while running a subprocess
///
/// This struct represents problems which occurred while
/// running a subprocess.
///
/// Running a subprocess is complex, and it is even possible for a single invocation
/// to give rise to more than one problem.
/// So this struct can contain zero or more such problems,
/// along with information about what was run, for error reporting.
///
/// ### Tolerating certain kinds of error
///
/// Don't check the fields of this struct one by one.
/// Future language revisions may add more fields representing new kinds of problem!
///
/// Instead:
///
///  * If you want to capture stderr but combine it with stdout,
///    TODO need way to combine them!
///    
///  * If you wish to tolerate only nonzero exit status, call `.just_status()`.
///
///  * If you wish to tolerate other particular kind(s) of problem,
///    set the field for the problems you want to tolerate to `None`
///    (doing any ncecessary checks on the the existing values,
///    to see if it's really something you want to ignore).
///    Then call `.has_problem()`.
#[must_use]
#[non_exhaustive]
struct ProcessError {
    /// The program, if we know it.
    //
    // Used in the `Display` impl so we get good error messages.
    pub program: Option<OsString>,

    /// The arguments, if we know them.
    //
    // Used in the `Display` impl so we get good error messages.
    pub args: Vec<OsString>,

    /// If the stdout was captured in memory, the stdout data.
    //
    // Needed so that a caller can have the output even if the program failed.
    pub stdout_bytes: Option<Vec<u8>>,

    /// If the process exited and we collected its status, the exit status.
    ///
    /// If this is present and not success, it is treated as a problem.
    pub status: Option<ExitStatus>,

    /// If the stderr was captured in memory, the stdout data.
    ///
    /// If this is present and nonempty, it is treated as a problem.
    pub stderr_bytes: Option<Vec<u8>>,

    /// If had a problem spawning, the spawn error.
    pub spawn_error: Option<io::Error>,

    /// If we had some other problem, that error.
    ///
    /// This could a problem talking to the child, or collecting its exit status.
    ///
    /// This might include problems which might be caused by child
    /// misbehaviour.
    //
    // In an earlier draft this was `communication_error: Option<io::Error>`.
    // But an `io::Error` is not ideal,
    // because we need to represent what we were doing, for reporting in messages,
    // (so it would have to a custom type boxed inside the `io::Error` anyway)
    // and the `io::ErrorKind` isn't very meaningful.
    //
    // Communication errors like this are going to be rare
    // (at least, on Unix, I think they "can never happen"
    // barring bugs in the kernel, stdlib or libc,
    // unreasonable signal dispositions,
    // UB or fd bugs in the Rust program, or the like).
    // We don't want to expose lots of complicated details here,
    //
    // Also, these aren't usefully tolerable by applications.
    // So a Box<dyn Error> is good enough.
    //
    // Making it `other` allows it to be used by outside-stdlib
    // constructors of process errors.
    pub other_error: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,

    /// If we had a problem converting stdout to UTF-8.
    ///
    /// The `error_len()` and `valid_up_to()` reference positions in `stdout_bytes`.
    pub utf8_error: Option<std::str::FromUtf8Error>,
}
impl Debug for ProcessError {
    // print all the fields except `stdout_bytes`.
}
impl Error for ProcessError {
    // fn cause() always returns None.
}
```

The `Display` implementation will print everything above
including the command's arguments.
The arguments will be escaped or quoted in some way that renders 
a resulting error message unambiguous.

### `impl From<ProcessError> for io::Error`

`ProcessError` must be convertible to `io::Error`
so that we can use it in `ChildOutputReader`'s
`Read` implementation.
This may also be convenient elsewhere.

The `io::ErrorKind` for a `ProcessError` will be:

  * The `io::ErrorKind` from the spawn error, if any.

  * Otherwise, a new kind `io::ErrorKind::ProcessFailed`,
    which means that the subprocess itself failed.

### Further APIs for `ProcessError`

A `ProcessError` is a transparent `Default` struct so it can be
constructed outside std, for example by async frameworks,
or other code which handles launching subprocesses.

We propose the following additional methods:

```rust
impl ProcessError {
    /// Makes a "blank" error which doesn't contain any useful information
    ///
    /// `has_problem()` will return `false` until one of the setters
    /// is used to store an actual problem.
    //
    // This is equivalent to `ProcessError::default()`, but provides
    // a more semantically meaningful name, making it clear that it
    // returns an empty error that needs filling in.
    fn new_empty() -> Self { }

    // If we keep ExitStatusError
    fn from_exit_status_error(status: ExitStatusError) -> Self { }

    /// Find out if this error contains any actual error information
    ///
    /// Returns `false` for a fresh blank error,
    /// or `true` for one which has any of the error fields set.
    /// (Currently equivalent to checking all of `status()`,
    /// `stderr_bytes()`, `spawn_error()` and `utf8_error()`.)
    //
    // We must provide this because it's needed for handling programs
    // with unusual exit status conventions (eg `diff(1)`)
    // and a caller can't reimplement it without making assumptons
    // about `ProcessError`'s contents.
    fn has_problem(&self) -> bool;

    /// Returns `Ok<ExitStatus>` if the only reason for the failure was a nonzero exit status.
    /// Otherwise returns `self`.
    ////
    /// Use this if you want to to tolerate some exit statuses,
    /// but still fail if there were other problems.
    //
    // This is optional, and could be a separate feature from the rest of the RFC.
    // But it does make running programs like `diff` considerably easier.
    // (It is also implementable externally in terms of .has_problem()`.)
    pub fn just_status(self) -> Result<ExitStatus, ProcessError>;
}
impl Default for ProcessError { ... }
```

# Drawbacks
[drawbacks]: #drawbacks

This is nontrivial new API surface.

Much of the new API surface is in `ProcessError`.
If we didn't want to try to make it easy for Rust programmers
to run subprocesses and produce good error messages,
we could omit this error type 
(perhaps using something like `ExitStatusError`).

Perhaps we don't need all the `read_stdout` variants,
and could require Bob to write out the boilerplate
or provide his own helper function.

Perhaps we don't need `stdout_reader`.
However,
avoiding deadlocks when reading subprocess output,
and also doing error checks properly,
is rather subtle.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Alternatives and prior proposals include:

 * `ExitStatusError` (tracking issue [#84908](https://github.com/rust-lang/rust/issues/84908))

    Currently, exists but unstable.
    Ergonomics of using this to produce good error messages are rather poor,
    because the `ExitStatusError` is just the exit status.

 * `impl Try for ExitStatus`
    [#93565](https://github.com/rust-lang/rust/pull/93565)

    libs-api team are
    "[hesitant](https://github.com/rust-lang/rust/pull/93565#issuecomment-1367557592) to add more `Try` implementations".
    Like `ExitStatusError`, it is difficult to see how this could produce good error messagesx
    without a lot of explicit code at call sites.

 * Previous attempt at `Command::run()` and `Command::read_stdout()`
   [#89004](https://github.com/rust-lang/rust/pull/89004).

   Seemed to be going in the right direction
   but got bogged down due to lack of consensus on overall direction
   and some bikeshed issues.

 * The `ProcessError` type could be opaque with getters and setters.
   Transparent structs are unfashionable in modern Rust.
   However, providing getters and setters obscures what's going on and
   greatly enlarges the API surface.

 * Instead of a single `ProcessError` type used everywhere,
   there could be a different error type for the different calls.
   For example, `run()` could have a different error type to
   `read_stdout()`,
   since `run` doesn't need to represent UTF-8 conversion errors.
   This would not be very in keeping with the rest of `std::process`,
   which tends to unified types with variation selected at runtime.
   There would still have to be *a* type as complex as `ProcessError`,
   since that's what `read_stdout_read` needs.

 * Maybe `ProcessError::other_error` ought not to exist yet,
   and we should have a separate `ProcessError::communication_error`.

# Prior art
[prior-art]: #prior-art

Many other languages have richer or more convenient APIs
for process invocation and output handling.

 * Perl's backquote operator has the command inherit the script's stderr. If you want to do something else you need to do a lot of hand-coding. It reports errors in a funky and not particularly convenient way (but, frankly, that is typical for Perl). It doesn't trim a final newline but Perl has a chomp operator that does that very conveniently.

 * Tcl's `exec` captures stderr by default (there are facilities for redirecting or inheriting it), calling any stderr an error (throwing a Tcl exception). It always calls nonzero exit status an error (and there is no easy way to get the stdout separately from the error in that situation). It unconditionally chomps a final newline.

 * Python3's `subprocess.run` fails to call nonzero exit status an exception. If you don't write explicit error handling code (highly unusual in Python) you have an unchecked exit status bug.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Printing command arguments in `impl Display for ProcessError`

Perhaps printing the command arguments is overly verbose,
and we should print only the command name.

Sometimes people pass sensitive information (such ass passwords) in command line arguments.
This is not a good idea in portable software,
because command line arguments are generally public on Unix.

Perhaps some option could be added in the future to control this.
For now, we propose always printing the arguments.

## `cause` in `Error` impl of `ProcessError`

Because maybe several things went wrong, ever providing a `Some` `cause`
would involve prioritising multiple possible problems.

Also, (IMO doubtful) EHWG guidelines about `Display` implementations
say that we shouldn't include information about the cause in our own `Display`.

This leaves us with the following options:

 1. Not include the actual thing that went wrong in our `Display`.
    This would in practice result in very poor error messages from many 
    natural uses of this API.
    Also whether problem A appears in the `Display` output might depend
    on whether "higher-priority cause" B is present.
    This seems madness.

 2. Violate the EHWG guideline.
    (Or try to get it deprecated.)

 3. Not include a `cause` at all.
    This is the option we propose.

## `read_stdout` etc. naming.

We have `fs::read_to_string`.

Possible names (taking `read_stdout_string` as the example):

 * `read_stdout_string` (proposed in this RFC)
 * `stdout_string` but `stdout` is alread taken.
 * `get_stdout_string`
 * `run_get_stdout_string`
 * `run_stdout_string`

It is difficult to convey everything that is needed in a short name.
In particular, all of these functions spawn the program,
and wait (at an appropriate point) for it to exit.

Should `read_stdout` be the one that returns `Vec<u8>`
or the one that returns `Vec<String>` ?
Precedent in the stdlib is usually to have the bytes version undecorated
(eg, `fs::read_to_string`).

## stderr handling default

What should happen if the Rust programmer didn't call `.stderr()` ?

Options are:

 1. Treat it as `inherit`.
   This is probably fine for a command line application,
   but may be too lax for many other contexts.

 2. Treat it as `piped`: call any stderr output an error.
   This is a fairly conservative choice,
   but it can lead to unexpected runtime errors,
   if a called program later starts printing warning messages.

 3. Call this a programming mistake, and panic.

 4. Somehow make this a compile error.
   This would involve a new `CommandForWhichWeHaveSpecifiedStderr`
   type (typestate pattern), or providing the stderr handling as a mandatory argument
   to all the new methods.
   These seem unpalatably unergonomic.

5. One of the above, and additionally provide a lint that makes a best-effort attempt to catch this at compile time.

Here we propose option 1: treat as `inherit`.

# Examples

## Conveniently running `diff`

```rust
    let result = Command::new("diff")
        .args(["before","after"])
        .stderr(Stdio::piped()) // optional, could just let it inherit
        .run();
    let status = match result {
        Ok(()) => 0,
        Err(err) => err.just_status()?.code(),
    };
```

# Future possibilities
[future-possibilities]: #future-possibilities

## Providing a way to combine and interleave stdout and stderr

Currently, `Command` insists on separating out stdout and stderr,
if you ask to capture them.
If you want them combined
(which is the only way to preserve the relative ordering)
you must do one of:

 * run a command which itself does the redirection
   (easy using the shell on Unix)

 * send them each to your own stdout/stderr with `inherit`
   and expect your caller to combine them

 * send them to the *same* one of your stdout/stderr
   which will be possible after
   https://github.com/rust-lang/rust/pull/88561)

We should provide something like this:

```
impl Command {
    /// Arranges that the command's stderr will be sent to wherever its stdout is going
    fn stderr_to_stdout();
}
```

(It [can be difficult or impossible](https://docs.rs/io-mux/latest/io_mux/) at least on Unix
to reliably get all of the stdout and stderr output
and find out *both* what order it out came in,
*and* which data was printed to which stream.
This is a limitation of the POSIX APIs.)

## Provide a way to read exactly a single line

```
 * `fn read_stdout_line(&mut self) -> Result<String, ProcessError>`:

   Runs the command and collects its stdout, as for `read_stdout`.
   Decodes the stdout as UTF-8, and fails if that's not possible.
   Fails unless the output is a single line (with or without line ending).
   Trims the line ending (if any).
   If program prints no output at all, returns an empty string
   (and this cannot be distinguished from the program printing just a newline).
```

It's not clear if this ought to live here or
as a method on `String`.

## Deprecating `Command.output()` and `std::process::Output`

The `.output()` and `Output` API has an error handling hazard,
see [#73126](https://github.com/rust-lang/rust/issues/73126).

Perhaps it should be deprecated at some point.

However, it is a popular API so that would be disruptive,
and some things are easier to do with `Output` than with `Result<..., ProcessError>`.

## Changes to `ExitStatusError`

Possibilities include:

 * Abolish `ExitStatusError`
 * Stabilise `ExitStatusError` as-is
 * `impl From<ExitStatusError> for ProcessError`
 * `impl From<ExitStatusError> for io::Error`

Error messages from `ExitStatusError` are rather poor,
and it is likely that `ProcessError` will
subsume most of its use cases.

## Async ecosystem could mirror these APIs

 * An async versions of `run()` seems like it would be convenient.
 * Async versions of the output-capturing runners too.
 * Async frameworks ought to (be able to) use `ProcessError`.

## More flexible and less synchronous output handling

We could provide a more concurrent API,
which allows a Rust program to experience the outcomes of
running a subprocess
(stdout output, stderr output, exit status)
as a series of events or callbacks,
and interleave waiting for the process with writing to its stdin.

Options might include:

 * Allow the Rust programmer to supply an implementation of `Write` for process stdout for handling process stdout and stderr, via a new constructor `std::process::Stdio::from_write`.

 * Provide and stabilise something like cargo's internal function
   [`read2`](https://github.com/rust-lang/cargo/blob/58a961314437258065e23cb6316dfc121d96fb71/crates/cargo-util/src/read2.rs)

 * Expect users who want this to use pipes by hand (perhaps with threads), or async.
