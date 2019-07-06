- Feature Name: `unwind-through-FFI`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: ???
- Rust Issue: ???

# Summary
[summary]: #summary

Provide a well-defined mechanism for unwinding through FFI boundaries.

* Stabilize the function annotation `#[unwind(allowed)]`,
  which explicitly permits `extern` functions
  to unwind (`panic`) without aborting.
* If this annotation is used anywhere in the dependency tree, 
  generation of the final product will fail
  unless the panic strategy is `unwind`
  and a non-default panic runtime is specified.
* Provide an `unwind` runtime in the standard library
  that guarantees compatibility with the native exception mechanism
  provided by the compiler backend.
* Provide a Cargo option under `[profile.def]` to specify a `panic` runtime.

# Motivation
[motivation]: #motivation

This will enable resolving
[rust-lang/rust#58794](https://github.com/rust-lang/rust/issues/58794)
without breaking existing code.

Currently, unwinding through an FFI boundary is always undefined behavior.
We would like to make the behavior safe
by aborting when the stack is unwound to an FFI boundary.
However, there are existing Rust crates
(notably, wrappers around the `libpng` and `libjpeg` C libraries)
that rely on the current implementation's compatibility
with the native exception handling mechanism in
GCC, LLVM, and MSVC.

This RFC will give crate authors the ability
to initiate stack-unwinding
that can propagate through other languages' stack frames,
as well as tools for ensuring compatibility with those languages.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Consider an `extern "C"` function that may `panic`:

```rust
extern "C" fn may_panic(i: i32) {
    if i < 0 {
        panic!("Oops, I should have used u32.");
    }
}
```

----------------------------------------------------------------------------
TODO below this line
----------------------------------------------------------------------------

In a future (TBD) version of Rust, calling this function with an argument less
than zero will cause the program to be aborted. This is the only way the Rust
compiler can ensure that the function is safe, because there is no way for it
to know whether or not the calling code supports the same implementation of
stack-unwinding used for Rust.

As a concrete example, Rust code marked `exern "C"` may be invoked from C code,
but C code on Linux or BSD may be compiled without support for native
stack-unwinding. This means that the runtime would lack the necessary metadata
to properly perform the unwinding operation.

However, there are cases in which a `panic` through an `extern` function can be
used safely. For instance, it is possible to invoke `extern` Rust functions
via Rust code built with the same toolchain, in which case it would be
irrelevant to the unwinding operation that the `panic`ing function is an
`extern` function:

```rust
fn main() {
    let result = panic::catch_unwind(|| {
        may_panic(-1);
    }
    assert!(result.is_err());
}
```

In order to ensure that `may_panic` will not simply abort in a future version
of Rust, it must be marked `#[unwind(Rust)]`. This annotation can only be used
with an `unsafe` function, since the compiler is unable to make guarantees
about the behavior of the caller.

```rust
#[unwind(Rust)]
unsafe extern "C" fn may_panic(i: i32) {
    if i < 0 {
        panic!("Oops, I should have used u32.");
    }
}
```

**PLEASE NOTE:** Using this annotation **does not** provide any guarantees
about the unwinding implementation. Therefore, using this feature to compile
functions intended to be called by C code **does not make the behavior
well-defined**; in particular, **the behavior may change in a future version of
Rust.**

The *only* well-defined behavior specified by this annotation is Rust-to-Rust
unwinding.

It is safe to call such a function from C or C++ code only if that code is
guaranteed to provide the same unwinding implementation as the Rust compiler
used to compile the function.

Since the behavior may be subject to change without notice as the Rust compiler
is updated, it is recommended that all projects that rely on unwinding from
Rust code into C code lock the project's `rustc` version and only update it
after ensuring that the behavior will remain correct.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Unwinding for functions marked `#[unwind(Rust)]` is performed as if the
function were not marked `extern`. This is identical to the behavior of `rustc`
for all versions prior to 1.35 except for 1.24.0.

This annotation has no effect on functions not marked `extern`. It has no
observable effect unless the marked function `panic`s (e.g. it has no
observable effect when a function returns normally or enters an infinite loop).

# Drawbacks
[drawbacks]: #drawbacks

Since the Rust unwinding implementation is not specified, this annotation is
explicitly designed to expose a potentially non-forward-compatible API. As
mentioned in [the guide-level explanation](#guide-level-explanation), use of
this annotation will make projects vulnerable to breakage (and specifically to
undefined behavior) simply by updating their Rust compiler.

Furthermore, this annotation will have different behaviors on different
platforms, and determining whether it is safe to use on a particular platform
is fairly difficult. So far, there are only two safe use cases identified:

* On Windows, C code built with MSVC always respects SEH, the unwinding
  mechanism used by both (MSVC) Rust and C++, so it should always be safe to
  invoke such a function when MSVC is the only toolchain used, as long as
  `rustc` uses SEH as its `panic` implementation.
* In projects using LLVM or GCC exclusively, the `-fexceptions` flag ensures
  that C code is compiled with C++ exception support, so the runtime behavior
  should be safe as long as `rustc` uses the native C++ exception mechanism as
  its `panic` implementation.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is the minimum possible change to the language that will permit making
`panic` safe by default while permitting existing users of the current behavior
a way to keep their code working after
[rust-lang/rust#58794](https://github.com/rust-lang/rust/issues/58794) is
resolved.

The language team has twice attempted to stabilize the desired default behavior
of aborting when unwinding across an FFI boundary without providing a way to
opt-out of this behavior, but it has become clear that the community will not
accept such a change without a means of opting out because of the impact on
existing projects (particularly [mozjpeg](https://crates.io/crates/mozjpeg)).

Any alternatives that provide guarantees about the specific Rust unwinding
implementation would make implementation more difficult and would lock us in to
a specific annotation semantics for describing unwinding mechanisms. Suggested
notations include:

- `#[unwind(c)]`
- `#[unwind(c++)]`
- `#[unwind(c++-native)]`
- `#[unwind(seh)]`

This proposal does not exclude the possibility of introducing one or more of
these notations at a later date, and indeed it will almost certainly be
necessary to introduce a way to specify an unwinding implementation if the
`rustc` default unwinding mechnanism ever changes. However, introducing such
a notation would be a larger change to the language, and there is no consensus
yet on what the notation should be.

# Prior art
[prior-art]: #prior-art

The proposed behavior of this annotation is simply to maintain the existing
behavior for un-annotated functions.

As mentioned above, GCC and LLVM provide an `-fexceptions` flag that makes the
C++ exception mechanism interoperable with C stackframes.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

As mentioned [above](#rationale-and-alternatives), further work will be
required to provide a means of specifying details of the unwinding
implementation to provide guarnateed-safe interoperability with (some) C and
C++ code. That work is out of scope for this RFC.

# Future possibilities
[future-possibilities]: #future-possibilities

As mentioned [above](#rationale-and-alternatives), further work will be
required to provide a means of specifying details of the unwinding
implementation to provide guarnateed-safe interoperability with (some) C and
C++ code. That work is out of scope for this RFC.

Note that this feature _does not_ commit the team to delivering future variants
of the `#[unwind(...)]` annotation. For instance, compatibility with C code
could be provided via a `rustc` flag specifying the (global) unwinding
implementation to use.
