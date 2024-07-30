- Feature Name: `core_detect`
- Start Date: 2023-08-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC moves the `is_*_feature_detected` macros into `core`, but keeps the logic for actually doing feature detection (which requires OS support) in `std`.

# Motivation
[motivation]: #motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

This has 2 main benefits:
- It allows `core` and `alloc` to use CPU-specific features, e.g. for string processing which can make use of newer CPU instructions specifically designed for this.
- It allows `#![no_std]` libraries to use CPU feature detection.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `std::arch::is_*_feature_detected` macros allow code to detect whether a particular CPU feature is enabled at runtime. This RFC does not change anything for users of these macros, except to make them available in `core` for use by no-std code.

While no-std library crates can just go ahead and use these macros, there are implications for no-std *programs*. The actual work of querying the OS for available CPU features is done by `std` at startup, which means that no-std programs will need to do this work themselves.

A new set of macros is introduced, named `core::arch::mark_*_feature_as_detected`, which accept the same feature names as the equivalent `is_*_feature_detected` macros. These macros are unsafe to call: the caller must ensure that the relevant CPU feature is actually available.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Static initialization

Runtime feature detection is performed in `std` using a static initializer, which allows it to work both when built as a program and as a static/dynamic library. This would perform the same work that is currently done by `std_detect` when the target features are first queried.

## Thread-safety

Modifying the available target features must be thread-safe. This is achieved by treating the target features as an array of `AtomicUsize` which are filled in with `AtomicUsize::fetch_or`.

For platforms without support for compare-exchange, this is instead represented as an array of `AtomicBool`.

# Drawbacks
[drawbacks]: #drawbacks

## Possible breaking change

The effect of `mark_*_feature_as_detected` is one-way: it is only possible to enable a feature, not disable them. However this does introduce a minor breaking change: the value returned by `is_*_feature_detected` could change from `false` to `true` at any point. This could potentially cause issues if some code in another thread is depending on `is_*_feature_detected` to select a code path, and switching code paths halfway through would produce an incorrect result.

There are 2 ways in which we can address this:
- Declare that it's users' responsibility to ensure their code is robust against CPU features changing at runtime.
- Require that `mark_*_feature_as_detected` is only called when no other threads are concurrently running. However this may be difficult/impossible when Rust code is compiled as a dynamic library: it could be loaded while other threads are already running.

## Use of static initializers

The use of static initializers is somewhat controversial, for example see https://github.com/rust-lang/rust/issues/111921.

## Size of `core`

This increases the size of the `.bss` section of `libcore` by a few bytes, which can have an impact on very minimial binaries.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Using a lang item to call back into `std`

Instead of having `std` "push" the CPU features to `core` at initialization time, an alternative design would be for `core` to "pull" this information from `std` by calling a lang item defined in `std`. The problem with this approach is that it doesn't provide a clear path for how this would be exposed to no-std programs which want to do their own feature detection.

## Using CPUID directly in `core`

On some platforms, such as x86, feature detection can be done entirely in userspace without OS support. Complete feature detection could in theory be done entirely in `core`. However no-std programs, even in userspace, tend to have very specific requirements (e.g. must not use SIMD registers). It is preferable to let users customize which CPU features they want the standard library to use in such cases.

# Prior art
[prior-art]: #prior-art

Today, no-std program can still perform feature detection by using the [`std_detect`](https://crates.io/crates/std_detect) crate. This comes from the same source code as the standard library's feature detection. However it cannot be used by no-std libraries since that would introduce a hard dependency on the host OS, which may be undesirable.

There is no general precedent for this in other languages since the clean split between std and no-std is somewhat unique to Rust.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Calling `set_*_feature_detected` ~100 times at startup may have some overhead, could it be optimized in any way?

# Future possibilities
[future-possibilities]: #future-possibilities

AArch64 targets have a feature called "outlined atomics" where atomic operations are compiled to a function call to a function in `compiler_builtins`. This function will either use ARMv8.0 LDX/STX instructions or ARMv8.1 atomic instructions (which are more efficient).

Currently, the implementation in `compiler_builtins` [always uses the ARMv8.0 path](https://github.com/rust-lang/compiler-builtins/pull/532) because feature detection would [require linking to libc](https://github.com/rust-lang/rust/issues/109064) which is not acceptable for `core`. 

This could be cleanly solved by using the feature detection from `core`, which is always present and can be used to determine whether ARMv8.1 instructions are available.
