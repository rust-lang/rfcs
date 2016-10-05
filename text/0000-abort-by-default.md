- Feature Name: abort_by_default
- Start Date: 2016-10-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Specify `panic=abort` in `Cargo.toml` when the user does `cargo new --bin`.

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

Have `cargo` generate `panic=abort` to the `Cargo.toml`. Whenever the user inteacts with `cargo` (by using a command) and the `Cargo.toml` has no specified panicking strategy, it will add `panic=unwind` and leave a note to the user: 

    Warning: No panic strategy was specified, added unwinding to the `Cargo.toml` file, please consider change it to your needs.

For libraries, nothing is modified, as they inherit the strategy of the binary.

After several release cycles, an extension could be added, which makes specifying the strategy mandatory.

# Drawbacks
[drawbacks]: #drawbacks

It makes panics global by default (i.e., panicking in some thread will abort the whole program).

It might make it tempting to ignore the existence of an unwind option and consequently shaping the code after panicking being aborting.

## Unwinding is not bad per se

Unwinding has numerous advantages. Especially for certain classes of applications. These includes better error handling and better cleanup.

Unwinding is especially important to long-running applications.

# Alternatives
[alternatives]: #alternatives

Keep unwinding as default.

Make Cargo set `panic=abort` in all new binaries.

Use unwind in `debug` and abort in `release`.

Make use of more exotic instructions like `bound`, allowing e.g. overflow or bound checking without branches. This comes at the expense of error output.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
