- Feature Name: expect_display
- Start Date: 2016-04-09
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Extend `expect` to take `&T` where `T: Display` instead of just `&str`.

# Motivation
[motivation]: #motivation

A lot of times, extra context for an `expect` error is desired, and today, it's
not possible to do this with the built-in `expect` method. One might want to
display further information about an object being handled or the current state
of the program in the panic message, which is not easily done with the `expect`
method.

Right now, the way to accomplish this is to either:

1. Call `unwrap_or_else(|e| panic!(...))`, which is less ergonomic.
2. Call `expect(&format!(...))`, which requires an unnecessary allocation.

# Detailed design
[design]: #detailed-design

Modify `Option::expect` to have the signature:

```
pub fn expect<D: Display>(self, msg: &D) -> T;
```

Modify `Result::expect` and `Result::expect_err` to have the signatures:

```
pub fn expect<D: Display>(self, msg: &D) -> T;
pub fn expect_err<D: Display>(self, msg: &D) -> E;
```

Because these methods simply defer to panic on failure, no substantial
implementation difference is needed.

We'd expect uses of this new function to include:

1. `expect(format_args!(...))`
2. `expect(err)` where `err` is a custom error type.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

Because all existing examples work, no additional documentation is needed beyond
the rustdoc to explain just using `expect` with a string. However, it may be
desired in the future to add a small note in the Rust book with an example that
uses `expect` with `format_args!` or a custom error type.

# Drawbacks
[drawbacks]: #drawbacks

1. It's not necessarily ergonomic by default; creating a struct which implements
   `Display` may be excessive, and using `format_args!` doesn't really feel
   idiomatic.
2. It's already possible with `unwrap_or_else(|e| panic!(...))`.
3. This is probably not a common use case.

# Alternatives
[alternatives]: #alternatives

1. Add another method that accepts a `Display` argument while leaving this
   method alone. (May be excessive.)
2. Let users continue using `unwrap_or_else(|e| panic!(...))`.

# Unresolved questions
[unresolved]: #unresolved-questions

1. Is this the most idiomatic way to do this?
2. Should there be another method which allows incorporating an existing error
   from `Result` into the error message? Is this any better than
   `unwrap_or_else(|e| panic!(...))` ?

