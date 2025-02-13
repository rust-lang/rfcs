- Feature Name: `target_modifiers`
- Start Date: 2024-10-24
- RFC PR: [rust-lang/rfcs#3716](https://github.com/rust-lang/rfcs/pull/3716)
- Rust Issue: None

# Summary
[summary]: #summary

* We introduce the concept of "target modifier". A target modifier is a flag
  where it may be unsound if you link together two compilation units that
  disagree on the flag.
* We fail the build if rustc can see two Rust compilation units that do not
  agree on the exact set of target modifier flags.
* There are already several existing flags that could fall into this category.
  There are also hypothetical new flags that do.
* The error can be silenced using the `-Cunsafe-allow-abi-mismatch` escape
  hatch.
* Not having a stable way to build stdlib crates does not block stabilization
  of target modifiers.
* As a future extension we may be able to relax the rules to allow some
  specific kinds of mismatches.
* This RFC does not stabilize any target modifiers. That should happen in
  follow-up MCPs/FCPs/RFCs/etc.

# Motivation
[motivation]: #motivation

As Rust expands into low-level domains, there will be a need for precise
control over how code is compiled. This often manifests as a new compiler flag.
Some of these flags trigger undefined behavior if used incorrectly, which is in
tension with Rust's safety goals. This RFC proposes a new mechanism to allow
use of such flags while also preventing undefined behavior.

The primary goal of this RFC is to unblock *stabilization* of target modifier
flags. Adding them as unstable (and unsound) flags is already happening today
without this RFC.

## The Linux Kernel

The Linux Kernel has run into a handful of cases where it is necessary to tweak
the ABI used in the kernel. Often, this is done conditionally depending on a
configuration option. A few examples:

* When using `CONFIG_SHADOW_CALLSTACK` the x18 register must be reserved in the
  ABI with `-Zfixed-x18`.
* The `-Ctarget-feature=-neon` flag is used to prevent use of floating points
  on arm.
* On 32-bit x86, `-Zreg-struct-return` and `-Zregparm=3` are used.
* When using `CONFIG_CFI_CLANG` the kCFI sanitizer is enabled with
  `-Zsanitizer=kcfi`. Unlike most other sanitizers, this sanitizer is used in
  production.
* In several different cases, `-Zpatchable-function-entry` is used to add nops
  before or after the function entrypoint. When mixed with `-Zsanitizer=kcfi`
  this causes special considerations as kcfi works by placing a tag before the
  function entrypoint.
* To support `MITIGATION_RETPOLINE` and `MITIGATION_SLS`, `target.json` is used
  on x86.

We expect there to be more examples in the future.

## Sanitizers

There is [an ongoing effort to stabilize some of the sanitizers][issue123615].
However, this effort explicitly aims to stabilize sanitizers that can be used
without rebuilding the stdlib. With this RFC, that is no longer a blocker as
the remaining sanitizers can be classified as a target modifier and then
stabilized.

[issue123615]: https://github.com/rust-lang/rust/issues/123615

## Embedded Targets

Currently, embedded platforms such as `thumb*` or `rv*` use separate targets
for configuration with significant ABI changes. For `thumb*` targets, this is
currently limited to Hard- vs Soft-Float which can cause issues when linked.
However for RISC-V targets, the F (32-bit hardware float), D (64-bit hardware
float), and Q (128-bit hardware float) extensions all can [potentially change
the ABI][riscv-float], which would increase the number of required targets. The
[E extension][riscv-e] may also change the ABI by limiting the number of
registers used by the I (integer operations) extension.

[riscv-float]: https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/master/riscv-cc.adoc#named-abis
[riscv-e]: https://github.com/riscv/riscv-isa-manual/blob/main/src/rv32e.adoc

## Existing -C flags that are unsound

It has recently been discovered that several existing `-C` flags modify the
ABI, making them unsound. Examples:

* [`-Csoft-float`](https://github.com/rust-lang/rust/issues/129893)
* [`-Ctarget-feature=-neon`](https://github.com/rust-lang/rust/issues/131058)
* [`-Clinker-plugin-lto`](https://github.com/rust-lang/rust/issues/127979)
* [`-Cllvm-args`](https://github.com/rust-lang/rust/issues/131800#issuecomment-2418595757)
* Possibly `-Ccode-model` and `-Crelocation-model`

This problem is a new discovery and it's still not clear how to solve it.
Target modifiers will not solve all of the flags; [some flags are just
unfixable and need to be removed][issue130968]. But I expect that target
modifiers will be the solution for some of these flags.

[issue130968]: https://github.com/rust-lang/rust/issues/130968

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The Rust compiler has many flags that affect how source code is turned into
machine code. Some flags can be turned on and off for each CU (compilation
unit) in your project separately, and other flags must be applied to the entire
application as a whole. The typical reason for flags to be in the latter
category is that change some aspect of the ABI. For example,
`-Zreg-struct-return` changes how to return a struct from a function call, and
both the caller and callee must agree on how to do that even if they are in
different CUs.

The Rust compiler will detect if you incorrectly use a flag that must be
applied to the application as a whole. For example, if you compile the standard
library with `-Zreg-struct-return`, but don't pass the flag when compiling a
dependency, then you will get the following error:
```
error: mixing -Zreg-struct-return will cause an ABI mismatch

help: This error occurs because the -Zreg-struct-return flag modifies the ABI, 
      and different crates in your project were compiled with inconsistent
      settings.
help: To resolve this, ensure that -Zreg-struct-return is set to the same value
      for all crates during compilation.
help: To ignore this error, recompile with the following flag:
      -Cunsafe-allow-abi-mismatch=reg-struct-return
```
As an escape hatch, you can use `-Cunsafe-allow-abi-mismatch=reg-struct-return`
to disable the error. Using this flag is unsafe as incorrect use of the ABI is
undefined behavior. However, there may be some cases where the check is too
strict, and you can use the flag to proceed in those cases.

The requirement that all CUs agree includes stdlib crates (core, alloc, std),
so using these flags usually requires that you compile your own standard
library with `-Zbuild-std` or by directly invoking `rustc`. That said, some
flags (e.g., `-Cpanic`) have mechanisms to avoid this requirement.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A compiler flag can be classified as a _target modifier_. When a flag is a
target modifier, it can be undefined behavior to link together two CUs that
disagree on the flag.

To avoid unsoundness from mixing target modifiers, rustc will store the set of
target modifiers in use in the crate metadata of each crate. Whenever rustc is
invoked, it will inspect the crate metadata of all crates that are visible to
it (usually the current crate and its direct dependencies) and emit an error if
any of them have mismatched target modifiers in use.

The `-Cunsafe-allow-abi-mismatch=flagname` flag can be used when compiling a
crate to indicate that it should not be included in the list of crates when
checking that all crates agree on `flagname`.

Note that `-Cunsafe-allow-abi-mismatch=flagname` should be passed to rustc when
compiling the crate that uses an incompatible value for `flagname`, which may
not be the same rustc invocation as the one where the mismatch is detected. For
example, if you build four CUs A,B,C,D where D depends on A,B,C and C is the
only one with a different value for `flagname`, then the mismatch is detected
by rustc when compiling D, but `-Cunsafe-allow-abi-mismatch` should be used
when compiling C.

## Stabilization

It is possible to stabilize target modifiers even if they cannot be utilized
without an unstable feature such as `-Zbuild-std`.

# Drawbacks
[drawbacks]: #drawbacks

## Teaching
[teaching]: #teaching

We should be careful to not introduce too many concepts that end-users have to
learn.

It is intentional that the guide-level section of this RFC does not use the
word "target modifier". The "target modifier" name is not intended to be used
outside of the compiler internals and very technical documentation. Compiler
errors should not say "error: trying to mix target modifiers" or something like
that; rather the error should just say that mixing `-Cfoo` may cause ABI
issues.

For similar reasons, the flag for silencing the error is called
`-Cunsafe-allow-abi-mismatch` with the word "ABI" to avoid having to teach the
user about mismatched flags or target modifiers.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not just add flags like normal?

Preventing undefined behaviour is an important goal of the Rust project. If we
add flags that change the abi, then that is in direct opposition to that goal,
as mixing them would lead to UB.

## Why not just add new targets?

The flag that started this entire discussion is `-Zfixed-x18`. This flag
changes the ABI by changing the x18 register from a caller-saved temporary
register to a reserved register. At the time, people suggested adding a new
target (e.g., `aarch64-unknown-none-fixed18`), instead of adding a dedicated
`-Zfixed-x18` flag.

The primary benefit of adding a new target is that it's a workaround for
`-Zbuild-std` being unstable. Each new target will get a prebuilt stdlib, which
sidesteps the need for building your own stdlib.

This RFC does not propose this solution because:

1. The primary benefit is not being blocked on stabilization of `-Zbuild-std`.
   However, I don't think we really are blocked on stabilization of
   `-Zbuild-std` in the first place. See [the stabilization of target modifier
   flags section][stabilization] below.
2. Target modifiers help with other problems such as unblocking the
   stabilization of sanitizers as well as existing `-C` flags that are unsound
   due to ABI issues. Adding new targets would leave these issues unsolved.
3. Adding new targets risks an exponential number of targets. In the kernel on
   x86 we would need 8 different targets to support the different possible
   kernel configurations. It's not hard to imagine that number growing to 16 or
   32 targets in the near future, especially once you consider that other
   embedded projects may have their own set of target modifier flags.
4. Adding new prebuilt stdlibs does not actually help the projects that need
   these flags. Even if a prebuilt stdlib is provided for every combination of
   ABI-affecting flags that the kernel may need, the kernel has other reasons
   that require building a custom `core`.

## Why not use `target.json`

Because the `target.json` feature is perma-unstable, and this RFC primarily
concerns itself with unblocking the _stabilization_ of these flags. Adding
target modifiers as unstable flags is already happening today. (However, if
this RFC gets accepted, it becomes a soundness bug to add such unstable flags
without wiring them up with the target modifier machinery.)

One possible alternative would be to stabilize a subset of `target.json`.
However, I don't think there's much benefit to this. It just means that you now
have to learn two different ways of passing flags to the compiler. See [the
Teaching section][teaching] above.

It would also be inconvenient to use in external build systems. Right now, the
kernel passes the `-Zfixed-x18` flag like this:
```make
ifeq ($(CONFIG_SHADOW_CALL_STACK), y)
KBUILD_CFLAGS    += -ffixed-x18
KBUILD_RUSTFLAGS += -Zfixed-x18
endif
```
If `-Zfixed-x18` had to be specified in a `target.json` file, it would need to
happen in an entirely different part of the kernel build system. It is better
to specify the rustc flag together with the clang/gcc flag.

## Stabilization of target modifiers
[stabilization]: #stabilization-of-target-modifiers

Using a target modifier without rebuilding the Rust stdlib is often not
possible. This means that some target modifiers can only be used in tandem with
`-Zbuild-std`, which is currently unstable.

However, there's no reason we _have_ to block the stabilization of target
modifiers on the stabilization of `-Zbuild-std`. If a target modifier `-Cfoo`
is stabilized, then you can break users of `-Cfoo` with the reason "we changed
the way you pass flags to `core`", but you can't break users with the reason
"we renamed `-Cfoo` to `-Cbar`; this is okay because you're also using
`-Zbuild-std` even though the rename is unrelated to `-Zbuild-std`".

## Not all mismatches are unsound
[not-all-unsound]: #not-all-mismatches-are-unsound

This RFC says that mismatching target modifiers in any way results in a build
error. However, there are a lot of cases where the real rules are more
complicated than that. For example, with the following three CUs:

* CU A compiled with `-Zfixed-x18 -Zsanitizer=shadow-call-stack`.
* CU B compiled with `-Zfixed-x18`.
* CU C compiled with neither flag.

It is unsound to link together CUs A and C, but linking A with B or B with C is
sound.

However, real-world scenarios where mismatching a target modifier is necessary
are quite uncommon. The only case I'm aware of is the runtime for a sanitizer.
For example, when ASAN (address sanitizer) detects a bug, it calls into a
special ASAN-failure-handler function. The function for handling ASAN-failures
should not be sanitized.

Making the compiler accept specific mismatches that are sound is out of scope
for this RFC. Such decisions will be made on a flag-by-flag basis in follow-up
decisions (most likely an MCP). Until then, end-users can use
`-Cunsafe-allow-abi-mismatch` to proceed in such cases.

## Cases that are not caught
[not-caught]: #cases-that-are-not-caught

This RFC proposes to store information in the crate metadata to detect ABI
mismatches. However, this means that there are two cases that could result in
mismatches being missed:

* When rustc is not doing the final link, different incompatible leaf modules
  might not get detected. For instance, using the CUs A,B,C from [the previous
  section][not-all-unsound], then if stdlib is CU B and there are two leaf CUs
  A and C, then the incompatibility between A and C would not get detected
  unless rustc performs the final link.
* With dynamic linking, you may have two shared objects compiled completely
  separately with incompatible ABIs.

Note that the first situation can never happen with the base proposal: if we
require exact matches, then all CUs must agree because all CUs depend on
`core`. The missed detection requires that we don't consider AB or BC
incompatible. This could be an argument in favor of not allowing any mismatches
with the shadow call stack sanitizer (which you never want to mix in practice
anyway).

The dynamic linking case is considered acceptable. Detecting it is out of scope
of this RFC.

## Name mangling

It has been proposed that the modified target could be encoded in the name
mangling scheme to help catch the two cases from [the previous
section][not-caught]. However, this raises a bunch of open questions:

1. It probably does not help catch the first case. Dynamic function calls
   between the leaf modules wouldn't get caught, so that would require that one
   of the leaf modules references a symbol defined by the other leaf module.
   However, I find it hard to imagine this happening in the real world unless
   the symbol is marked `#[no_mangle]`.
2. Similarly, if two dynamic objects are compiled completely separately, they
   probably do not reference each other through anything other than symbols
   marked `#[no_mangle]`. While it could potentially identify a mismatch where
   component A depends on component B, and component B is recompiled with a
   different ABI while using the old version of A, this scenario is not
   well-supported to start with because of Rust's unstable ABI.
3. Some ABI-affecting flags only change the C ABI, but those symbols are
   usually using `#[no_mangle]`.
4. Do we really want to make our symbol names even longer?

For the above reasons, name mangling is not proposed as a mechanism for
detection for now. However, it could be a potential future addition.

## Policy around flags that might not be ABI affecting

Some flags have an unclear status where it is unclear whether it affects the
ABI. For example, `-Zpatchable-function-entry` (which adds nop instructions
before/after the function entrypoint) generally isn't considered to affect the
ABI, but if combined with `-Zsanitizer=kcfi` then it does affect it since kcfi
works by placing a hash of the function signature before the function
entrypoint. Since `-Zsanitizer=kcfi` already needs to be a target modifier
_anyway_, you could argue that `-Zpatchable-function-entry` doesn't need to be
one.

However, for these cases where we are uncertain, we take a conservative
approach and mark them as a target modifier. It is not breaking to relax the
rules in future releases.

As for flags such as `-Cllvm-args` that can do basically anything, it may make
more sense to just rename it to `-Cunsafe-llvm-args` rather than use the target
modifier functionality.

## Problems with mixing non-target-modifiers

I discussed this proposal with people from other communities (mainly kernel and
C folks), and they shared several other cases where mixing flags are a problem.
They pointed out that there are some flags where mixing them is really bad and
should be detected, but which are not ABI issues or unsound per se. The most
common example of this is exploit mitigations, where mixing the flags will
silently lead to a vulnerable binary. On the other hand, ABI mismatches usually
fail in a loud way, so they were not as concerned about those.

The sections below describe several such cases. They are intended to provide
additional context for the reader to better understand the problem space. We
will likely want to use the same infrastructure for detecting some of the
mismatches mentioned below, but the precise list is out of scope of this RFC.

Since the cases below are not unsound, the flag for overriding them should not
include the word "unsafe".

### Exploit mitigations

There are some mitigations that are used to mitigate CPU speculation
vulnerabilities (e.g., SPECTRE) or used to make exploitation of vulnerabilities
harder (e.g., control flow protection), which work by either telling the
compiler to generate code that include instructions to prevent CPU speculation
in some specific locations, or telling the compiler to generate code that
checks that destinations of indirect branches are one of their valid
destinations in the control flow graph. These mitigations usually don't change
the ABI, as they just change how code is generated within functions.

The problem is that the attacks you are trying to mitigate involve either
forcing CPU speculation in some specific locations or changing the control flow
to an arbitrary attacker-controlled address. If you have an unprotected
specific location or unprotected indirect branch anywhere in your program, an
attacker may still be able to use it either forcing CPU speculation in these
unprotected specific locations, or by changing addresses in memory used by
these unprotected indirect branches. This means that if you only apply the
mitigation to some CUs, then the CUs that lack the mitigation will be
completely unprotected, and the mitigation might be bypassable.

### Sanitizers

This case is rather similar to exploit mitigations.

Some sanitizers can be mixed and matched between CUs without breaking the ABI.
For example, on the Android aarch64 target, the shadow call stack sanitizer
does not change the ABI, and can be freely mixed between CUs. However, the
sanitizer does not catch violations in CUs that don't enable the sanitizer.

For sanitizers used in production (such as shadow call stack or kcfi) this is
particularly problematic, as a vulnerability in sanitized code may allow you to
jump into unsanitized code.

### .note.gnu.property

In the case of BTI (`-Zbranch-protection=bti`), the mitigation relies on the
kernel's ELF loader setting a special bit in the page table. However, setting
this bit is only valid if BTI is enabled everywhere. The compiler will use a
section called `.note.gnu.property` to tell the linker whether BTI is enabled,
and the linker only propagates `.note.gnu.property` if all CUs agree on it.
This means that if one CU is missing BTI, the linker will disable it for the
entire executable, and the kernel's ELF loader will not set the bit in the page
tables when loading the machine code, rendering BTI ineffective.

### Performance

Another reason is performance. One some targets, the precompiled stdlib always
comes with panic landing pads, even if you're using `-Cpanic=abort`. It's also
usually compiled with a very minimal set of target features for greater
compatibility. These discrepancies can have an unacceptable impact on
performance.

### Code patching

You might use `-Zbranch-protection=pac-ret` or `-Zpatchable-function-entry` to
insert special instructions at the beginning/end of all functions so you can
use runtime code-patching to replace them later. It is only because of the
runtime code-patching logic that these flags need to be used everywhere.

### Debugging information

Mixing CUs with different options for `-Cforce-unwind-tables`,
`-Zdwarf-version`, or `-Zdebuginfo-compression` may result in a binary that you
consider to be invalid as you may be unable to read the debugging information.
But it would not be an ABI issue.

# Prior art
[prior-art]: #prior-art

## The panic strategy

The Rust compiler already *has* infrastructure to detect flag mismatches: the
flags `-Cpanic` and `-Zpanic-in-drop`. The prebuilt stdlib comes with different
pieces depending on which strategy is used, although panic landing flags are
not entirely removed when using `-Cpanic=abort`, as only part of the prebuilt
stdlib is switched out.

## Global target modifiers

A suggestion that has come up several times
([1](https://github.com/rust-lang/rust/issues/116972),
[2](https://github.com/rust-lang/rust/issues/116973),
[3](https://github.com/rust-lang/rust/issues/121970#issuecomment-1978605782))
is to have a variation of `-Ctarget-feature=` that must be applied globally,
which could be called `-Cglobal-target-features=`. This is very similar to this
RFC, though it is broader as the "target modifier" concept can apply to any
compiler flag and not just to a single `-Cglobal-target-features=` flag.

## Stabilization of things that require nightly features

This RFC proposes that we shouldn't block stabilization of target modifiers on
a stable way to build libcore. There is precedent in the rust project for
unblocking stabilizations in this manner: When `#![no_std]` was stabilized,
[the RFC][rfc1184] said the following:

> As mentioned above, there are three separate lang items which are required by
> the libcore library to link correctly. These items are:
> 
> * `panic_fmt`
> * `stack_exhausted`
> * `eh_personality`
> 
> This RFC does not attempt to stabilize these lang items for a number of
> reasons:
> 
> * The exact set of these lang items is somewhat nebulous and may change over
>   time.
> * The signatures of each of these lang items can either be platform-specific
>   or it’s just “too weird” to stabilize.
> * These items are pretty obscure and it’s not very widely known what they do
>   or how they should be implemented.
> 
> Stabilization of these lang items (in any form) will be considered in a
> future RFC.

This means that no-std can't actually be used without providing these symbols
in some other way. Doing so is unstable.

[rfc1184]: https://rust-lang.github.io/rfcs/1184-stabilize-no_std.html

## .note.gnu.property

The `.note.gnu.property` section discussed previously is an example of C code
detecting mismatches of a flag at link time.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

This RFC does not stabilize any target modifiers. Such decisions should be made
as a follow-up to this RFC on a flag-by-flag basis using the usual process for
stabilizing a compiler flag.

The `-Cunsafe-allow-abi-mismatch` flag will be stabilized when the first target
modifier is stabilized.

# Future possibilities
[future-possibilities]: #future-possibilities

A possible future extension could be to detect inconsistencies between the ABI
of C code and Rust code. This would be an interesting extension, but it is not
critical for target modifiers as calling into C is inherently unsafe to start
with. Similarly, another possible future extension could be to catch ABI
mismatches when using dynamic linking.
