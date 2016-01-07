- Feature Name: bitfield
- Start Date: 2016-01-07
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add support for bitfields via a single per-field attribute.

# Motivation
[motivation]: #motivation

Bitfields are commonly encountered when interacting with C code. Since many of
the details of the behavior of bitfields are left to the implementation, it is
hard to write cross-platform rust code that uses bitfields correctly.

Currently, such code often manually collapses bitfields into a single integer
field and attempts to extract and set the individual fields via bitshifts. This
approach works for simple bitfields and when the full set of target platforms is
known, however, once the bitfields become more complicated or if the code is
compiled for an unexpected target, such code will silently fail. In particular,
code that assumes that bitfields are placed into the integer in increasing bit
order will likely break on big-endian machines.

Appending A contains various examples of the (to some) surprising behavior of
bitfields on the x86-64 linux platform.

This rfc introduces a single attribute that can be applied to fields inside of
`#[repr(C)]` structs to turn them into bitfields.

# Detailed design
[design]: #detailed-design

The attribute `#[bitfield(N)]` is added. Here, `N` can be any constant
expression that evaluates to a value of type `usize`.

The attribute can be applied to fields inside a `#[repr(C)]` struct which have
any of following types:

* `u8`, `u16`, `u32`, `u64`, `usize`,
* `i8`, `i16`, `i32`, `i64`, `isize`.

For example:

```rust
#[repr(C)]
pub struct perf_event_attr {
    /* ... */

    #[bitfield(1)] pub disabled: u64,
    #[bitfield(1)] pub inherit: u64,
    #[bitfield(1)] pub pinned: u64,
    #[bitfield(1)] pub exclusive: u64,
    #[bitfield(1)] pub exclude_user: u64,
    #[bitfield(1)] pub exclude_kernel: u64,
    #[bitfield(1)] pub exclude_hv: u64,
    #[bitfield(1)] pub exclude_idle: u64,
    #[bitfield(1)] pub mmap: u64,
    #[bitfield(1)] pub comm: u64,
    #[bitfield(1)] pub freq: u64,
    #[bitfield(1)] pub inherit_stat: u64,
    #[bitfield(1)] pub enable_on_exec: u64,
    #[bitfield(1)] pub task: u64,
    #[bitfield(1)] pub watermark: u64,
    #[bitfield(1)] pub precise_ip: u64,
    #[bitfield(1)] pub mmap_data: u64,
    #[bitfield(1)] pub sample_id_all: u64,
    #[bitfield(1)] pub exclude_host: u64,
    #[bitfield(1)] pub exclude_guest: u64,
    #[bitfield(1)] pub exclude_callchain_kernel: u64,
    #[bitfield(1)] pub exclude_callchain_user: u64,
    #[bitfield(1)] pub mmap2: u64,
    #[bitfield(1)] pub comm_exec: u64,
    #[bitfield(39)] pub __reserved_1: u64,

    /* ... */
}
```

`N` must evaluate to an integer in the range `[0, sizeof(T)]` where `T` is the
declared type of the field. It specifies the width of the field.

Many details of the behavior of such fields are left to the implementation.
However, the implementation is encouraged to follow the behavior of the dominant
C ABI on the target platform.

Those details that are specified are taken from the C11 standard and reproduced
here for completeness. However, some of the details specified in said standard
are omitted here since they are implied by the `#[repr(C)]` attribute and thus
already covered by the previous paragraph.

Concurrent access to different bitfields inside of an object is a data race
unless,

* none of the accesses mutate the bitfields,
* the accesses are synchronized,
* the bitfields are separated (in declaration order) by a non-bitfield, or
* the bitfields are separated (in declaration order) by a bitfield of size 0.

For example:

```rust
#[repr(C)]
struct X {
    #[bitfield(8)] pub a: u8,
    #[bitfield(8)] pub b: u8,
    #[bitfield(0)] pub c: u8,
    #[bitfield(1)] pub d: u8,
}
```

In this structure, `a` and `b` must not be modified concurrently. `a` and `d`
can be accessed concurrently.

Values stored in a bitfield have a binary representation using the `m` bits it
occupies in its memory location.

The `&` and `&mut` operators cannot be applied to bitfields.

The type of a bitfield is its declared type. Reading from a bitfield returns a
value of the declared type. Only values of the declared type can be written to
a bitfield. The bits of the binary representation of a bitfield are interpreted
as the bits of a signed or unsigned integer of the width of the bitfield. In
particular, a signed bitfield of non-zero size contains a sign bit. When a
zero-sized bitfield is read, `0` is returned.

For the purpose of overflow checking, bitfields are treated as signed or
unsigned integers of their declared width. In this context, the only value that
should be written to a bitfield of size `0` is `0`.

# Drawbacks
[drawbacks]: #drawbacks

None.

# Alternatives
[alternatives]: #alternatives

Some have suggested using attributes at the top of the structure definition as
follows:

```rust
#[repr(C; bitfields(foo: 2, bar: 1))]
struct A {
    foo: c_uint,
    bar: c_uint
}
```

This is infeasible given the number of bitfields that can be contained in a
single structure.

# Unresolved questions
[unresolved]: #unresolved-questions

None come to mind.

# Appendix A

This appendix contains C code which should be compiled with either clang or gcc
for the x86-64 linux platform.

The basic setup is as follows:

```c
#include <stdio.h>

union {
 	struct {
 		unsigned char a: 1;
 		unsigned char b: 1;
 		unsigned char :0;
 		unsigned char c: 1;
 	} bf;
	unsigned short val[2];
} x;

int main(void) {
	x.bf.a = 1;
	x.bf.b = 1;
	x.bf.c = 1;

	printf("0x%04x_%04x\n", x.val[1], x.val[0]);
}
```

The following cases will modify the definition of the `x.bf` struct and show the
output of the program.

## Case 1

```c
 	struct {
 		unsigned char a: 1;
 		unsigned char b: 1;
 		unsigned char :0;
 		unsigned char c: 1;
 	} bf;
```

**Output:** `0x0000_0103`

This shows the behavior of fields of size `0`:

>As a special case, a bit-field structure member with a width of 0 indicates
>that no further bit-field is to be packed into the unit in which the previous
>bit-field, if any, was placed.

## Case 2

```c
	struct {
		unsigned char a: 4;
		unsigned char b: 4;
		unsigned char :0;
		unsigned char c: 1;
	} bf;
```

**Output:** `0x0000_0111`

This is the expected output.

## Case 3

```c
	struct {
		unsigned char a: 4;
		unsigned char b: 5;
		unsigned char :0;
		unsigned char c: 1;
	} bf;
```

**Output:** `0x0001_0101`

Here we see the behavior if two adjacent bitfields are too large for their
storage unit. The `b` field is moved entirely to the second byte. However, this
behavior is not mandated:

>If insufficient space remains, whether a bit-field that does not fit is put
>into the next unit or overlaps adjacent units is implementation-defined.

## Case 4

```c
	struct {
		unsigned char a: 4;
		unsigned short b: 5;
		unsigned char :0;
		unsigned char c: 1;
	} bf;
```

**Output:** `0x0001_0011`

Now that we've changed the type of `b` from `char` to `short`, `b` is placed in
the first byte. It is unexpected that declaring a larger type decreases the
position of the field in memory. The explanation is that `a` and `b` are
allocated in a single storage unit of size `2`, the size of the `short` type.
The size of the storage units allocated for bitfields is implementation defined.
It is perfectly legal for the following struct to occupy a megabyte of memory:

```c
struct {
    unsiged char: 1;
};
```

>An implementation may allocate any addressable storage unit large enough to
>hold a bit-field.

## Case 5

```c
	struct {
		unsigned char a: 1;
		unsigned char b: 1;
		unsigned char :0;
		unsigned short c: 8;
	} bf;
```

**Output:** `0x0000_0103`

Here we can see that declaring the end of a storage unit with a zero-sized field
does not imply that the following bitfield is properly aligned for its type.

## Case 6

```c
	struct {
		unsigned char a: 1;
		unsigned char b: 1;
		unsigned short :0;
		unsigned char c: 1;
	} bf;
```

**Output:** `0x0001_0003`

On the other hand, changing the type of the zero-sized field itself can change
the alignment of the following fields. This, too, is not required by the
standard:

>The alignment of the addressable storage unit is unspecified.

## Case 7

```c
	struct {
		unsigned char a: 1;
		unsigned char b: 1;
		unsigned char :0;
		unsigned short c: 9;
	} bf;
```

**Output:** `0x0001_0003`

This is similar to case 5, but see case 3.

## Case 8

```c
	struct {
		unsigned short a: 1;
		unsigned short b: 1;
		unsigned char :0;
		unsigned short c: 1;
	} bf;
```

**Output:** `0x0000_0103`

Another consequence of the size of the storage unit being unspecified. Even
though we request a new storage unit with a zero-sized field, and even though
all non-zero fields are `short`, the fields on both sides of the break are
packed into a single `short` storage unit.
