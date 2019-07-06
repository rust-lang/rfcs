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
* Stabilize the `#![panic_runtime]` annotation (from
  [RFC #1513](1513-less-unwinding.md)).
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

In a future (TBD) version of Rust,
calling this function with an argument less than
zero will cause the program to be aborted.
This is the only way the compiler can ensure that the function is safe,
because there is no way for it to know whether or not the calling code
supports the same implementation of stack-unwinding used for Rust.

As a concrete example,
this function may be invoked from C code on Linux or BSD
compiled without support for native stack-unwinding.
The C runtime would lack the necessary metadata
to properly propagate the unwinding operation,
so it would be undefined behavior to let the runtime attempt
to unwind the C stack frames.

However, if the C code is compiled
with support for the same stack-unwinding mechanism
used to by the Rust code,
unwinding across the FFI boundary is well-defined and safe,
and in fact it can be a useful way to handle errors
when working with certain C libraries, such as `libjpeg`.

Thus, the `may_panic` function may be makred `#[unwind(allow)]`,
which ensures that the unwinding operation will be propagated to the caller
rather than aborting the process.
This annotation will also prevent the compiler
from marking the function as `noexcept`
(which permits the backend toolchain from optimizing based on the assumption
that an unwinding operation cannot escape from a function).

This annotation can only be used with an `unsafe` function,
since the compiler is unable to guarantee
that the caller will be compiled with a compatible stack-unwinding mechanism.

This RFC does, however, provide a tool that will enable users
to provide this guarantee themselves.
We will stabilize the `!#[panic_runtime]` annotation,
which [designates a crate as the provider of the final product's panic runtime]
(1513-less-unwinding.md).
Additionally, the standard library's current `panic=unwind` runtime crate,
`libpanic_unwind`, which is compatible with native C++ style exceptions,
will be provided under another name (such as `libpanic_native`)
that guarantees the implementation will remain compatible with C++ exceptions
(while the implementation of `libpanic_unwind` itself may change
to no longer maintain that compatibility).

In order for Cargo users to be able
to specify the C++ compatible `panic_runtime` implementation,
a new optional value, runtime, will be added to the `profile.dev.panic` option:

```toml
[profile.dev]
panic = { 'unwind', runtime = 'native' }
```

`panic.runtime` may only be specified if the panic strategy is `unwind`.
And just as there may only be one strategy
in the dependency graph for a final product,
there may only be one `runtime`.

For now, the only valid `runtime` keys will be:

* `default` - identical to `panic = 'unwind'` with no `runtime` selected.
  This will use Rust's default unwinding runtime,
  unless another crate in the final product's dependency graph
specifies `panic = 'abort'`,
  in which case the `abort` strategy will take precedence as usual.
  which is not guaranteed to be compatible with native exceptions.
* `native` - as discussed above, this will preserve the behavior of the current
  implementation of `libpanic_unwind`.
* `crate` - indicates that a non-`std` crate will use `#![panic_runtime]` to
  provide the runtime.
* `self` - indicates that this crate itself provides `#![panic_runtime]`.

If the `native` or `crate` key is specified
anywhere in a final product's dependency graph,
no crate in that dependency graph may specify the `panic = abort` strategy;
this mismatch will cause the build to fail.

The function annotation `#[panic(allowed)]` will only be permitted in crates
that specify a non-default `panic.runtime`.

Crates that do not use the `profile.dev.panic` option at all
will remain compatible with any `profile.dev.panic` configuration
used to generate the final product.

For non-Cargo users, equivalent `rustc` flags will be provided
(which will be how Cargo itself implements the option).

-------------------------------------------------------------------------------
TODO - below this line
-------------------------------------------------------------------------------

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
  its `panic` implementation.            `

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
