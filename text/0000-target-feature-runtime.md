- Feature Name: `target_feature_runtime`
- Start Date: 2019-07-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC allows `#![no_std]` binaries and libraries (e.g. like `libcore` and
`liballoc`) to perform run-time feature detection.

# Motivation
[motivation]: #motivation

Binaries and libraries using the `std` library can perform run-time feature
detection via the `is_x86_feature_detected!("avx")` architecture-specific
macros. 

This operation requires, in general, operating system support, and is therefore
not available in `libcore`, which is operating system agnostic.

That is, `#![no_std]` libraries, like `liballoc` and `libcore`, cannot perform
run-time feature detection, even though `libstd` often ends up being linked into
the final binary. 

This results in some crates in crates.io having much better performance than the
methods of the types provided by `libcore`, like `&str`, `[T]`, `Iterator`, etc.

One example is the `is_sorted` crate, which provides an implementation of
`Iterator::is_sorted`, which performs 16x better than the `libcore`
implementation by using AVX. Another example include the `memchr` crate, as well
as crates implementing algorithms to compute whether a `[u8]` is an ASCII
string, or an UTF-8 string. These perform on the ballpark of about 1.6x better
than the `libcore` implementations, by using AVX on x86.

For `#![no_std]` binaries, the standard library is not linked into the final
binary, and they cannot use any library that uses the runtime feature detection
macros, because they are not available.

The goal of this RFC is to enable `#![no_std]` libraries and binaries to perform
run-time feature detection.

# Constraints on the design

It helps to first enumerate the "self-imposed" constraints on the design:

* **zero-cost abstraction**: it shouldn't be possible to do runtime feature
  detection better than via the APIs provided here.
* **don't pay for what you don't use**: programs that don't need to do any
  runtime feature detection should not pay anything for it, in terms of costs in
  binary size, memory usage, time spent on binary initialization, etc.
* **99.9%** The majority of Rust users should be able to benefit from this,
  e.g., via `libcore` using it, without having to know that this exists.
* **reliable**: it should be possible to reliably do run-time feature detection,
  since it is required to prove that some `unsafe` code is actually safe.
* **portable libraries**: libraries that use run-time feature detection should
  be able to do so, without restricting which users can use the library.
* **cross-domain**: operating system kernels and user-space applications often
  need to do run-time feature detection in very different ways - all use cases
  should be supported.
* **cdylibs**: dynamic libraries should be able to do run-time feature detection.
* **embedded systems**: binaries running on read-only memory (e.g. in a ROM)
  should be able to do runtime feature detection.

These constraints motivate the design. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Users can continue to perform run-time feature detection by using the
`is_{architecture}_feature_detected!` macros. These macros were previously only
available from libstd, and are now available in `libcore`. That is, `#![no_std]`
libraries and binaries can use them.

Users can now provide their own target-feature detection run-time:

```rust
#[target_feature_detection_runtime]
static TargetFeatureRT: impl core::detect::Runtime;
```

by using the `#[target_feature_detection_runtime]` attribute on a `static`
variable of a type that implements the `core::detect::Runtime` `trait` (see
[definition below][runtime-trait]).

This is analogous to how the `#[global_allocator]` is currently defined in Rust
programs.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC introduces:

* a new attribute: `#[target_feature_detection_runtime]`,
* a new trait: `core::detect::Runtime`, and
* a new function: `core::detect::is_target_feature_detected`.

## The `#[target_feature_detection_runtime]` attribute

The `#[target_feature_runtime]` can be used to _define_ a target-feature
detection run-time by applying it to a `static` variable as follows:

```rust
#[target_feature_detection_runtime]
static TargetFeatureRT: impl core::detect::Runtime;
```

Only one such definition is allowed per binary artifact (binary, cdylib, etc.),
similarly to how only one `#[global_allocator]` or `#[panic_handler]` is
allowed in the dependency graph.

The `static` variable must implement the `core::detect::Runtime` `trait`.

If no `#[target_feature_detection_runtime]` is provided anywhere in the
dependency graph, Rust provides a default definition. For `#![no_std]` binaries
and dynamic libraries, that is, for binaries and libraries that do not link
against `libstd`, this definition always returns `false` (it does nothing).

## The `core::detect::Runtime` trait
[runtime-trait]: #runtime-trait

The runtime must be a `static` variable of a type that implements the
`core::detect::Runtime` trait:

```rust
unsafe trait core::detect::Runtime {
    /// Returns `true` if the `feature` is known to be supported by the 
    /// current thread of execution and `false` otherwise.
    #[rustc_const_function_arg(0)]
    fn is_target_feature_detected(feature: &'static str) -> bool;
}
```

This `trait`, which is part of `libcore`, is `unsafe` to implement. A correct
implementation, satisfying the specified semantics of its methods is required
for soundness of safe Rust code. That is, an incorrect implementation can cause
safe Rust code to have undefined behavior.

Forcing the `&'static str` to be a constant expression allows the
feature-detection macros to reliably produce compilation-errors on unknown
features, as well as on features that have not been stabilized yet. This type of
validation happens at compile-time, before the user-defined run-time is called.

## The `core::detect::is_target_feature_detected` function

Finally, the following function is added to `libcore`:

```rust
/// Returns `true` if the `feature` is known to be supported by the 
/// current thread of execution and `false` otherwise.
#[rustc_const_function_arg(0)]
fn is_target_feature_detected(feature: &'static str) -> bool;
```

This function calls the `Runtime::is_target_feature_detected` method. Its
argument must be a constant-expression.

---

Finally, this RFC moves the feature-detection macros of `libstd` to `libcore`.
Right now, the only stable feature-detection macro is
`is_x86_feature_detected!("target_feature_name")`.

The semantics of these macros are modified to:

```rust
/// Returns `true` if `cfg!(target_feature = feature)` is `true`, and
/// returns the value of `core::detect::is_feature_detected(feature)` 
/// otherwise.
///
/// If `feature` is not known to be a valid feature for the current 
/// `architecture`, the program is ill-formed, and a compile-time 
/// diagnostic is emitted.
is_{architecture}_feature_detected!(feature: &'static str) -> bool;
```

# Drawbacks
[drawbacks]: #drawbacks

This increases the complexity of the implementation, adding another singleton
run-time component. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

This approach satisfies all self-imposed constraints:

* **zero-cost abstraction**: the APIs provided just call the run-time. If the
  user can do better than, e.g., the run-time provided by Rust, they can just
  override it with their own.
  
* **don't pay for what you don't use**: programs that never do run-time feature
  detection, never call any of the APIs. LTO should be able to optimize the
  run-time away. If it isn't, users can provide their own "empty" run-time.

* **99.9%** This enables `libcore`, `liballoc`, and `#![no_std]` libraries in
  general to do run-time feature detection. The majority of Rust users, benefits
  from that silently even though they might never use this feature themselves.
  
* **reliable**: the default `#![no_std]` run-time provided by Rust always
  returns `false`, that is, that a feature is not enabled, such that the
  run-time feature detection macros will return `true` only for the features
  enabled at compile-time; this is always correct. The `Runtime` trait is also
  `unsafe` to implement.
  
* **portable libraries**: libraries that use run-time feature detection are not
  restricted to `#![std]` binaries anymore - they can be used by `#![no_std]`
  libraries and binaries as well.
  
* **cross-domain**: the run-time provided by Rust by default requires operating
  system support, that is, for custom targets, no run-time will be provided.
  Users of these targets can use any run-time that satisfies their constraints.
  
* **cdylibs**: dynamic libraries get the same default run-time as Rust binaries,
  i.e., the `libstd` one if `libstd` is linked, and one that returns `false` if
  the `cdylib` is `#![no_std]`, in which case the `cdylib` can provide their
  own.

* **embedded systems**: binaries running on read-only memory (e.g. in a ROM) can
  implement a run-time that, e.g., does not cache any results, which would
  require read-write memory, and instead, recomputes results on all invocations,
  always returns false, contains features for different CPUs pre-computed in
  read-only memory, and only detects the CPU type, etc. Even when implementing a
  feature cache, one often needs to choose between using atomics, thread-locals,
  mutexes, or no synchronization if the application is single-threaded. Not all
  embedded systems support all these features.

## Alternatives

We could not solve this problem. In which case, `libcore` can't use run-time
feature detection, e.g., to use advanced SIMD instructions.

We also could do something different. For example, we could provide a "cache" in
libcore, and an API for users or only for the standard library, to initialize
this cache externally, e.g., during the standard library initialization routine.

This runs into problems with `cdylibs`, where these routines might not be
called. It also runs into problems with often imposing a cost on users, e.g.,
due to a cache in libcore, even though users might never use it. This would be
limiting, if e.g. having a cache in read-write memory prevents libcore from
being compiled to a read-only binary. We would need to feature gate this
functionality to avoid these issues. 

It isn't cross-domain either, e.g., an OS kernel would need to disable this
functionality, and wouldn't be able to provide their own. So while they could
use libraries that would do run-time feature detection, no meaningful detection
would be performed.

# Prior art
[prior-art]: #prior-art

This feature is very similar to `#[global_allocator]` and `#[panic_handler]`.
Since a default implementation is provided if the user does not provide one,
this is a backward compatible change.

This feature does not exist in any programming languages I know. Clang and GCC
do have a feature-detection run-time, which is not configurable, nor does it
work for all users.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

None. After this RFC, the run-time feature detection part of the Rust language
should be complete.
