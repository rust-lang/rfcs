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

However, `#![no_std]` libraries and binaries are used in a wider-range of
applications than `#![std]` libraries ones, and they might often want to perform
run-time feature detection differently. Among others:

* **user-space applications**: performing run-time feature detection often
  requires executing privileged CPU instructions that are illegal to execute
  from user-space code. User-space applications query the available
  target-feature set from the operating system. Often, they might also want to
  cache the result to avoid repeating system calls.
  
* **privileged applications**: operating-system kernels, embedded applications,
  etc. are often able to execute privileged CPU instructions, and they have no
  "OS" they can query available features from. They are also often subjected to
  additional constraints. For example, they might not want to use certain
  features, like floating point or SIMD registers, to avoid saving them on
  context switches, or a feature cache that's modified at run-time, to allow
  them to run on read-only memory, e.g., on ROM. They are also limited on how to
  implement a feature cache, depending on the availability of atomic
  instructions, mutexes, thread local, and many of these applications are
  actually single-threaded, so they should be able to implement a cache without
  any synchronization at all.

* **cdylibs**: dynamically-linked Rust libraries with a C ABI cannot often
  perform any sort of initialization at link-time. That is, they should be able
  to initialize their target-feature cache, if they have one, on first use.

Libraries use run-time feature detection to prove that some `unsafe` code is
safe. So it is crucial that users can easily implement feature-detection
run-times that are correct. 

On top of these constraints, we impose the classical constraints on new Rust
features. This must be a zero-cost abstraction, that all Rust code can just use,
without any "but"s. Also, applications that do not perform any run-time feature
detection should not pay any price for it. This includes no run-time or
initialization overhead, no extra memory usage, and no code-size or binary size.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Users can continue to perform run-time feature detection by using the
`is_{architecture}_feature_detected!` macros. These macros were previously only
available from libstd, and are now available in `libcore`. That is, `#![no_std]`
libraries and binaries can use them.

Users can now provide their own target-feature detection run-time:

```rust
#[target_feature_detection_runtime]
static TargetFeatureRT: impl core::detect::TargetFeatureRuntime;
```

by using the `#[target_feature_detection_runtime]` attribute on a `static`
variable of a type that implements the `core::detect::TargetFeatureRuntime` `trait` (see
[definition below][runtime-trait]).

This is analogous to how the `#[global_allocator]` is currently defined in Rust
programs.

For example, an embedded application running on `aarch64`, can implement a
run-time as follows to detect some target-feature without caching them:

```rust
struct Runtime;
unsafe impl core::detect::TargetFeatureRuntime for Runtime {
    fn is_feature_detected(feature: core::detect::TargetFeature) -> bool {
      // note: `TargetFeature` is a `#[non_exhaustive]` enum.
      use core::detect::TargetFeature;

      // note: `mrs` is a privileged instruction:
      match feature {
          Aes => {
              let aa64isar0: u64; // Instruction Set Attribute Register 0
              unsafe { asm!("mrs $0, ID_AA64ISAR0_EL1" : "=r"(aa64isar0)); }
              bits_shift(aa64isar0, 7, 4) >= 1
          },
          Asimd => {
              let aa64pfr0: u64; // Processor Feature Register 0
              unsafe { asm!("mrs $0, ID_AA64PFR0_EL1" : "=r"(aa64pfr0)); }
              bits_shift(aa64pfr0, 23, 20) < 0xF
          },
          // features that we don't detect are reported as "disabled":
          _ => false,
      }
   }
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC introduces:

* a new attribute: `#[target_feature_detection_runtime]`,
* a new trait: `core::detect::TargetFeatureRuntime`,
* a new enum: `core::detect::TargetFeature`, and
* a new function: `core::detect::is_target_feature_detected`.

## The `#[target_feature_detection_runtime]` attribute

The `#[target_feature_runtime]` can be used to _define_ a target-feature
detection run-time by applying it to a `static` variable as follows:

```rust
#[target_feature_detection_runtime]
static TargetFeatureRT: impl core::detect::TargetFeatureRuntime;
```

Only one such definition is allowed per binary artifact (binary, cdylib, etc.),
similarly to how only one `#[global_allocator]` or `#[panic_handler]` is
allowed in the dependency graph.

The `static` variable must implement the `core::detect::TargetFeatureRuntime`
`trait`.

If no `#[target_feature_detection_runtime]` is provided anywhere in the
dependency graph, Rust provides a default definition. For `#![no_std]` binaries
and dynamic libraries, that is, for binaries and libraries that do not link
against `libstd`, this definition always returns `false` (it does nothing).

## The `core::detect::TargetFeatureRuntime` trait
[runtime-trait]: #runtime-trait

The run-time must be a `static` variable of a type that implements the
`core::detect::TargetFeatureRuntime` trait:

```rust
unsafe trait core::detect::TargetFeatureRuntime {
    /// Returns `true` if the `feature` is known to be supported by the 
    /// current thread of execution and `false` otherwise.
    fn is_target_feature_detected(feature: core::detect::TargetFeature) 
        -> bool;
}
```

This `trait`, which is part of `libcore`, is `unsafe` to implement. A correct
implementation, satisfying the specified semantics of its methods is required
for soundness of safe Rust code. That is, an incorrect implementation can cause
safe Rust code to have undefined behavior.

## The `core::detect::TargetFeature` enum

A `#[non_exhaustive]` `enum` is added to the `core::detect` module:

```rust
#[non_exhaustive] enum TargetFeature { ... }
```

> Unresolved question: should this `enum` be in `core::arch::{arch}` ?

The variants of this `enum` are architecture-specific, and adding new variants
to the `enum` is a forward-compatible change. 

Each enum variant is named as a target-feature of the target, where the
target-feature strings accepted by the run-time feature detection macros are
mapped to variants by capitalizing their first letter. 

For example, `is_x86_feature_detected!("avx")` corresponds to
`TargetFeature::Avx`. Variants corresponding to unstable target-features are
gated behind their feature flag. For example, using `TargetFeature::Avx512f`
requires enabling `feature(avx512_target_feature)`.

## The `core::detect::is_target_feature_detected` function

Finally, the following function is added to `libcore`:

```rust
/// Returns `true` if the `feature` is known to be supported by the 
/// current thread of execution and `false` otherwise.
fn is_target_feature_detected(feature: core::detect::TargetFeature) -> bool;
```

This function calls the `TargetFeatureRuntime::is_target_feature_detected`
method.

---

Finally, this RFC moves the feature-detection macros of `libstd` to `libcore`.
Right now, the only stable feature-detection macro is
`is_x86_feature_detected!("target_feature_name")`.

The semantics of these macros are modified to:

```rust
/// Returns `true` if `cfg!(target_feature = string-literal)` is `true`, and
/// returns the value of `core::detect::is_feature_detected` for the feature
/// otherwise.
///
/// If `feature` is not known to be a valid feature for the current 
/// `architecture`, or the required `feature()` gates to use the feature are
/// not enabled, the program is ill-formed, and a compile-time diagnostic is 
/// emitted.
is_{architecture}_feature_detected!(string-literal) -> bool;
```

> Implementation note: currently, the compilation-errors are emitted by the
> macro by pattern-matching on the literals. The mapping from the literals to
> the variants of the `TargetFeature` enum happens also at compile-time by
> pattern matching the literals.

# Drawbacks
[drawbacks]: #drawbacks

This increases the complexity of the implementation, adding another singleton
run-time component. 

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

This approach satisfies all considered use-cases:

* `libcore`, `liballoc` and other `#![no_std]` libraries and applications can
  just use the run-time feature detection macros to use extra CPU features, when
  available. This will happen automatically if a meaningful run-time is linked,
  but will not introduce unsoundness if no run-time is available, since all
  features are then reported as disabled.

* **user-space applications**: can implement run-times that query the operating
  system for features, or use CPU instructions for those architectures in which
  they are not privileged. They can cache the results in various ways, or
  disable target-feature detection completely if they so desired, e.g., by
  providing a run-time that always returns false. By default, `libstd` will
  provide a run-time that's meaningful for user-space, such that these
  applications don't have to do anything, and such that their `#![no_std]`
  dependencies like `libcore` can perform run-time feature detection.
  
* **privileged applications**: OS kernels and embedded applications can provide
  a run-time that satisfies their use case and constraints. 

* **cdylibs**: dynamic libraries linked against the standard library get by
  default the `libstd` run-time. If these are `#![no_std]`, but have access to
  system APIs, e.g., via `libc`, they might be able to just include the `libstd`
  run-time from crates.io, without having to depend on `libstd` itself.
  Otherwise, they can use their knowledge of the target they are running on to
  implement their own run-time.
  
Implementing a run-time requires an `unsafe` trait impl, making it clear that
care must be taken. The API requires run-times to just return `false` on
unknown features, making them conservative in such a way that prevents
unsoundness in safe Rust code. If a run-time doesn't support a feature, safe
Rust might panic, or run slower, but it will not try to run code that requires
an unsupported feature.
  
If a program never performs any run-time feature detection, all
detection-related code is dead. LTO should be able to remove this code, but if
this were to fail, users can always define a dummy run-time that always returns
false, and has no caches, etc.

The run-time feature-detection API dispatches calls to the run-time only when
necessary. If the default run-time isn't "the best" along some axis for some
application, this RFC allows the application to replace them with a better one.
With this RFC, there is no reason not to use the run-time feature detection
macros.

## Alternatives

We don't have to solve this problem. This means that `libcore` and other
`#![no_std]` libraries can't use run-time feature detection, and can't benefit,
e.g., of advanced SIMD instructions.

We also could do something different. For example, we could provide a "cache" in
libcore, and an API for users or only for the standard library, to initialize
this cache externally, e.g., during the standard library initialization routine.

This runs into problems with `cdylib`s, where these routines might not be called
automatically, potentially requiring C code to have to manually call into
`libstd` initialization routines. It also runs into problems with often imposing
a cost on users, e.g., due to a cache in libcore, even though users might never
use it. This would be limiting, if e.g. having a cache in read-write memory
prevents libcore from being compiled to a read-only binary. We would need to
feature gate this functionality to avoid these issues.

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

* Should the API use a `TargetFeature` `enum` or be stringly-typed like the
  macros and use string literals?
  
* Since the `TargetFeature` `enum` is architecture-specific, should it live in
  `core::arch::{target_arch}::TargetFeature` ?

# Future possibilities
[future-possibilities]: #future-possibilities

None. After this RFC, the run-time feature detection part of the Rust language
should be complete.
