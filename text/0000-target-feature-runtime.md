- Feature Name: `target_feature_runtime`
- Start Date: 2019-07-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Right now, the `is_..._feature_detected!("target-feature")` macros exported by
`libstd` are the only proper way in which Rust libraries and binaries can
perform target-feature detection at run-time.

This RFC extends that support to `#![no_std]` libraries, by moving the
target-feature detection macros to `libcore`. This enables all Rust libraries,
including `libcore`, to perform target-feature detection at run-time.

The implementation proposed can be, as an extension, stabilized. This would
allow `#![no_std]` binaries to provide their own target-feature-detection
run-time and benefit from it as well.

# Motivation
[motivation]: #motivation

## Refresher on target features

> You can safely skip this sub-section if you are familiar with compile-time and
> run-time target-feature detection in Rust.

A Rust target triple, like `x86_64-apple-darwin`, produce binaries that can run
on all CPUs of the `x86_64` family that support certain architecture
"extensions". This particular target requires SSE3 vector extensions, and Rust
will emits them whenever it deems fit. As a consequence, binaries compiled for
this target can only on CPUs that support SSE3 extension. Other targets require
different sets of extensions. For example, `x86_64-unknown-linux-gnu` only
requires SSE2 support, allowing binaries to run on CPUs that do not support
SSE3. In Rust, we call `x86_64` the target architecture "family", and extensions
like SSE2 or SSE3 "target-features".

Many Rust applications compiled for `x86_64-unknonw-linux-gnu` do want to use
SSE3 extensions when the CPU the binary runs on, and Rust allows enabling these
extensions via the `#[target_feature]` function attribute. The behavior of a
program that attempts to execute code that uses an extension that is not
supported by the CPU in which the binary runs on is undefined, and the compiler
generates machine code under the assumption that this does not happen. For such
programs to be safe, they need to detect whether the CPU in which the binary
runs on supports the particular features that they want to use, and only use
them when the CPU actually supports them.

Currently, target-features can be detected:

* at compile-time: using `#[cfg(target_feature = literal)]` to conditionally
  compile code.
* at run-time: using the `is_{target_arch}_feature_detected!(literal)` macros
  from the standard library to query whether the system the binary runs on
  actually supports a feature or not.

## Problem statement

The `cfg(target_feature = "target_feature_literal")` macro can be used by all
Rust code, but is limited to the set of features that are unconditionally
enabled for the target.

The architecture-specific
`is_{target_arch}_feature_detected!(target_feature_literal)` macros require
operating-system support and are therefore only exposed by the standard library;
`#![no_std]` libraries, like `liballoc` and `libcore` are platform agnostic and
cannot currently perform run-time feature detection.

That is, currently, libraries have to choose between being `#![no_std]`-compatible,
or performing target-feature detection at run-time.

As a consequence, there are crates in `crates.io` re-implementing methods of
`libcore` types like `&str`, `[T]`, `Iterator`, etc. but with much better
performance, by using target-feature detection at run-time.

One example is the `is_sorted` crate, which provides an implementation of
`Iterator::is_sorted`, which performs 16x better for some inputs than the
`libcore` implementation by using AVX when available. Another example include
the `memchr` crate, as well as crates implementing algorithms to compute whether
a `[u8]` is an ASCII string or an UTF-8 string, which end up being used every
time a program calls `String::from_utf8`. By using AVX on x86, these perform on
the ballpark of about 1.6x better than the `libcore` implementations, and could
probably do better using AVX-512. Most Rust does not, however, benefit from
these, because this code calls `str::from_utf8` which is part of `libcore` which
cannot use run-time target-feature detection..

This is a shame. Whether a library is `#![no_std]` or not is orthogonal to
whether the final binary is able to perform run-time feature detection and most
binaries using `#![no_std]` crates do end up linking `libstd` into the final
binary. Simultaneously, `#![no_std]` binaries cannot use any library that
performs run-time target-feature detection, even though it would be perfectly
safe for the API to just return that no features are detected at run-time.

The goal of this RFC is to enable `#![no_std]` libraries and binaries to perform
run-time feature detection.

## Use cases

`#![no_std]` libraries and binaries are used in a wider-range of
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

Users can continue to perform run-time feature detection by using the stable
`is_{architecture}_feature_detected!` macros. This RFC makes this macros
available in `libcore`, such that `#![no_std]` libraries and binaries can use
them.

As an extension, this RFC also allows users to provide their own target-feature
detection run-time:

```rust
#[target_feature_detection_runtime]
static TargetFeatureRT: impl core::detect::TargetFeatureRuntime;
```

by using the `#[target_feature_detection_runtime]` attribute on a `static`
variable of a type that implements the `core::detect::TargetFeatureRuntime`
`trait` (see [definition below][runtime-trait]).

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

The initial user of this feature will be `libstd` itself, which will use it to
implement its own target-feature detection run-time. When `libstd` is linked
into the final binary, the target-feature detection macros will use this
run-time to detect the available target-features.

This extension could be considered an "implementation-detail" of how to expose
the feature-detection macros in `libcore`, and can be technically stabilized at
a later time. That is, we could expose the feature-detection macros in libcore
first, worrying about the details of how to make that configurable at a later
time.

This RFC works these details out and proposes a concrete design for them.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC exports the target-feature detection macros from `libcore`, and
introduces:

* a new attribute: `#[target_feature_detection_runtime]`,
* a new trait: `core::detect::TargetFeatureRuntime`,
* a new enum: `core::detect::TargetFeature`, and
* a new function: `core::detect::is_target_feature_detected`.

Stabilizing the usage of the target-feature detection macros from `libcore`
could be done before stabilizing the rest of the APIs proposed here, and would
allow all `#![no_std]` libraries including `libcore` to use run-time
target-feature detection, and benefit from it if `libstd` is linked into the
final binary. 

The rest of the API could be initially left as unstable and remain only used by
`libstd`. Stabilizing it would, however, allow `#![no_std]` binaries to benefit
from proper target-feature detection as well.

## Export target-feature detection macros from libcore

This RFC exports the feature-detection macros from `libcore`. Right now, the
only stable feature-detection macro is
`is_x86_feature_detected!("target_feature_name")`.

If the rest of the API is stabilized, the semantics of these macros could be
made more precise, by using the rest of the API proposed here in their
specification:

```rust
/// Returns `true` if `cfg!(target_feature = target-feature-literal)` is 
/// `true`, and returns the value of `core::detect::is_feature_detected` 
/// for the target-feature otherwise.
///
/// If the target-feature is not a known target-feature for the current 
/// `architecture`, or the required `feature()` gate to use the feature 
/// is not enabled, the program is ill-formed, and a compile-time 
/// diagnostic is emitted.
is_{architecture}_feature_detected!(string-literal) -> bool;
```

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
dependency graph, Rust provides a default definition that always returns `false`
(no feature is detected). 

The standard library provides a target-feature detection run-time for some Rust
targets, and attempting to provide a user-defined run-time for these targets is
illegal, since that would result in two run-times being part of the dependency
graph.

Being able to override the run-time provided by `libstd` could be pursued as an
extension, but at the time of this writing no use cases for this feature have
been found. This extension would work by only linking the `libstd` run-time if
there is no run-time in the dependency graph, similarly to how
`#[global_allocator]` currently works.

## The `core::detect::TargetFeatureRuntime` trait
[runtime-trait]: #runtime-trait

The target-feature detection run-time must be a `static` variable of a type that
implements the `core::detect::TargetFeatureRuntime` trait:

```rust
unsafe trait core::detect::TargetFeatureRuntime {
    /// Returns `true` if the `feature` is known to be supported by the 
    /// current thread of execution and `false` otherwise.
    fn is_target_feature_detected(feature: core::detect::TargetFeature) 
        -> bool;
}
```

This `trait` is `unsafe` to implement, and a correct implementation is required
for soundness of safe Rust code. In particular, the trait method shall only
return that a feature is supported by the current thread of execution if this is
actually the case. An incorrect implementation of this trait could cause "safe"
Rust code to have undefined behavior.

Note that the `TargetFeature` enum (see below) is `#[non_exhaustive]`, that is,
matching on this enum is required to handle unknown enum variants, and it is
always correct to return that unknown features are not available at run-time.

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

# Drawbacks
[drawbacks]: #drawbacks

This increases the complexity of the implementation, adding another "singleton"
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

The main alternative is that we don't have to stabilize everything at the same
time. We could implement this as proposed, but only stabilize using the
target-feature detection macros via `libcore`. This would mean that initially,
`#![no_std]` binaries won't be able to implement their own run-times, but that
would unlock using the macros on all `#![no_std]` libraries, and these macros
would do something meaningful if `libstd` is linked into the final binary.

## libcore pulls target-features approach

We could provide a "cache" in libcore, and an API for users or only for the
standard library, to initialize this cache externally, e.g., during the standard
library initialization routine.

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

* We could implement this RFC, without making any APIs public, by just moving
  the feature-detection macros to `libcore`. That would allow `#![no_std]`
  libraries to use them, and they will do something meaninful if `libstd` is
  linked. `#![no_std]` binaries won't be able to provide their own run-time, but
  the APIs for this (the trait, enum, and `#[target_feature_detection_runtime]`
  attribute) could be stabilized at a later time.

* Should the API use a `TargetFeature` `enum` or be stringly-typed like the
  macros and use string literals?
  
* Since the `TargetFeature` `enum` is architecture-specific, should it live in
  `core::arch::{target_arch}::TargetFeature` ?

* How does it fit with the Roadmap? Does it fit with the Roadmap at all? Would
  it fit with any future Roadmap?

* Should the `libstd` run-time be overridable? For example, by only providing it
  if no other crate in the dependency graph provides a runtime ? This would be a
  forward-compatible extension, but no use case considered requires it.

# Future possibilities
[future-possibilities]: #future-possibilities

None. After this RFC, the run-time feature detection part of the Rust language
should be complete.
