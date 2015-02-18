- Feature Name: one_sided_match_ranges
- Start Date: 2015-02-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow inclusive ranges in match patterns that are bounded on only one side,
analogous to the `RangeTo` and `RangeFrom` exclusive ranges used in expressions.

Also have the compiler check whether integer patterns exhaust all possible
values, so that providing an unreachable match arm for certain integer match
expressions is no longer necessary (or in fact allowed).

# Motivation

This clears up a minor wart that adds some confusion and perceived complexity to
the language. Allowed range patterns include `i..j`, `..j`, `i..`, and simply
`..`.  However, match pattern ranges can only have the form `i...j`.

Furthermore, even if the patterns in a match statement are provably exhaustive,
the reference compiler currently requires a catch-all `_` pattern, which in
such cases is actually unreachable.

A more speculative use case relates to the future addition of dependent types
(or at least types that depend on integer constants, as `[T; N]` arrays already
do). For such types, match patterns could be useful for expressing bounds on
constant parameters in types, in a way that's limited enough to inform coherence
checks and allow for specialization of impls.

# Detailed design

Just as the match pattern `i...j` matches numbers between `i` and `j`, `i...`
will match all numbers greater than `i`, and `...j` will match all numbers less
than `j`. This capability is extended to `char` as well, under the same ordering
used for match patterns now.

Additionally, exhaustiveness checks will be performed for patterns that match
integers and `char`, as opposed to the current implementation, which requires
the last arm of such match patterns to be a blanket match.

Consider the following example:

```rust
let error_code: u8 = do_something();
match error_code {
    0 => Success,
    1 | 3 => Failure,
    2...255 => Error,
    _ => unreachable!(),
}
```

Assuming that all referenced items have been defined, the above code is legal
today.

Under this proposal, this code would need to be adjusted to the following, since
the compiler would verify that these possibilities exhaust the entire allowed
range of integers:

```rust
let error_code: u8 = do_something();
match error_code {
    0 => Success,
    1 | 3 => Failure,
    2...255 => Error,
}
```

Since `255` is the maximum representable value for `u8`, the following would be
equivalent:

```rust
let error_code: u8 = do_something();
match error_code {
    0 => Success,
    1 | 3 => Failure,
    2... => Error,
}
```

A set of patterns will be considered to exhaustively cover an integer type if
all possible values for that type are covered. A set of patterns will be
considered to exhaustively cover `char` if all valid `char` values (that is, all
Unicode Scalar Values) are covered.

Floating point values will benefit from the new syntax (e.g. `...1.0f32` will be
an allowed pattern), but will not be subject to the changes in exhaustiveness
checking.  The main rationale for this inconsistency is that floating-point
numbers have much more complex, and in some cases even platform-dependent,
semantics.

An prototype implementation of an algorithm that can perform most of the work
for the exhaustiveness check is
[here](https://github.com/quantheory/int_range_check). This is sufficient to
check whether a series of ranges cover all allowed values of an integer type.

# Drawbacks

Since more match arms can be proven unreachable, some code that currently
compiles may be broken. However, such match expressions are probably rare.

Pattern syntax will become slightly more complex.

# Alternatives

## Do nothing

The current behavior is only a mild nuisance right now, so keeping it is
definitely an option.

## Keep backwards compatibility

We could avoid breaking existing code by accepting match expressions with
unreachable arms in some cases. In the examples above, the match expression that
is allowed today would still be allowed, but the new expressions would also be
allowed.

## Take only one of the two features

The new syntax and the change to exhaustiveness checks are independent
features. They are included together mainly because the new syntax would make it
easier to write exhaustive matches without using wildcards.

# Unresolved questions

N/A
