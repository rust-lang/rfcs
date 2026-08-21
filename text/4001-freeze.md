- Feature Name: `freeze`
- Start Date: 2026-08-19
- RFC PR: [rust-lang/rfcs#4001](https://github.com/rust-lang/rfcs/pull/4001)

## Summary
[summary]: #summary

Introduce an operation similar to the LLVM `freeze` instruction, which converts
uninitialized values into initialized but arbitrary values:

```rust
impl<T> MaybeUninit<T> {
    const fn freeze(self) -> MaybeUninit<T>;
}
```

## Motivation
[motivation]: #motivation

At the moment, it is not possible in Rust to observe bytes in uninitialized
memory without invoking Undefined Behavior (UB). The two most important places
where uninitialized memory appears in a Rust program are:

- In padding between fields in an aggregate type
- In buffers which are allocated but have not been initialized

On real hardware, loads from uninitialized memory return an arbitrary bit
pattern, but in the Abstract Machine that defines the formal semantics of Rust
programs, uninitialized memory is modeled as containing a special
"uninitialized" value, and it is UB to perform almost any operation with such
value. Even reading an uninitialized byte into a `u8` is UB, because Rust
integer types must contain initialized values.

(A better and more detailed explanation of the concept of undefined memory is
the [classic article "What The Hardware Does is not What Your Program
Does"][ralfj-uninit].)

[ralfj-uninit]: https://www.ralfj.de/blog/2019/07/14/uninit.html

However, it is sometimes useful to bypass this restriction and be able to read
values in uninitialized memory as arbitrary but defined values. 

### Use case 1: serialization by `memcpy`

If we have a struct that contains only simple value types, the cheapest way to
serialize the struct is to use the in-memory representation as-is. This is
sometimes referred to as "serialization by `memcpy`":

```rust
struct Order {
    order_id: u32,
    price: f64,
    qty: f64,
}

// convert `&Order` into a `&[u8]`, which we can then `memcpy` to an I/O buffer
fn serialize_order_unsound(order: &Order) -> &[u8] {
    let ptr = (order as *const Order).cast::<u8>();
    let len = mem::size_of::<Order>();
    // this is unsound!
    unsafe { slice::from_raw_parts(ptr, len) }
}
```

However, this is unsafe, because `Order` contains padding bytes which are not
initialized, and it is an immediate UB to create a `&[u8]` that points to
uninitialized bytes.

We can use `MaybeUninit<u8>` to make this sound:

```rust
fn serialize_order(order: &Order) -> &[MaybeUninit<u8>] {
    let ptr = (order as *const Order).cast::<MaybeUninit<u8>>();
    let len = mem::size_of::<Order>();
    unsafe { slice::from_raw_parts(ptr, len) }
}
```

This function is sound, but if we actually want to copy the returned
`&[MaybeUninit<u8>]` into a buffer that contains `u8`-s, such as a `Vec<u8>`,
`BytesMut` or `&mut [u8]`, so that we can write it to a file, we hit an impasse:
only initialized `MaybeUninit<u8>` can be turned into a `u8`.

#### Solution with `freeze`

With the `freeze()` function, we can freeze and copy the `MaybeUninit<u8>`
values into a `&mut [u8]`:

```rust
fn serialize_order_into(order: &Order, dst: &mut [u8]) {
    let ptr = (order as *const Order).cast::<MaybeUninit<u8>>();
    let len = mem::size_of::<Order>();
    let bytes: &[MaybeUninit<u8>] = unsafe { slice::from_raw_parts(ptr, len) };

    for i in 0..len {
        // we need `unsafe` because of `assume_init()`; it is safe because
        // every bit pattern possibly produced by `freeze()` is a valid `u8`
        dst[i] = unsafe { bytes[i].freeze().assume_init() };
    }
}
```

### Use case 2: masked reads

Suppose that we have a small struct with simple value types:

```rust
#[repr(C, align(8))]
struct Level {
    side: Side,
    price: u32,
}

#[repr(u8)]
enum Side {
    Bid,
    Ask,
}
```

We use this struct as a key in an important hash map, so we want to make `Eq`
and `Hash` as fast as possible. The default derived implementations of `Eq` and
`Hash` will read the `side` and `price` separately, so the compiler will have to
generate two memory loads. However, it is also possible to load both fields with
a single 64-bit load, mask out the padding between the two fields, and compute
the equality and hash using `u64`, [saving a few
instructions][godbolt-masked-reads]:

[godbolt-masked-reads]: https://godbolt.org/z/oxrq8o7n5

```rust
fn level_as_u64(level: &Level) -> u64 {
    let ptr = (level as *const Level).cast::<u64>();
    // load the value of the `Level` structure as a `u64`
    let loaded = unsafe { ptr.read() };
    // mask out the padding (assuming we are on a little-endian machine)
    loaded & 0xffffffff_000000ff
}

impl Hash for Level {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(level_as_u64(self));
    }
}

impl PartialEq for Level {
    fn eq(&self, other: &Level) -> bool {
        level_as_u64(self) == level_as_u64(other)
    }
}
impl Eq for Level {}
```

However, the `level_as_u64()` function is unsound, because even though we know
that `ptr` points to 8 bytes of allocated memory, some of the bytes are
uninitialized, so reading a `u64` from this memory is UB, even if we actually
mask out the padding bytes.

#### Solution with `freeze`

However, with the `freeze()` function, it is possible to implement this in a
safe way by loading the struct as `MaybeUninit<u64>` and then freezing it to
`u64`:

```rust
fn level_as_u64(level: &Level) -> u64 {
    let ptr = (level as *const Level).cast::<MaybeUninit<u64>>();
    // load the value of the `Level` structure as a `MaybeUninit<u64>`
    let loaded = unsafe { ptr.read() };
    // freeze the `MaybeUninit<u64>` and convert it to a `u64`
    let frozen = unsafe { loaded.freeze().assume_init() };
    // mask out the padding (assuming we are on a little-endian machine)
    frozen & 0xffffffff_000000ff
}
```

### Use case 3: safe access to C bitfields

Suppose that we have a C library that defines a struct with bitfields:

```c
typedef struct Order {
    int side: 1;
    int price: 13;
    int qty: 7;
} Order;

Order* alloc_order(int side, int price, int qty) {
    Order* o = (Order*)malloc(sizeof(Order));
    o->side = side;
    o->price = price;
    o->qty = qty;
    return o;
}
```

We can use this library from Rust as follows, using code similar to what
`bindgen` would generate:

```rust
#[repr(C, align(4))]
struct Order {
    bitfield: [u8; 3],
}

impl Order {
    fn qty(&self) -> i32 {
        ((self.bitfield[1] & 0xc0) >> 6) as i32 | ((self.bitfield[2] & 0x1f) << 2) as i32
    }
}

unsafe extern "C" {
    unsafe fn alloc_order(side: i32, price: i32, qty: i32) -> *mut Order;
}

fn main() {
    let order: *mut Order = unsafe { alloc_order(1, 1234, 56) };
    let qty = unsafe { &*order }.qty();
    assert_eq!(qty, 56);
}
```

The problem is, the C code in `alloc_order()` that initializes `Order` only
initializes the first 21 bits (bytes 0 and 1, and first 5 bits from byte 2).
This means that `Order::qty()` accesses a partially uninitialized byte
`self.bitfield[2]`.

In Rust semantics, a byte can never be partially initialized, and the machine
code for the C function `alloc_order()` will also always write byte 2 in full,
so we can consider this byte to be initialized both on the Rust level and on the
machine level.

However, on the LLVM IR level, values can be defined or undefined on the level
of individual bits, and if the program uses LTO, the LLVM IR from C and Rust
will be linked and optimized together, which will lead to undefined behavior on
the LLVM level.

#### Solution with `freeze`

Using `freeze()`, it is possible to implement the bitfields in a safe way:

```rust
#[repr(C, align(4))]
struct Order {
    bitfield: [MaybeUninit<u8>; 3],
}

impl Order {
    fn qty(&self) -> i32 {
        let byte_1 = unsafe { self.bitfield[1].freeze().assume_init() };
        let byte_2 = unsafe { self.bitfield[2].freeze().assume_init() };
        ((byte_1 & 0xc0) >> 6) as i32 | ((byte_2 & 0x1f) << 2) as i32
    }
}
```

### Use case 4: clever data structures

In safe Rust, every data structure that uses _O(N)_ memory also necessarily has
at least _O(N)_ time complexity, because all that memory needs to be
initialized. However, there are some clever data structures, for example [this
sparse set][sparse], which can store a set of _N_ integers in _O(N)_ memory
without needing to initialize that memory. Algorithms like these assume that
uninitialized memory can contain an arbitrary value, but we currently can't use
them in safe Rust, because uninitialized memory can't be read at all.

[sparse]: https://research.swtch.com/sparse

## Detailed design
[detailed-design]: #detailed-design

The proposal is to add this function into the standard library:

```rust
impl<T> MaybeUninit<T> {
    const fn freeze(self) -> MaybeUninit<T>;
}
```

The `freeze()` function replaces all uninitialized bytes in `self` with arbitrary
but initialized bytes, and it keeps initialized bytes unchanged. The return
value of `freeze` is also a `MaybeUninit<T>`, because even if all the bytes are
initialized, it does not mean that the bit pattern can be interpreted as a valid
value of type `T`.

In practice, the `MaybeUninit::freeze()` call will usually be immediately
followed by `MaybeUninit::assume_init()`, which will reinterpret the bytes as a
value of type `T`. For integers and floats this is always safe, because every
bit pattern is a valid value of these types.

Multiple calls of the `freeze()` function on the same value may return different
values of the uninitialized bytes, the argument to this function is not modified
and continues to be uninitialized. This means that we can't use this function to
turn a slice of uninitialized memory `&[MaybeUninit<u8>]` into a slice of
initialized memory `&[u8]` without actually writing initialized values into the
memory.

`freeze()` keeps the provenance associated with the initialized bytes, and the
arbitrary bytes that are created from uninitialized bytes are provenance-free.

If `T` contains any padding bytes, they are immediately turned into
uninitialized bytes when the `MaybeUninit<T>` is returned from `freeze()`.

### Implementation

The `freeze()` function will be implemented in terms of a `freeze()` intrinsic
added to the compiler:

```rust
fn freeze<T>(input: MaybeUninit<T>) -> MaybeUninit<T>;
```

This intrinsic will have to be implemented in each codegen backend separately.

#### LLVM

In LLVM, uninitialized memory is modeled by using [undefined values
(`undef`)][llvm-undef]. However, LLVM is moving away from using `undef`,
replacing it with `poison` and a ["byte" type][llvm-byte-type]. The byte type
can faithfully represent a piece of memory, including definedness and pointer
provenance, in an LLVM IR value. (Somewhat confusingly, the byte type can have
arbitrary width, not just 8 bits.)

(A more detailed introduction to the byte type and the problems that it solves
is the [report of the GSoC project that added it][llvm-gsoc-byte-type] and the
paper ["Towards Removing Undef Values from LLVM IR"][llvm-removing-undef].)

[llvm-undef]: https://llvm.org/docs/LangRef.html#undefined-values
[llvm-byte-type]: https://llvm.org/docs/LangRef.html#byte-type
[llvm-freeze]: https://llvm.org/docs/LangRef.html#freeze-instruction
[llvm-gsoc-byte-type]: https://blog.llvm.org/posts/2025-08-29-gsoc-byte-type/
[llvm-removing-undef]: https://web.ist.utl.pt/nuno.lopes/pubs.php?id=byte-type-pldi26

The properties of the byte type exactly match the semantics of Rust unions,
including `MaybeUninit<T>`, so the compiler should lower the Rust type
`MaybeUninit<T>` to the LLVM byte type of corresponding width. Lowering
`MaybeUninit<T>` to an integer type is not correct, because the information
about definedness and pointer provenance would not be preserved.

##### Scalars: `freeze` operation

For small types, which are represented as a single LLVM scalar or a pair of
scalars, the `freeze()` intrinsic can translate to a `freeze` instruction:

```llvm
; %input is a value of the byte type, representing the `input: MaybeUninit<T>`
%output = freeze bX %input
```

where `bX` refers to the byte type of corresponding width (e.g. `b32` for a
4-byte value).

##### Non-scalars: freezing `memcpy`

However, values of non-scalar types, such as `MaybeUninit<[u8; 1000]>` are not
represented as values in LLVM registers, but are stored in memory and passed as
pointers, even if they are passed by-value in Rust. To implement the `freeze()`
intrinsic for these types, we have to split the value into smaller pieces that
we can load from the input into registers, apply the `freeze` operation, and
store into the output.

One way to do it is to simply use the `load` and `store` instructions with an
appropriately sized byte type, and let LLVM split it into a sequence of smaller
loads and stores that the CPU can actually execute:

```llvm
; %input_ptr is a pointer from which we read the input
; %output_ptr is a pointer into which we write the output
%input = load bX, ptr %input_ptr
%output = freeze bX %input
store bX %output, %output_ptr
```

The problem with this approach is that the number of instructions that it
generates is proportional to the size of the type. For types that are larger
than some threshold, we need to emit a loop that does a sequence of `load`,
`freeze` and `store` operations for each register-sized chunk.

This is very similar to the `memcpy` operation, for which LLVM has a dedicated
intrinsic, `llvm.memcpy`. The best long-term solution is to add a variant of
`llvm.memcpy` that freezes the copied values into LLVM. There are at least two
options:

1. Add a new intrinsic (`llvm.memcpy.freeze`?)
2. Add an additional `isfreeze` argument to the existing `llvm.memcpy`
   instrinsic.

On the hardware level, "freezing `memcpy`" works exactly the same as normal
`memcpy`, so it may make more sense to go with the second option.

However, until LLVM adds support for this intrinsic, the compiler can generate a
call to a function that performs the copy, or a call to `memcpy()` (if LLVM can
be persuaded not to treat this as equivalent to the `llvm.memcpy` intrinsic).

##### LLVM 22

One issue with the byte type in LLVM is that it is only available starting from
LLVM 23. At the time of this writing (August 2026), [support for LLVM 23 has
just been merged][rustc-llvm-23-pr]. However, according to the rustc-dev-guide,
the compiler needs to support ["one or two preceding major
versions"][dev-guide-llvm] of LLVM, which suggests that to use features
introduced in LLVM 23, we would have to wait for LLVM versions 24 or 25, which
may take another year.

[rustc-llvm-23-pr]: https://github.com/rust-lang/rust/pull/158734
[dev-guide-llvm]: https://rustc-dev-guide.rust-lang.org/backend/updating-llvm.html

Fortunately, LLVM 22 already contains the `freeze` instruction, so the compiler
can just continue to use the integer type instead of the byte type to represent
`MaybeUninit<T>`.

#### Cranelift

Cranelift does not have any notion of undefined values in the IR registers; in
effect, all values loaded from memory are frozen. This means that the `freeze()`
intrinsic is an identity function: it translates to a no-op for types that are
represented as Cranelift scalars, and to a `memcpy` for types which are passed
in memory.

#### GCC

To be determined.

#### Const eval/Miri

The `freeze` intrinsic also needs to be implemented for constant evaluation and
inside Miri. It might be useful to replace the uninitialized bytes with
pseudo-random values, to help uncover bugs in programs that use `freeze()`.

## Drawbacks
[drawbacks]: #drawbacks

### Leaking secrets

> when considering the security of programs that do non-deterministic
> operations, you should assume the result to be either random or your secret
> key, depending on what is worse.
>
> -- [Ralf Jung][ralfj-saying]

[ralfj-saying]: https://github.com/rust-lang/rfcs/pull/3605#issuecomment-2050472692

The biggest disadvantage of adding the `freeze` operation is that **a Rust
program can leak a secret that was previously stored in the uninitialized memory
without triggering UB**.

In fact, the Use case 1 described above suffers from exactly this problem: it
includes uninitialized bytes in serialized values that are possibly sent over
the network, and these uninitialized bytes can contain secrets.

There has been a proposal in 2015 to [disallow reading from uninitialized
memory][rfc-no-uninit-read], even if it would not otherwise cause undefined
behavior. That RFC is in direct contradiction to this proposal.

[rfc-no-uninit-read]: https://github.com/rust-lang/rfcs/pull/837

#### Why `freeze` makes this problem worse

It is of course already possible to leak secrets by reading uninitialized memory
by using `unsafe`. However, at the moment this either requires inline assembly
or FFI, or it triggers UB if done from `unsafe` Rust, and there is a strong
community norm against causing UB (at least in libraries), and powerful tools
for detecting it (Miri, Valgrind, ...).

Once the `freeze` operation is added to the language, sound Rust code can leak
contents of uninitialized memory without using inline assembly or FFI and
without triggering UB, so neither the norm against UB nor the tooling will
protect against it. This is especially insiduous, because any `freeze` in the
whole program can realistically leak any secret processed by the program.

### False positives in Valgrind

The `freeze` operation does not compile to any machine code, so memory debuggers
like Valgrind won't see that an uninitialized value that the program loaded from
memory has been frozen, so they will report an error if the value is then used.
This may cause a large number of false positives in sound Rust programs.

## Rationale and alternative designs
[rationale-and-alternative-designs]: #rationale-and-alternative-designs

The `freeze()` function proposed in this RFC has this shape:

```rust
fn freeze<T>(input: MaybeUninit<T>) -> MaybeUninit<T>;
```

In particular:
- the input is `MaybeUninit<T>`
- input is taken by value (moved), not by reference
- the output is `MaybeUninit<T>`

In the following sections, each of these decisions is justified by comparing it
with alternative proposals.

### Read + freeze

One alternative is to read the input from a reference or a pointer, instead of
taking the input by value:

```rust
fn freeze<T>(input: &MaybeUninit<T>) -> MaybeUninit<T>;
fn read_freeze<T>(ptr: *const T) -> T;
```

The first disadvantage is that `MaybeUninit<T>` is `Copy` only if `T` is also
`Copy`. If we introduce an operation `&MaybeUninit<T> -> MaybeUninit<T>`, it
means that `MaybeUninit<T>` would in effect be copyable without using `unsafe`,
even if `T: !Copy`, removing one guardrail provided by `MaybeUninit`.

The second disadvantage is that this forces the compiler to emit LLVM stores and
loads even for scalar types, which could otherwise be frozen as-is. This will
make it slightly harder for LLVM to optimize the code.

### Freeze into `T`

Instead of returning a `MaybeUninit<T>`, the `freeze()` function can return `T`.
This is only valid if any bit pattern that can be produced by the freezing forms
a valid value of `T`; this can be either enforced with a trait bound (like
[`AnyBitPattern`][trait-any-bit-pattern] from `bytemuck`) or by making the
`freeze()` function `unsafe` and placing the burden of proof on the caller:

```rust
fn freeze<T>(input: MaybeUninit<T>) -> T where T: AnyBitPattern;
unsafe fn freeze<T>(input: MaybeUninit<T>) -> T;
```

[trait-any-bit-pattern]: https://docs.rs/bytemuck/latest/bytemuck/trait.AnyBitPattern.html

The first disadvantage is that this is strictly less powerful than returning
`MaybeUninit<T>`, because there can be valid use cases when the caller might
want to freeze a `MaybeUninit<T>` even if this does not immediately produce a
valid `T`: for example, if `T` is a struct, and some fields need to be "fixed
up" after freezing to produce a valid `T`.

Another disadvantage of the `T: AnyBitPattern` approach is that the caller might
know that some bytes in the input are already initialized, so freezing the input
will produce a valid `T` even if `T: !AnyBitPattern`. For example, if `input` is
a `MaybeUninit<char>`, the caller may know that the upper 3 bytes are
initialized to zero and only the least significant byte is uninitialized, so the
frozen value is always between 0 and 255 and thus it is a valid `char`, even
though not every 32-bit pattern is a valid `char` (and thus `char:
!AnyBitPattern`).

### Freeze into bytes

It has also been proposed that the `freeze()` function can return an array of
bytes:

```rust
fn freeze<T>(input: MaybeUninit<T>) -> [u8; mem::size_of::<T>()];
```

The first disadvantage is that if the caller wants to work with `T` instead of
`[u8]`, they would need to convert the bytes back into `T`, which adds
additional complexity and may lead to worse generated code.

The second disadvantage is that using const expressions like
`mem::size_if::<T>()` in function signatures is [unstable and
incomplete][feature-generic-const-exprs], so it is [not
possible][ralfj-should-not-be-used] to use it in the standard library at the
moment (let alone in the signature of an exported function).

[generic-const-exprs]: https://doc.rust-lang.org/beta/unstable-book/language-features/generic-const-exprs.html
[ralfj-should-not-be-used]: https://github.com/rust-lang/rfcs/pull/3605#discussion_r1553734874

The third disadvantage is that if `T` contains pointers or references, the
roundtrip through `[u8]` will remove their provenance (both on the Rust level
and on the LLVM level), so the pointers or references could not be dereferenced
even if the caller knows that they were initialized in the input.

### In-place freeze

Finally, there have been many proposals for an in-place freeze operation, which
replaces any possibly uninitialized bytes in memory with arbitrary but
initialized values.

```rust
fn freeze<T>(input: &MaybeUninit<T>) -> &MaybeUninit<T>;
fn freeze<T>(input: &MaybeUninit<T>) -> &[u8];
fn freeze<T>(target: &MaybeUninit<T>);
fn freeze<T>(target: &mut MaybeUninit<T>);
fn freeze<T>(target: &mut [MaybeUninit<T>]);
fn freeze<T>(target: &mut [MaybeUninit<u8>]) -> &mut [u8];
```

The canonical motivation for this operation is [`Read::read()`][read-read],
which needs a slice of initialized bytes (`&mut [u8]`), even if it never reads
those bytes and immediately rewrites them with the bytes produced by the reader.

[read-read]: https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read

The problem with this approach is that, as we will show in the next sections,
replacing uninitialized bytes with initialized bytes in memory needs to actually
overwrite the memory, both on the level of the Rust and LLVM Abstract Machines,
and on the level of actual hardware.

#### Rust semantics

In the Rust Abstract Machine, the in-place freeze [has to be modelled as a
write][ralfj-writes] because it modifies the state of the machine. In
particular, `&[MaybeUninit<u8>]` is `Send + Sync`, so multiple threads could do
an in-place freeze on this slice, and both would have to observe exactly the
same result without any synchronization between them. This means that an
in-place freeze operation needs a `&mut` reference.

[ralfj-writes]: https://github.com/rust-lang/rfcs/pull/3605#discussion_r1556391605

#### LLVM semantics

LLVM does not expose any instruction or instrinsic for in-place freezing, so the
in-place freeze would need to overwrite each byte in memory with the frozen
value. Even if LLVM had an in-place freeze intrinsic, it would need to be
modelled as a write in the LLVM semantics, for the same reasons as in Rust.

#### Real hardware

Finally, the in-place freeze needs to translate to actual memory writes even on
the level of actual hardware. Memory modelled as `MaybeUninit<u8>` may be
"uninitialized" on the hardware level, returning arbitrary values for each read
of a memory location until that location is initialized by writing the first
value into it.

The most common example of this behavior is `MADV_FREE`, an operation performed
by the [`madvise()`][man-madvise] syscall on Linux. When a memory page is marked
with `MADV_FREE`, the kernel can reclaim it any time, until the userland process
writes into it again. From the point of view of the process, it appears as if
the page is replaced with all zeros at some point.

[man-madvise]: https://man7.org/linux/man-pages/man2/madvise.2.html

Applying `MADV_FREE` on memory used by a Rust data structure is unsafe and UB,
but some memory allocators (e.g. jemalloc) can return memory allocated from
pages with the `MADV_FREE` flag set. This memory is modeled as uninitialized in
Rust, so without the presence of in-place freeze, this is actually safe.

For this reason, to account for `MADV_FREE`-d memory, an in-place freeze on a
Linux system must always write at least one byte to every page in the frozen
range.

However, it would be unwise to assume that `MADV_FREE` is the only situation in
which uninitialized memory exists on the hardware level. It is likely that there
are other circumstances where this may arise: for example, by using
[`userfaultfd` in Linux][userfaultfd], with [mismatched memory attributes on
ARM][arm-mma], or when the program is running under a memory sanitizer like
Valgrind.

[userfaultfd]: https://docs.kernel.org/admin-guide/mm/userfaultfd.html
[arm-mma]: https://support.arm.com/documentation/ddi0487/mb/-Part-B-The-AArch64-Application-Level-Architecture/-Chapter-B2-The-AArch64-Application-Level-Memory-Model/-B2-11-Mismatched-memory-attributes

For this reason, a cautious and future-proof implementation of in-place freeze
should read and write back each byte to guarantee that the uninitialized memory
is properly initialized.

However, once the `freeze()` operation is added to the language, users will be
able to safely write in-place freeze that does not overwrite every byte in the
frozen range, if they can guarantee that a smaller number of writes is
sufficient to turn the memory into arbitrary but initialized state. For example,
on a Linux system, they may write inline assembly that reads and writes just a
single byte per page (to account for `MADV_FREE` and other page-level
mechanisms), and the "story" of this inline assembly can be a loop that
reads, `freeze`-s and writes every byte in the range.

### Naming: `freeze` vs `frozen`

The method name `MaybeUninit<T>::freeze()` can look like it mutates the
receiver, so a better name might be `MaybeUninit<T>::frozen()`, which makes it
more clear that only the returned value is frozen, not the receiver. `frozen()`
would also be consistent with the existing `zeroed()` method.

However, the name `freeze` for this operation is already well established by the
LLVM instruction, so this RFC proposes to use `freeze` instead of `frozen`.

### Can we just use a volatile read?

The compiler must translate volatile reads into actual memory loads, it cannot
elide or reorder them, because a volatile read may have external side effects
(like reading from a memory-mapped hardware device).

However, volatile reads that are inside a Rust allocation (memory accessible to
a Rust program), still need to [obey the same rules][read-volatile] as normal
reads. In particular, the read must still produce a properly initialized value:
reading a `u8` from uninitialized memory with a volatile read is UB, so a
volatile read cannot simulate a `freeze`.

[read-volatile]: https://doc.rust-lang.org/std/ptr/fn.read_volatile.html

### Can we just use FFI or inline assembly?

Similarly, it is not sound to use FFI (or inline assembly) to "hide" the fact
that memory might be uninitialized from the compiler, and implement the `freeze`
operation like this:

```c
// C
int not_really_freeze(int input) {
    return input;
}
```

```rust
// Rust
unsafe extern "C" {
    safe fn not_really_freeze(input: MaybeUninit<i32>) -> i32;
}
```

While the exact interaction of FFI with the Rust Abstract Machine is not yet
defined, the [current thinking][ralfj-inline-asm] is that a FFI call is sound
only if its behavior can be expressed as a "story", a Rust code that gives the
same observable behavior as the call.

[ralfj-inline-asm]: https://www.ralfj.de/blog/2026/03/13/inline-asm.html

Without the `freeze` operation, there is no valid story for the
`not_really_freeze()` function: the story could be that the function returns an
arbitrary initialized value, but then the Rust code could not assume any
relation between the input and output, even if the input happens to be
initialized.

### Freeze granularity

A Rust program can never produce a partially initialized byte: a byte is either
initialized, with a value between 0 and 255, or it is uninitialized. However, in
LLVM, initialization is tracked per bit: each bit of a byte type is either a 0,
1 or poison. Other languages compiled to LLVM IR have features that allow them
to initialize only some bits in a byte; one example is C bitfields (see [the use
case above][use-case-c-bitfields]). This means that a Rust program that is
compiled with LTO can observe partially uninitialized bytes, even if Rust can
never produce them.

For this reason, we should guarantee that the `freeze()` function will be
lowered into the `freeze` instruction in LLVM, at least for scalar types, so
that code that interacts with C bitfields can rely on `freeze()` preserving
values of initialized bits in partly uninitialized bits.

## Prior art
[prior-art]: #prior-art

It seems that no other language (C++, Zig, ...) provides anything similar to the
`freeze` operation. There have been many discussions about adding the `freeze`
operation to Rust:

- [RFC: Add `freeze` intrinsic and related library
  functions](https://github.com/rust-lang/rfcs/pull/3605)
- [IRLO: `freeze(MaybeUninit<T>) -> MaybeUninit<T>` for masked
  reads](https://internals.rust-lang.org/t/freeze-maybeuninit-t-maybeuninit-t-for-masked-reads/17188)
- [URLO: Is it possible to read uninitialized memory without invoking
  UB?](https://users.rust-lang.org/t/is-it-possible-to-read-uninitialized-memory-without-invoking-ub/63092)
- [IRLO: API to acquire arbitrarily initialized
  buffer?](https://internals.rust-lang.org/t/api-to-acquire-arbitrarily-initialized-buffer/13213)
- [IRLO: WHAT-IF: Reading uninit RAM was not
  UB](https://internals.rust-lang.org/t/what-if-reading-uninit-ram-was-not-ub/13231)

Other related discussions were:

- [RFC: Never allow reads from uninitialized memory in safe
  Rust](https://github.com/rust-lang/rfcs/pull/837): this is the exact opposite
  of this RFC, mandates that observing values of uninitialized memory is always
  UB. This RFC was not accepted.
- [Unsafe code guidelines: What are the values of a union
  type?](https://github.com/rust-lang/unsafe-code-guidelines/issues/438):
  discussion that ended in a team decision that unions have no validity
  invariants, which means that unions (and in particular `MaybeUninit<T>`) have
  no validity invariants and can be treated as a sequence of potentially
  uninitialized bits, possibly carrying provenance.
- [Zig proposal: Eliminate uninitialized
  memory](https://github.com/ziglang/zig/issues/7115): proposes to remove the
  concept of uninitialized memory from Zig.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are the security holes opened by `freeze()` worth the benefits?
- How to implement freezing `memcpy` to freeze non-scalar values?

## Future possibilities
[future-possibilities]: #future-possibilities

- Expose the freezing `memcpy` as a method that combines `freeze()` and
  `copy_from_slice()`:

    ```rust
    impl<T> [MaybeUninit<T>] {
        const fn freeze_copy_from_slice(&mut self, src: &[MaybeUninit<T>])
            where T: Copy
        {
            assert_eq!(self.len(), src.len());
            for i in 0..self.len() {
                self[i] = src[i].freeze();
            }
        }
    }
    ```

- Add an operation that combines the `freeze()` + `assume_init()` for types for
  which every bit pattern is valid:

    ```rust
    impl<T> MaybeUninit<T> {
        const fn freeze_init(self) -> T where T: AnyBitPattern {
            // SAFETY: this is safe because the `T: AnyBitPattern` ensures that
            the result of `self.freeze()` is a valid `T`
            unsafe { self.freeze().assume_init() }
        }
    }
    ```

  However, this would require a new trait in the standard library, so this might
  better be placed into [`bytemuck`][crate-bytemuck]

  [crate-bytemuck]: https://docs.rs/bytemuck/latest/bytemuck/index.html

- Add an operation that combines a read (bitwise copy) and `freeze()`, similar
  to `MaybeUninit<T>::assume_init_read()`:

    ```rust
    impl<T> MaybeUninit<T> {
        const fn freeze_read(&self) -> MaybeUninit<T> {
            unsafe { (self as *const MaybeUninit<T>).read().freeze() }
        }
    }
    ```

  Note that similar to `MaybeUninit<T>::assume_init_read()`, this performs a
  bitwise copy even if `T` is not `Copy`.
