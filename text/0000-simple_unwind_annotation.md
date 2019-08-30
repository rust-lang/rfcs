- Feature Name: `simple_unwind_attribute`
- Start Date: 2019-08-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Provides an annotation to permit functions with explicit ABI specifications
(such as `extern "C"`) to unwind

# Motivation
[motivation]: #motivation

TODO

- soundness & optimization
- libjpeg/mozjpeg

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `#[unwind(allowed)]` attribute permits functions with non-Rust ABIs (e.g. `extern "C" fn`) to unwind rather than terminating the process.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

By default, Rust assumes that an external function imported with `extern "C" {
... }` cannot unwind, and Rust will abort if a panic would propagate out of a
Rust function with a non-"Rust" ABI ("`extern "ABI" fn`") specification. If you specify
the `#[unwind(allowed)]` attribute on a function with a non-"Rust" ABI, Rust
will instead allow an unwind (such as a panic) to proceed through that function
boundary using Rust's normal unwind mechanism. This may potentially allow Rust
code to call non-Rust code that calls back into Rust code, and then allow a
panic to propagate from Rust to Rust across the non-Rust code.

The Rust unwind mechanism is intentionally not specified here. Catching a Rust
panic in Rust code compiled with a different Rust toolchain or options is not
specified. Catching a Rust panic in another language or vice versa is not
specified. The Rust unwind may or may not run non-Rust destructors as it
unwinds. Propagating a Rust panic through non-Rust code is unspecified;
implementations that define the behavior may require target-specific options
for the non-Rust code, or this feature may not be supported at all.

# Drawbacks
[drawbacks]: #drawbacks

- Only works as long as Rust uses the same unwinding mechanism as C++.
- Does not allow external library bindings to specify whether callbacks they accept are expected to unwind.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

TODO

- https://github.com/rust-lang/rfcs/pull/2699

# Prior art
[prior-art]: #prior-art

TODO

# Unresolved questions
[unresolved-questions]: #unresolved-questions

TODO

# Future possibilities
[future-possibilities]: #future-possibilities

TODO

- `unwind(abort)`
- non-"C" ABIs
