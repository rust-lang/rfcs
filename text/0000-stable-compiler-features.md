- Feature Name: stable-compiler-features
- Start Date: 2016-01-22
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Stabilize some low level feature flags.

# Motivation
[motivation]: #motivation

Currently there is no way to use LLVM intrinsics or `asm!` block in stable Rust
as they are considered incompatible with hypothetical alternative
implementations. But some of these features are highly desired in some
environments where currently it is required either to:

- use nightly
- compile `rustc` from source
- use external files (assembly)

# Detailed design
[design]: #detailed-design

Some low-level features should be hidden behind stable feature gates. This would
allow compiler to signalize user which features aren't supported on current
platform/implementation.

Ex. `#[feature(asm)]` would be stable in mainline Rust but not on GCC Rust, so
GCC Rust can at compile time inform user that `asm` feature is not available and
provide alternative (like `asm_gcc`) flag.

# Drawbacks
[drawbacks]: #drawbacks

There will be some crates that will require given compiler to work, but I think
that it can be mitigated by Cargo's feature flags.

Also there is non-0 possibility that these flags can be overused but I think
that most Rust users would not be interested in them if we keep only some
low-level stuff behind stable gates.

# Alternatives
[alternatives]: #alternatives

Keep current status quo.

# Unresolved questions
[unresolved]: #unresolved-questions

- Which flags should be stabilized? I think that for now only:
  + `asm`
  * `llvm_intrinsics`
