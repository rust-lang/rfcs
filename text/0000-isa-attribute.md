- Feature Name: isa_attribute
- Start Date: 2020-02-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes a new function attribute, `#[isa]`.  The minimal initial implementation will provide `#[isa = "a32"]` and `#[isa = "t32"]` on ARM targets, corresponding respectively to disabling and enabling the LLVM feature `thumb-mode` for the annotated function.

# Motivation
[motivation]: #motivation

Starting with `ARMv4T`, ARM cores support a slimmed-down instruction set called Thumb.  (Due to the introduction of AArch64, the original ARM and Thumb instruction sets are now referred to as A32 and T32, and this RFC will use this terminology from here on in.) Switching between these instruction sets ("interworking") can be done at the instruction level by clearing or setting the lowest bit of the program counter.  LLVM already knows how to insert interworking shims, but Rust lacks the necessary language-level support.  Prior to the adoption of [RFC 2045], it was possible to use the unstable `target_feature` attribute to disable or enable `thumb-mode`, but the stabilised syntax for that attribute focused on enabling opt-in features such as SIMD and vector instructions; since `thumb-mode` is an "either-or" feature, it is no longer the right tool for the job.

[RFC 2045]: https://github.com/rust-lang/rfcs/blob/master/text/2045-target-feature.md

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Some platforms have multiple different instruction sets, optimised for different requirements; for example, ARM targets have a denser but less feature-packed instruction set named T32 alongside the normal A32.  Rust supports configuring which instruction set any given function is compiled to via the `#[isa]` attribute.  For example, if on an ARM target you wish for a particular function to be compiled to T32 instructions for reduced code size, you would annotate the function like so.

```rust
#[isa = "t32"]
fn some_function() {
    // ...
}
```

That's all you need to do - LLVM inserts interworking shims where necessary, so the change is completely transparent.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Functions are inlined across ISA boundaries as if the `#[isa]` attribute did not exist.

Annotating a function with an ISA that does not exist yields a compile-time error.

```rust
#[isa = "unicorn"]
fn some_function() {
    // ...
}
```

```
error: "unicorn" is not a recognised ISA for the target `armv5te-unknown-linux-gnueabi`
  --> src/lib.rs:1:1
   |
 1 | #[isa = "unicorn"]
   |         ^^^^^^^^^ help: valid ISAs are `a32`, `t32`
```

Annotating a function with two different ISAs at once yields a compile-time error.  (This is likely to be the result of a typo when editing code.)

```rust
#[isa = "a32"]
#[isa = "t32"]
fn some_function() {
    // ...
}
```

```
error: a function cannot have two `#[isa]` attributes at the same time
  --> src/lib.rs:1:1
   |
 1 | #[isa = "a32"]
   | -------------- first attribute was here
...
 2 | #[isa = "t32"]
   | ^^^^^^^^^^^^^^ help: remove one of these attributes
```

# Drawbacks
[drawbacks]: #drawbacks

Adding another attribute complicates Rust's design.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Extending `target_feature` to allow `#[target_feature(disable = "...")]` and adding `thumb-mode` to the whitelist would support this functionality without adding another attribute; however, this is verbose, and does not fit with the `target_feature` attribute's focus on features such as AVX and SSE whose absence is not necessarily compensated by the presence of something else.

Doing nothing is an option; it is possible to incorporate code using other instruction sets through other means such as external assembly.  However, this steps outside Rust's safety guarantees.

# Prior art
[prior-art]: #prior-art

GCC supports opting into interworking with the `--thumb-interwork` flag - its syntactic equivalents to `#[isa = "a32"]` and `#[isa = "t32"]` are `__attribute__((target("arm")))` and `__attribute__((target("thumb")))`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are there any presently-supported architectures with a mechanism like A32/T32 which `#[isa]` could support?

# Future possibilities
[future-possibilities]: #future-possibilities

- RISC-V allegedly supports truncated instructions in a similar fashion to T32; the `#[isa]` attribute may benefit users of that architecture in the future.
- Should Rust gain support for the 65C816, the `#[isa]` attribute might be extended to allow shifting into its 65C02 compatibility mode and back again.
