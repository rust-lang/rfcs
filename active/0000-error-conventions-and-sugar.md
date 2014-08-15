- Start Date: (fill me in with today's date, 2014-08-15)
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

This RFC sets out a vision for refining error handling in Rust, in two
parts:

* A stronger set of conventions around `fail!` and `Result`
* Sugar to make working with `Result` highly ergonomic

In a nutshell, the proposal is to isolate uses of `fail!` to an extremely small
set of well-known methods (and assertion violations), with all other kinds of
errors going through `Result` for maximal flexibility. Since the main downside
of taking this extreme position is ergonomics, the RFC also proposes notation
for consuming `Result`s:

* Change macro invocation syntax from `macro_name!` to `@macro_name`.
* Use `foo!` for today's `foo.unwrap()`
* Use `foo?` for today's `try!(foo)`

While the two parts of this proposal reinforce each other, it's possible to
consider each of them separately.

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

The goal of this RFC is to lay out a vision for an extreme but ergonomic
position on error signaling that would support a clearer set of guidelines and
therefore more consistent library APIs.

# Detailed design

The proposal has two pieces. First, a set of clear-cut conventions on when to
use `fail!`. Second, since `fail!` is often used for ergonomic reasons, a
proposal for making `Result` easier to work with.

## Error conventions

The use of `fail!` is restricted to:

* Assertion violations (`assert!`, `debug_assert!`, etc.), which should *not* be
  used for input validation.

* Unwrapping an `Option` or `Result` (which will need to be renamed; see below)

* Out-of-bounds or key-not-found errors when using *sugared notation* for
  indexing `foo[n]` (or the proposed
  [slicing notation](https://github.com/rust-lang/rfcs/pull/198)). (As opposed
  to "normal" methods like `get`; see below.)

* Perhaps a few others, TBD.

All other errors, be they contract violations on inputs or external problems
like file-not-found should use a `Result` (or in limited cases, `Option`) for
error signaling.

In particular, collections will offer methods like `get` that work like indexing
but return an `Option` for signaling out-of-bounds or key-not-found.

The result of these conventions is that:

1. Failure is very clearly marked (easily grepped for) and under the client's
   control. This allows clients to take advantage of built-in error checking
   provided by an API without having to cope with task failure.
2. API designers have extremely clear guidelines on when to `fail!`.

### Tangent: renaming `unwrap`

At the moment, `unwrap` is used for `Option`/`Result` (where it can fail) as
well as other types (where it cannot fail). These must be renamed apart if we
want failure to be clearly signaled.  Some proposals include:

* Rename the `Option`/`Result` versions to `assert_some` and
  `assert_ok`/`assert_err`.

* Rename the `Option`/`Result` versions to `expect`/`expect_err`, and rename
  `Option::expect` to `expect_msg`.

* Rename other (non-`Option`/`Result`) uses of `unwrap` to `inner` or `into_inner`.

If we adopt the shorthand syntax suggested below, we could cope with a much
longer name, such as `unwrap_or_fail`.

The advantage of having `assert` in the name is a clearer signal about possible
`fail!` invocation, but
[many feel](https://github.com/rust-lang/rust/pull/16436) that
newcomers are likely to be surprised that `assert` returns a value.

The specific proposal here will need to be pinned down before the RFC is
finalized or accepted, but I want to open the floor to discussion first.

## Ergonomics for error handling

Many operations that currently use `fail!` on bad inputs do so for ergonomic
reasons -- perhaps bad inputs are rare, and the API author wants to avoid a lot
of `.unwrap` noise.

To help relieve this ergonomic pressure, we propose three syntax changes:

1. Macro invocation is written with a leading `@` (as in `@println`) rather than
   trailing `!` (as in `println!`). This frees up `!`.

2. The "unwrap" method (whatever it ends up being called) can be invoked via a
   postfix `!` operator:

    ```rust
    // under above conventions, borrow would yield an Option
    String::from_utf8(my_ref_cell.borrow()!)!

    // Equivalent to:
    String::from_utf8(my_ref_cell.borrow().unwrap()).unwrap()
    ```

3. The `try!` macro can be invoked via a postfix `?` operator:

    ```rust
    use std::io::{File, Open, Write, IoError};

    struct Info {
        name: String,
        age: int,
        rating: int
    }

    fn write_info(info: &Info) -> Result<(), IoError> {
        let mut file = File::open_mode(&Path::new("my_best_friends.txt"), Open, Write)?;
        file.write_line(format!("name: {}", info.name).as_slice())?;
        file.write_line(format!("age: {}", info.age).as_slice())?;
        file.write_line(format!("rating: {}", info.rating).as_slice())?;
        Ok(());
    }
    ```

The `!` and `?` operators would bind more tightly than all existing binary
or unary operators:

```rust
// The following are equivalent:
foo + !bar!
foo + (!(bar!))
```

It is common for unary operators to bind more tightly than binary operators, and
usually unwrapping/propagating a `Result` is the innermost step in some compound
computation.

# Drawbacks

An obvious drawback is that `println!` looks lighter weight than `@println` (or,
in any case, we're all quite used to it). On the other hand, the `!` and `?`
pairing for error handling seems very appealing, and has
[some precedent](https://developer.apple.com/library/prerelease/mac/documentation/Swift/Conceptual/Swift_Programming_Language/OptionalChaining.html)
(note however that Swift's `?` notation works different from that being proposed
here; see Alternatives).

Some people feel that unwrapping should be _un_ergonomic, to prevent
abuse. There are a few counterpoints:

* The current ergonomics have led to APIs failing *internally* so that their
  clients don't have to `unwrap` -- leading to *more* task failure, not less.

* If the above conventions are adopted, `Result`/`Option` will be used in many
  cases to signal the possibility of contract violation. Unwrapping is then just
  an assertion that the contract has, in fact, been met. With the overall
  proposal, programmers will *clearly know* when and where a contract is being
  checked, but the assertion is lightweight.

* By placing `try!` and `unwrap` on equal footing via a simple and clear marker,
  programmers are both aware of the potential for errors and easily able to
  choose between two extreme ways of handling them: failing immediately, or
  passing them on.


# Alternatives

## Piecing apart the proposal

At a coarse grain, we could:

* Stick with the status quo, where using `fail!` on contract violation is an API
  designer's choice.

* Just change the conventions as proposed, without adding sugar. Many have
  expressed concern about the ergonomics of such an approach.

* Just add the `!` and `?` sugar, without setting firmer conventions about
  `fail!`. This would be an improvement over the status quo, but also a missed
  opportunity. Without extremely clear guidelines about error signaling and
  handling, we risk
  [fragmentation](http://www.randomhacks.net/2007/03/10/haskell-8-ways-to-report-errors/).

## Syntax alternatives

### Tying `!` and `?` to identifiers

An alternative to making `!` and `?` work as postfix operators would be to treat
them as modifiers on identifiers. For example, rather than writing this:

```rust
foo!                 // unwrap a variable
self.foo!            // unwrap field foo of type Option<T>
self.foo(x, y, z)!   // invoke a Result-returning method and unwrap
foo(x)!.bar(y)!.baz! // method and field-access chaining
```

you could instead imagine writing this:

```rust
foo!                 // unwrap a variable
self.foo!            // unwrap field foo of type Option<T>
self.foo!(x, y, z)   // invoke a Result-returning method and unwrap
foo!(x).bar!(y).baz! // method and field-access chaining
```

Arguably, `foo!(x).bar!(y)` reads better than `foo(x)!.bar(y)!`, and the extra
flexibility of a general postfix operator is problably not needed. On the other
hand, postfix operators are simpler and more familiar syntactic forms.

### `?` as `map`

In the above proposal, the `?` operator is shorthand for a use of the `try`
macro. An alternative, used in the
[Swift language](https://developer.apple.com/library/prerelease/mac/documentation/Swift/Conceptual/Swift_Programming_Language/OptionalChaining.html)
among others, is to treat `?` as shorthand for `map` (called "option chaining"):

```rust
foo(x, y)?.bar(z)?.baz                  // sugared version

try!(try!(foo(x, y)).bar(z)).baz        // this RFC's interpretation

foo(x, y).map(|t1|                      // Option-chaining alternative
    t1.bar(z).map(|t2|
        t2.bar))
```

Both interpretations of `?` work similarly to monadic `do` notation:

* You write a chain of computations as if every operation succeeds.

* An error at any point aborts the "rest of the computation" and returns the
  `Err`.

The difference between the `try!` and `map` interpretation is just what counts
as "the rest of the computation". With `try!`, it is the entire function, while
with `map` it is the rest of the expression.

(Note that Rust's `io` module is built with implicit `map`-like semantics:
errors are silently propagated in expressions like
`File::open(some_file).read_to_end()`, so that errors on opening *or* reading
both just return `Err` for the whole expression.)

Anecdotally, `try!` seems to be the most important and common means of
propagating errors, but it might be worthwhile to measure the usage compared to
`map`.
