- Feature Name: `unsized_thin_pointers`
- Start Date: 2023-11-29
- RFC PR: [rust-lang/rfcs#3536](https://github.com/rust-lang/rfcs/pull/3536)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Enable user code to define dynamically-sized thin pointers. Such types are
`!Sized`, but references to them are pointer-sized (i.e. not "fat pointers").
The implementation of [`core::mem::size_of_val()`][size_of_val] delegates to
a new `core::mem::DynSized` trait at runtime.

[size_of_val]: https://doc.rust-lang.org/core/mem/fn.size_of_val.html

# Motivation
[motivation]: #motivation

Enable ergonomic and efficient references to dynamically-sized values that
are capable of computing their own size.

It should be possible to declare a Rust type that is `!Sized`, but has
references that are pointer-sized and therefore only require a single register
on most architectures.

In particular this RFC aims to support a common pattern in other low-level
languages, such as C, where a value may consist of a fixed-layout header
followed by dynamically-sized data:

```c
struct __attribute__((aligned(8))) request {
    uint32_t size;
    uint16_t id; 
    uint16_t flags;
    /* uint8_t request_data[]; */
};

void handle_request(struct request *req) { /* ... */ }
```

This pattern is used frequently in zero-copy APIs that transmit structured data
between address spaces of differing trust levels.

# Background
[motivation]: #motivation

There are currently two approved RFCs that cover similar functionality:
* [RFC 1861] adds `extern type` for declaring types that are opaque to Rust's
  type system. One of the capabilities available to extern types is that they
  can be embedded into a `struct` as the last field, and that `struct` will
  become an unsized type with thin references.

  Stabilizing `extern type` is currently blocked on questions of how to handle
  Rust layout intrinsics such as [`core::mem::size_of_val()`][size_of_val] and
  [`core::mem::align_of_val()`][align_of_val] for fully opaque types.

* [RFC 2580] adds traits and intrinsics for custom DSTs either with or without
  associated "fat pointer" metadata. A custom DST with thin references can be
  represented as `Pointee<Metadata = ()>`.

  Stabilizing custom DSTs is currently blocked on multiple questions involving
  the content and representation of complex metadata, such as `&dyn` vtables.

In both of these cases the ability to declare custom DSTs with thin references
is a minor footnote to the overall feature, and stabilization is blocked by
issues unrelated to thin-pointer DSTs.

The objective of this RFC is to extract custom thin-pointer DSTs into its own
feature, which would hopefully be free of known issues and could be stabilized
without significant changes to the compiler or ecosystem.

[RFC 1861]: https://rust-lang.github.io/rfcs/1861-extern-types.html
[RFC 2580]: https://rust-lang.github.io/rfcs/2580-ptr-meta.html

[align_of_val]: https://doc.rust-lang.org/core/mem/fn.align_of_val.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The unsafe trait `core::mem::DynSized` may be implemented for a `!Sized` type
to configure how the size of a value is computed from a reference. References
to a type that implements `DynSized` are not required to store the value size
as pointer metadata.

If a type that implements `DynSized` has no other associated pointer metadata
(such as a vtable), then references to that type will have the same size and
layout as a normal pointer.

```rust
#[repr(C, align(8))]
struct Request {
	size: u32,
	id: u16,
	flags: u16,
	data: [u8],
}

unsafe impl core::mem::DynSized for Request {
	fn size_of_val(&self) -> usize {
		usize::try_from(self.size).unwrap_or(usize::MAX)
	}
}

// size_of::<&Request>() == size_of::<*const ()>()
```

The `DynSized` trait has a single required method, `size_of_val()`, which
has the same semantics as `core::mem::size_of_val()`.

```rust
// core::mem
pub unsafe trait DynSized {
	// Returns the size of the pointed-to value in bytes.
	fn size_of_val(&self) -> usize;
}
```

It is an error to `impl DynSized` for a type that is `Sized`. In other words,
the following code is invalid:

```rust
#[repr(C, align(8))]
struct SizedRequest {
	size: u32,
	id: u16,
	flags: u16,
	data: [u8; 1024],
}

// Compiler error: `impl DynSized` on a type that isn't `!Sized`.
unsafe impl core::mem::DynSized for SizedRequest {
	fn size_of_val(&self) -> usize {
		usize::try_from(self.size).unwrap_or(usize::MAX)
	}
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `core::mem::DynSized` trait acts as a signal to the compiler that the
size of a value can be computed dynamically by the user-provided trait
implementation. If references to that type would otherwise be of the layout
`(ptr, usize)` due to being `!Sized`, then they can be reduced to `ptr`.

The `DynSized` trait does not _guarantee_ that a type will have thin pointers,
it merely enables it. This definition is intended to be compatible with RFC
2580, in that types with complex pointer metadata would continue to have fat
pointers. Such types may choose to implement `DynSized` by extracting their
custom pointer metadata from `&self`.

Implementing `DynSized` does not affect alignment, so the questions of how to
handle unknown alignments of RFC 1861 `extern type` DSTs do not apply.

In current Rust, a DST used as a `struct` field must be the final field of the
`struct`. This restriction remains unchanged, as the offsets of any fields after
a DST would be impossible to compute statically.
- This also implies that any given `struct` may have at most one field that
  implements `DynSized`.

A `struct` with a field that implements `DynSized` will also implicitly
implement `DynSized`. The implicit implementation of `DynSized` computes the
size of the struct up until the `DynSized` field, and then adds the result of
calling `DynSized::size_of_val()` on the final field.
- This implies it's not permitted to manually `impl DynSize` for a type that
  contains a field that implements `DynSize`.

# Drawbacks
[drawbacks]: #drawbacks

## Mutability of value sizes

If the size of a value is stored in the value itself, then that implies it can
change at runtime.

```rust
struct MutableSize { size: usize }
unsafe impl core::mem::DynSized for MutableSize {
	fn size_of_val(&self) -> usize { self.size }
}

let mut v = MutableSize { size: 8 };
println!("{:?}", core::mem::size_of_val(&v)); // prints "8"
v.size = 16;
println!("{:?}", core::mem::size_of_val(&v)); // prints "16"
```

There may be existing code that assumes `size_of_val()` is constant for a given
value, which is true in today's Rust due to the nature of fat pointers, but
would no longer be true if `size_of_val()` is truly dynamic.

Alternatively, the API contract for `DynSized` implementations could require
that the result of `size_of_val()` not change for the lifetime of the allocated
object. This would likely be true for nearly all interesting use cases, and
would let `DynSized` values be stored in a `Box`.

## Compatibility with existing fat-pointer DSTs

It may be desirable for certain existing stabilized DSTs to implement
`DynSized` -- for example, it is a natural fit for the planned redefinition of
[`&core::ffi::CStr`][cstr] as a thin pointer.

[cstr]: https://doc.rust-lang.org/core/ffi/struct.CStr.html

Such a change to existing types might be backwards-incompatible for code that
embeds those types as a `struct` field, because it would change the reference
layout. For example, the following code compiles in stable Rust v1.73 but would
be a compilation error if `&CStr` does not have the same layout as `&[u8]`.

```rust
struct ContainsCStr {
	cstr: core::ffi::CStr,
}
impl ContainsCStr {
	fn as_bytes(&self) -> &[u8] {
		unsafe { core::mem::transmute(self) }
	}
}
```

The above incompatibility of a redefined `&CStr` exists regardless of this RFC,
but it's worth noting that implementing `DynSized` would be a backwards
incompatible change for existing DSTs.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This design is less generic than some of the alternatives (including custom DSTs
and extern types), but has the advantage being much more tightly scoped and
therefore is expected to have no major blockers. It directly addresses one of
the pain points for use of Rust in a low-level performance-sensitive codebase,
while avoiding large-scale language changes to the extent possible.

Without this change, people will continue to either use thick-pointer DSTs
(reducing performance relative to C), or write Rust types that claim to be
`Sized` but actually aren't (the infamous `_data: [u8; 0]` hack).

# Prior art
[prior-art]: #prior-art

The canonical prior art is the C language idiom of a `struct` that's implicitly
followed by a dynamically-sized value. This idiom was standardized in C99 under
the term "flexible array member":

> As a special case, the last element of a structure with more than one named
> member may have an incomplete array type; this is called a flexible array
> member. [...] However, when a `.` (or `->`) operator has a left operand that
> is (a pointer to) a structure with a flexible array member and the right
> operand names that member, it behaves as if that member were replaced with the
> longest array (with the same element type) that would not make the structure
> larger than the object being accessed;

The use of flexible array members (either with C99 syntax or not) is widespread
in C APIs, especially when sending structured data between processes ([IPC]) or
between a process and the kernel. For example, the Linux kernel's [FUSE]
protocol communicates with userspace via length-prefixed dynamically-sized
request/response buffers.

They're also common when implementing low-level network protocols, which have
length-delimited frames comprising a fixed-layout header followed by a variable
amount of payload data.

[IPC]: https://en.wikipedia.org/wiki/Inter-process_communication
[FUSE]: https://www.kernel.org/doc/html/v6.3/filesystems/fuse.html

In the context of Rust, the two RFCs mentioned earlier both cover thin-pointer
DSTs as part of their more general extensions to the Rust type system:
- [RFC 1861: `extern_types`](https://rust-lang.github.io/rfcs/1861-extern-types.html)
- [RFC 2580: `ptr_metadata`](https://rust-lang.github.io/rfcs/2580-ptr-meta.html)

Also, there have been non-approved RFC proposals involving thin-pointer DSTs:
- [[rfcs/pull#709] truly unsized types](https://github.com/rust-lang/rfcs/pull/709)
- [[rfcs/pull#1524] Custom Dynamically Sized Types](https://github.com/rust-lang/rfcs/pull/1524)
- [[rfcs/pull#2255] More implicit bounds (?Sized, ?DynSized, ?Move)](https://github.com/rust-lang/rfcs/issues/2255)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None so far

# Future possibilities
[future-possibilities]: #future-possibilities

None so far. Further exploration of opaque types and/or custom pointer metadata
already has separate dedicated RFCs. This one is just to get an MVP for types
that should be `!Sized` without fat pointers.
