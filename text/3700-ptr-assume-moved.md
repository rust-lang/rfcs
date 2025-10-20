- Feature Name: `ptr_assume_moved`
- Start Date: 2024-09-26
- RFC PR: [rust-lang/rfcs#3700](https://github.com/rust-lang/rfcs/pull/3700)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add a helper for primitive pointer types to facilitate modifying the address of a pointer.  This
mechanism is intended to enable the use of architecture features such as AArch64 Top-Byte Ignore
(TBI) to facilitate use-cases such as high-bit pointer tagging.  An example application of this
mechanism would be writing a tagging memory allocator.

# Motivation
[motivation]: #motivation

The term "pointer tagging" could be used to mean either high-bit tagging or low-bit tagging.
Architecture extensions such as AArch64 Top-Byte Ignore make the CPU disregard the top bits of a
pointer when determining the memory address, leaving them free for other uses.

This RFC is specifically concerned with creating those high-bit tagged pointers for systems which
can make use of such architecture features. High-bit tagged pointers pose a somewhat tricky
challenge for Rust, as the memory model still considers those high bits to be part of the address.
Thus, from the memory model's perspective, changing those bits puts the pointer outside of its
original allocation, despite it not being the case as far as the hardware & OS are concerned.  This
makes loads and stores using the pointer undefined behavior, despite the fact that if such loads
and stores were to be directly done in assembly they would be perfectly safe and valid.

Whenever this RFC refers to a "tagged pointer", it should be taken to mean a pointer that had some
of its top bits set to non-0 values.

Tagged pointers are pointers in which the unused top bits are set to contain some metadata - the
tag.  No 64-bit architecture today actually uses a 64-bit address space. Most operating systems only
use the lower 48 bits, leaving higher bits unused. The remaining bits are for the most part used to
distinguish userspace pointers (0x00) from kernelspace pointers (0xff), at least on Linux.  Certain
architectures provide extensions, such as TBI on AArch64, that make it easier for programs to make
use of those unused bits to insert custom metadata into the pointer without having to manually mask
them out prior to every load and store. This tagging method can be used without said architecture
extensions - by masking out the bits manually - albeit said extensions make it more efficient.

Currently, Rust does not support directly using TBI and related architecture extensions that
facilitate the use of tagged pointers. This could potentially cause issues in cases such as working
with TBI-enabled C/C++ components over FFI, or when writing a tagging memory allocator. While there
is no explicit support for this in C/C++, due to there not being Strict Provenance restrictions it
is straightforward to write a 'correct' pointer tagging implementation by simply doing a `inttoptr`
cast inside the memory allocator implementation, be it a custom C `malloc` or using a custom C++
`Allocator`. The goal of this effort is to create a Rust API for implementing this type of
functionality that is guaranteed to be free of undefined behavior.

There needs to be a low-level helper in the standard library, despite the relatively niche use case
and relative simplicity, so that there is a single known location where Miri hooks can be called to
update the canonical address, and so that the helper can be appropriately annotated for LLVM in the
codegen stage.  This will make it easier to modify pointer addresses without breaking the Rust
memory model in the process.

Different architectures use different tagging schemes, and there are applications for something like
this even completely outside of pointer tagging. This proposal is meant to only add the low-level
helper for all of those use-cases, so that it is trivial to implement a helper for any arbitrary
tagging scheme just by having it call `assume_moved(..)` with whatever the address should be.

For one-off tagging applications, using this helper is not even necessary. As long as the tag bits
are masked out prior to every load and store, it is possible to indirectly use a tagged pointer even
without this. However, the way these kinds of architecture features tend to be applied involves
every heap allocation coming with a tag, and then having to mask out every heap-allocated pointer
would be putting a much heavier burden on the end-user. With this helper, we can have the library do
the heavy lifting and the end-user does not even have to be aware that the pointers they are using
happen to be tagged.

For those one-off applications, an LLVM optimisation pass that optimises away applying the mask into
NOPs on TBI-enabled platforms would be nice to have, although obviously that is out of the scope for
this RFC.
 
# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC adds one associated function to `core::ptr`:
`pub unsafe fn assume_moved<T>(original: *mut T, new_address: usize) -> *mut T`

```rust
use core::ptr::assume_moved;

let tag = 63;
let new_addr = ptr as usize | tag << 56;
let tagged_ptr = unsafe { assume_moved(ptr, new_addr) };
```

The purpose of this function is to indicate to the compiler that an allocation that used to be
pointed to by a given pointer can now only be accessed by the new pointer with the provided new
address. This is supposed to be semantically equivalent to a `realloc` from the untagged address to
the tagged address, and conceptually similar to a `move` - it is no longer valid to access the
allocation through the untagged pointer or any derived pointers. That being said, no actual
moving is done - the underlying memory does not change, it only changes within the Rust memory
model.

Importantly, this is only designed to work on pointers to heap allocations. Trying to tag a
stack-allocated variable in this way will almost definitely result in undefined behavior because
the compiler will not be able to reason about whether writes to the tagged pointer can modify the
value of the local variable.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As previously explained, the memory model we currently have is not fully compatible with memory
tagging and tagged pointers. Setting the high bits of a pointer must be done with great care in
order to avoid introducing undefined behavior, which could arise as a result of violating pointer
aliasing rules - using two 'live' pointers which have different 64-bit addresses but do point to
the same chunk of memory would weaken alias analysis and related optimisations.

We can avoid this issue by assuming that a move from the untagged address to the tagged address has
happened. To do so, we need the helper function to return a pointer with a brand new provenance,
disjoint from the provenance of the original pointer.

Every change to the high bits has to at least simulate a move operation and we must ensure the old
pointers are invalidated. This is due to the aforementioned discrepancy between how Rust & LLVM see
a memory address and how the OS & hardware see memory addresses.  From the OS & hardware
perspective, the high bits are reserved for metadata and do not actually form part of the address
(in the sense of an 'address' being an index into the memory array).  From the LLVM perspective, the
high bits are part of the address and changing them means we are now dealing with a different
address altogether.  Having to reconcile those two views necessarily creates some friction and extra
considerations.

Function signature, documentation and an example implementation:

```rust
/// Assume that the object pointed to by a pointer has been moved to a new address
///
/// Intended for use with pointer tagging architecture features such as AArch64 TBI.
/// This function creates a new pointer with the address `new_address` and a brand new provenance,
/// then assumes that a move from the original address to the new address has taken place.
/// Note that this is only an indication for the compiler - nothing actually gets moved or reallocated.
///
/// SAFETY: Users *must* ensure that `new_address` actually contains the same memory as the original.
/// The primary use-case is working with various architecture pointer tagging schemes, where two
/// different 64-bit addresses can point to the same chunk of memory due to some bits being ignored.
/// When used incorrectly, this function can be used to violate the memory model in arbitrary ways.
/// Furthermore, after using this function, users must ensure that the underlying memory is only ever
/// accessed through the newly created pointer. Any accesses through the original pointer
/// (or any pointers derived from it) would be undefined behavior.
/// Additionally, this function is only designed for use with heap allocations. Trying to use it with
/// a pointer to a stack-allocated variable will result in undefined behavior and
/// should not be done under any circumstances.
#[inline(never)]
#[unstable(feature = "ptr_assume_moved", issue = "none")]
#[cfg_attr(not(bootstrap), rustc_simulate_allocator)]
#[allow(fuzzy_provenance_casts)]
pub unsafe fn assume_moved<T>(original: *mut T, new_address: usize) -> *mut T {
    // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
    let mut ptr = new_address as *mut T;
    // SAFETY: This does not do anything
    unsafe {
        asm!(
            "/* simulate a move from {original} to {ptr} */",
            original = in(reg) original,
            ptr = inout(reg) ptr
        );
    }
    // FIXME: call Miri hooks to update the address of the original allocation
    ptr
}
```

Importantly, this function is very much *not* the same as `ptr.with_addr()`. `ptr.with_addr()`
internally uses `wrapping_offset()` which in turn uses LLVM's `getelementptr`. This makes it come
with certain aliasing restrictions which this function would not have. That is to say, it is only
valid to use `ptr.with_addr()` if the resulting address is still within the bounds of the same
original allocation, for instance when getting a pointer to a different struct field. This function
is supposed to make it so that it will be valid to access the resulting pointer as long it
dereferences to *any* allocated object.

# Drawbacks
[drawbacks]: #drawbacks

Such a low-level helper is inherently highly unsafe and could be used to violate the memory model in
many different ways, so it will have to be used with great care.  The approach of assuming that a
move has occurred is unfortunate in that it makes the support we add to the language more
restrictive than the actual hardware reality allows for, but this seems to be the only solution
available for the time being as modifying the entire stack to support disregarding the top bits of a
pointer would be a non-trivial endeavour.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Without having a dedicated library helper for modifying the address, users wanting to make use of
high-bit tagging would have to resort to manually using bitwise operations and would be at risk of
inadvertently introducing undefined behavior.  Having a helper for doing so in the library creates
a place where e.g. Miri hooks can be called to let Miri know that a pointer's cannonical address has
been updated. It also makes it possible for the helper to receive special treatment in the codegen
stage, which is required for it to have the desired behaviour.

It is most likely not feasible to make `assume_moved()` safe to use regardless of the context,
hence the current approach is to make it an unsafe function with a safety notice about the user's
responsibilities.

The current working name for the new function is `assume_moved`, but it is not the only candidate.
Possible alternatives that were discussed were `simulate_realloc` and `simulate_move`. Preferences
appear to be rather subjective and the function can easily be renamed if need be.

# Implementation considerations

In order for the function to behave more like an actual memory allocation function, it may also be
desirable to annotate it with [noalias](https://llvm.org/docs/LangRef.html#noalias) in LLVM.
This would enable additional optimisations, as per the LLVM documentation:

> On function return values, the noalias attribute indicates that the function acts like a system
> memory allocation function, returning a pointer to allocated storage disjoint from the storage for
> any other object accessible to the caller.

One way to do so would be through a rustc built-in attribute similar to e.g. `rustc_allocator` -
`rustc_simulate_allocator`. This attribute will be passed down to the codegen stage so that the
codegen can appropriately annotate the function.

In the implementation stage, a discussion should be had on whether it is preferable to use a pure
pointer cast (LLVM `inttoptr`) or an existing function like `ptr.with_addr()` (LLVM `getelementptr`)
to create the pointer that will then be passed through an inline asm block. This is however purely
an implementation detail and not a question that needs to be answered definitively in the RFC stage.

# Prior art
[prior-art]: #prior-art

TBI already works in C, though mostly by default and care must be taken to make sure no Undefined
Behaviour is introduced. The compiler does not take special steps to preserve the tags, but it
doesn't try to remove them either.  That being said, the C/C++ standard library does not take
tagging schemes into account during alias analysis. With this proposal, Rust would have much better
defined and safer support for TBI than C or C++.

Notably, [Android](https://source.android.com/docs/security/test/tagged-pointers) already makes
extensive use of TBI by tagging all heap allocations.

The idea is also not one specific to AArch64, as there are similar extensions present on other
architectures that facilitate working with tagged pointers.
This proposal is in no way AArch64-specific. As proposed, `assume_moved` could be used on x86
for Intel's LAM or AMD's UAI as well. It could even be used for applications other than pointer
tagging, such as using `mmap` to map the same allocation to two adjacent addresses and switch between
them by using `assume_moved()`. Although that use-case requires its own documentation and
additional concurrency safety considerations that do not apply to pointer tagging.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

What is the best way to make this compatible with Strict Provenance? We want to be able to create a
pointer with an arbitrary address, detached from any existing pointers and with a brand-new
provenance. Does this proposal fit as-is, or does it need any specific accommodations?

# Future possibilities
[future-possibilities]: #future-possibilities

With a low-level helper for changing the address such as the one proposed here, it would be trivial
to add helper functions for supporting specific tagging schemes to `std::arch`. All of those
architecture-specific functions would internally use this helper.

Whilst the assumed move approach is restrictive today, at some point in the future if the LLVM
memory model gains an understanding that the address is only made up of the lower 56 bits, this
restriction could be relaxed. It would then allow both the original and the tagged pointer to be
valid and aliased at the same time.

On compatible platforms, interesting use-cases might be possible, e.g. tagging pointers when
allocating memory in Rust in order to insert metadata that could be used in experiments with pointer
strict provenance.
