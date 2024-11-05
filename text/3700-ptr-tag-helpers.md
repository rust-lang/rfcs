- Feature Name: `ptr_tag_helpers`
- Start Date: 2024-09-26
- RFC PR: [rust-lang/rfcs#3700](https://github.com/rust-lang/rfcs/pull/3700)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Add helper methods on primitive pointer types to facilitate getting and setting the tag of a pointer.
Intended to work with programs that make use of architecture features such as AArch64
Top-Byte Ignore (TBI), the primary use-case being writing tagging memory allocators.

# Motivation
[motivation]: #motivation

Tagged pointers are pointers in which the unused top bits are set to contain some metadata - the tag.
No 64-bit architecture today actually uses a 64-bit address space. Most operating systems only use
the lower 48 bits, leaving higher bits unused. The remaining bits are for the most part used to
distinguish userspace pointers (0x00) from kernelspace pointers (0xff).
Certain architectures provide extensions, such as TBI on AArch64, that allow programs to make use of
those unused bits to insert custom metadata into the pointer.

Currently, Rust does not acknowledge TBI and related architecture extensions that enable the use of
tagged pointers. This could potentially cause issues in cases such as working with TBI-enabled C/C++
components over FFI, or when writing a tagging memory allocator.
These functions are worth including in the standard library, despite their relatively niche use case
and relative simplicity, so that there is a single known location where Miri hooks can be called to
update the canonical address.
This will make it easier to modify tagged pointers without breaking the Rust memory model.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC adds two methods on each primitive pointer type - `ptr.tag()` and
`ptr.with_tag(tag: u8)`.

```
assert!(ptr.tag() == 0);
let tagged_ptr = unsafe { ptr.with_tag(63) };
assert!(tagged_ptr.tag() == 63);
```

The primary use-case is implementing an allocator that tags pointers before returning them to the
caller.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Within Rust's memory model, modifying the high bits offsets the pointer outside of the bounds of
its original allocation, making any use of it Undefined Behaviour.

The `with_tag()` method is only designed to be a helper for writing tagging allocators.
Users *must* ensure that code using this method simulates a realloc from the untagged
address to the tagged address, and that the underlying memory is only ever accessed
using the tagged address from there onwards. Anything short of that explicitly violates the
Rust memory model and will cause the program to break in unexpected ways.

# Drawbacks
[drawbacks]: #drawbacks

Because the memory model we currently have is not fully compatible with memory tagging and
tagged pointers, setting the high bits of a pointer must be done with great care in order to
avoid introducing Undefined Behaviour.

Every change to the high bits has to at least simulate a realloc and we must ensure the old pointers
are invalidated. This is due to a fundamental discrepancy between how Rust & LLVM see a memory
address and how the OS & hardware see memory addresses.
From the OS & hardware perspective, the high bits are reserved for metadata and do not actually form
part of the address (in the sense of an 'address' being an index into the memory array).
From the LLVM perspective, the high bits are part of the address and changing them means we are now
dealing with a different address altogether. Having to reconcile those two views necessarily creates
some friction and extra considerations around Undefined Behaviour.

More context on the aforementioned discrepancy can be found in a discussion about memory tagging
on GitHub, [here](https://github.com/rust-lang/rust/issues/129010).

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Without having a dedicated library method for modifying the high bits, users wanting to write
tagging memory allocators would have to resort to manually using bitwise operations.
Having a method for doing so in the library creates a place where e.g. Miri hooks can be called
to let Miri know that a pointer's cannonical address has been updated.

# Prior art
[prior-art]: #prior-art

TBI already works in C, though mostly by default and care must be taken to make sure no
Undefined Behaviour is introduced. The compiler does not take special steps to preserve the tags,
but it doesn't try to remove them either.
That being said, the C/C++ standard library does not take tags into account during alias analysis.

Notably, [Android](https://source.android.com/docs/security/test/tagged-pointers) already makes
extensive use of TBI by tagging all heap allocations.

The idea is also not one specific to AArch64, as there are similar extensions present on other
architectures that facilitate working with tagged pointers.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

How exactly should the API be structured in order to accommodate differences between architectures?
Different architectures use different tagging schemes. For instance, on AArch64 the tag is the entire
top byte. The number of bits used by other architectures can be different, and equally bit 63 might
be reserved thus making the tag start at bit 62.
To make the interface flexible enough for any arbitrary tagging scheme, the caller would have to pass
the tag, the number of bits used for the tag in the architecture and the offset where the tag starts.
This might make the interface somewhat unwieldy and require different invocations for each
architecture.
Alternatively, we could abstract away the architecture details by adding `cfg(target_arch)` checks
inside the methods, so that the caller would just be able to write `ptr.with_tag(tag)` and have that
automatically use whichever tagging scheme the code is being compiled for.

It is most likely not feasible to make `with_tag()` safe to use regardless of the context,
hence the current approach is to make it an unsafe method with a safety notice about the user's
responsibilities.

# Future possibilities
[future-possibilities]: #future-possibilities

The interface could be extended (or similar interfaces could be added) to accommodate similar
architecture extensions e.g. on x86-64. There are subtle differences between platforms, e.g.
on x86-64 modifying bit 63 is not allowed for pointer-tagging purposes, so unlike AArch64
not all possible u8 values would be safe to use.

On compatible platforms, interesting use-cases might be possible, e.g. tagging pointers when
allocating memory in Rust in order to insert metadata that could be used in experiments with
pointer strict provenance.
