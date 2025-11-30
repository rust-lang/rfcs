- Feature Name:  `cmse_nonsecure_entry` and `abi_cmse_nonsecure_call`
- Start Date: 2025-11-24
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3884)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Support for the `cmse-nonsecure-entry` and `cmse-nonsecure-call` calling conventions on   Armv8-M (`thumbv8m*`) targets, and a lint preventing (partially) uninitialized values from crossing the security boundary.

The implementation is tracked in:

- https://github.com/rust-lang/rust/issues/75835
- https://github.com/rust-lang/rust/issues/81391
- https://github.com/rust-lang/rust/pull/147697

# Motivation
[motivation]: #motivation

Rust and Trustzone form an excellent pairing for developing embedded projects that are secure and robust.

Trustzone creates a security boundary between a secure and non-secure application. The secure application can work with secure information (e.g. encryption keys). By limiting the interactions between the secure and non-secure applications, large classes of security bugs are statically prevented.

In embedded systems it is common to have an extra physical chip, a secure enclave, to handle secure information. With Trustzone, this additional chip is not needed: a secure enclave is simulated on the main chip instead.

The secure and non-secure applications communicate over an FFI boundary: the two applications run on the same chip and use the same address space, but are not linked together.  The cmse calling conventions are used to cross this FFI boundary, and apply restrictions on how it can be crossed.

Functions that use these ABIs must be reviewed carefully because they mark where secure data might be leaked. This is analogous to `unsafe` limiting  where UB might be introduced in a program. The calling conventions automatically handle the clearing of registers before the secure boundary is crossed, so that a malicious non-secure application cannot read lingering secure data.

Without compiler support it is much harder to know where to focus review effort, and every call that crosses the secure boundary requires inline assembly, which is inconvenient and error-prone.

A specific use case is encapsulating C APIs. Providing a C interface is still the standard way for a hardware vendor to provide access to system components. Libraries for networking (LTE, Bluetooth) are notorious for their bugs. Running such code in non-secure mode significantly reduces the risk of bugs leaking secure information.

Trustzone is growing in availability and use. More and more of the new medium and large ARM microcontrollers have support. Large industry players have requested Rust support for Trustzone.
# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The cmse calling conventions are part of the *Cortex-M Security Extension* that are available on Armv8-M systems (the relevant targets start with `thumbv8m`). They are used together with Trustzone (hardware isolation) to create more secure embedded applications.

The main idea of Trustzone  is to split an embedded application into two executables. The secure executable has access to secrets (e.g. encryption keys), and must be careful not to leak those secrets. The non-secure executable cannot access these secrets or any memory that is marked as secure: the system will raise a SecureFault when a program dereferences a pointer to memory that it does not have access to. In this way a whole class of security issues is prevented in the non-secure app.

The cmse calling conventions facilitate interactions between the secure and non-secure executables. To ensure that secrets do not leak, these calling conventions impose some custom restrictions on top of the system's standard AAPCS calling convention.

The `cmse-nonsecure-entry` calling convention is used in the secure executable to define entry points that the non-secure executable can call. The use of this calling convention hooks into the tooling (LLVM and the linker) to  generate a shim (the arm terminology is *veneer*) that switches the security mode, and an import library (an object file with only declarations, pointing to the addresses of the shims, not actual instructions) that can be linked into the non-secure executable.

The `cmse-nonsecure-call` calling convention is used in the other direction, when the secure executable wants to call into the non-secure executable. This calling convention can only occur on function pointers, not on definitions or extern blocks. The secure executable can acquire a non-secure function pointer via shared memory, or a non-secure callback can be passed to an entry function.

Both calling conventions are based on the platform's C calling convention, but will not use the stack to pass arguments or the return value. In practice that means that the arguments must fit in the 4 available argument registers, and the return value must fit in a single 32-bit register, or be abi-compatible with a 64-bit integer or float. The compiler checks that the signature is valid.
# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Arm defines the toolchain requirements in  [ARMv8-M Security Extensions: Requirements on Development Tools - Engineering Specification](https://developer.arm.com/documentation/ecm0359818/latest/), but of course this specification needs to be interpreted in a Rust context.
## ABI Details

The `cmse-nonsecure-call` and `cmse-nonsecure-entry` ABIs are only accepted on Armv8-M targets (currently `thumbv8m.base-*-eabi`, `thumbv8m.main-*-eabi{,hf}`). On all other targets their use emits an invalid ABI error.

The foundation of the cmse ABIs is the platform's standard AAPCS calling convention. On `thumbv8m` targets `extern "aapcs"` is the default C ABI and equivalent to `extern "C"`.

The `cmse-nonsecure-call` ABI can only be used on function pointers. Using it in for a function definition or extern block emits an error. It is invalid to cast to or from `extern "aapcs"`.

The `cmse-nonsecure-entry` ABI is allowed on function definitions, extern blocks and function pointers. It is sound and valid (in some cases even encouraged) to cast such a function to `extern "aapcs"`. Calling the function is valid and will behave as expected in both the secure and non-secure applications. Casting from `extern "aapcs"` to `extern "C"` is invalid.

### Argument passing

The main technical limitation of the cmse ABIs versus plain AAPCS is that the cmse ABIs cannot use the stack for passing function arguments or return values. That leaves only the 4 standard registers to pass arguments, and only supports 1 register worth of return value, unless the return type is ABI-compatible with a 64-bit scalar, which is supported.

```rust
// Valid
type T0 = extern "cmse-nonsecure-call" fn(_: i32, _: i32, _: i32, _: i32) -> i32;
type T1 = extern "cmse-nonsecure-call" fn(_: i64, _: i64) -> i64;

#[repr(transparent)] struct U64(u64);
type T2 = extern "cmse-nonsecure-call" fn() -> U64;

// Invalid: too many argument registers used
type T3 = extern "cmse-nonsecure-call" fn(_: i64, _: u8, _: u8, _: u8) -> i64;

// Invalid: return type too large
type T4 = extern "cmse-nonsecure-call" fn() -> i128;

// Invalid: return type does not fit in one register, and is not abi-compatible with a 64-bit scalar
#[repr(C)] struct WrappedI64(i64);
type T5 = extern "cmse-nonsecure-call" fn(_: i64, _: i64) -> WrappedI64;
```

An error is emitted when the program contains a signature that violates the calling convention's constraints:

```
error[E0798]: arguments for `"cmse-nonsecure-entry"` function too large to pass via registers
  --> $DIR/params-via-stack.rs:15:76
   |
LL | pub extern "cmse-nonsecure-entry" fn f1(_: u32, _: u32, _: u32, _: u32, _: u32, _: u32) {}
   |                                                                            ^^^^^^^^^^^ these arguments don't fit in the available registers
   |
   = note: functions with the `"cmse-nonsecure-entry"` ABI must pass all their arguments via the 4 32-bit available argument registers
```

The error is generated after type checking but before monomorphization, meaning that even a `cargo check` will emit these errors, and the errors are emitted even for unused functions. Note that LLVM will also check the ABI constraints, but it generates poor error messages late in the compilation process.

Because Rust is not C, we impose a couple additional restrictions, based on how these ABIs are (meant to be) used.

### No Generics

No generics are allowed. That includes both standard generics, const generics, and any `impl Trait` in argument or return position. By extension, `async` cannot be used in combination with the cmse ABIs.

```
error[E0798]: functions with the `"cmse-nonsecure-entry"` ABI cannot contain generics in their type
  --> $DIR/generics.rs:69:1
   |
LL | extern "cmse-nonsecure-entry" fn return_impl_trait(_: impl Copy) -> impl Copy {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

The `cmse-nonsecure-call` calling convention can only be used on function pointers, which already disallows generics. For `cmse-nonsecure-entry`,  it is standard to add a `#[no_mangle]` or similar attribute, which also disallows generics. Explicitly disallowing generics enables the pre-monomorphization layout calculation that is required for good error messages for signatures that use too many registers.
### No C-variadics (currently)

Currently both ABIs disallow the use of c-variadics. For `cmse-nonsecure-entry`, the toolchain actually does not support c-variadic signatures (likely because of how they interact with shim that switches to secure mode, though the specification does not say that explicitly).

- clang rejects c-variadic entry functions: https://godbolt.org/z/MaPjzGcE1
- but accepts c-variadic nonsecure calls: https://godbolt.org/z/5rdK58ar4

For `cmse-nonsecure-call`, we may support and stabilize c-variadics at some point in the future.

## No tail calls

Neither cmse ABI can tail call another function, per the LLVM source:

> For both the non-secure calls and the returns from a CMSE entry function, the function needs to do some extra work after the call, or before the return, respectively, thus it cannot end with a tail call

The unstable implementation of guaranteed tail calls in rust requires the caller and callee to have the same ABI. That means that calls to `cmse-nonsecure-call` are never eligible for a tail call (there are no definitions with this ABI). For tail calls to a `cmse-nonsecure-entry` function we emit an explicit error.

Functions with the `extern "cmse-nonsecure-entry"` ABI may themselves be tail-called, though this is only possible when the function is first cast to `extern "C"` to satisfy the restriction that caller and callee have the same ABI.
### Support for `const fn`

No special support for calling cmse functions is needed.

Evaluating entry functions during constant evaluation is valid. The context switch from non-secure to secure mode is handled by the shim that switches to secure mode, which is not visible to rust code. Clearing of registers is not relevant for constant evaluation.

The `cmse-nonsecure-call` calling convention can only be used on function pointers, which cannot be evaluated during constant evaluation.

Miri is not a register machine, so the clearing of registers is not relevant. The context switching also does not need to be considered, because a Miri input program cannot use FFI and therefore cannot cross the secure boundary. Any attempt to do so would rely on a transmute or similar and would for that reason be unsound.
### Warn on partially uninitialized values crossing the secure boundary

Unions and types with padding or niches can contain uninitialized memory, and this uninitialized memory can contain stale secure information. Clang warns when union values cross the security boundary (see https://godbolt.org/z/vq9xnrnEs), and rust does the same.

```
warning: passing a (partially) uninitialized value across the secure boundary may leak information
  --> $DIR/params-via-stack.rs:43:41
   |
LL |     f4: extern "cmse-nonsecure-call" fn(MaybeUninit<u64>),
   |                                         ^^^^^^^^^^^^^^^^
   |
   = note: the bytes not used by the current variant may contain stale secure data
```

Like clang, the lint is emitted at the use site. That means that in the case where passing such a value is deliberate, each use site can be annotated with `#[allow(cmse_uninitialized_leak)]`. In most cases this lint should be considered an error, and an alternative way of returning/passing the value should be found that does not run the risk of leaking secure information.

Unlike clang, the rust lint also warns on other instances where a value may be (partially) uninitialized based on its type. For instance, clang does not warn when a struct crossing the secure boundary contains padding (e.g. https://godbolt.org/z/rcM65YG1s).

The lint is implemented in https://github.com/rust-lang/rust/pull/147697, and checks whether transmuting a type `T` to `[u8; size_of::<T>]` is valid. The transmute is only valid when all bytes of `T` are guaranteed to be initialized.

```rust
#[repr(C)]
struct PaddedStruct {
    a: u8,
	// There is a byte of padding here.
    b: u16,
}

#[no_mangle]
extern "cmse-nonsecure-entry" fn padded_struct() -> PaddedStruct {
    PaddedStruct { a: 0, b: 1 }
    //~^ WARN passing a (partially) uninitialized value across the security boundary may leak information
}
```

A `cmse-nonsecure-call` function call will emit a warning when any of its arguments has a partially uninitialized type, and a `cmse-nonsecure-entry` function warns at any (implicit) return when the return type may be partially uninitialized.

Ultimately guaranteeing the security properties of the system is up to the programmer, but warning on types with potentially uninitialized memory is a helpful signal that the compiler can provide.
## Background

Additional background on what these calling conventions do, and how they are meant to be used. This information is not strictly required to understand the RFC, but has informed the design and may explain certain design choices.

### The `extern "cmse-nonsecure-entry`" CC

Functions that use the `cmse-nonsecure-entry` calling convention are called *entry functions*.

An entry function has two ELF symbols labeling it:

- the standard rust symbol name
- A special symbol that prefixes the standard name with `__acle_se_`

The presence of the prefixed name is used by the linker to generate a *secure gateway veneer*: a shim that uses the *secure gate* (`sg`) instruction to switch security modes and then branches to the real definition. The non-secure executable must call this shim, not the real definition. Calling the read definition from the non-secure executable would cause a SecureFault.

It is customary for entry functions to use `no_mangle`, `export_name` or similar so that the symbol is not mangled. The use of the `cmse-nonsecure-entry` calling convention makes LLVM emit the additional prefixed symbol. For instance this function:

```rust
#[unsafe(no_mangle)]
pub extern "cmse-nonsecure-entry" fn encrypt_the_data(/* ... */) {
	/* ... */
}
```

Will generate a symbol with the two labels like so:

```asm
	.globl	__acle_se_encrypt_the_data
	.type	__acle_se_encrypt_the_data,%function
__acle_se_encrypt_the_data:
encrypt_the_data:
```

The `arm-none-eabi-ld` linker will generate so-called veneers for function symbols that start with `__acle_se_` if requested via linker flags:

```toml
  "-C", "linker=arm-none-eabi-ld",

  # Output secure veneer library
  "-C", "link-arg=--cmse-implib",
  "-C", "link-arg=--out-implib=target/veneers.o",
```

The link step adds an additional `.gnu.sgstubs` section to the binary, which contains a veneer (or *shim* in rust terminology) that first calls the `sg` instruction, switching to the secure state. It then branches to the actual function it veneers:

```
Disassembly of section .gnu.sgstubs:

100025e0 <encrypt_the_data>:
100025e0: e97f e97f    	sg
10008844: f7f8 bb79    	b.w	0x10000f3a <__acle_se_encrypt_the_data> @ imm = #-0x790e
```

Before returning, entry functions clear all unused registers (to make sure secrets don't linger there). The return instruction switches back to the caller's security state based on the return address.

Additionally the linker produces a `veneers.o` file, which can be linked into the non-secure application. This `veneers.o` just contains the unprefixed symbols but maps them to their veneer addresses.  Like an import library, `veneers.o` does not contain any instructions, in fact it does not even have a `.text` section.

```
> arm-none-eabi-objdump -td target/veneers.o

target/veneers.o:     file format elf32-littlearm

SYMBOL TABLE:
100025e0 g     F *ABS*	00000008 encrypt_the_data
```

The non-secure executable can use this import library to link the entry functions (or really their veneers, but using the name of the underlying function):

```rust
unsafe extern "cmse-nonsecure-entry" {
	safe fn encrypt_the_data(/* ... */);
}
```

This works because the secure and non-secure applications share their address space, they just each use different chunks of that address space.
###  The `extern "cmse-nonsecure-call`" CC

The `cmse-nonsecure-call` calling convention is used for *non-secure function calls*: function calls that switch from secure to non-secure mode. Because secure and non-secure code are separated into different executables, the only way to perform a non-secure function call is via function pointers. Hence, the `cmse-function-call` calling convention is only allowed on function pointers, not in function definitions or `extern` blocks.

To ensure that the non-secure executable cannot read any lingering secret values from those registers, a call to a `cmse-nonsecure-call` function will clear all registers except those used to pass arguments.

A *non-secure function pointer*, i.e. a function pointer using the `cmse-nonsecure-call` calling convention, has its least significant bit (LSB) unset. Checking for whether this bit is set provides a way to test at runtime which security state is targeted by the function.

The secure executable can get its hands on a non-secure function pointer in two ways: the function address can be an argument to an entry function, or it can be in memory at a statically-known address.

# Drawbacks
[drawbacks]: #drawbacks

The usual reasons: this is a niche feature (although it is requested by large industry players) with a fair amount of complexity that must be maintained. However to be fair, these calling conventions have been in the compiler for around 5 years and so far the maintenance burden has been acceptable.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The straightforward alternative is to have users emulate these calling conventions. Not having any compiler support is fairly fragile: all function calls that cross the boundary must use a special calling instruction, and great care must be taken that the signature really does not use the stack for argument passing.

For users, these calling conventions should not come up unless someone seeks them out. Interactions with other language features are similarly only relevant to this niche group of users.

For a true ergonomic experience more work is needed, but we believe this can all be done in the package ecosystem.
# Prior art
[prior-art]: #prior-art

Clang and GCC support CMSE using the `__attribute__((cmse_nonsecure_entry))` and `__attribute__((cmse_nonsecure_call))` attributes. As mentioned the ABI restrictions are checked, but only late in the compilation process.

The [`cortex_m`](https://docs.rs/cortex-m/latest/cortex_m/cmse/index.html) crate already provides some primitives for building cmse applications, e.g. to query whether a pointer points to secure or non-secure memory.

### Sources

- [ARMv8-M Security Extensions: Requirements on Development Tools - Engineering Specification](https://developer.arm.com/documentation/ecm0359818/latest/)
- https://tweedegolf.nl/en/blog/85/trustzone-trials-tribulations

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- is the lint relying on the (unstable) internals of safe transmute a problem. I believe this is fine because it's just a lint.

# Future possibilities
[future-possibilities]: #future-possibilities

## Lint on references crossing the secure boundary

The secure application should never accept a reference because there is no guarantee that a hostile non-secure application does not provide an invalid value (`NULL`, not properly aligned, etc.). There are other types with layout assumptions (e.g. `NonZeroU64` and friends) that are almost certainly invalid for a secure application to accept.

We'd like to wait with adding further lints until we see more usage of Trustzone, so that we can design a lint that covers all practical cases of too-strong assumptions.
## An "initialize padding" attribute

The current lint for partially uninitialized values crossing the security boundary does not have a proper workaround: the advice is to just not send such values over the secure boundary, and essentially treat the warning as an error.

A suggestion that was floated is to provide some mechanism to ensure that a value is fully initialized, e.g. by zeroing any potentially uninitialized parts.

One potential method is to extend the`repr` attribute with an option that adds fields where padding is needed internally. These user-hidden padding fields would be zeroed upon creation.
```rust
#[repr(C, align(8), initialized)]
struct Foo {
	a: u8,
	// implicit _padding0: [u8; 1],
	b: u16,
	// implicit _padding1: [u8; 4],
}
```

This feature still has many open design questions. We don't think such an attribute is required for practical Trustzone development, so we defer it for now.
