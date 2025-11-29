- Feature Name: `explicit_extern_abis`
- Start Date: 2024-10-30
- RFC PR: [rust-lang/rfcs#3722](https://github.com/rust-lang/rfcs/pull/3722)
- Tracking Issue: [rust-lang/rust#134986](https://github.com/rust-lang/rust/issues/134986)

## Summary

Disallow `extern` without an explicit ABI in a new edition. Write `extern "C"` (or another ABI) instead of just `extern`.

```diff
- extern { … }
+ extern "C" { … }

- extern fn foo() { … }
+ extern "C" fn foo() { … }
```

## Motivation

Originally, `"C"` was a very reasable default for `extern`.
However, with work ongoing to add other ABIs to Rust, it is no longer obvious that `"C"` should forever stay the default.

By making the ABI explicit, it becomes much clearer that `"C"` is just one of the possible choices, rather than the "standard" way for external functions.
Removing the default makes it easier to add a new ABI on equal footing as `"C"`.

Right now, "extern", "FFI" and "C" are somewhat used interchangeably in Rust. For example, this is the diagnostic when using a `String` in an `extern` function:

```
warning: `extern` fn uses type `String`, which is not FFI-safe
 --> src/main.rs:1:16
  |
1 | extern fn a(s: String) {}
  |                ^^^^^^ not FFI-safe
  |
  = help: consider adding a `#[repr(C)]` or `#[repr(transparent)]` attribute to this struct
  = note: this struct has unspecified layout
  = note: `#[warn(improper_ctypes_definitions)]` on by default
```

If another future ABI will support `String`, this error should make it clearer that the problem is not that `String` doesn't support FFI, but rather that the `"C"` ABI doesn't support `String`.
This would be easier if there was actually a `"C"` token to point at in the source code. E.g.:

```
warning: `extern` fn uses type `String`, which is not supported by the "C" ABI
 --> src/main.rs:1:16
  |
1 | extern "C" fn a(s: String) {}
  |        ---         ^^^^^^ String type not supported by this ABI
  |         |
  |         the "C" ABI does not support this type
```

It would also make it clearer that swapping `"C"` for another ABI might be an option.

## Guilde-level explanation

Up to the previous edition, `extern` without an explicit ABI was equivalent to `extern "C"`.
In the new edition, writing `extern` without an ABI is an error.
Instead, you must write `extern "C"` explicitly.

## Automatic migration

Automatic migration (for `cargo fix --edition`) is trivial: Insert `"C"` after `extern` if there is no ABI.

## Drawbacks

- This is a breaking change and needs to be done in a new edition.

## Prior art

This was proposed before Rust 1.0 in 2015 in [RFC 697](https://github.com/rust-lang/rfcs/pull/697).
It was not accepted at the time, because "C" seemed like the only resonable default.
It was later closed because it'd be a backwards incompatible change, and editions were not yet invented.

## Unresolved questions

- ~~In which edition do we make this change?~~
  - It's too late for the 2024 edition: https://github.com/rust-lang/rfcs/pull/3722#issuecomment-2447333966
- ~~Do we warn about `extern` without an explicit ABI in previous editions?~~
   - Yes, with separate FCP: https://github.com/rust-lang/rfcs/pull/3722#issuecomment-2447719047

## Future possibilities

In the future, we might want to add a new default ABI.
For example, if `extern "stable-rust-abi"` becomes a thing and e.g. dynamically linking Rust from Rust becomes very popular, it might make sense to make that the default when writing `extern fn` without an ABI.
That is, however, a separate discussion; it might also be reasonable to never have a default ABI again.
