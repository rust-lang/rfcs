- Feature Name: `unwind-through-FFI`
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: ???
- Rust Issue: ???

# Summary
[summary]: #summary

Provide a well-defined mechanism for unwinding through FFI boundaries.

* Add an `#[unwind(allowed)]` function attribute that's supported on `extern` functions and `extern` function declarations.
  which explicitly permits `extern` functions
  to unwind (`panic`) without aborting.
* If this annotation is used anywhere in the dependency tree, 
  generation of the final (binary) product will fail
  unless the panic strategy is `unwind`
  and a non-default panic runtime is specified.
* Stabilize the `#![panic_runtime]` annotation (from
  [RFC #1513](less-unwinding)).
* Provide an `unwind` runtime in the standard library
  that guarantees compatibility with the native exception mechanism
  provided by the compiler backend.
* Provide a Cargo option under `[profile.foo]` to specify a `panic` runtime.

[less-unwinding]: https://github.com/rust-lang/rfcs/blob/master/text/1513-less-unwinding.md

# Motivation
[motivation]: #motivation

This will enable resolving
[rust-lang/rust#58794][rust-ffi-issue]
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

[rust-ffi-issue]: https://github.com/rust-lang/rust/issues/58794

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
from marking the function as `nounwind`
(which permits the backend toolchain from optimizing based on the assumption
that an unwinding operation cannot escape from a function).

This annotation can only be used with an `unsafe` function,
since the compiler is unable to guarantee
that the caller will be compiled with a compatible stack-unwinding mechanism.

This RFC does, however, provide a tool that will enable users
to provide this guarantee themselves.
We will stabilize the `!#[panic_runtime]` annotation,
which [designates a crate as the provider of the final product's panic runtime]
(less-unwinding).
Additionally, the standard library's current `panic=unwind` runtime crate,
`libpanic_unwind`, which is compatible with native C++ style exceptions,
will be provided under another name (such as `libpanic_native`)
that guarantees the implementation will remain compatible with C++ exceptions
(while the implementation of `libpanic_unwind` itself may change
to no longer maintain that compatibility).

In order for Cargo users to be able
to specify the C++ compatible `panic_runtime` implementation,
a new optional value, runtime, will be added to the `profile.foo.panic` option:

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

The function annotation `#[unwind(allowed)]` will only be permitted in crates
that specify a non-default `panic.runtime`.

Crates that do not use the `profile.foo.panic` option at all
will remain compatible with any `profile.foo.panic` configuration
used to generate the final product.

For non-Cargo users, equivalent `rustc` flags will be provided
(which will be how Cargo itself implements the option).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Unwinding for functions with the `#[unwind(allowed)]` annotation
is performed as if the function were not marked `extern`.
This annotation is not permitted on functions not marked `extern`.
It has no observable effect unless the marked function `panic`s
(e.g. it has no observable effect
when a function returns normally or enters an infinite loop).
The LLVM IR for such functions must not be marked `nounwind`.

The compiler will have a new stable flag, `-C panic.runtime`,
which will be required to enable the `#[unwind(allowed)]` annotation;
as explained above, the flag will specify the expected source
of the panic runtime to be used in the final (binary) product.
If the source is `default`,
the `libpanic_unwind` crate will provide the runtime,
and the `#[unwind(allowed)]` annotation will not be permitted in that crate.

Different values of `panic.runtime` will not be permitted for different crates
in a single dependency graph,
with the exception of `self`, which may only be used once
in a dependency graph,
and is only compatible with crates using the `crate` value.
Because the `panic` runtime is lazily selected and linked,
crates that do not specify a value for `panic.strategy`
are compatible with crates using any of the four values.
Crates that explicitly specify the `default` runtime, however,
are not compatible with crates using other runtimes.

In order to verify that all crates in a dependency graph
are compatible in this way,
new metadata will need to be added to the `rlib` format.
It may be possible to combine this metadata with the `panic` strategy metadata.

# Drawbacks
[drawbacks]: #drawbacks

* This change involves several elements of the toolchain,
  from Cargo all the way to codegen and linking.
  It also involves the simultaneous stabilization of multiple features.
  Although it may be possible to stabilize some of these features
  independently of the others,
  the proposed changes are still somewhat complex,
  even though implementation should be fairly simple.
* The rules regarding the interaction between
  `-C panic`, `-C panic.strategy`, and `#[unwind(allowed)]`
  are somewhat complex.
* Even with custom `panic` runtimes,
  users may still inadvertently cause undefined behavior
  by trying to link shared libraries that use different unwind runtimes.
  For instance, there is no easy way to know whether a C shared library
  on a non-Windows system
  was compiled with the `-fexceptions` GCC or LLVM flag;
  with this flag, such a library would be unwind-runtime-compatible
  with a Rust shared library compiled with `-C panic.strategy=native`,
  but without this flag, any attempt to unwind through the C stack frames
  (regardless of the runtime used)
  would be undefined behavior.
  There is no way for Rust to guard against this scenario.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This version of this RFC replaces a prior version
that suggested a much more minimal change.
Specifically, it suggested introduing a different function annotation,
`#[unwind(Rust)]`, which would simply ensure that the marked function
would not be marked `noexcept` and would not `abort` on `panic`.
Because the current implementation of `libpanic_unwind`
is compatible with native (C++ style) exceptions,
no further guarantees were made
regarding the behavior of the unwinding operation itself.

Another possibility would be to add the `panic.runtime` options
as new `strategy` values for the `-C panic=foo` flag.
This may be simpler to teach and to implement,
but crates using `-C panic=native`
would be incompatible with all existing crates.

The current proposal ensures
that Rust provides full control over unwinding behavior
for maximum compatibility with other languages' runtimes
and with existing crates.
The interaction between the `#[unwind(allowed)]` annotation
and the `panic.strategy` option
provides the maximum safety that the toolchain can provide
without limiting Rust's ability to interoperate
with shared libraries in other languages
(particularly `libpng` and `libjpeg` as mentioned
[above](#motivation)).

# Prior art
[prior-art]: #prior-art

[RFC #1513](less-unwinding) introduced
the `#![panic_runtime]` attributes, the `-C panic` compiler flag, and
the corresponding `profile.foo.panic` Cargo option.

The `#[unwind(allowed)]` annotation already exists as an unstable feature,
though the current implementation does not impose any requirements
on the `panic` implementation.

As mentioned above, GCC and LLVM provide an `-fexceptions` flag
that makes the C++ exception mechanism interoperable with C stackframes.
The C standard does not refer to exceptions at all, however,
and the C++ standard does not make any guarantees
regarding the behavior of exceptions that escape from C++ stack frames
into stack frames written in another language.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should the `panic.runtime` option and its permitted values be renamed?
* What is the minimal subset of features listed here that must be released
  prior to stabilizing the FFI abort-on-`panic` logic
  in order to prevent breaking crates that link with `libpng` and `libjpeg`?
* Would multiple `panic` runtimes within a single process space
  be a potentially useful feature in the future?
  If so, is this RFC forward-compatible with possible approaches
  for providing this feature?
* Will C++ code be able to interact with `panic.runtime=native`
  by catching the native exception?
  What operations (`Drop`ing, re-throwing, introspecting...) would be safe
  for the C++ runtime to perform?

# Future possibilities
[future-possibilities]: #future-possibilities

If it is safe for C++ code to interact with `panic` objects from Rust,
it may be useful to create a C header declaring an interface to do so.
Such a header would not necessarily be provided by the Rust team
or as part of the Rust toolchain distribution,
but the author of such a header may need assistance
from the author(s) of the native-unwinding runtime crate.
