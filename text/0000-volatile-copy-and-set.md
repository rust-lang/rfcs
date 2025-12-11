- Feature Name: volatile-copy-and-set
- Start Date: 2019-04-17
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#00000](https://github.com/rust-lang/rust/issues/00000)

# Summary
[summary]: #summary

Stabilize the `volatile_copy_memory`, `volatile_copy_nonoverlapping_memory`
and `volatile_set_memory` intrinsics as `ptr::copy_volatile`,
`ptr::copy_nonoverlapping_volatile` and `ptr::write_bytes_volatile`,
respectively.

# Motivation
[motivation]: #motivation

`ptr::read_volatile` and `ptr::write_volatile` were stabilized in RFC
[1467](https://github.com/rust-lang/rfcs/pull/1467).  The stated motivation
at the time was that this allowed "volatile access to memory-mapped I/O
in stable code", something that was only previously possible using unstable
intrinsics or "by abusing a bug in the `load` and `store` functions on
atomic types which gives them volatile semantics
([rust-lang/rust#30962](https://github.com/rust-lang/rust/pull/30962))."

At the time, the decision was made not to also provide stable
interfaces for the `volatile_copy_memory` or `volatile_set_memory`
intrinsics, as they were "not used often" nor provided in C.
However, when writing low-level code, it is sometimes also useful
to be able to execute volatile copy and set operations.

For example, when booting x86_64 "application processor" (AP) logical
processors, code copies a sequence of instructions that for the AP to
execute into a page in low physical memory, and then sends a startup
inter-processor interrupt (SIPI) to the AP's local interrupt
controller: the target interrupt vector number given in the SIPI is
multiplied by the page size to determine the physical memory address
where the AP should start executing.  So a SIPI sent to vector 7 of
an AP causes that processor to begin executing instructions at
physical memory address 0x7000.

That is:

```
extern "C" {
    fn copy_proto_page_to_phys_mem(src: usize, phys: u64);
    fn send_init_ipi(cpu: u32);
    fn send_sipi(cpu: u32, vector: u8);

    static INIT_CODE: *const u8;
    static INIT_CODE_LEN: usize;
}

// A contrived type for illustration; not actually useful.
pub struct SIPIPage {
    // Note that `bytes` is not visible outside of `SIPIPage`.
    bytes: [u8; 4096],
}

impl SIPIPage {
    // Note that the _only_ operation on the `bytes` field
    // of `SIPIPage` is in `new`.  The compiler could, in
    // theory, elide the `copy`.
    pub fn new() -> SIPIPage {
        let mut bytes = [0; 4096];
        unsafe {
            core::ptr::copy(INIT_CODE, bytes.as_mut_ptr(), INIT_CODE_LEN);
        }
        SIPIPage { bytes }
    }
}

fn main() {
    let proto_sipi_page = SIPIPage::new();
    let some_core = 2;
    unsafe {
        copy_proto_page_to_phys_mem(&proto_sipi_page as *const _ as usize, 0x7000);
        send_init_ipi(some_core);
        send_sipi(some_core, 7);
    }
}
```

Obviously this is an unlikely way of initializing the SIPI page and
a real kernel would not do it this way.

Hoever, this code snippet is specifically constructed such that the
sequence of sending IPIs makes no reference to `proto_sipi_page` and
since the `bytes` field is not visible outside of `new`, this
illustrates a situation in which the compiler _could_ theoretically
elect to elide the copy.

If this sequenced used `core::ptr::copy_volatile` then the compiler
would know that the copy had some externally visible side-effect
and could not be elided.

When writing a multi-processor operating system kernel for x86_64 in
Rust, the programmer would copy the instruction text to some address
and write to the local programmable interrupt controller to send a
SIPI to start AP cores, but from the compiler's perspective, it might
appear that the memory holding the AP startup code is never referred
to again.  The compiler could potentially choose to elide the copy
entirely, and the AP might start executing junk instructions from
uninitialized memory.  In the worst case, this may silently corrupt
kernel state.

Using a volatile copy can inform the compiler that there is an
externally observable side-effect forcing it to preserve the copy.
Similarly, volatile "write_bytes" allows a program to preserve a
write that has some side-effect (for example, initializing register
state in a device, or clearing a frame buffer).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Given these operations, one would write, for example, the following:

```
#[no_mangle]
pub unsafe extern "C" fn maybe_called_via_ffi(ptr: *mut u8; len: usize) {
    println!("this function has a side-effect, and it is not just the println!");
    core::ptr::write_bytes_volatile(ptr, SOME_DATA, SOME_DATA_LEN);
}
```

and assert that the `write_bytes_volatile` call is not be elided.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`ptr::copy_volatile`, `ptr::copy_nonoverlapping_volatile` and
`ptr::write_bytes_volatile` will work the same way as `ptr::copy`,
`ptr_copy_nonoverlapping` and `ptr::write_bytes` respectively, but
with volatile semantics.  As stated in RFC 1467, "the semantics of
a volatile access are already pretty well defined by the C standard.

We further propose enhancing the documentation for these functions
to the same level of the existing volatile functions.

Documentation presently refers to LLVM implementation details
to explain the memory model, etc, here:
http://llvm.org/docs/LangRef.html#volatile-memory-accesses.
We propose modifying existing documentation, and writing new
docuemntation, referring to the memory model in the C standard
instead.

# Drawbacks
[drawbacks]: #drawbacks

Volatile semantics are not well defined by the C standard, but
that is out of the scope of this proposal.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The intrinsics operations already exist and have the semantics
required by operating system implementors and others.

There are several alternatives, each with their own drawbacks:

1.  Continue using the unstable `core_intrinsics` feature and use the
    existing unstable intrinsics.  However, this ties the programmer
    to unstable Rust, which is undesirable in some environments.
2.  Use the existing copy and set interfaces without volatile qualifiers
    and hope that the compiler does not elide the relevant calls.  While
    likely workable in practice for most likely scenarios, this could
    lead to surprising behavior if the compiler ever incorporates
    sufficiently advanced analyses that allow it to determine that those
    elisions are possible from its perspective.  Hope is not a strategy.
3.  Use the foreign function interface to call separately written code
    in another language that provides the required semantics.  This
    is inelegant and complicates the build process.
4.  Hand-code copy and set loops in terms of the existing `write_volatile`
    function.  This is inelegant, leads to duplicated code, and opens
    up the possibility of bugs.  For example, compare:

    ```
        for (i, elem) in some_slice.iter().enumerate() {
            unsafe {
                core::ptr::write_volatile(&mut dest[i], *elem);
            }
        }
    ```
    to,
    ```
        unsafe {
            core::ptr::copy_volatile(some_slice.as_ptr(), dest.as_mut_ptr(), some_slice.len());
        }
    ```

Finally, it is important that this proposal not tie the Rust language
to the semantics of any given implementation, such as those defined by
LLVM.  Futher Rust does not yet have a well-defined memory model we can
refer to for defining volatile behavior, and C does not define volatile
`memset`, `memcpy` or `memmove` functions.  However, since the existing
`core::ptr::write_volatile` and `core::ptr::read_volatile` functions
are implemented in terms of well-defined semantics, it makes sense to
use similar semantics here.  We therefore specify that
`copy_volatile`, `copy_nonoverlapping_volatile` and
`write_bytes_volatile` adopt semantics similar to those of `read_volatile`
and `write_volatile`: resulting loads and stores cannot be elided, and
the relative order to loads and stores cannot be reordered with respect
to one another, though other operations can be reordered with respect
to volatile operations.

# Prior art
[prior-art]: #prior-art

Other languages support volatile style accesses, notably C and C++.
Interestingly, volatile semantics in those languages are associated with
individual objects, and `volatile` is a type qualifier, not an operaton
attribute.  In those systems, any number of operations on a
volatile-qualified datum result in volatile memory semantics; since
any identifier used by the standard library is defined to be reserved
for special treatment by the compiler, this means that the standard
`memcpy`, `memmove` and `memset` operations can all be expected to exhibit
volatile semantics if applied to volatle-qualified objects.

# Unresolved questions
[unresolved]: #unresolved-questions

None.

# Future possibilities
[future-possibilities]: #future-possibilities

A some point, a well-defined memory model for Rust may be stabilized that
would widen the design space and permit revisiting these primitives.  For
example, "volatile" currently means that a write cannot be elided, but it
also imposes strict ordering semantics with respect to other volatile
accesses.  One can envision a sufficiently rich memory model that one
might be some way to specify an "unelidable" write, but without ordering
constraints.
