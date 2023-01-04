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
        .args(&["/dev/enoent/touch-1"]);
    // ^ programmer surely wanted to actually run the command

    Command::new("touch")
        .args(&["/dev/enoent/touch-2"])
        .spawn()?;
    // ^ accidentally failed to wait, if programmer wanted to make a daemon
    //   or zombie or something they should have to write let _ =.

    Command::new("touch")
        .args(&["/dev/enoent/touch-3"])
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

```
impl Command {
    fn run(&mut self) -> Result<(), SubprocessError>;
    fn get_output_bytes(&mut self) -> Result<Vec<u8>, SubprocessError>;
    fn get_output(&mut self) -> Result<String, SubprocessError>;
    fn get_output_line(&mut self) -> Result<String, SubprocessError>;
    fn get_output_read(&mut self) -> impl std::io::Read;
}
struct SubprocessError { ... }
impl From<SubprocessError> for io::Error { ... }
```

The `.output()` function and `std::process::Output`
will be deprecated.

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

 * `fn run(&mut self) -> Result<(), SubprocessError>`:

   Runs the command.
   Equivalent to `.spawn()` followed by `.status()`,
   but with better error handling.

 * `fn get_output_bytes(&mut self) -> Result<Vec<u8>, SubprocessError>`:

   Runs the command and collects its stdout.
   After the child indicates EOF on its stdout,
   we will wait for it to finish and check the exit status.

 * `fn get_output(&mut self) -> Result<Vec<String>, SubprocessError>`:

   Runs the command and collects its stdout.
   Decodes the stdout as UTF-8, and fails if that's not possible.
   Does not trim any trailing line ending.

 * `fn get_output_line(&mut self) -> Result<Vec<String>, SubprocessError>`:

   Runs the command and collects its stdout.
   Decodes the stdout as UTF-8, and fails if that's not possible.
   Fails unless the output is a single line (with or without line ending).
   Trims the line ending (if any).

 * `fn get_output_read(&mut self) -> std::process::ChildOutputStream`
   (where `struct ChildOutputStream` implements `io::Read`
   and is `Send + Sync + 'static`).

   Starts the command, allowing the caller to
   read the stdout in a streaming way.
   Neither EOF nor an error will be reported by `ChildOutputStream`
   until the child has exited, *and* the stdout pipe reports EOF.
   (This includes errors due to nonempty stderr,
   if stderr was set to `piped`.)

Most callers should use these methods,
rather than `.spawn()` or `.status()`.
These new methods should be be
recommended by docs for other, lower-level functions
which people were previously required to use.

## Deprecations

 * Deprecate `std::process::Command::output()`.
   This API cannot be fixed;
   see [#73126](https://github.com/rust-lang/rust/issues/73126).

 * Apply `#[must_use]` to `Command` and `Child`.

 * Apply `#[must_use]` to `ExitStatus`.
   (May require fixing quite a few of the examples.)

## stderr handling

If `stderr(Stdio::piped())`,
these new functions all collect the child's stderr.
Then,
if the stderr output is nonempty, this is considered an error,
and reported in the `SubprocessError`.

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

## New `struct SubprocessError`

This new struct is used as the error type for the new methods.

It can represents zero or more of the various
distinct problems that can occur while running a process.
A `SubprocessError` returned by a `std` function will always
represent at least one problem (unless otherwise stated),
but it may represent several

For example a process which exited nonzero
probably printed to stderr;
with `piped` we capture that, and represent both the stderr
and the exit status as problems within the `SubprocessError`.

```
impl SubprocessError {
    /// The program, if we know it.
    fn program(&self) -> Option<&OsStr>;

    /// The arguments, if we know them.
    fn args(&self) -> Option<std::process::CommandArgs<'_>>;

    /// If the stdout was captured in memory, the stdout data.
    fn stdout_bytes(&self) -> Option<&[u8]>;

    /// If the process exited and we collected its status, the exit status.
    fn status(&self) -> Option<ExitStatus>;

    /// If trouble included nonempty stderr, the captured stderr
    fn stderr_bytes(&self) -> Option<&[u8]>;

    /// If trouble included failure to spawn, the spawn error.
    fn spawn_error() -> Option<&io::Error>;

    /// If trouble included failure to talk to the child, the IO error.
    ///
    /// This might include problems which might be caused by child
    /// misbehaviour.
    fn communication_error() -> Option<&io::Error>;

    /// If trouble included failed UTF-8 conversion.
    fn utf8_error(&self) -> Option<&std::str::FromUtf8Error>;
}
```

The `Display` implementation will print everything above
including the command's arguments.
The arguments will be escaped or quoted in some way that renders 
a resulting error message unambiguous.

### `impl From<SubprocessError> for io::Error`

`SubprocessError` must be convertible to `io::Error`
so that we can use it in `ChildOutputStream`'s
`Read` implementation.
This may also be convenient elsewhere.

The `io::ErrorKind` for a `SubprocessError` will be:

  * The `io::ErrorKind` from the spawn error, if any.

  * Otherwise, a new kind `io::ErrorKind::ProcessFailed`,
    which means that the subprocess itself failed.

### Further necessary APIs for `SubprocessError`

We also provide ways for this new error to be constructed,
which will be needed by other lower level libraries besides std,
notably async frameworks:

```
impl SubprocessError {
    /// Makes a "blank" error which doesn't contain any useful information
    ///
    /// `has_problem()` will return `false` until one of the setters
    /// is used to store an actual problem.
    fn new_empty() -> Self { }

    // If we keep ExitStatusError
    fn from_exit_status_error(status: ExitStatusError) -> Self { }

    fn set_program(&mut self, impl Into<OsString>);
    fn set_args(&mut self, impl IntoIterator<Item=impl Into<OsString>>);
    fn set_stdout_bytes(output: Option<Into<Box<[u8]>>>);

    fn set_status(&mut self, status: ExitStatus);
    fn set_stderr_bytes(&mut self, stderr: impl Into<Box<u8>>);
    fn set_spawn_error(&mut self, error: Option<io::Error>);
    fn set_communication_error(&mut self, error: Option<io::Error>);
    fn set_utf8_error(&mut self, error: Option<std::str::FromUtf8Error>);

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
    // about `SubprocessError`'s contents.
    fn has_problem(&self) -> bool;
}
impl Default for SubprocessError { ... }
impl Clone for SubprocessError { ... } // contained io:Errors are in Arcs
```

# Drawbacks
[drawbacks]: #drawbacks

This is nontrivial new API surface.

Much of the new API surface is in `SubprocessError`.
If we didn't want to try to make it easy for Rust programmers
to run subprocesses and produce good error messages,
we could omit this error type 
(perhaps using something like `ExitStatusError`).

Perhaps we don't need all the `get_output` variants,
and could require Bob to write out the boilerplate
or provide his own helper function.

Perhaps we don't need `get_output_read`.
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

# Prior art
[prior-art]: #prior-art

Many other languages have richer or more convenient APIs
for process invocation and output handling.

 * Perl's backquote operator has the command inherit the script's stderr. If you want to do something else you need to do a lot of hand-coding. It reports errors in a funky and not particularly convenient way (but, frankly, that is typical for Perl). It doesn't trim a final newline but Perl has a chomp operator that does that very conveniently.

 * Tcl's `exec` captures stderr by default (there are facilities for redirecting or inheriting it), calling any stderr an error (throwing a Tcl exception). It always calls nonzero exit status an error (and there is no easy way to get the stdout separately from the error in that situation). It unconditionally chomps a final newline.

 * Python3's `subprocess.run` fails to call nonzero exit status an exception. If you don't write explicit error handling code (highly unusual in Python) you have an unchecked exit status bug.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Printing command arguments in `impl Display for SubprocessError`

Perhaps printing the command arguments is overly verbose,
and we should print onliy the command name.

## `get_output` vs `read_output` naming.

We have `fs::read_to_string`.

Possible names (taking `get_output_bytes` as the example):

 * `get_output_bytes` (proposed in this RFC)
 * `output_bytes` but `output` is alread taken for bad `Output`.
 * `run_get_output_bytes`
 * `run_output_bytes`
 * `read_output_bytes`

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

Here we propose option 1: treat as `inherit`.

# Future possibilities
[future-possibilities]: #future-possibilities

## Changes to `ExitStatusError`

Possibilities include:

 * Abolish `ExitStatusError`
 * Stabilise `ExitStatusError` as-is
 * `impl From<ExitStatusError> for SubprocessError`
 * `impl From<ExitStatusError> for io::Error`

Error messages from `ExitStatusError` are rather poor,
and it is likely that `SubprocessError` will
subsume most of its use cases.

## Async ecosystem could mirror these APIs

 * An async versions of `run()` seems like it would be convenient.
 * Async versions of the output-capturing runners too.
 * Async frameworks ought to (be able to) use `SubprocessError`.

Maybe `SubprocessError` would have to be able to contain a nested
`Arc<dyn Error + Send + Sync + 'static>`.
That doesn't need to happen now.
But it is one reason why `.has_problem()` needs to exist.

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

## More convenient way to run `diff`

With the proposed API,
completely correctly running `diff(1)` would look a bit like this:

```
    let result = Command::new("diff")
        .args(["before","after"])
        .run();
    let status = match result {
        Ok(()) => 0,
        Err(err) => {
            let status = err.status();
            err.set_status(ExitStatusExt::from_raw(0));
            if err.has_problem() {
                return Err(err);
            }
            status.code()
        }
    };
```

This is doable but cumbersome.
A naive Dionysus is likely to write:

```
    let status = match result {
        Ok(()) => 0,
        Err(err) => {
            if ! err.status().success() {
                err.status().code()
            } else {
                return Err(err);
            }
        }
    };
```

As it happens, this is correct in the sense that it won't malfunction,
since actually `run()`, without piped stderr,
cannot produce a `SubprocessError`
containing a nonzero exit status *and* any other problem.
But in a more complex situation it might be wrong.

Perhaps:
```
impl SubprocessError {
    /// Returns `Ok<ExitStatus>` if the only reason for the failure was a nonzero exit status.  Otherwise returns `self`.
    ////
    /// Use this if you want to to tolerate some exit statuses, but still fail if there were other problems.
    pub fn just_status(self) -> Result<ExitStatus, SubprocessError>;
}
```

Then Dionysus can write:
```
    let status = match result {
        Ok(()) => 0,
        Err(err) => err.just_status()?.code(),
    };
```
