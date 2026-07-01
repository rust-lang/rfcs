- Feature Name: `abi_custom` 
- Start Date: 2026-07-01)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#140829)](https://github.com/rust-lang/rust/issues/140829)

## Summary
[summary]: #summary

An `extern "custom" fn` is a function with a custom ABI that is unknown to rust. Often these are low-level functions that pass arguments in different registers than any standard calling convention, so using this helps rustc block you from using this in any place where rustc would need to understand that calling convention.


```rust
/// # SAFETY
///
/// - Expects the dividend and the divisor in r0 and r1.
/// - Returns the quotient in r0 and the remainder in r1.
#[unsafe(naked)]
pub unsafe extern "custom" fn __aeabi_uidivmod() {
    core::arch::naked_asm!(
        "push {{lr}}",
        "sub sp, sp, #4",
        "mov r2, sp",
        "bl {trampoline}",
        "ldr r1, [sp]",
        "add sp, sp, #4",
        "pop {{pc}}",
        trampoline = sym crate::arm::__udivmodsi4
    );
}

unsafe extern "custom" {
	fn __fentry__();
}
```

### History

* Proposal: https://github.com/rust-lang/rust/issues/140566
* Tracking issue: https://github.com/rust-lang/rust/issues/140829
* Implementation: https://github.com/rust-lang/rust/pull/140770
* Stabilization PR: https://github.com/rust-lang/rust/pull/158504

## Motivation
[motivation]: #motivation

In some low-level scenarios we must define naked functions with a custom ABI. This comes up in `rust-lang/compiler-builtins` (e.g. for `__rust_probestack` and `__aeabi_uidivmod`), and also in systems programming (e.g. when defining `__fentry__`, a symbol used the mcount mechanism).

The current solution is often to use `extern "C"`, but that is misleading: the function does not use the C calling convention, and calling it as if it were is almost certainly causing UB.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Because rust doesn't know what calling convention to use, an `extern "custom"` function can only be called via inline assembly or FFI. 

```
error: functions with the "custom" ABI cannot be called
 --> <source>:5:5
  |
5 |     bar();
  |     ^^^^^
  |
note: an `extern "custom"` function can only be called using inline assembly
```

An `extern "custom"` function definition must be a naked function:

```
error: items with the "custom" ABI can only be declared externally or defined via naked functions
  --> <source>:10:1
   |
10 | unsafe extern "custom" fn bar() {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
help: convert this to an `#[unsafe(naked)]` function
   |
10 + #[unsafe(naked)]
11 | unsafe extern "custom" fn bar() {
   |
```

An `extern "custom"` function definition must be unsafe. The intent here is that a safety comment is written on how this function may be called. 

```
error: functions with the "custom" ABI must be unsafe
  --> <source>:10:1
   |
10 | extern "custom" fn bar() {
   | ^^^^^^^^^^^^^^^^^^^^^^^^
   |
help: add the `unsafe` keyword to this definition
   |
10 | unsafe extern "custom" fn bar() {
   | ++++++
```

In an `extern "custom"` block, functions cannot be marked as `safe`:

```
error: foreign functions with the "custom" ABI cannot be safe
  --> <source>:16:5
   |
16 |     safe fn foobar();
   |     ^^^^^^^^^^^^^^^^^
   |
help: remove the `safe` keyword from this definition
   |
16 -     safe fn foobar();
16 +     fn foobar();
```

An `extern "custom"` function cannot have any arguments or a return type:

```
error: invalid signature for `extern "custom"` function
 --> <source>:6:31
  |
6 | unsafe extern "custom" fn foo(a: i32) -> i32 {
  |                               ^^^^^^     ^^^
  |
  = note: functions with the "custom" ABI cannot have any parameters or return type
help: remove the parameters and return type
  |
6 - unsafe extern "custom" fn foo(a: i32) -> i32 {
6 + unsafe extern "custom" fn foo() {
  |
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

See https://github.com/rust-lang/reference/pull/2300.

## Drawbacks
[drawbacks]: #drawbacks

No specific drawback.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

https://github.com/rust-lang/rust/issues/140566

The name has already been debated. The name `unknown` has been mentioned, but to write the implementation you really do need to know the ABI. It is custom in the sense that rustc does not know about it, but the author definitely does.

Arguments and return types are forbidden because they are not consumed in any way. The compiler has no way of validating them against the function body, similar to other naked functions, but the functions can never be invoked directly so they serve no purpose when calling. The intent is that correct parameter passing and returning will be covered in documentation.

## Prior art
=======
[prior-art]: #prior-art

https://github.com/rust-lang/rust/issues/140566#issuecomment-2846205457

We do have some other calling conventions that cannot be called using rust's function calling syntax:

- extern "*-interrupt`
- extern "gpu-kernel"

Those however do have a known ABI, it's just that semantically it does not make sense to call them from a rust program.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

## Future possibilities
[future-possibilities]: #future-possibilities

None currently.
