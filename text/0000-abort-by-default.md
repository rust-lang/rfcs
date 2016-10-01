- Feature Name: abort_by_default
- Start Date: 2016-10-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Use abort as the standard panic method rather than unwind.

# Motivation
[motivation]: #motivation

## Performance

Generally, the performance of bigger programs [improves by 10%](https://www.youtube.com/watch?v=mRGb4hoGuPs), which is no small amount. When overflow checking is enabled, it is as high as 2x, making overflowing checking plausible in production.

The performance gains mainly originates in the fact that it is a lot easier for the compiler to optimize, due to the unwind path disappearing, and hence reasoning about the code becomes easier.

## Binary size

Binary size improves by 10% as well. This is due to the lack of landing pads which are injected in the stack when unwinding is enabled.

## Compile time

Compile time in debug mode improves by 20% on average. In release mode, the number is around 10%.

## Runtime size

Unwinding requires a fair amount of runtime code, which can be seen as conflicting with the goals of Rust.

## Correctness

You often see people abusing `std::panic::catch_unwind` for exception handling. Forcing the user to explicitly opt in to unwinding will make it harder to unintentionally misuse it.

# Detailed design
[design]: #detailed-design

Default to `panic=abort`.

## Backwards compatibility

No code should rely on a particular default panicking strategy, and code which do is already broken. However, this broken code might start actually encountering the issues. In a sense, that is actually positive, since it reveals that the code is broken.

No invariants are broken, and no code is going to emit Undefined Behavior due to this change (this can be seen by observing that aborting is globally diverging, and hence no code is run after).

# Drawbacks
[drawbacks]: #drawbacks

While not breaking, the behavior of certain broken programs can change.

It makes panics global by default (i.e., panicking in some thread will abort the whole program).

It might make it tempting to ignore the existence of an unwind option and consequently shaping the code after panicking being aborting.

# Alternatives
[alternatives]: #alternatives

Keep unwinding as default.

Make Cargo set `panic=abort` in all new binaries.

Use unwind in `debug` and abort in `release`.

Make use of more exotic instructions like `bound`, allowing e.g. overflow or bound checking without branches. This comes at the expense of error output.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
