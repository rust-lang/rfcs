- Feature Name: abort_by_default
- Start Date: 2016-10-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Specify abort-by-default in `Cargo.toml` when the user does `cargo new --bin`, as well as various other refinements to the panick strategy system.

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

First of all, we add a new possible value to the `panic` field. We call this `any`. It specifies that the library or binary is compatible with any panicking strategy. `any` will default to the `abort` strategy, if compatible with all of its dependencies. If not, it will fall back to `unwind`, and leave a warning.

When `cargo new` is invoked, it will generate `panic=any` in the `Cargo.toml`, both for new libraries and new binaries.

Whenever the user inteacts with `cargo` (by using a command) in an existing (binary) crate and the `Cargo.toml` has no specified panicking strategy, it will add `panic=unwind` and leave a note to the user:

    Warning: No panic strategy was specified, added unwinding to the
             `Cargo.toml` file, please consider change it to your needs. If
             your crate's behavior does not depend on unwinding, please add
             `panic=any` instead.

This will not happen to libraries, since they must only rely on unwind, if the specify it. Instead, it will add `panic=any` to libraries and give warning:

    Warning: No panic strategy was specified, so we default to aborting. If
             your crate depends on unwinding, please put `panic=unwind` in
             `Cargo.toml`.

## Libraries

For libraries, the `Cargo.toml` is not modified, as they inherit the strategy of the binary.

### Relying on unwinding

If a library specifies `panic=unwind`, it will stored in a rlib metadata field, `unwind_needed`. If this field does not match with the crate which is linking against the library (`abort`), `rustc` will produce an error.

This is done in order to make sure that applications can rely on unwinding without leading to unsafety when being linked against by an aborting runtime.

## Extensions

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

Use crate attributes instead.

# Unresolved questions
[unresolved]: #unresolved-questions

Is there any way we can detect crates which do not rely on unwinding? Search for `catch_unwind`?
