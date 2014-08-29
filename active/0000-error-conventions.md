- Start Date: (fill me in with today's date, 2014-08-15)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This RFC sets out the first part of a vision for refining error handling in
Rust, by proposing a more clear-cut set of conventions around `fail!` and
`Result`. A separate follow-up RFC will propose syntactic sugar to make the
proposed convention more ergonomic.

In a nutshell, the proposal is to isolate uses of `fail!` to an extremely small
set of well-known methods (and assertion violations), with all other kinds of
errors going through `Result` for maximal flexibility.

# Motivation

Rust has been steadily moving away from task failure as a primary means of error
handling, and has also discouraged providing both `fail!` and `Result` variants
of methods. However, it is very difficult to craft a clear set of guidelines
that clearly says when `fail!` is appropriate, and so the libraries remain
inconsistent in their error signaling approach.

(The draft guidelines [here](http://aturon.github.io/errors/signaling.html) are
an attempt to capture today's "rules", but are not clear-cut enough to resolve
disputes about uses of `fail!`.)

The main challenge is dealing with "programmer errors" or "contract violations"
in APIs -- things like out-of-bounds errors, unexpected interior nulls, calling
`RefCell::borrow` on a mutably-borrowed value, and so on. In today's libraries,
the API designer can choose whether to treat usage errors as assertion
violations (and hence `fail!`) or as permitted (and hence return a useful `Err`
value). The problem is that "programming error" is often in the eye of the
beholder, and even in the case of things like array indexing there are useful
patterns based on returning a `Result` rather than `fail!`ing.

The goal of this RFC is to lay out a vision for error signaling that would
support a clearer set of guidelines and therefore more consistent library APIs.

# Detailed design

## Proposed error conventions

The use of `fail!` is restricted to:

* Assertion violations (`assert!`, `debug_assert!`, etc.), which should *not* be
  used for input validation. That is, explicit assertions within a function
  should be expected to succeed *regardless* of the function's inputs.

* Unwrapping an `Option` or `Result` (which will need to be renamed; see below)

* Out-of-bounds or key-not-found errors when using *sugared notation* for
  indexing `foo[n]` (or the proposed
  [slicing notation](https://github.com/rust-lang/rfcs/pull/198)). (As opposed
  to "normal" methods like `get`; see below.)

* Perhaps a small list of others, TBD.

All other errors, be they contract violations on inputs or external problems
like file-not-found should use a `Result` (or `Option`) for error signaling.

In particular, collections will offer methods like `get` that work like indexing
but return an `Option` for signaling out-of-bounds or key-not-found.

The result of these conventions is that:

1. Potential task failure is very clearly marked and easily grepped for.

2. Since fewer functions will fail the task by default, task failure is placed
   under the client's control to a greater degree. This allows clients to take
   advantage of built-in error checking provided by an API without having to
   cope with task failure.

3. API designers have extremely clear guidelines on when to `fail!`.

### Renaming `unwrap`

At the moment, `unwrap` is used for `Option`/`Result` (where it can fail) as
well as other types (where it cannot fail). These must be renamed apart if we
want failure to be clearly signaled.

The proposal is:

* `Option::expect` is today's `Option::unwrap`
* `Option::expect_msg` is today's `Option::expect`
* `Result::expect` is today's `Result::unwrap`
* `Result::expect_err` is today's `Result::unwrap_err`

The `unwrap_or` and `unwrap_or_else` methods will keep their current names,
because the `expect` prefix marks an operation that will fail the task, while
`unwrap` marks an operation that will extract internal data without failure.
(This does make the methods less congruent, but part of the point is that that
today's `unwrap` and `unwrap_or` are *crucially different* from the perspective
of task failure.)

# Drawbacks

These conventions may greatly increase the use of `Option` and `Result`, which
in turn has ergonomic consequences: clients of APIs will have to call
`.expect()` in many new places as a way of asserting that they have satisfied
the API's contract.

An [earlier version](https://github.com/rust-lang/rfcs/pull/204) of this RFC
proposed to alleviate the ergonomic problems by introducing some syntactic
sugar, but this is now being broken out as a separate proposal. This way, we can
try implementing the convention and see how painful it is in reality.

Moving to `Option`/`Result` also complicates function signatures, but it's not
clear how much of a drawback that is: essentially, it means that the signatures
more clearly represent the possibility of an error or contract violation.

# Alternatives

## Naming for `unwrap`

Some other proposals include:

* Rename the `Option`/`Result` versions to `assert_some` and
  `assert_ok`/`assert_err`.

* Rename other (non-`Option`/`Result`) uses of `unwrap` to `inner` or `into_inner`.

If we adopt separate syntactic sugar below, we could cope with a much longer
name, such as `unwrap_or_fail`.

The advantage of having `assert` in the name is a clearer signal about possible
`fail!` invocation, but
[many feel](https://github.com/rust-lang/rust/pull/16436) that newcomers are
likely to be surprised that `assert` returns a value. The `expect` name seems
like a good compromise.
