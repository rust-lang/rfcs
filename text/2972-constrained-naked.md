- Feature Name: `constrained_naked`
- Start Date: 2020-08-06
- RFC PR: [rust-lang/rfcs#2972](https://github.com/rust-lang/rfcs/pull/2972)
- Rust Issue: [rust-lang/rust#90957](https://github.com/rust-lang/rust/issues/90957)

# Summary
This document attempts to increase the utility of [naked functions](https://github.com/rust-lang/rfcs/blob/master/text/1201-naked-fns.md) by constraining their use and increasing their defined invariants.

# Motivation

Naked functions have long been a feature of compilers. These functions are typically defined as normal functions in every regard, except that the compiler does not emit the function prologue and epilogue. Rust's early attempt to support this feature ([RFC 1201](https://github.com/rust-lang/rfcs/blob/master/text/1201-naked-fns.md)) mostly copied the existing compiler behaviors.

However, naked functions are often avoided in practice because their behavior is not well defined. The root cause of this problem is that naked functions are defined by negation: they are functions which lack a prologue and epilogue. Unfortunately, functions that lack a prologue and epilogue present a number of complicated problems that the compiler needs to solve and developers need to work around. And there is a long history of compilers and developers getting this wrong.

This document seeks to define naked functions in a much more constrained, positivistic way. In doing so, naked functions can become more useful.

# Naked function definition

A naked function has a defined calling convention and a body which contains only assembly code which can rely upon the defined calling convention.

A naked function is identified by the `#[naked]` attribute and:
1. should specify a calling convention besides `extern "Rust"`.
1. should define only FFI-safe arguments and return types.
1. must not specify the `#[inline]` or `#[inline(*)]` attribute.
1. must have a body which contains only a single `asm!()` statement which:
    1. may be wrapped in an `unsafe` block.
    1. must not contain any operands except `const` or `sym`.
    1. must contain the `noreturn` option.
    1. must not contain any other options except `att_syntax`.
    1. must ensure that the calling convention is followed or the function is `unsafe`.

In exchange for the above constraints, the compiler commits to:
1. produce a clear error if any of the above requirements are violated.
1. produce a clear warning if any of the above suggestions are not heeded.
1. disable the unused argument lint for the function (implicit `#[allow(unused_variables)]`).
1. never inline the function (implicit `#[inline(never)]`).
1. emit no additional instructions to the function body before the `asm!()` statement.

As a (weaker) corollary to the last compiler commitment, the initial state of all registers in the `asm!()` statement conform to the specified calling convention.

# Explanation

Since a naked function has no prologue, any naive attempt to use the stack can produce invalid code. This certainly includes local variables. But this can also include attempts to reference function arguments which may be placed on the stack. This is why a naked function may only contain a single `asm!()` statement.

Further, since many platforms store the return address on the stack, it is the responsibility of the `asm!()` statement to return in the appropriate way. This is why the `options(noreturn)` option is required.

Any attempt to use function arguments, even as operands, may cause stack access or modification. Likewise, any register operands may cause the compiler to attempt to preserve registers on the stack. Since the function has no prologue, this is problematic. To avoid this problem, we simply refuse to allow the use of any function arguments in Rust.

If this were the end of the story, naked functions would not be very useful. In order to re-enable access to the function arguments, the compiler ensures that the initial state of the registers in the `asm!()` statement conform to the function's calling convention. This allows hand-crafted assembly access to the function arguments through the calling convention. Since the `extern "Rust"` calling convention is undefined, its use is discouraged and an alternative, well-defined calling convention should be specified. Likewise, since the `asm!()` statement can access the function arguments through the calling convention, the arguments themselves should be FFI safe to ensure that they can be reliably accessed from assembly.

Because naked functions depend upon the calling convention so heavily, inlining of these functions would make code generation extremely difficult. Therefore, we disallow inlining.

Since the `const` and `sym` operands modify neither the stack nor the registers, their use is permitted.

## Examples

This function adds three to a number and returns the result:

```rust
const THREE: usize = 3;

#[naked]
pub extern "sysv64" fn add_n(number: usize) -> usize {
    unsafe {
        asm!(
            "add rdi, {}"
            "mov rax, rdi"
            "ret",
            const THREE,
            options(noreturn)
        );
    }
}
```

The calling convention is defined as `extern "sysv64"`, therefore we know that the input is in the `rdi` register and the return value is in the `rax` register. The `asm!()` statement contains `options(noreturn)` and therefore we handle the return directly through the `ret` instruction. We can provide a `const` operand since it modifies neither registers nor stack. Since we have strong guarantees about the state of the registers, we can mark this function as safe and wrap the `asm!()` statement in an `unsafe` block.

# Drawbacks

Implementing this will break compatibility of existing uses of the nightly `#[naked]` attribute. All of these uses likely depend on undefined behavior. If this is a problem, we could simply use a different attribute.

This definition may be overly strict. There is certainly some code that would work without this. The counter argument is that this code relies on undefined behavior and is probably not worth preserving. It might also be possible to reasonably ease the constraints over time.

This proposal requires the use of assembly where it is theoretically possible to use Rust in a naked function. However, practically the use of Rust in naked functions is nearly impossible and relies on extensively undefined behavior.

Adopting this definition changes the invariants of `asm!()`. Currently, all registers not supplied as operands to `asm!()` contain undefined values. This proposal changes this to define that the initial register state is unmodified from the function call. This is an even stronger commitment than merely guaranteeing calling convention conformance, which some may dislike. However, the change to permit defined initial register state applies **only** to the use of `asm!()` as the body of a naked function.

Refusing to allow argument operands means that architectures that have multiple calling conventions (i.e. x86_64 SystemV vs Windows) cannot share function bodies. This could be remedied with a future improvement.

# Alternatives

## Do nothing

We could do nothing and let naked functions work as they currently do. However, this is likely to be a source of a long stream of difficult compiler bugs and therefore there is no clear path to stabilization. Further, because of the lack of clear constraints, naked functions today are hard to use correctly. And when the developer fails to get every detail right, the result can be hard to debug.

## Remove naked functions

Another possibility is to simply remove support for naked functions altogether. This does solve the undefined behavior problem. But it forces the developer to pursue other options. Most notably `global_asm!()` or using an external assembler.

It would be possible to use `global_asm!()` to define functions with existing constraints. However, there is not currently a clear path to stabilization for `global_asm!()` since it is a thin wrapper around LLVM functionality. Further, `global_asm!()` does not provide features like namespacing and documentation. Nor can you use `global_asm!()` to provide `const` or `sym` operands, which are very useful.

Alternatively, developers could use an external assembler and link in the result. This approach is similar to `global_asm!()` but offloads the problem to external tooling such as the `cc` crate. It has the same drawbacks as `global_asm!()` and also puts additional requirements on compilers of the software.

# Prior art

All languages represented here follow the weaker definition of naked functions:

|                   | supported |
|-------------------|-----------|
|                   |           |
| C/C\+\+ \(GCC\)   | x         |
| C/C\+\+ \(Clang\) | x         |
| C/C\+\+ \(MSVC\)  | x         |
| C/C\+\+ \(ICC\)   |           |
| D                 | x         |
| Go                |           |
| Nim               | x         |
| Rust              | x         |
| Zig               | x         |

# Unresolved questions

All outstanding questions have been resolved.

# Future possibilities

It would be possible to define new calling conventions that can be used with naked functions.

A previous version of this document defined an `extern "custom"` calling convention. It was observed in conversation that calling conventions are really a *type* and that it could be useful to have calling conventions as part of the type system. In the interest of moving forward with constrained naked functions, it is best to limit the scope of this RFC and defer this (very good) conversation to a future RFC. As a simple workaround, naked functions which do not conform to their specified calling convention should be marked as unsafe and the caller requirements should be documented in the safety section of the documentation per standard convention.

It may also be possible to loosen the definition of a naked function in a future RFC. For example, it might be possible to allow the use of some additional, possibly new, operands to the `asm!()` block.
